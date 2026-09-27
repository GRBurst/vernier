//! vernier — readability and syntactic-complexity analyzer for Markdown.
//!
//! The imperative shell: read files, call the pure core, print, and map the outcome to an exit code.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use vernier::analysis::{self, Thresholds};
use vernier::cli::{Args, Cli, Command, OutputFormat};
use vernier::diagnostic::{self, Diagnostic, Style};
use vernier::json::{self, FileReport};
use vernier::position::PositionError;
use vernier::prose::SourceFormat;
use vernier::summary::{FileSummary, render, summarize};

/// Exit code when a sentence is flagged (spec 001 M5).
const EXIT_FLAGGED: u8 = 1;
/// Exit code for an unreadable file or an unusable model (spec 001 M1, M5).
const EXIT_UNREADABLE: u8 = 2;

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Analyze(args) => run(&args, Mode::Analyze),
        Command::Check(args) => run(&args, Mode::Check),
    }
}

/// Which command runs: `analyze` reports, `check` lints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Analyze,
    Check,
}

/// What became of one file named on the command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileOutcome {
    Clean,
    Flagged,
    Unreadable,
}

/// One readable file's summary and diagnostics.
struct Report {
    path: PathBuf,
    summary: FileSummary,
    diagnostics: Vec<Diagnostic>,
}

/// Processes every file, prints the JSON document after the last one when `--format json`, then
/// exits 2 if any file was unreadable (or the document could not be written), else 1 if `check`
/// flagged a sentence, else 0.
fn run(args: &Args, mode: Mode) -> ExitCode {
    let mut reports = Vec::new();
    let mut outcomes: Vec<FileOutcome> = args
        .files
        .iter()
        .map(|path| process(args, mode, path, &mut reports))
        .collect();
    if args.format == OutputFormat::Json && !print_document(&reports) {
        outcomes.push(FileOutcome::Unreadable);
    }
    if outcomes.contains(&FileOutcome::Unreadable) {
        ExitCode::from(EXIT_UNREADABLE)
    } else if outcomes.contains(&FileOutcome::Flagged) {
        ExitCode::from(EXIT_FLAGGED)
    } else {
        ExitCode::SUCCESS
    }
}

/// Reads and reports one file, or names it on stderr when it cannot be read (or is not UTF-8).
/// Its report is printed now, or kept in `reports` for the JSON document.
fn process(args: &Args, mode: Mode, path: &Path, reports: &mut Vec<Report>) -> FileOutcome {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("vernier: cannot read {}: {err}", path.display());
            return FileOutcome::Unreadable;
        }
    };
    let report = match examine(args, path, &source) {
        Ok(report) => report,
        Err(err) => {
            eprintln!(
                "vernier: cannot locate a sentence in {}: {err}",
                path.display()
            );
            return FileOutcome::Unreadable;
        }
    };
    let outcome = match mode {
        Mode::Check if !report.diagnostics.is_empty() => FileOutcome::Flagged,
        Mode::Analyze | Mode::Check => FileOutcome::Clean,
    };
    match args.format {
        OutputFormat::Json => reports.push(report),
        OutputFormat::Text | OutputFormat::Compact => {
            print_report(args.format, mode, &source, &report)
        }
    }
    outcome
}

/// The summary and the diagnostics of one file under the command line's thresholds.
fn examine(args: &Args, path: &Path, source: &str) -> Result<Report, PositionError> {
    let thresholds = Thresholds {
        max_sentence_len: args.max_sentence_len,
        max_mdd: args.max_mdd,
        max_tree_depth: args.max_tree_depth,
        max_clauses: args.max_clauses,
    };
    let format = SourceFormat::from_path(path);
    let analysis = analysis::analyze(source, format, &thresholds);
    Ok(Report {
        path: path.to_owned(),
        summary: summarize(source, format),
        diagnostics: diagnostic::diagnostics(source, &analysis, &thresholds)?,
    })
}

/// `analyze`: the file's table. `check`: one annotated block (text) or line (compact) per diagnostic.
fn print_report(format: OutputFormat, mode: Mode, source: &str, report: &Report) {
    let path = report.path.display().to_string();
    match (mode, format) {
        (Mode::Analyze, _) => println!("{}", render(&report.path, &report.summary)),
        (Mode::Check, OutputFormat::Compact) => report
            .diagnostics
            .iter()
            .for_each(|d| println!("{}", diagnostic::render_compact(&path, d))),
        (Mode::Check, OutputFormat::Json) => {} // printed once, after the last file
        (Mode::Check, OutputFormat::Text) => {
            let style = Style::for_output(
                std::io::stdout().is_terminal(),
                std::env::var_os("NO_COLOR").as_deref(),
            );
            report.diagnostics.iter().for_each(|d| {
                println!("{}\n", diagnostic::render_text(&path, source, d, style));
            });
        }
    }
}

/// Prints the JSON document of `reports`, or names the failure on stderr; true when printed.
fn print_document(reports: &[Report]) -> bool {
    let files: Vec<FileReport<'_>> = reports
        .iter()
        .map(|r| FileReport {
            path: r.path.display().to_string(),
            summary: &r.summary,
            diagnostics: &r.diagnostics,
        })
        .collect();
    match json::document(&files) {
        Ok(document) => {
            println!("{document}");
            true
        }
        Err(err) => {
            eprintln!("vernier: cannot write the JSON document: {err}");
            false
        }
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
