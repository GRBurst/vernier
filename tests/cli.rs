//! End-to-end checks of the `vernier` binary (spec 001 M1, M2, M4, M5).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use vernier::prose::SourceFormat;
use vernier::summary::{FileSummary, render, summarize};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn missing() -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join("vernier-no-such-file.md")
}

// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn vernier(args: &[&str], files: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vernier"))
        .args(args)
        .args(files)
        .output()
        .unwrap()
}

/// The summary line the library computes for a file, so the expectation comes from the core.
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn expected_line(path: &Path) -> String {
    let source = std::fs::read_to_string(path).unwrap();
    render(path, &summarize(&source, SourceFormat::from_path(path)))
}

/// The library's summary of a file.
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn library_summary(path: &Path) -> FileSummary {
    let source = std::fs::read_to_string(path).unwrap();
    summarize(&source, SourceFormat::from_path(path))
}

/// The value in the table row of `name` among `lines`, if the row is there.
fn row<'a>(lines: &[&'a str], name: &str) -> Option<&'a str> {
    lines.iter().find_map(|line| {
        let rest = line.trim_start().strip_prefix(name)?;
        rest.starts_with("  ").then(|| rest.trim())
    })
}

/// Asserts that the table rows of `path` hold the library's counts and scores (2 decimals).
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn assert_rows_follow_the_library(table: &[&str], path: &Path) {
    let summary = library_summary(path);
    let (c, r) = (summary.counts, summary.readability.unwrap());
    for (name, count) in [
        ("sentences", c.sentences),
        ("syllables", c.syllables),
        ("complex words", c.complex_words),
    ] {
        assert_eq!(row(table, name), Some(count.to_string().as_str()), "{name}");
    }
    for (name, value) in [
        ("Flesch Reading Ease", r.flesch_reading_ease),
        ("Flesch-Kincaid Grade", r.flesch_kincaid_grade),
        ("Gunning Fog", r.gunning_fog),
        ("average sentence length", r.average_sentence_length),
    ] {
        assert_eq!(
            row(table, name),
            Some(format!("{value:.2}").as_str()),
            "{name}"
        );
    }
}

