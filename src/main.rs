//! vernier — readability and syntactic-complexity analyzer for Markdown.
//!
//! The imperative shell: read files, call the pure core, print, and map the outcome to an exit code.

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use vernier::analysis::{self, AnalysisError, FileAnalysis, Syntax, Thresholds};
use vernier::cli::{Args, Cli, Command, OutputFormat};
use vernier::dependency::Parser as SentenceParser;
use vernier::diagnostic::{self, Diagnostic, Style};
use vernier::json::{self, FileReport};
use vernier::onnx::OnnxParser;
use vernier::position::{LineIndex, Position, PositionError};
use vernier::prose::SourceFormat;
use vernier::summary::{FileSummary, render, summarize, with_parse};

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

/// One readable file's summary and diagnostics, and the sentences too long for the model.
struct Report {
    path: PathBuf,
    summary: FileSummary,
    diagnostics: Vec<Diagnostic>,
    too_long: Vec<TooLong>,
}

/// A sentence the model could not take: where it starts, its subword pieces and the maximum.
struct TooLong {
    at: Position,
    pieces: usize,
    max: usize,
}

/// Why a readable file could not be reported.
enum ExamineError<E: std::error::Error> {
    Locate(PositionError),
    Parse(AnalysisError<E>),
}

/// Loads the model once (exit 2, no file processed, if it cannot be loaded), processes every
/// file, prints the JSON document after the last one when `--format json`, then exits 2 if any
/// file was unreadable (or the document could not be written), else 1 if `check` flagged a
/// sentence, else 0.
fn run(args: &Args, mode: Mode) -> ExitCode {
    let mut parser = match load_parser(args.model_path.as_deref()) {
        Ok(parser) => parser,
        Err(code) => return code,
    };
    let mut reports = Vec::new();
    let mut outcomes: Vec<FileOutcome> = args
        .files
        .iter()
        .map(|path| process(args, mode, path, parser.as_mut(), &mut reports))
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

/// The model of `--model-path`, loaded with the ONNX Runtime library of `ORT_DYLIB_PATH` (else
/// the loader's `libonnxruntime.so`); without `--model-path`, none, said once on stderr. A model
/// that cannot be loaded is named on stderr and ends the run with exit 2.
fn load_parser(model_path: Option<&Path>) -> Result<Option<OnnxParser>, ExitCode> {
    let Some(dir) = model_path else {
        eprintln!("vernier: no --model-path given; syntactic metrics skipped");
        return Ok(None);
    };
    let runtime = std::env::var_os("ORT_DYLIB_PATH")
        .filter(|path| !path.is_empty())
        .map_or_else(|| PathBuf::from("libonnxruntime.so"), PathBuf::from);
    match OnnxParser::load(dir, &runtime) {
        Ok(parser) => Ok(Some(parser)),
        Err(err) => {
            eprintln!("vernier: cannot load the model {}: {err}", dir.display());
            Err(ExitCode::from(EXIT_UNREADABLE))
        }
    }
}

/// Reads and reports one file, or names it on stderr when it cannot be read (or is not UTF-8)
/// or a sentence cannot be parsed. Its report is printed now, or kept in `reports` for the JSON
/// document; each sentence too long for the model gets a notice on stderr.
fn process(
    args: &Args,
    mode: Mode,
    path: &Path,
    parser: Option<&mut OnnxParser>,
    reports: &mut Vec<Report>,
) -> FileOutcome {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("vernier: cannot read {}: {err}", path.display());
            return FileOutcome::Unreadable;
        }
    };
    let report = match examine(args, path, &source, parser) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("vernier: {}", failure(path, &source, &err));
            return FileOutcome::Unreadable;
        }
    };
    for skipped in &report.too_long {
        eprintln!(
            "vernier: {}:{}:{}: sentence too long for the model ({} subword pieces, max {}); \
             syntactic metrics skipped",
            path.display(),
            skipped.at.line(),
            skipped.at.column(),
            skipped.pieces,
            skipped.max
        );
    }
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

/// The summary and the diagnostics of one file under the command line's thresholds, with every
/// sentence parsed once by `parser` when there is one.
fn examine<P: SentenceParser>(
    args: &Args,
    path: &Path,
    source: &str,
    parser: Option<&mut P>,
) -> Result<Report, ExamineError<P::Error>> {
    let thresholds = Thresholds {
        max_sentence_len: args.max_sentence_len,
        max_mdd: args.max_mdd,
        max_tree_depth: args.max_tree_depth,
        max_clauses: args.max_clauses,
    };
    let format = SourceFormat::from_path(path);
    let surface = summarize(source, format);
    let (analysis, summary) = match parser {
        None => (analysis::analyze(source, format, &thresholds), surface),
        Some(parser) => {
            let file = analysis::analyze_parsed(source, format, &thresholds, parser)
                .map_err(ExamineError::Parse)?;
            let summary = with_parse(surface, &file);
            (file, summary)
        }
    };
    let diagnostics =
        diagnostic::diagnostics(source, &analysis, &thresholds).map_err(ExamineError::Locate)?;
    Ok(Report {
        path: path.to_owned(),
        summary,
        diagnostics,
        too_long: too_long(source, &analysis).map_err(ExamineError::Locate)?,
    })
}

