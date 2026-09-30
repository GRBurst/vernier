//! End-to-end checks of `vernier` with a real parser (spec 001 M3b criteria 3 and 4). They run
//! only with `VERNIER_TEST_MODEL` set to a model directory (and ONNX Runtime found through
//! `ORT_DYLIB_PATH` or the loader); otherwise each prints why it is skipped.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The model directory of `VERNIER_TEST_MODEL`, or `None` after printing why the test is skipped.
// why: a test helper that says why its test is skipped; allow-print-in-tests covers only
// `#[test]` items (audit 003).
#[allow(clippy::print_stdout)]
fn model() -> Option<OsString> {
    let model = std::env::var_os("VERNIER_TEST_MODEL");
    if model.is_none() {
        println!("skipped: VERNIER_TEST_MODEL is not set");
    }
    model
}

/// Runs `vernier` with `--model-path model` on `sample.md`.
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn vernier(args: &[&str], model: &OsString) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vernier"))
        .args(args)
        .arg("--model-path")
        .arg(model)
        .arg(fixture("sample.md"))
        .output()
        .unwrap()
}

/// Given the spec's example sentence at line 7 of `sample.md`, and the model
/// When `vernier check --format compact` runs on it
/// Then it flags the example's center-embedding — subject `proposal` 8 words from verb
/// `caused` — at `:7:1:` and exits 1 (M3b criterion 4)
#[test]
fn check_reports_the_examples_center_embedding() {
    let Some(model) = model() else { return };
    let out = vernier(&["check", "--format", "compact"], &model);
    let (stdout, stderr) = (
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    let expected =
        "CenterEmbedding: subject \"proposal\" separated from verb \"caused\" by 8 words";
    assert!(
        stdout
            .lines()
            .any(|line| line.contains(":7:1:") && line.contains(expected)),
        "{stdout}{stderr}"
    );
}

/// Given `sample.md` and the model
/// When `vernier analyze --format json` runs on it
/// Then the file's mean dependency distance and passive count are numbers, not `null`
/// (M3b criterion 3)
#[test]
fn analyze_json_fills_the_parse_metrics() {
    let Some(model) = model() else { return };
    let out = vernier(&["analyze", "--format", "json"], &model);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let document: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_default();
    let metrics = &document["files"][0]["metrics"];
    assert!(metrics["mean_dependency_distance"].is_number(), "{stdout}");
    assert!(metrics["passives"].is_u64(), "{stdout}");
}

/// A copy of the model in `model` under `CARGO_TARGET_TMPDIR` whose `config.json` keeps only
/// four labels, so it no longer matches the graph's `logits`; the other files are linked.
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn with_four_labels(model: &Path) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("model-four-labels");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("onnx")).unwrap();
    let config = std::fs::read_to_string(model.join("config.json")).unwrap();
    let mut config: serde_json::Value = serde_json::from_str(&config).unwrap();
    config["id2label"] = serde_json::json!(
        {"0": "-|_|dep", "1": "NOUN|_|nsubj", "2": "VERB|_|root", "3": "X|_|goeswith"}
    );
    std::fs::write(dir.join("config.json"), config.to_string()).unwrap();
    for file in ["tokenizer.json", "onnx/model.onnx"] {
        std::os::unix::fs::symlink(model.join(file).canonicalize().unwrap(), dir.join(file))
            .unwrap();
    }
    dir
}

/// Given a copy of the model whose `config.json` has four labels while its graph's `logits` has
/// one per label of the original, and `sample.md` with a missing file after it
/// When `vernier analyze` and `vernier check` run with it in each format
/// Then each says in one stderr line that the model directory cannot be loaded because the graph
/// does not match `config.json`, processes no file (nothing on stdout, neither file named) and
/// exits 2 (M3b criterion 6)
#[test]
fn a_model_whose_labels_do_not_match_its_graph_processes_no_file() {
    let Some(model) = model() else { return };
    let dir = with_four_labels(Path::new(&model));
    let gone = fixture("no-such-file.md");
    for command in ["analyze", "check"] {
        for format in ["text", "compact", "json"] {
            let out = Command::new(env!("CARGO_BIN_EXE_vernier"))
                .args([command, "--format", format, "--model-path"])
                .arg(&dir)
                .arg(fixture("sample.md"))
                .arg(&gone)
                .output()
                .unwrap();
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(2), "{command} {format}: {stderr}");
            assert!(out.stdout.is_empty(), "{command} {format}");
            let expected = format!(
                "vernier: cannot load the model {}: {} does not match config.json: ",
                dir.display(),
                dir.join("onnx/model.onnx").display()
            );
            assert_eq!(stderr.lines().count(), 1, "{command} {format}: {stderr}");
            assert!(
                stderr.starts_with(&expected),
                "{command} {format}: {stderr}"
            );
        }
    }
}