/// Given a readable Markdown file and a readable plain-text file
/// When `vernier analyze` runs on both
/// Then each file's summary line is followed by its table, whose rows give its sentence counts
/// and its four scores to 2 decimals as the library computes them, and it exits 0
#[test]
fn analyze_prints_surface_metrics() {
    let (md, txt) = (fixture("sample.md"), fixture("plain.txt"));
    let out = vernier(&["analyze"], &[&md, &txt]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    let per_file = 2 + vernier::summary::METRICS.len();
    assert_eq!(lines.len(), 2 * per_file, "{stdout}");
    for (i, path) in [&md, &txt].into_iter().enumerate() {
        assert_rows_follow_the_library(&lines[per_file * i..per_file * (i + 1)], path);
    }
    // Witnesses: sample.md holds 4 sentences (paragraph, two tight items, blockquote); plain.txt
    // holds 2 hard-wrapped sentences of 9 syllables and no complex word.
    assert_eq!(row(&lines[..per_file], "sentences"), Some("4"), "{stdout}");
    let plain = &lines[per_file..];
    assert_eq!(
        (
            row(plain, "sentences"),
            row(plain, "syllables"),
            row(plain, "complex words")
        ),
        (Some("2"), Some("9"), Some("0"))
    );
}

/// Given a Markdown file holding the NOMZ sentence (4 nominalizations of 14 words)
/// When `vernier analyze` runs on it
/// Then its nominalization row gives the ratio as the library renders it, `4 of 14` words, and
/// its passive voice row says absent (no parse)
#[test]
fn analyze_prints_the_nominalization_ratio() {
    let path = fixture("nominal.md");
    let out = vernier(&["analyze"], &[&path]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let expected = expected_line(&path);
    let (lines, library): (Vec<&str>, Vec<&str>) =
        (stdout.lines().collect(), expected.lines().collect());
    assert_eq!(
        row(&lines, "nominalization ratio"),
        row(&library, "nominalization ratio"),
        "{stdout}"
    );
    let ratio = row(&lines, "nominalization ratio").unwrap_or_default();
    assert!(ratio.contains("(4 of 14 words)"), "{ratio}");
    assert_eq!(
        row(&lines, "passive voice"),
        Some("absent (no parse)"),
        "{stdout}"
    );
}

/// Given the file with a 30-word sentence, which `check` flags
/// When `vernier analyze` runs on it
/// Then it prints M1's line and the table with one row per metric, and exits 0
#[test]
fn analyze_prints_a_metrics_table_and_exits_0_on_a_flagged_file() {
    let path = fixture("long.md");
    let out = vernier(&["analyze"], &[&path]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines[0].starts_with(&format!("{}: ", path.display())),
        "{stdout}"
    );
    assert!(lines[1].trim_start().starts_with("metric"), "{stdout}");
    for name in vernier::summary::METRICS {
        assert!(row(&lines, name).is_some(), "{name}: {stdout}");
    }
}

/// Given a readable Markdown file and a readable plain-text file
/// When `vernier analyze` runs on both
/// Then it prints one summary line per file, in order, and exits 0
#[test]
fn analyze_prints_span_and_word_counts_per_file() {
    let (md, txt) = (fixture("sample.md"), fixture("plain.txt"));
    let out = vernier(&["analyze"], &[&md, &txt]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout,
        format!("{}\n{}\n", expected_line(&md), expected_line(&txt))
    );
}

/// Given a path that does not exist
/// When `vernier analyze` and `vernier check` run on it
/// Then each names the file on stderr and exits 2
#[test]
fn unreadable_file_is_named_and_exits_2() {
    let path = missing();
    for command in ["analyze", "check"] {
        let out = vernier(&[command], &[&path]);
        assert_eq!(out.status.code(), Some(2), "{command}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            stderr.contains(&path.display().to_string()),
            "{command}: {stderr}"
        );
    }
}

/// Given a readable file followed by an unreadable one
/// When `vernier analyze` runs on both
/// Then the readable file is still summarized, the unreadable one is named, and it exits 2
#[test]
fn an_unreadable_file_does_not_hide_the_others() {
    let (md, gone) = (fixture("sample.md"), missing());
    let out = vernier(&["analyze"], &[&md, &gone]);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("{}\n", expected_line(&md))
    );
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(&gone.display().to_string())
    );
}

/// Given a file that is not UTF-8
/// When `vernier analyze` runs on it
/// Then it counts as unreadable: named on stderr, exit 2
#[test]
fn a_non_utf8_file_is_unreadable() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("vernier-latin1.txt");
    std::fs::write(&path, b"caf\xe9\n").unwrap();
    let out = vernier(&["analyze"], &[&path]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        String::from_utf8(out.stderr)
            .unwrap()
            .contains(&path.display().to_string())
    );
}

