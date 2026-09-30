//! vernier — readability and syntactic-complexity analyzer for Markdown.
//!
//! The imperative shell: read files, call the pure core, print, and map the outcome to an exit code.

use std::fmt::Display;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anstream::{AutoStream, ColorChoice};
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

/// Exit code when every file was reported and, for `check`, no sentence was flagged.
const EXIT_CLEAN: u8 = 0;
/// Exit code when a sentence is flagged (spec 001 M5).
const EXIT_FLAGGED: u8 = 1;
/// Exit code for an unreadable file, an unusable model, a sentence that cannot be parsed, a
/// failed write to stdout, or a usage error (spec 001 M1, M5).
const EXIT_UNREADABLE: u8 = 2;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(refused) => {
            // why: clap's own printing colours by the stream's choice (anstream's `Auto`) and
            // drops a failed write; the same streams keep its rendering, and a failed stdout
            // write is reported (M5 criterion 2).
            let mut out = AutoStream::new(io::stdout().lock(), ColorChoice::Auto);
            let mut err = AutoStream::new(io::stderr().lock(), ColorChoice::Auto);
            return ExitCode::from(show_refusal(&refused, &mut out, &mut err));
        }
    };
    let (args, mode) = match cli.command {
        Command::Analyze(args) => (args, Mode::Analyze),
        Command::Check(args) => (args, Mode::Check),
    };
    let mut err = io::stderr().lock();
    let mut parser = match load_parser(args.model_path.as_deref(), &mut err) {
        Ok(parser) => parser,
        Err(code) => return code,
    };
    let stdout = io::stdout();
    let style = Style::for_output(
        stdout.is_terminal(),
        std::env::var_os("NO_COLOR").as_deref(),
    );
    let code = run(
        &args,
        mode,
        parser.as_mut(),
        style,
        &mut stdout.lock(),
        &mut err,
    );
    ExitCode::from(code)
}

/// Shows clap's refusal to run, rendered as clap renders it: a help or the version on `out`
/// (stdout), exit 0, or exit 2 with `vernier: cannot write to stdout: <cause>` on `err` when that
/// write fails; a usage error on `err` (stderr), exit 2, a failed write dropped as `say` does.
fn show_refusal<W: Write, E: Write>(refused: &clap::Error, out: &mut W, err: &mut E) -> u8 {
    let text = refused.render();
    if refused.use_stderr() {
        // why: as in `say`, a usage error that cannot be written cannot be reported either.
        let _unreported = write!(err, "{}", text.ansi()).and_then(|()| err.flush());
        return EXIT_UNREADABLE;
    }
    match write!(out, "{}", text.ansi()).and_then(|()| out.flush()) {
        Ok(()) => EXIT_CLEAN,
        Err(failure) => {
            say(
                err,
                format_args!("vernier: cannot write to stdout: {failure}"),
            );
            EXIT_UNREADABLE
        }
    }
}

