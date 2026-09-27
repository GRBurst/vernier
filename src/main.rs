//! vernier — readability and syntactic-complexity analyzer for Markdown.
//!
//! The imperative shell: read files, call the pure core, print, and map the outcome to an exit code.

use std::io::IsTerminal;
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use vernier::analysis::{self, Thresholds};
use vernier::cli::{Args, Cli, Command, OutputFormat};
use vernier::diagnostic::{self, Diagnostic, Style};
use vernier::prose::SourceFormat;
use vernier::summary::{render, summarize};

/// Exit code when a sentence is flagged (spec 001 M5).
const EXIT_FLAGGED: u8 = 1;
/// Exit code for an unreadable file or an unusable model (spec 001 M1, M5).
const EXIT_UNREADABLE: u8 = 2;

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Analyze(args) => run(&args, analyze),
        Command::Check(args) => run(&args, check),
    }
}

/// What became of one file named on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileOutcome {
    Clean,
    Flagged,
    Unreadable,
}

/// What `each` does with one readable file.
type Each = fn(&Args, &Path, &str) -> FileOutcome;

/// Processes every file with `each`, then exits 2 if any file was unreadable, else 1 if any
/// sentence was flagged, else 0.
fn run(args: &Args, each: Each) -> ExitCode {
    let outcomes: Vec<FileOutcome> = args
        .files
        .iter()
        .map(|path| process(args, path, each))
        .collect();
    if outcomes.contains(&FileOutcome::Unreadable) {
        ExitCode::from(EXIT_UNREADABLE)
    } else if outcomes.contains(&FileOutcome::Flagged) {
        ExitCode::from(EXIT_FLAGGED)
    } else {
        ExitCode::SUCCESS
    }
}

/// Reads one file and hands it to `each`, or names it on stderr when it cannot be read (or is not UTF-8).
fn process(args: &Args, path: &Path, each: Each) -> FileOutcome {
    match std::fs::read_to_string(path) {
        Ok(source) => each(args, path, &source),
        Err(err) => {
            eprintln!("vernier: cannot read {}: {err}", path.display());
            FileOutcome::Unreadable
        }
    }
}

fn analyze(_args: &Args, path: &Path, source: &str) -> FileOutcome {
    println!(
        "{}",
        render(path, &summarize(source, SourceFormat::from_path(path)))
    );
    FileOutcome::Clean
}

/// Prints the diagnostics of one file in the chosen format; `json` prints per-flag lines until
/// M5 T9 renders it.
fn check(args: &Args, path: &Path, source: &str) -> FileOutcome {
    let thresholds = Thresholds {
        max_sentence_len: args.max_sentence_len,
        max_mdd: args.max_mdd,
        max_tree_depth: args.max_tree_depth,
        max_clauses: args.max_clauses,
    };
    let analysis = analysis::analyze(source, SourceFormat::from_path(path), &thresholds);
    match diagnostic::diagnostics(source, &analysis, &thresholds) {
        Ok(found) if found.is_empty() => FileOutcome::Clean,
        Ok(found) => {
            rendered(args.format, path, source, &found)
                .iter()
                .for_each(|block| println!("{block}"));
            FileOutcome::Flagged
        }
        Err(err) => {
            eprintln!(
                "vernier: cannot locate a sentence in {}: {err}",
                path.display()
            );
            FileOutcome::Unreadable
        }
    }
}

/// What `check` prints for the diagnostics of one file: one block or line per diagnostic.
fn rendered(format: OutputFormat, path: &Path, source: &str, found: &[Diagnostic]) -> Vec<String> {
    let path = path.display().to_string();
    match format {
        OutputFormat::Text => {
            let style = Style::for_output(
                std::io::stdout().is_terminal(),
                std::env::var_os("NO_COLOR").as_deref(),
            );
            found
                .iter()
                .map(|d| format!("{}\n", diagnostic::render_text(&path, source, d, style)))
                .collect()
        }
        OutputFormat::Compact => found
            .iter()
            .map(|d| diagnostic::render_compact(&path, d))
            .collect(),
        OutputFormat::Json => per_flag_lines(&path, found),
    }
}

/// One line per flag of every diagnostic, at the position of its sentence's first character.
fn per_flag_lines(path: &str, found: &[Diagnostic]) -> Vec<String> {
    found
        .iter()
        .flat_map(|d| {
            d.flags
                .iter()
                .map(move |flag| format!("{path}:{}:{}: {flag}", d.start.line(), d.start.column()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use vernier::analysis::{FileAnalysis, Flag, SentenceAnalysis, SyntacticFlag};

    const THRESHOLDS: Thresholds = Thresholds {
        max_sentence_len: 25,
        max_mdd: 3.0,
        max_tree_depth: 5,
        max_clauses: 2,
    };

    fn sentence(
        source_range: std::ops::Range<usize>,
        words: usize,
        flags: Vec<Flag>,
        syntactic: Vec<SyntacticFlag>,
    ) -> SentenceAnalysis {
        let metrics = vernier::syntax::SyntacticMetrics {
            distance: vernier::syntax::DependencyDistance::default(),
            depth: 0,
            clauses: 0,
            center_embeddings: Vec::new(),
        };
        SentenceAnalysis {
            source_range,
            counts: vernier::readability::SurfaceCounts {
                words,
                sentences: 1,
                ..Default::default()
            },
            readability: None,
            flags,
            syntax: Some(analysis::SentenceSyntax {
                metrics,
                flags: syntactic,
                passives: Vec::new(),
            }),
            nominalizations: vernier::nominalization::NominalizationCount::default(),
        }
    }

    /// Given two sentences, the first flagged LongSentence and DeepTree, the second HighMdd
    /// When the diagnostics are rendered
    /// Then each sentence's surface flags come before its syntactic flags, in source order, each
    /// at the sentence's first character
    #[test]
    fn diagnostics_list_surface_then_syntactic_flags_per_sentence() {
        let source = "First sentence.\nSecond one.\n";
        let analysis = FileAnalysis {
            spans: 2,
            sentences: vec![
                sentence(
                    0..15,
                    30,
                    vec![Flag::LongSentence],
                    vec![SyntacticFlag::DeepTree { depth: 6 }],
                ),
                sentence(16..27, 2, vec![], vec![SyntacticFlag::HighMdd { mdd: 3.5 }]),
            ],
            totals: vernier::readability::SurfaceCounts::default(),
            readability: None,
            dependency_distance: None,
            nominalizations: vernier::nominalization::NominalizationCount::default(),
            passives: None,
        };
        let found = diagnostic::diagnostics(source, &analysis, &THRESHOLDS).unwrap();
        let lines = per_flag_lines("f.md", &found);
        assert_eq!(
            lines,
            [
                "f.md:1:1: LongSentence: sentence has 30 words (max 25)",
                "f.md:1:1: DeepTree: dependency tree depth 6 edges (max 5)",
                "f.md:2:1: HighMdd: mean dependency distance 3.50 (max 3.00)",
            ]
        );
    }

    #[test]
    fn version_is_semver_shaped() {
        let parts: Vec<&str> = env!("CARGO_PKG_VERSION").split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
    }
}
