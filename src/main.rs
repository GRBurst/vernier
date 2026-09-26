//! vernier — readability and syntactic-complexity analyzer for Markdown.
//!
//! The imperative shell: read files, call the pure core, print, and map the outcome to an exit code.

use std::path::Path;
use std::process::ExitCode;

use clap::Parser;
use vernier::cli::{Args, Cli, Command};
use vernier::prose::SourceFormat;
use vernier::summary::{render, summarize};

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
    Read,
    Unreadable,
}

/// Processes every file with `each`, then exits 2 if any file was unreadable, else 0.
fn run(args: &Args, each: fn(&Path, &str)) -> ExitCode {
    let outcomes: Vec<FileOutcome> = args.files.iter().map(|path| process(path, each)).collect();
    if outcomes.contains(&FileOutcome::Unreadable) {
        ExitCode::from(EXIT_UNREADABLE)
    } else {
        ExitCode::SUCCESS
    }
}

/// Reads one file and hands it to `each`, or names it on stderr when it cannot be read (or is not UTF-8).
fn process(path: &Path, each: fn(&Path, &str)) -> FileOutcome {
    match std::fs::read_to_string(path) {
        Ok(source) => {
            each(path, &source);
            FileOutcome::Read
        }
        Err(err) => {
            eprintln!("vernier: cannot read {}: {err}", path.display());
            FileOutcome::Unreadable
        }
    }
}

fn analyze(path: &Path, source: &str) {
    println!(
        "{}",
        render(path, &summarize(source, SourceFormat::from_path(path)))
    );
}

/// No rule exists before M2, so a readable file is never flagged.
fn check(_path: &Path, _source: &str) {}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_semver_shaped() {
        let parts: Vec<&str> = env!("CARGO_PKG_VERSION").split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
    }
}
