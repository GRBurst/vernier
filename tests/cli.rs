//! End-to-end checks of the `vernier` binary (spec 001 M1, M2).

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

/// Given a readable Markdown file and a readable plain-text file
/// When `vernier analyze` runs on both
/// Then each file's summary line is followed by its sentence counts and its four scores to
/// 2 decimals, as the library computes them, and it exits 0
#[test]
fn analyze_prints_surface_metrics() {
    let (md, txt) = (fixture("sample.md"), fixture("plain.txt"));
    let out = vernier(&["analyze"], &[&md, &txt]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 8, "{stdout}");
    for (i, path) in [&md, &txt].into_iter().enumerate() {
        let summary = library_summary(path);
        let (c, r) = (summary.counts, summary.readability.unwrap());
        assert_eq!(
            lines[4 * i + 1],
            format!(
                "  {} sentences, {} syllables, {} complex words",
                c.sentences, c.syllables, c.complex_words
            )
        );
        assert_eq!(
            lines[4 * i + 2],
            format!(
                "  Flesch Reading Ease {:.2}, Flesch-Kincaid Grade {:.2}, Gunning Fog {:.2}, average sentence length {:.2}",
                r.flesch_reading_ease,
                r.flesch_kincaid_grade,
                r.gunning_fog,
                r.average_sentence_length
            )
        );
    }
    // Witnesses: sample.md holds 4 sentences (paragraph, two tight items, blockquote); plain.txt
    // holds 2 hard-wrapped sentences of 9 syllables and no complex word.
    assert!(lines[1].starts_with("  4 sentences, "), "{stdout}");
    assert_eq!(lines[5], "  2 sentences, 9 syllables, 0 complex words");
}

/// Given a Markdown file holding the NOMZ sentence (4 nominalizations of 14 words)
/// When `vernier analyze` runs on it
/// Then its fourth line gives the nominalization ratio as the library renders it, `4 of 14`
/// words, and passive voice absent (no parse)
#[test]
fn analyze_prints_the_nominalization_ratio() {
    let path = fixture("nominal.md");
    let out = vernier(&["analyze"], &[&path]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8(out.stdout).unwrap();
    let expected = expected_line(&path);
    assert_eq!(stdout.lines().nth(3), expected.lines().nth(3), "{stdout}");
    let line = stdout.lines().nth(3).unwrap_or_default();
    assert!(line.contains("(4 of 14 words)"), "{line}");
    assert!(line.ends_with("passive voice absent (no parse)"), "{line}");
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