/// Given a readable file whose longest sentence has 13 words, and every M5 flag with a
/// sentence limit above that
/// When `vernier check` runs
/// Then it accepts the flags, flags nothing and exits 0
#[test]
fn check_accepts_every_m5_flag_and_passes_without_rules() {
    let out = vernier(
        &[
            "check",
            "--format",
            "compact",
            "--max-sentence-len",
            "30",
            "--max-mdd",
            "2.0",
            "--max-tree-depth",
            "3",
            "--max-clauses",
            "1",
            "--model-path",
            "none.udpipe",
        ],
        &[&fixture("sample.md")],
    );
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Runs `vernier check` and returns its exit code and stdout.
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn check(args: &[&str], files: &[&Path]) -> (Option<i32>, String) {
    let out = vernier(&[&["check"], args].concat(), files);
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

/// Given a file with one 30-word sentence, starting on line 3, column 20
/// When `vernier check --format compact` runs with the default limit of 25 words
/// Then it prints that sentence's position and LongSentence on stdout and exits 1
#[test]
fn check_flags_a_long_sentence_and_exits_1() {
    let path = fixture("long.md");
    assert_eq!(
        check(&["--format", "compact"], &[&path]),
        (
            Some(1),
            format!(
                "{}:3:20: CognitiveOverload: LongSentence: sentence has 30 words (max 25)\n",
                path.display()
            )
        )
    );
}

/// Given the file with one 30-word sentence
/// When `vernier check` runs with the default `text` format
/// Then it prints one `warning[CognitiveOverload]` diagnostic at 3:20 showing the whole line, with
/// the words and LongSentence and the syntactic metrics absent (no parse), and exits 1
#[test]
fn check_prints_a_cognitive_overload_diagnostic() {
    let path = fixture("long.md");
    let (code, stdout) = check(&[], &[&path]);
    assert_eq!(code, Some(1));
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines[0],
        "warning[CognitiveOverload]: Sentence exceeds human working-memory capacity"
    );
    assert_eq!(lines[1].trim(), format!("--> {}:3:20", path.display()));
    let source_line = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .nth(2)
        .unwrap()
        .to_owned();
    assert!(lines.iter().any(|l| l.ends_with(&source_line)), "{stdout}");
    let listed: Vec<&str> = lines.iter().map(|l| l.trim()).collect();
    assert!(
        listed.contains(&"- words: 30 (LongSentence, max 25)"),
        "{stdout}"
    );
    assert!(
        listed.contains(&"- mean dependency distance: absent (no parse)"),
        "{stdout}"
    );
}

/// Given a sentence that starts at line 1, column 15 and is hard-wrapped over three lines
/// When `vernier check --max-sentence-len 5` runs
/// Then the diagnostic points at 1:15, opens the underline under column 15 of line 1 and
/// closes it on line 3
#[test]
fn check_underlines_a_hard_wrapped_sentence() {
    let path = fixture("wrapped.md");
    let (code, stdout) = check(&["--max-sentence-len", "5"], &[&path]);
    assert_eq!(code, Some(1));
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[1].trim(), format!("--> {}:1:15", path.display()));
    let first = lines
        .iter()
        .position(|l| l.ends_with("Short opener. This sentence is"))
        .unwrap();
    let text_at = lines[first].len() - "Short opener. This sentence is".len();
    let opening = lines[first + 1];
    assert!(opening.trim_end().ends_with('^'), "{stdout}");
    assert_eq!(opening.trim_end().len() - 1, text_at + 14, "{stdout}");
    let last = lines
        .iter()
        .position(|l| l.ends_with("of the file until it ends."))
        .unwrap();
    let closing = lines[last + 1].trim();
    assert!(
        closing.starts_with("| |_") && closing.ends_with('^'),
        "{stdout}"
    );
}

/// Given the file with a long sentence
/// When `vernier check` writes to a pipe, with `NO_COLOR` unset and set
/// Then the output holds no escape codes
#[test]
fn check_writes_no_escape_codes_off_a_terminal() {
    for no_color in [None, Some("1")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_vernier"));
        command
            .arg("check")
            .arg(fixture("long.md"))
            .env_remove("NO_COLOR");
        if let Some(value) = no_color {
            command.env("NO_COLOR", value);
        }
        let out = command.output().unwrap();
        assert_eq!(out.status.code(), Some(1));
        assert!(!out.stdout.contains(&0x1b), "{no_color:?}");
        assert!(
            out.stdout.starts_with(b"warning[CognitiveOverload]"),
            "{no_color:?}"
        );
    }
}

/// Given the file with one 30-word sentence
/// When `vernier check --format compact` runs
/// Then it prints one `path:line:col: CognitiveOverload: message` line for that sentence and exits 1
#[test]
fn check_compact_prints_one_line_per_flagged_sentence() {
    let path = fixture("long.md");
    assert_eq!(
        check(&["--format", "compact"], &[&path]),
        (
            Some(1),
            format!(
                "{}:3:20: CognitiveOverload: LongSentence: sentence has 30 words (max 25)\n",
                path.display()
            )
        )
    );
}

/// Given the file with one 30-word sentence
/// When `vernier check --format compact` runs with `--max-sentence-len` 29, then 30
/// Then 29 flags it and exits 1, while 30 flags nothing and exits 0
#[test]
fn max_sentence_len_raises_the_bar() {
    let path = fixture("long.md");
    let (code, stdout) = check(
        &["--format", "compact", "--max-sentence-len", "29"],
        &[&path],
    );
    assert_eq!(code, Some(1));
    assert!(
        stdout.ends_with("LongSentence: sentence has 30 words (max 29)\n"),
        "{stdout}"
    );
    assert_eq!(
        check(
            &["--format", "compact", "--max-sentence-len", "30"],
            &[&path]
        ),
        (Some(0), String::new())
    );
}

