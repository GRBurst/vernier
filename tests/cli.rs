//! End-to-end checks of the `vernier` binary (spec 001 M1, M2, M4, M5).

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

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

/// The notice `vernier` prints once per run without a model (M3b criterion 5).
const NO_MODEL: &str = "vernier: no --model-path given; syntactic metrics skipped";

/// Given two readable files and no `--model-path`
/// When `vernier analyze` and `vernier check --format compact` run on both
/// Then stderr holds the no-model notice exactly once, stdout is the surface output, and the
/// exit code is judged on surface rules
#[test]
fn no_model_prints_one_notice_and_keeps_stdout() {
    let (md, long) = (fixture("sample.md"), fixture("long.md"));
    let out = vernier(&["analyze"], &[&md, &long]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("{}\n{}\n", expected_line(&md), expected_line(&long))
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        format!("{NO_MODEL}\n")
    );
    let out = vernier(&["check", "--format", "compact"], &[&md, &long]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        format!("{NO_MODEL}\n")
    );
}

/// A usable `config.json` for a model directory whose other files are fakes.
const CONFIG: &str = r#"{
    "max_position_embeddings": 514,
    "pad_token_id": 1,
    "id2label": {"0": "-|_|dep", "1": "NOUN|_|nsubj", "2": "VERB|_|root", "3": "X|_|goeswith"}
}"#;

/// A usable word-level `tokenizer.json` with `<s>`, `</s>` and `<mask>`.
const TOKENIZER: &str = r#"{"version": "1.0", "truncation": null, "padding": null,
    "added_tokens": [], "normalizer": null, "pre_tokenizer": {"type": "Whitespace"},
    "post_processor": null, "decoder": null,
    "model": {"type": "WordLevel", "unk_token": "[UNK]",
              "vocab": {"<s>": 0, "[UNK]": 1, "</s>": 2, "<mask>": 3}}}"#;

/// A fresh model directory named `name` holding `files` (relative path, contents).
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn model_dir(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("cli-model-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("onnx")).unwrap();
    for (path, contents) in files {
        std::fs::write(dir.join(path), contents).unwrap();
    }
    dir
}

/// Runs `vernier` with `ORT_DYLIB_PATH` set to `runtime`.
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn vernier_with_runtime(args: &[&str], runtime: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vernier"))
        .args(args)
        .arg(fixture("sample.md"))
        .env("ORT_DYLIB_PATH", runtime)
        .output()
        .unwrap()
}

/// Asserts that every command in every format, with `--model-path dir` and the runtime
/// `runtime`, exits 2, prints nothing on stdout and names `named` on stderr.
fn assert_load_fails(dir: &Path, runtime: &Path, named: &Path) {
    let dir = dir.display().to_string();
    for command in ["analyze", "check"] {
        for format in ["text", "compact", "json"] {
            let args = [command, "--format", format, "--model-path", &dir];
            let out = vernier_with_runtime(&args, runtime);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(2), "{command} {format}: {stderr}");
            assert!(out.stdout.is_empty(), "{command} {format}");
            let expected = format!("vernier: cannot load the model {dir}: ");
            assert!(
                stderr.starts_with(&expected),
                "{command} {format}: {stderr}"
            );
            let named = named.display().to_string();
            assert!(stderr.contains(&named), "{command} {format}: {stderr}");
        }
    }
}

/// Given `--model-path` naming a directory that does not exist
/// When `vernier analyze` and `vernier check` run in each format
/// Then each names the directory on stderr, processes no file and exits 2 (M3b 6, M5 2)
#[test]
fn a_missing_model_is_named_and_exits_2() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-model-none");
    assert_load_fails(&dir, &dir.join("libonnxruntime.so"), &dir);
}

/// Given a model directory without `tokenizer.json`, and a runtime library that does not exist
/// When `vernier` runs with it
/// Then stderr names `tokenizer.json` (the files are checked before the runtime) and it exits 2
#[test]
fn a_model_without_its_tokenizer_is_named_and_exits_2() {
    let dir = model_dir(
        "no-tokenizer",
        &[("config.json", CONFIG), ("onnx/model.onnx", "not a model")],
    );
    assert_load_fails(
        &dir,
        &dir.join("no-such-lib.so"),
        &dir.join("tokenizer.json"),
    );
}

