//! vernier — readability and syntactic-complexity analyzer for Markdown.
//!
//! The imperative shell: read files, call the pure core, print, and map the outcome to an exit code.

use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use vernier::analysis::{self, FileAnalysis, Flag, SentenceAnalysis, SyntacticFlag, Thresholds};
use vernier::cli::{Args, Cli, Command};
use vernier::position::{LineIndex, PositionError};
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

/// Prints one `path:line:col: Flag: message` line per flag; `--format` is accepted and ignored
/// until M5 renders it.
fn check(args: &Args, path: &Path, source: &str) -> FileOutcome {
    let thresholds = Thresholds {
        max_sentence_len: args.max_sentence_len,
        max_mdd: args.max_mdd,
        max_tree_depth: args.max_tree_depth,
        max_clauses: args.max_clauses,
    };
    let analysis = analysis::analyze(source, SourceFormat::from_path(path), &thresholds);
    match diagnostics(path, source, &analysis, &thresholds) {
        Ok(lines) if lines.is_empty() => FileOutcome::Clean,
        Ok(lines) => {
            lines.iter().for_each(|line| println!("{line}"));
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

/// One line per flag of every sentence, at the position of the sentence's first character: each
/// sentence's surface flags, then its syntactic flags.
fn diagnostics(
    path: &Path,
    source: &str,
    analysis: &FileAnalysis,
    thresholds: &Thresholds,
) -> Result<Vec<String>, PositionError> {
    let index = LineIndex::new(source);
    analysis
        .sentences
        .iter()
        .flat_map(|sentence| messages(sentence, thresholds).map(move |message| (sentence, message)))
        .map(|(sentence, message)| {
            let at = index.position(sentence.source_range.start)?;
            Ok(format!(
                "{}:{}:{}: {message}",
                path.display(),
                at.line(),
                at.column(),
            ))
        })
        .collect()
}

/// The message of each of a sentence's flags, surface flags first.
fn messages<'a>(
    sentence: &'a SentenceAnalysis,
    thresholds: &'a Thresholds,
) -> impl Iterator<Item = String> + 'a {
    let surface = sentence
        .flags
        .iter()
        .map(move |flag| describe(*flag, sentence, thresholds));
    let syntactic = sentence
        .syntax
        .iter()
        .flat_map(|syntax| syntax.flags.iter())
        .map(move |flag| describe_syntactic(flag, thresholds));
    surface.chain(syntactic)
}

fn describe(flag: Flag, sentence: &SentenceAnalysis, thresholds: &Thresholds) -> String {
    match flag {
        Flag::LongSentence => format!(
            "LongSentence: sentence has {} words (max {})",
            sentence.counts.words, thresholds.max_sentence_len
        ),
    }
}

fn describe_syntactic(flag: &SyntacticFlag, thresholds: &Thresholds) -> String {
    match flag {
        SyntacticFlag::HighMdd { mdd } => format!(
            "HighMdd: mean dependency distance {mdd:.2} (max {:.2})",
            thresholds.max_mdd
        ),
        SyntacticFlag::DeepTree { depth } => format!(
            "DeepTree: dependency tree depth {depth} edges (max {})",
            thresholds.max_tree_depth
        ),
        SyntacticFlag::ClauseOverload { clauses } => format!(
            "ClauseOverload: {clauses} subordinate clauses (max {})",
            thresholds.max_clauses
        ),
        SyntacticFlag::CenterEmbedding(e) => format!(
            "CenterEmbedding: subject \"{}\" separated from verb \"{}\" by {} words",
            e.subject, e.verb, e.words_between
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vernier::syntax::CenterEmbedding;

    const THRESHOLDS: Thresholds = Thresholds {
        max_sentence_len: 25,
        max_mdd: 3.0,
        max_tree_depth: 5,
        max_clauses: 2,
    };

    /// Given each syntactic flag with its measured value
    /// When it is described
    /// Then the message names the flag, the value and, for a threshold rule, the maximum
    #[test]
    fn describes_each_syntactic_flag_with_its_value() {
        let embedding = CenterEmbedding {
            subject: "proposal".to_owned(),
            verb: "caused".to_owned(),
            words_between: 8,
        };
        let cases = [
            (
                SyntacticFlag::HighMdd { mdd: 32.0 / 12.0 },
                "HighMdd: mean dependency distance 2.67 (max 3.00)",
            ),
            (
                SyntacticFlag::DeepTree { depth: 6 },
                "DeepTree: dependency tree depth 6 edges (max 5)",
            ),
            (
                SyntacticFlag::ClauseOverload { clauses: 3 },
                "ClauseOverload: 3 subordinate clauses (max 2)",
            ),
            (
                SyntacticFlag::CenterEmbedding(embedding),
                "CenterEmbedding: subject \"proposal\" separated from verb \"caused\" by 8 words",
            ),
        ];
        for (flag, expected) in cases {
            assert_eq!(describe_syntactic(&flag, &THRESHOLDS), expected);
        }
    }

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
        let lines = diagnostics(Path::new("f.md"), source, &analysis, &THRESHOLDS).unwrap();
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
