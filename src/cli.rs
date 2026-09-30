//! The command line (spec 001 M1, M5 criterion 8): both commands accept the same files and
//! flags.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Measures how hard Markdown prose is to read.
#[derive(Debug, Parser)]
#[command(name = "vernier", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Report the metrics of each file; exits 0, or 2 on an error.
    Analyze(Args),
    /// Lint each file; exits 1 when a sentence is flagged, 2 on an error.
    Check(Args),
}

/// The files and thresholds shared by both commands.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// Markdown (`.md`, `.markdown`) or plain-text files.
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
    /// Flag a sentence with more words than this (LongSentence).
    #[arg(long, default_value_t = 25)]
    pub max_sentence_len: usize,
    /// Flag a sentence whose mean dependency distance exceeds this (HighMdd).
    #[arg(long, default_value_t = 3.0, value_parser = parse_max_mdd)]
    pub max_mdd: f64,
    /// Flag a sentence whose dependency tree is deeper than this, counting edges from a projected root (DeepTree).
    #[arg(long, default_value_t = 5)]
    pub max_tree_depth: usize,
    /// Flag a sentence with more subordinate clauses than this (ClauseOverload).
    #[arg(long, default_value_t = 2)]
    pub max_clauses: usize,
    /// A Universal Dependencies model for the syntactic metrics.
    #[arg(long)]
    pub model_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    Compact,
}

/// A mean-dependency-distance threshold: a finite number above 0.
fn parse_max_mdd(raw: &str) -> Result<f64, String> {
    let value: f64 = raw
        .parse()
        .map_err(|_| format!("`{raw}` is not a number"))?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err(format!("`{raw}` must be a finite number above 0"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(std::iter::once("vernier").chain(args.iter().copied()))
    }

    fn args_of(cli: Cli) -> Args {
        match cli.command {
            Command::Analyze(args) | Command::Check(args) => args,
        }
    }

    /// Given every flag of M5
    /// When `analyze` and `check` are parsed with them
    /// Then both accept them and keep their values
    #[test]
    fn accepts_every_m5_flag_on_both_commands() {
        for command in ["analyze", "check"] {
            let cli = parse(&[
                command,
                "--format",
                "json",
                "--max-sentence-len",
                "30",
                "--max-mdd",
                "2.5",
                "--max-tree-depth",
                "7",
                "--max-clauses",
                "4",
                "--model-path",
                "m.udpipe",
                "a.md",
                "b.txt",
            ])
            .unwrap();
            let args = args_of(cli);
            assert_eq!(args.files, [PathBuf::from("a.md"), PathBuf::from("b.txt")]);
            assert_eq!(args.format, OutputFormat::Json);
            assert_eq!(
                (
                    args.max_sentence_len,
                    args.max_mdd,
                    args.max_tree_depth,
                    args.max_clauses
                ),
                (30, 2.5, 7, 4)
            );
            assert_eq!(args.model_path, Some(PathBuf::from("m.udpipe")));
        }
    }

    /// Given no flags
    /// When a command is parsed
    /// Then the thresholds are the spec's defaults and no model is set
    #[test]
    fn defaults_match_the_spec() {
        let args = args_of(parse(&["check", "a.md"]).unwrap());
        assert_eq!(args.format, OutputFormat::Text);
        assert_eq!(
            (
                args.max_sentence_len,
                args.max_mdd,
                args.max_tree_depth,
                args.max_clauses
            ),
            (25, 3.0, 5, 2)
        );
        assert_eq!(args.model_path, None);
    }

    /// Given each output format name
    /// When `--format` is parsed
    /// Then text, json and compact are accepted and anything else is refused
    #[test]
    fn accepts_exactly_the_three_formats() {
        for (name, format) in [
            ("text", OutputFormat::Text),
            ("json", OutputFormat::Json),
            ("compact", OutputFormat::Compact),
        ] {
            assert_eq!(
                args_of(parse(&["analyze", "--format", name, "a.md"]).unwrap()).format,
                format
            );
        }
        assert!(parse(&["analyze", "--format", "xml", "a.md"]).is_err());
    }

    /// Given a non-positive or non-finite `--max-mdd`
    /// When it is parsed
    /// Then it is refused, and any finite positive value is kept
    #[test]
    fn rejects_a_non_positive_or_non_finite_max_mdd() {
        for bad in ["0", "-1", "NaN", "inf", "-inf", "three"] {
            assert!(
                parse(&["check", "--max-mdd", bad, "a.md"]).is_err(),
                "{bad}"
            );
        }
        for good in ["0.5", "3", "12.25"] {
            let args = args_of(parse(&["check", "--max-mdd", good, "a.md"]).unwrap());
            assert_eq!(
                args.max_mdd.to_string(),
                good.parse::<f64>().unwrap().to_string()
            );
        }
    }

    /// Given the rendered help of both commands
    /// When the `--max-tree-depth` help text is read
    /// Then it says that depth counts edges from a projected root (spec 001 M3a criterion 4)
    #[test]
    fn max_tree_depth_help_says_edges() {
        use clap::CommandFactory;
        let cli = Cli::command();
        for command in ["analyze", "check"] {
            let help = cli
                .find_subcommand(command)
                .unwrap()
                .get_arguments()
                .find(|arg| arg.get_id() == "max_tree_depth")
                .and_then(|arg| arg.get_help())
                .map(ToString::to_string)
                .unwrap_or_default();
            assert!(help.contains("edges"), "{command}: {help:?}");
            assert!(
                help.contains("from a projected root"),
                "{command}: {help:?}"
            );
        }
    }

    /// Given the command list
    /// When the `analyze` summary is read
    /// Then it does not promise exit 0, which an unparsable sentence or a failed write breaks
    /// (spec 001 M5 criterion 2)
    #[test]
    fn analyze_help_names_exit_2() {
        use clap::CommandFactory;
        let about = Cli::command()
            .find_subcommand("analyze")
            .and_then(|c| c.get_about())
            .map(ToString::to_string)
            .unwrap_or_default();
        assert!(!about.contains("always"), "{about:?}");
        assert!(about.contains("2 on an error"), "{about:?}");
    }

    /// Given a command without files
    /// When it is parsed
    /// Then it is refused
    #[test]
    fn requires_at_least_one_file() {
        assert!(parse(&["analyze"]).is_err());
    }
}
