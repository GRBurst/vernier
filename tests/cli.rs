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
    assert_eq!(lines.len(), 6, "{stdout}");
    for (i, path) in [&md, &txt].into_iter().enumerate() {
        let summary = library_summary(path);
        let (c, r) = (summary.counts, summary.readability.unwrap());
        assert_eq!(
            lines[3 * i + 1],
            format!(
                "  {} sentences, {} syllables, {} complex words",
                c.sentences, c.syllables, c.complex_words
            )
        );
        assert_eq!(
            lines[3 * i + 2],
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
    assert_eq!(lines[4], "  2 sentences, 9 syllables, 0 complex words");
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

/// Given a readable file and every M5 flag
/// When `vernier check` runs (no rule exists in M1)
/// Then it accepts the flags, flags nothing and exits 0
#[test]
fn check_accepts_every_m5_flag_and_passes_without_rules() {
    let out = vernier(
        &[
            "check",
            "--format",
            "compact",
            "--max-sentence-len",
            "10",
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