/// The sentences of `analysis` too long for the model, with their start positions in `source`.
fn too_long(source: &str, analysis: &FileAnalysis) -> Result<Vec<TooLong>, PositionError> {
    let index = LineIndex::new(source);
    analysis
        .sentences
        .iter()
        .filter_map(|sentence| match sentence.syntax {
            Syntax::TooLong { pieces, max } => Some((sentence.source_range.start, pieces, max)),
            Syntax::Unparsed | Syntax::Parsed(_) => None,
        })
        .map(|(start, pieces, max)| {
            Ok(TooLong {
                at: index.position(start)?,
                pieces,
                max,
            })
        })
        .collect()
}

/// What stderr says when `path` could not be reported: the sentence that could not be located,
/// or the position of the sentence that could not be parsed and why.
fn failure<E: std::error::Error>(path: &Path, source: &str, err: &ExamineError<E>) -> String {
    let (start, cause) = match err {
        ExamineError::Locate(err) => {
            return format!("cannot locate a sentence in {}: {err}", path.display());
        }
        ExamineError::Parse(AnalysisError::Parse {
            sentence_start,
            source,
        }) => (*sentence_start, source.to_string()),
        ExamineError::Parse(AnalysisError::Malformed {
            sentence_start,
            source,
        }) => (
            *sentence_start,
            format!("the parse is not a tree: {source}"),
        ),
    };
    let at = LineIndex::new(source).position(start).map_or_else(
        |_| format!(" (byte {start})"),
        |at| format!(":{}:{}", at.line(), at.column()),
    );
    format!(
        "cannot parse the sentence at {}{at}: {cause}",
        path.display()
    )
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
    use super::*;

    /// Answers `TooLong` (`.0` pieces, max 1) for every sentence and counts its calls, or fails
    /// on every sentence when `.0` is 0.
    struct CountingParser {
        pieces: usize,
        calls: usize,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("no parse")]
    struct NoParse;

    impl SentenceParser for CountingParser {
        type Error = NoParse;

        fn parse(&mut self, _sentence: &str) -> Result<vernier::dependency::Parse, NoParse> {
            self.calls += 1;
            if self.pieces == 0 {
                return Err(NoParse);
            }
            Ok(vernier::dependency::Parse::TooLong {
                pieces: self.pieces,
                max: 1,
            })
        }
    }

    // why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
    #[allow(clippy::unwrap_used, clippy::panic)]
    fn args() -> Args {
        match Cli::try_parse_from(["vernier", "check", "x.md"])
            .unwrap()
            .command
        {
            Command::Check(args) => args,
            Command::Analyze(_) => panic!("parsed as analyze"),
        }
    }

    const SOURCE: &str = "# Title\n\nOne two three. Four five six.\n\n- Seven eight.\n";

    /// Given a file of three sentences and a parser that counts its calls
    /// When the file is examined as by a whole run
    /// Then the parser was called exactly once per sentence (P14), and each sentence it found too
    /// long is reported at its start with its pieces and the maximum
    #[test]
    fn examine_parses_each_sentence_once() {
        let mut parser = CountingParser {
            pieces: 7,
            calls: 0,
        };
        let Ok(report) = examine(&args(), Path::new("x.md"), SOURCE, Some(&mut parser)) else {
            panic!("examine failed");
        };
        assert_eq!(report.summary.counts.sentences, 3);
        assert_eq!(parser.calls, 3);
        let skipped: Vec<(usize, usize, usize, usize)> = report
            .too_long
            .iter()
            .map(|t| (t.at.line(), t.at.column(), t.pieces, t.max))
            .collect();
        assert_eq!(skipped, [(3, 1, 7, 1), (3, 16, 7, 1), (5, 3, 7, 1)]);
    }

    /// Given a file whose first sentence the parser fails on
    /// When the file is examined and the failure described
    /// Then the description names the file and the sentence's line and column, and the cause
    #[test]
    fn a_parse_failure_names_the_sentence_position() {
        let mut parser = CountingParser {
            pieces: 0,
            calls: 0,
        };
        let Err(err) = examine(&args(), Path::new("x.md"), SOURCE, Some(&mut parser)) else {
            panic!("examine succeeded");
        };
        assert_eq!(
            failure(Path::new("x.md"), SOURCE, &err),
            "cannot parse the sentence at x.md:3:1: no parse"
        );
    }

    #[test]
    fn version_is_semver_shaped() {
        let parts: Vec<&str> = env!("CARGO_PKG_VERSION").split('.').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().all(|p| p.parse::<u32>().is_ok()));
    }
}
