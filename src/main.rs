//! vernier — readability and syntactic-complexity analyzer for Markdown.
//!
//! The imperative shell: read files, call the pure core, print, and map the outcome to an exit code.

use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use vernier::analysis::{self, FileAnalysis, Flag, SentenceAnalysis, Thresholds};
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

/// One line per flag of every sentence, at the position of the sentence's first character.
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
        .flat_map(|sentence| sentence.flags.iter().map(move |flag| (sentence, *flag)))
        .map(|(sentence, flag)| {
            let at = index.position(sentence.source_range.start)?;
            Ok(format!(
                "{}:{}:{}: {}",
                path.display(),
                at.line(),
                at.column(),
                describe(flag, sentence, thresholds)
            ))
        })
        .collect()
}

fn describe(flag: Flag, sentence: &SentenceAnalysis, thresholds: &Thresholds) -> String {
    match flag {
        Flag::LongSentence => format!(
            "LongSentence: sentence has {} words (max {})",
            sentence.counts.words, thresholds.max_sentence_len
        ),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_semver_shaped() {
        let parts: Vec<&str> = env!("CARGO_PKG_VERSION").split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
    }
}