/// Says `message` as one line on `err` (stderr). A failed write is dropped: stderr is where it
/// would be reported, and the exit code already carries the outcome (M5 criterion 2).
fn say<E: Write>(err: &mut E, message: impl Display) {
    // why: a diagnostic that cannot be written cannot be reported either; never a panic.
    let _unreported = writeln!(err, "{message}");
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

/// Processes every file, writing to `out`, and the JSON document after the last one when
/// `--format json`; the exit code is 2 if any file was unreadable (or the document could not be
/// serialized), else 1 if `check` flagged a sentence, else 0. The first write to `out` that fails
/// (a closed pipe) is said once on `err` (stderr) and ends the run with 2.
fn run<P: SentenceParser, W: Write, E: Write>(
    args: &Args,
    mode: Mode,
    mut parser: Option<&mut P>,
    style: Style,
    out: &mut W,
    err: &mut E,
) -> u8 {
    match write_all(args, mode, &mut parser, style, out, err) {
        Ok(outcomes) => exit_code(&outcomes),
        Err(failure) => {
            say(
                err,
                format_args!("vernier: cannot write to stdout: {failure}"),
            );
            EXIT_UNREADABLE
        }
    }
}

/// Every file's outcome, with its report written to `out` (the JSON document after the last
/// file), or the first failed write; what cannot be reported is said on `err`.
fn write_all<P: SentenceParser, W: Write, E: Write>(
    args: &Args,
    mode: Mode,
    parser: &mut Option<&mut P>,
    style: Style,
    out: &mut W,
    err: &mut E,
) -> io::Result<Vec<FileOutcome>> {
    let mut reports = Vec::new();
    let mut outcomes = Vec::with_capacity(args.files.len() + 1);
    for path in &args.files {
        let (outcome, read) = process(args, mode, path, parser.as_deref_mut(), err);
        match (args.format, read) {
            (_, None) => {}
            (OutputFormat::Json, Some((_, report))) => reports.push(report),
            (OutputFormat::Text | OutputFormat::Compact, Some((source, report))) => {
                print_report(args.format, mode, &source, &report, style, out)?;
            }
        }
        outcomes.push(outcome);
    }
    if args.format == OutputFormat::Json {
        outcomes.push(print_document(&reports, out, err)?);
    }
    out.flush()?;
    Ok(outcomes)
}

/// 2 if any file was unreadable, else 1 if one was flagged, else 0.
fn exit_code(outcomes: &[FileOutcome]) -> u8 {
    if outcomes.contains(&FileOutcome::Unreadable) {
        EXIT_UNREADABLE
    } else if outcomes.contains(&FileOutcome::Flagged) {
        EXIT_FLAGGED
    } else {
        EXIT_CLEAN
    }
}

/// The model of `--model-path`, loaded with the ONNX Runtime library of `ORT_DYLIB_PATH` (else
/// the loader's `libonnxruntime.so`); without `--model-path`, none, said once on stderr. A model
/// that cannot be loaded is named on `err` (stderr) and ends the run with exit 2.
fn load_parser<E: Write>(
    model_path: Option<&Path>,
    err: &mut E,
) -> Result<Option<OnnxParser>, ExitCode> {
    let Some(dir) = model_path else {
        say(
            err,
            "vernier: no --model-path given; syntactic metrics skipped",
        );
        return Ok(None);
    };
    let runtime = std::env::var_os("ORT_DYLIB_PATH")
        .filter(|path| !path.is_empty())
        .map_or_else(|| PathBuf::from("libonnxruntime.so"), PathBuf::from);
    match OnnxParser::load(dir, &runtime) {
        Ok(parser) => Ok(Some(parser)),
        Err(cause) => {
            let dir = dir.display();
            say(
                err,
                format_args!("vernier: cannot load the model {dir}: {cause}"),
            );
            Err(ExitCode::from(EXIT_UNREADABLE))
        }
    }
}

/// Reads and examines one file: its outcome and, when it can be reported, its source and report.
/// A file that cannot be read (or is not UTF-8), or holds a sentence that cannot be parsed, is
/// named on `err` (stderr) and not reported; each sentence too long for the model gets a notice
/// there.
fn process<P: SentenceParser, E: Write>(
    args: &Args,
    mode: Mode,
    path: &Path,
    parser: Option<&mut P>,
    err: &mut E,
) -> (FileOutcome, Option<(String, Report)>) {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(cause) => {
            say(
                err,
                format_args!("vernier: cannot read {}: {cause}", path.display()),
            );
            return (FileOutcome::Unreadable, None);
        }
    };
    let report = match examine(args, path, &source, parser) {
        Ok(report) => report,
        Err(cause) => {
            say(
                err,
                format_args!("vernier: {}", failure(path, &source, &cause)),
            );
            return (FileOutcome::Unreadable, None);
        }
    };
    for skipped in &report.too_long {
        say(err, too_long_notice(path, skipped));
    }
    let outcome = match mode {
        Mode::Check if !report.diagnostics.is_empty() => FileOutcome::Flagged,
        Mode::Analyze | Mode::Check => FileOutcome::Clean,
    };
    (outcome, Some((source, report)))
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

/// The stderr notice for a sentence of `path` too long for the model (spec 001 M3b criterion 1).
fn too_long_notice(path: &Path, skipped: &TooLong) -> String {
    format!(
        "vernier: {}:{}:{}: sentence too long for the model ({} subword pieces, max {}); \
         syntactic metrics skipped",
        path.display(),
        skipped.at.line(),
        skipped.at.column(),
        skipped.pieces,
        skipped.max
    )
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

/// `analyze`: the file's table. `check`: one annotated block (text) or line (compact) per
/// diagnostic. Written to `out`, which may fail (a closed pipe).
fn print_report<W: Write>(
    format: OutputFormat,
    mode: Mode,
    source: &str,
    report: &Report,
    style: Style,
    out: &mut W,
) -> io::Result<()> {
    let path = report.path.display().to_string();
    match (mode, format) {
        (Mode::Analyze, _) => writeln!(out, "{}", render(&report.path, &report.summary)),
        (Mode::Check, OutputFormat::Compact) => report
            .diagnostics
            .iter()
            .try_for_each(|d| writeln!(out, "{}", diagnostic::render_compact(&path, d))),
        (Mode::Check, OutputFormat::Json) => Ok(()), // written once, after the last file
        (Mode::Check, OutputFormat::Text) => report.diagnostics.iter().try_for_each(|d| {
            writeln!(
                out,
                "{}\n",
                diagnostic::render_text(&path, source, d, style)
            )
        }),
    }
}

/// Writes the JSON document of `reports` to `out`: `Clean` when written, `Unreadable` (named on
/// `err`) when it cannot be serialized, or the failed write.
fn print_document<W: Write, E: Write>(
    reports: &[Report],
    out: &mut W,
    err: &mut E,
) -> io::Result<FileOutcome> {
    let files: Vec<FileReport<'_>> = reports
        .iter()
        .map(|r| FileReport {
            path: r.path.display().to_string(),
            summary: &r.summary,
            diagnostics: &r.diagnostics,
        })
        .collect();
    match json::document(&files) {
        Ok(document) => writeln!(out, "{document}").map(|()| FileOutcome::Clean),
        Err(cause) => {
            say(
                err,
                format_args!("vernier: cannot write the JSON document: {cause}"),
            );
            Ok(FileOutcome::Unreadable)
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

    /// The command, format and files of a command line, with the default thresholds.
    // why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
    #[allow(clippy::unwrap_used)]
    fn command_line(command: &str, format: &str, files: &[&Path]) -> (Args, Mode) {
        let line = ["vernier", command, "--format", format]
            .map(std::ffi::OsString::from)
            .into_iter()
            .chain(files.iter().map(|f| f.as_os_str().to_owned()));
        match Cli::try_parse_from(line).unwrap().command {
            Command::Analyze(args) => (args, Mode::Analyze),
            Command::Check(args) => (args, Mode::Check),
        }
    }

    /// A committed fixture of the integration tests.
    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    /// A stdout whose every write fails, as a pipe whose reader has gone.
    struct ClosedPipe;

    impl Write for ClosedPipe {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }

    const FORMATS: [&str; 3] = ["text", "compact", "json"];

    /// Given a readable file whose first sentence the parser fails on
    /// When `analyze` and `check` run on it in each format
    /// Then the run exits 2 and the file is not reported: nothing on stdout, where JSON lists no
    /// file (M5 criterion 2)
    #[test]
    fn a_sentence_that_cannot_be_parsed_exits_2() {
        for command in ["analyze", "check"] {
            for format in FORMATS {
                let (args, mode) = command_line(command, format, &[&fixture("sample.md")]);
                let mut failing = CountingParser {
                    pieces: 0,
                    calls: 0,
                };
                let (mut out, mut err) = (Vec::new(), Vec::new());
                let code = run(
                    &args,
                    mode,
                    Some(&mut failing),
                    Style::Plain,
                    &mut out,
                    &mut err,
                );
                assert_eq!(code, EXIT_UNREADABLE, "{command} {format}");
                assert_eq!(failing.calls, 1, "{command} {format}");
                let said = format!(
                    "cannot parse the sentence at {}:",
                    fixture("sample.md").display()
                );
                let stderr = String::from_utf8(err).unwrap();
                assert!(stderr.contains(&said), "{command} {format}: {stderr}");
                let stdout = String::from_utf8(out).unwrap();
                match format {
                    "json" => {
                        let doc: serde_json::Value = serde_json::from_str(&stdout).unwrap();
                        assert_eq!(doc["files"], serde_json::json!([]), "{command}");
                    }
                    _ => assert_eq!(stdout, "", "{command} {format}"),
                }
            }
        }
    }

    /// Given a readable file every command and format writes something for, and a stdout that
    /// refuses every write
    /// When `analyze` and `check` run in each format
    /// Then the run exits 2 instead of 0 or 1, and says so on stderr in exactly one line,
    /// `vernier: cannot write to stdout: <cause>` (M5 criterion 2)
    #[test]
    fn a_failed_write_exits_2() {
        let cause = io::Error::from(io::ErrorKind::BrokenPipe);
        for command in ["analyze", "check"] {
            for format in FORMATS {
                let (args, mode) = command_line(command, format, &[&fixture("long.md")]);
                let mut err = Vec::new();
                let code = run::<CountingParser, _, _>(
                    &args,
                    mode,
                    None,
                    Style::Plain,
                    &mut ClosedPipe,
                    &mut err,
                );
                assert_eq!(code, EXIT_UNREADABLE, "{command} {format}");
                assert_eq!(
                    String::from_utf8(err).unwrap(),
                    format!("vernier: cannot write to stdout: {cause}\n"),
                    "{command} {format}"
                );
            }
        }
    }

    /// Clap's refusal of `line`: its help, the version, or a usage error.
    // why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
    #[allow(clippy::unwrap_used)]
    fn refusal(line: &[&str]) -> clap::Error {
        Cli::try_parse_from([&["vernier"], line].concat()).unwrap_err()
    }

    /// Given clap's help and version for `vernier`, and a stdout that takes every write or refuses
    /// every write
    /// When the refusal is shown
    /// Then a working stdout gets clap's rendering exactly and the exit code is 0, while a refusing
    /// one gives exit 2 and exactly one stderr line, `vernier: cannot write to stdout: <cause>`
    /// (M5 criterion 2)
    #[test]
    fn help_or_version_is_written_through_the_fallible_stdout() {
        let cause = io::Error::from(io::ErrorKind::BrokenPipe);
        for line in [&["--help"][..], &["check", "-h"], &["--version"]] {
            let refused = refusal(line);
            let (mut out, mut err) = (Vec::new(), Vec::new());
            assert_eq!(show_refusal(&refused, &mut out, &mut err), EXIT_CLEAN);
            assert_eq!(out, refused.render().ansi().to_string().into_bytes());
            assert!(err.is_empty(), "{line:?}");
            let mut err = Vec::new();
            let code = show_refusal(&refused, &mut ClosedPipe, &mut err);
            assert_eq!(code, EXIT_UNREADABLE, "{line:?}");
            assert_eq!(
                String::from_utf8(err).unwrap(),
                format!("vernier: cannot write to stdout: {cause}\n"),
                "{line:?}"
            );
        }
    }

    /// Given usage errors clap refuses, and a stderr that takes every write or refuses every write
    /// When the refusal is shown
    /// Then it exits 2 either way, with clap's rendering exactly on a working stderr and nothing
    /// on stdout (M5 criterion 2)
    #[test]
    fn a_usage_error_is_written_through_the_fallible_stderr() {
        for line in [&["check"][..], &["check", "--max-mdd", "0", "x.md"], &[]] {
            let refused = refusal(line);
            let (mut out, mut err) = (Vec::new(), Vec::new());
            assert_eq!(show_refusal(&refused, &mut out, &mut err), EXIT_UNREADABLE);
            assert!(out.is_empty(), "{line:?}");
            assert_eq!(err, refused.render().ansi().to_string().into_bytes());
            let mut out = Vec::new();
            let code = show_refusal(&refused, &mut out, &mut ClosedPipe);
            assert_eq!(code, EXIT_UNREADABLE, "{line:?}");
            assert!(out.is_empty(), "{line:?}");
        }
    }

    /// Given the long-sentence file and a missing one, a stdout that takes every write, and a
    /// stderr that refuses every write
    /// When `analyze` and `check` run on each in each format
    /// Then the exit code is the one the other clauses give (2 for the missing file, else 0 for
    /// analyze and 1 for check), with no panic (M5 criterion 2)
    #[test]
    fn a_failed_stderr_write_keeps_the_exit_code() {
        let gone = fixture("no-such-file.md");
        for (command, flagged) in [("analyze", EXIT_CLEAN), ("check", EXIT_FLAGGED)] {
            for format in FORMATS {
                for (file, expected) in [
                    (fixture("long.md"), flagged),
                    (gone.clone(), EXIT_UNREADABLE),
                ] {
                    let (args, mode) = command_line(command, format, &[&file]);
                    let mut out = Vec::new();
                    let code = run::<CountingParser, _, _>(
                        &args,
                        mode,
                        None,
                        Style::Plain,
                        &mut out,
                        &mut ClosedPipe,
                    );
                    assert_eq!(code, expected, "{command} {format} {}", file.display());
                }
            }
        }
    }

    /// Given the long-sentence file and a stdout that takes every write
    /// When `analyze` and `check` run on it without a model
    /// Then analyze exits 0 and check 1: the writer is not what decides (witness for the above)
    #[test]
    fn a_working_stdout_keeps_the_exit_code() {
        for (command, expected) in [("analyze", EXIT_CLEAN), ("check", EXIT_FLAGGED)] {
            for format in FORMATS {
                let (args, mode) = command_line(command, format, &[&fixture("long.md")]);
                let (mut out, mut err) = (Vec::new(), Vec::new());
                let code = run::<CountingParser, _, _>(
                    &args,
                    mode,
                    None,
                    Style::Plain,
                    &mut out,
                    &mut err,
                );
                assert_eq!(code, expected, "{command} {format}");
                assert!(!out.is_empty(), "{command} {format}");
            }
        }
    }

    /// Given the long-sentence file (one sentence over the length rule) and the sample file (no
    /// sentence flagged), and a parser that finds every sentence too long for the model
    /// When `analyze` and `check` run on each in each format, with that parser and without one
    /// Then the exit code is the same with and without it: 1 for check on the long file, else 0
    /// (M3b criterion 1: a too-long sentence does not change the exit code)
    #[test]
    fn a_too_long_sentence_keeps_the_exit_code() {
        for (command, flagged) in [("analyze", EXIT_CLEAN), ("check", EXIT_FLAGGED)] {
            for format in FORMATS {
                for (file, expected) in [("long.md", flagged), ("sample.md", EXIT_CLEAN)] {
                    let (args, mode) = command_line(command, format, &[&fixture(file)]);
                    let mut too_long = CountingParser {
                        pieces: 510,
                        calls: 0,
                    };
                    let (mut out, mut err) = (Vec::new(), Vec::new());
                    let with = run(
                        &args,
                        mode,
                        Some(&mut too_long),
                        Style::Plain,
                        &mut out,
                        &mut err,
                    );
                    let without = run::<CountingParser, _, _>(
                        &args,
                        mode,
                        None,
                        Style::Plain,
                        &mut out,
                        &mut err,
                    );
                    assert!(too_long.calls > 0, "{command} {format} {file}");
                    assert_eq!(
                        (with, without),
                        (expected, expected),
                        "{command} {format} {file}"
                    );
                }
            }
        }
    }

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

    /// Given a file of three sentences, each too long for the model (7 subword pieces, max 1)
    /// When the file is examined and a notice is written for each sentence it skipped
    /// Then there is one notice per sentence, in the spec's wording, at the sentence's first
    /// character, with its pieces and the model's maximum (M3b criterion 1)
    #[test]
    fn each_too_long_sentence_gets_the_specified_notice() {
        let mut parser = CountingParser {
            pieces: 7,
            calls: 0,
        };
        let Ok(report) = examine(&args(), Path::new("x.md"), SOURCE, Some(&mut parser)) else {
            panic!("examine failed");
        };
        let notices: Vec<String> = report
            .too_long
            .iter()
            .map(|skipped| too_long_notice(Path::new("x.md"), skipped))
            .collect();
        let expected: Vec<String> = [(3, 1), (3, 16), (5, 3)]
            .iter()
            .map(|(line, column)| {
                format!(
                    "vernier: x.md:{line}:{column}: sentence too long for the model \
                     (7 subword pieces, max 1); syntactic metrics skipped"
                )
            })
            .collect();
        assert_eq!(notices, expected);
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