/// Given a model whose `max_position_embeddings` is `pad_token_id` + 4 (a limit of 0 subword
/// pieces), with every other file present and a runtime library that does not exist
/// When `vernier` runs with it
/// Then stderr names `config.json` as unusable, no file is processed and it exits 2 (M3b 1, 6)
#[test]
fn a_model_without_room_for_a_piece_is_named_and_exits_2() {
    let config = CONFIG.replace("514", "5");
    let dir = model_dir(
        "no-room",
        &[
            ("config.json", &config),
            ("tokenizer.json", TOKENIZER),
            ("onnx/model.onnx", "not a model"),
        ],
    );
    assert_load_fails(&dir, &dir.join("no-such-lib.so"), &dir.join("config.json"));
}

/// A shared library other than ONNX Runtime: one this test process has loaded (Linux), else a
/// file that is not a library at all.
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn other_library(dir: &Path) -> PathBuf {
    let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
    let loaded = maps
        .lines()
        .filter_map(|line| line.split_whitespace().nth(5))
        .find(|path| path.contains("/libc.so"));
    loaded.map_or_else(
        || {
            let fake = dir.join("not-a-library.so");
            std::fs::write(&fake, "not a library").unwrap();
            fake
        },
        PathBuf::from,
    )
}

/// Given a model directory with all three files, and `ORT_DYLIB_PATH` naming a library that does
/// not exist, then one that is not ONNX Runtime
/// When `vernier` runs with it
/// Then stderr names the library, it exits 2 and it does not panic
#[test]
fn an_unloadable_runtime_is_named_and_exits_2() {
    let dir = model_dir(
        "fake",
        &[
            ("config.json", CONFIG),
            ("tokenizer.json", TOKENIZER),
            ("onnx/model.onnx", "not a model"),
        ],
    );
    let missing = dir.join("no-such-libonnxruntime.so");
    assert_load_fails(&dir, &missing, &missing);
    let other = other_library(&dir);
    assert_load_fails(&dir, &other, &other);
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
/// the words and LongSentence and the syntactic metrics absent (no parse), then LongSentence's
/// advice as the one `= help:` line, and exits 1
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
    let helps: Vec<&str> = listed
        .iter()
        .copied()
        .filter(|l| l.starts_with("= help:"))
        .collect();
    assert_eq!(
        helps,
        ["= help: Split the sentence into shorter ones."],
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

/// Given a sentence hard-wrapped over the 14 lines 3 to 16 of `tall.md`, after a first paragraph
/// When `vernier check --max-sentence-len 5` runs
/// Then the diagnostic shows every one of those lines, each with its line number and whole text,
/// folds none away (`...`), shows no line outside the sentence, and closes the underline after
/// line 16 (M5 criterion 1: "over all its lines")
#[test]
fn check_shows_every_line_of_a_tall_sentence() {
    let path = fixture("tall.md");
    let source = std::fs::read_to_string(&path).unwrap();
    let (code, stdout) = check(&["--max-sentence-len", "5"], &[&path]);
    assert_eq!(code, Some(1));
    assert_eq!(
        stdout.lines().nth(1).map(str::trim),
        Some(format!("--> {}:3:15", path.display()).as_str())
    );
    let numbered: Vec<usize> = stdout
        .lines()
        .filter_map(|l| l.split_once(" |").and_then(|(n, _)| n.trim().parse().ok()))
        .collect();
    assert_eq!(numbered, (3..=16).collect::<Vec<_>>(), "{stdout}");
    for (n, text) in source.lines().enumerate().skip(2) {
        let shown = stdout
            .lines()
            .find(|l| l.trim_start().starts_with(&format!("{} |", n + 1)));
        assert!(shown.is_some_and(|l| l.ends_with(text)), "{n}: {stdout}");
    }
    assert!(!stdout.lines().any(|l| l.starts_with("...")), "{stdout}");
    let last = stdout
        .lines()
        .position(|l| l.trim_start().starts_with("16 |"))
        .unwrap();
    let closing = stdout.lines().nth(last + 1).unwrap().trim();
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

/// Runs `vernier` with its stdout on a pipe whose read end is already closed, so every write to
/// stdout fails (a broken pipe, as under `| head -1`).
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn vernier_into_a_closed_pipe(args: &[&str], files: &[&Path]) -> Output {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    Command::new(env!("CARGO_BIN_EXE_vernier"))
        .args(args)
        .args(files)
        .stdout(writer)
        .output()
        .unwrap()
}

/// Given the long-sentence file, which every command and format writes something for, and a
/// stdout whose reader has gone
/// When `analyze` and `check` run in each format
/// Then each exits 2 without panicking, and says so besides the no-model notice in exactly one
/// stderr line, `vernier: cannot write to stdout: <cause>` (M5 2)
#[test]
fn a_closed_stdout_exits_2_without_a_panic() {
    let long = fixture("long.md");
    let broken_pipe = std::io::Error::from_raw_os_error(EPIPE);
    let expected = format!("vernier: cannot write to stdout: {broken_pipe}");
    for command in ["analyze", "check"] {
        for format in ["text", "compact", "json"] {
            let out = vernier_into_a_closed_pipe(&[command, "--format", format], &[&long, &long]);
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(2), "{command} {format}: {stderr}");
            assert!(!stderr.contains("panicked"), "{command} {format}: {stderr}");
            let others: Vec<&str> = stderr.lines().filter(|line| *line != NO_MODEL).collect();
            assert_eq!(others, [expected.as_str()], "{command} {format}");
        }
    }
}

/// Linux's `EPIPE`: the error of a write to a pipe whose reader has gone.
const EPIPE: i32 = 32;

/// A stderr whose every write fails: a pipe whose reader has gone, and `/dev/full`.
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn failing_stderrs() -> [(&'static str, Stdio); 2] {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    [("closed pipe", writer.into()), ("/dev/full", full.into())]
}

/// Given a missing file, or the long-sentence file for `check`, and a stderr that refuses every
/// write (a pipe whose reader has gone, or `/dev/full`)
/// When `check`, `check --format json` and `analyze` run on it
/// Then none panics (exit 101): each exits with the code the other clauses give, 2 for the
/// missing file and 1 for the flagged one, and JSON still lists no file (M5 2)
#[test]
fn a_failed_stderr_write_keeps_the_exit_code() {
    let (gone, long) = (missing(), fixture("long.md"));
    let runs: [(&[&str], &Path, i32); 4] = [
        (&["check"], &gone, 2),
        (&["check", "--format", "json"], &gone, 2),
        (&["analyze"], &gone, 2),
        (&["check"], &long, 1),
    ];
    for (args, file, expected) in runs {
        for (stderr, sink) in failing_stderrs() {
            let out = Command::new(env!("CARGO_BIN_EXE_vernier"))
                .args(args)
                .arg(file)
                .stderr(sink)
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(expected), "{args:?} {stderr}");
            if args.contains(&"json") {
                let doc: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
                assert_eq!(doc["files"], serde_json::json!([]), "{stderr}");
            }
        }
    }
}

/// Given the long-sentence file, and stdout and stderr each closed (`>&-`, `2>&-`) rather than a
/// pipe whose reader has gone
/// When `check` runs on it in each format
/// Then it exits 1 as with a working stdout: the standard library reopens a closed standard
/// descriptor on `/dev/null`, so a closed descriptor reads as `/dev/null` (M5 2)
#[test]
fn a_closed_descriptor_reads_as_dev_null() {
    let long = fixture("long.md");
    for format in ["text", "compact", "json"] {
        for closed in [">&-", "2>&-", ">&- 2>&-"] {
            let out = Command::new("sh")
                .arg("-c")
                .arg(format!("exec \"$0\" \"$@\" {closed}"))
                .arg(env!("CARGO_BIN_EXE_vernier"))
                .args(["check", "--format", format])
                .arg(&long)
                .output()
                .unwrap();
            assert_eq!(out.status.code(), Some(1), "{format} {closed}");
        }
    }
}

/// Given command lines clap refuses: a command without files, an unknown format, a `--max-mdd`
/// of 0, an unknown flag, no command
/// When `vernier` runs with each
/// Then it exits 2, prints nothing on stdout and says why on stderr (M5 2)
#[test]
fn a_usage_error_exits_2() {
    let sample = fixture("sample.md");
    let sample = sample.to_str().unwrap_or_default();
    for args in [
        vec!["check"],
        vec!["analyze"],
        vec!["check", "--format", "xml", sample],
        vec!["check", "--max-mdd", "0", sample],
        vec!["analyze", "--no-such-flag", sample],
        vec![],
    ] {
        let out = vernier(&args, &[]);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(!out.stderr.is_empty(), "{args:?}");
    }
}