/// Given the sample file, whose longest sentence has 13 words
/// When `vernier check` runs with the default limit of 25
/// Then it flags nothing and exits 0
#[test]
fn check_on_sample_exits_0() {
    assert_eq!(
        check(&[], &[&fixture("sample.md")]),
        (Some(0), String::new())
    );
}

/// Given the sample file and a limit of 10 words
/// When `vernier check --format compact` runs
/// Then exactly its 13-word sentence (line 7, column 1) is flagged and it exits 1
#[test]
fn check_flags_the_samples_13_word_sentence_over_a_limit_of_10() {
    let path = fixture("sample.md");
    assert_eq!(
        check(
            &["--format", "compact", "--max-sentence-len", "10"],
            &[&path]
        ),
        (
            Some(1),
            format!(
                "{}:7:1: CognitiveOverload: LongSentence: sentence has 13 words (max 10)\n",
                path.display()
            )
        )
    );
}

/// Given a file with a long sentence and a file that cannot be read
/// When `vernier check` runs on both
/// Then the long sentence is still flagged, and the unreadable file wins: exit 2
#[test]
fn an_unreadable_file_wins_over_a_flag() {
    let (code, stdout) = check(&[], &[&fixture("long.md"), &missing()]);
    assert_eq!(code, Some(2));
    assert!(stdout.contains("LongSentence"), "{stdout}");
}

/// Runs `vernier` and parses its stdout as one JSON document.
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn json(args: &[&str], files: &[&Path]) -> (Option<i32>, serde_json::Value) {
    let out = vernier(args, files);
    let stdout = String::from_utf8(out.stdout).unwrap();
    (out.status.code(), serde_json::from_str(&stdout).unwrap())
}

/// Given the long-sentence file and the sample file
/// When `vernier check --format json` runs on both
/// Then stdout is one document listing both files in order, the long sentence's diagnostic at
/// the position compact prints, the sample without diagnostics, and it exits 1
#[test]
fn json_is_one_document_for_all_files() {
    let (long, sample) = (fixture("long.md"), fixture("sample.md"));
    let (code, value) = json(&["check", "--format", "json"], &[&long, &sample]);
    assert_eq!(code, Some(1));
    assert_eq!(value["schema_version"], 1);
    let files = value["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0]["path"], long.display().to_string());
    assert_eq!(files[1]["path"], sample.display().to_string());
    let d = &files[0]["diagnostics"][0];
    let (_, compact) = check(&["--format", "compact"], &[&long]);
    let position = format!("{}:{}:{}:", long.display(), d["line"], d["column"]);
    assert!(compact.starts_with(&position), "{compact} vs {position}");
    assert_eq!(
        files[1]["diagnostics"],
        serde_json::Value::Array(Vec::new())
    );
    assert_eq!(files[1]["metrics"]["sentences"], 4);
}

/// Given the long-sentence file, the sample file and a missing file
/// When `analyze` and `check` run with `--format json`
/// Then analyze exits 0 even with a diagnostic, check exits 1 on the long file and 0 on the
/// sample, and a missing file makes it exit 2 while the document lists the readable files
#[test]
fn json_exit_codes_follow_the_command() {
    let (long, sample, gone) = (fixture("long.md"), fixture("sample.md"), missing());
    let (code, value) = json(&["analyze", "--format", "json"], &[&long]);
    assert_eq!(code, Some(0));
    assert_eq!(
        value["files"][0]["diagnostics"].as_array().map(Vec::len),
        Some(1)
    );
    assert_eq!(json(&["check", "--format", "json"], &[&sample]).0, Some(0));
    assert_eq!(json(&["check", "--format", "json"], &[&long]).0, Some(1));
    let (code, value) = json(&["check", "--format", "json"], &[&long, &gone]);
    assert_eq!(code, Some(2));
    assert_eq!(value["files"].as_array().map(Vec::len), Some(1));
}

/// Given the sample file
/// When `vernier analyze` runs with `--format compact`
/// Then it prints the same table as the text format
#[test]
fn analyze_prints_the_table_in_compact_too() {
    let path = fixture("sample.md");
    let text = vernier(&["analyze"], &[&path]).stdout;
    let compact = vernier(&["analyze", "--format", "compact"], &[&path]).stdout;
    assert_eq!(
        String::from_utf8(compact).unwrap(),
        String::from_utf8(text).unwrap()
    );
}
