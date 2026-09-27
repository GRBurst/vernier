//! The ONNX parser's shell (spec 001 M3b): model-directory load errors without a model or a
//! runtime, and — with `VERNIER_TEST_MODEL` set to a model directory — a real parse.

use std::fs;
use std::path::{Path, PathBuf};

use vernier::decoder::LabelError;
use vernier::dependency::{DependencyTree, Parse, Parser};
use vernier::onnx::{LoadError, OnnxParser};

/// A usable `config.json`: four labels and RoBERTa's positions.
const CONFIG: &str = r#"{
    "max_position_embeddings": 514,
    "pad_token_id": 1,
    "id2label": {"0": "-|_|dep", "1": "NOUN|_|nsubj", "2": "VERB|_|root", "3": "X|_|goeswith"}
}"#;

/// A usable word-level `tokenizer.json` with the three special tokens, or without `<mask>`.
fn tokenizer(with_mask: bool) -> String {
    let mask = if with_mask { r#", "<mask>": 3"# } else { "" };
    format!(
        r#"{{"version": "1.0", "truncation": null, "padding": null, "added_tokens": [],
            "normalizer": null, "pre_tokenizer": {{"type": "Whitespace"}}, "post_processor": null,
            "decoder": null,
            "model": {{"type": "WordLevel", "unk_token": "[UNK]",
                       "vocab": {{"<s>": 0, "[UNK]": 1, "</s>": 2{mask}}}}}}}"#
    )
}

/// A fresh model directory named `name` holding `files` (relative path, contents).
// why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::unwrap_used)]
fn model_dir(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("onnx-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("onnx")).unwrap();
    for (path, contents) in files {
        fs::write(dir.join(path), contents).unwrap();
    }
    dir
}

/// A runtime library that does not exist: loading reaches it only after every file is checked.
fn no_runtime(dir: &Path) -> PathBuf {
    dir.join("no-such-libonnxruntime.so")
}

// why: a test helper; clippy's allow-panic-in-tests covers only `#[test]` items (audit 003).
#[allow(clippy::panic)]
fn load_error(dir: &Path) -> LoadError {
    match OnnxParser::load(dir, &no_runtime(dir)) {
        Ok(_) => panic!("{} loaded", dir.display()),
        Err(error) => error,
    }
}

/// Given a model directory that does not exist
/// When it is loaded
/// Then the error names the directory
#[test]
fn a_missing_model_directory_is_named() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("onnx-no-such-model");
    match load_error(&dir) {
        LoadError::MissingFile { path } => assert_eq!(path, dir),
        other => panic!("{other:?}"),
    }
}

/// Given a model directory in which one of the three files is missing, each in turn
/// When it is loaded (with a runtime that does not exist)
/// Then the error names the missing file, not the runtime
#[test]
fn each_missing_model_file_is_named_before_the_runtime() {
    let tokenizer = tokenizer(true);
    let all = [
        ("config.json", CONFIG),
        ("tokenizer.json", tokenizer.as_str()),
        ("onnx/model.onnx", "not a model"),
    ];
    for missing in 0..all.len() {
        let files: Vec<(&str, &str)> = (0..all.len())
            .filter(|&i| i != missing)
            .map(|i| all[i])
            .collect();
        let dir = model_dir(&format!("missing-{missing}"), &files);
        match load_error(&dir) {
            LoadError::MissingFile { path } => assert_eq!(path, dir.join(all[missing].0)),
            other => panic!("{}: {other:?}", all[missing].0),
        }
    }
}

/// Given a model directory whose `config.json` has no `id2label`
/// When it is loaded
/// Then the error names `config.json` and says what is wrong with it
#[test]
fn a_bad_config_is_named() {
    let tokenizer = tokenizer(true);
    let dir = model_dir(
        "bad-config",
        &[
            (
                "config.json",
                r#"{"max_position_embeddings": 514, "pad_token_id": 1}"#,
            ),
            ("tokenizer.json", &tokenizer),
            ("onnx/model.onnx", "not a model"),
        ],
    );
    match load_error(&dir) {
        LoadError::Config { path, source } => {
            assert_eq!(path, dir.join("config.json"));
            assert!(matches!(source, LabelError::NoId2Label), "{source:?}");
        }
        other => panic!("{other:?}"),
    }
}

/// Given a model directory whose `tokenizer.json` is not a tokenizer
/// When it is loaded
/// Then the error names `tokenizer.json`
#[test]
fn a_bad_tokenizer_is_named() {
    let dir = model_dir(
        "bad-tokenizer",
        &[
            ("config.json", CONFIG),
            ("tokenizer.json", "{}"),
            ("onnx/model.onnx", "not a model"),
        ],
    );
    match load_error(&dir) {
        LoadError::Tokenizer { path, .. } => assert_eq!(path, dir.join("tokenizer.json")),
        other => panic!("{other:?}"),
    }
}

/// Given a model directory whose tokenizer has `<s>` and `</s>` but no `<mask>`
/// When it is loaded
/// Then the error names the tokenizer and the missing `<mask>`
#[test]
fn a_tokenizer_without_mask_is_named() {
    let tokenizer = tokenizer(false);
    let dir = model_dir(
        "no-mask",
        &[
            ("config.json", CONFIG),
            ("tokenizer.json", &tokenizer),
            ("onnx/model.onnx", "not a model"),
        ],
    );
    match load_error(&dir) {
        LoadError::SpecialToken { path, name } => {
            assert_eq!(path, dir.join("tokenizer.json"));
            assert_eq!(name, "<mask>");
        }
        other => panic!("{other:?}"),
    }
}

/// The spec's example sentence (`tests/fixtures/sample.md` line 7).
const EXAMPLE: &str = "The proposal, which the executive committee rejected after extensive \
                       deliberation, caused significant delays.";

/// Given the model in `VERNIER_TEST_MODEL` and the runtime in `ORT_DYLIB_PATH`
/// When the example sentence is parsed
/// Then the tokens are its 16 words, they form a tree, and every word but the punctuation has
/// UDPipe's head (M3b criterion 3; the spike's parse)
#[test]
fn parses_the_example_into_a_tree() {
    let Some(model) = std::env::var_os("VERNIER_TEST_MODEL") else {
        println!("skipped: VERNIER_TEST_MODEL is not set");
        return;
    };
    let runtime = std::env::var_os("ORT_DYLIB_PATH")
        .filter(|path| !path.is_empty())
        .map_or_else(|| PathBuf::from("libonnxruntime.so"), PathBuf::from);
    let mut parser = OnnxParser::load(Path::new(&model), &runtime).unwrap();
    let Parse::Tokens(tokens) = parser.parse(EXAMPLE).unwrap() else {
        panic!("the example is too long");
    };
    let forms: Vec<&str> = tokens.iter().map(|t| t.form.as_str()).collect();
    let words = "The proposal , which the executive committee rejected after extensive \
                 deliberation , caused significant delays .";
    assert_eq!(forms, words.split(' ').collect::<Vec<_>>());
    let udpipe_heads = [2, 13, 8, 8, 7, 7, 8, 2, 11, 11, 8, 2, 0, 15, 13, 13];
    for (token, head) in tokens.iter().zip(udpipe_heads) {
        if token.upostag != "PUNCT" {
            assert_eq!(token.head, head, "{token:?}");
        }
    }
    assert!(DependencyTree::new(tokens).is_ok());
}
