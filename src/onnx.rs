//! The ONNX parser (spec 001 M3b): a Universal Dependencies model run by ONNX Runtime behind the
//! `Parser` seam. This is shell code — it reads the model's files, loads the runtime library and
//! runs inference; everything between the tokenizer and the tokens is the pure `decoder`.

use std::ops::Range;
use std::path::{Path, PathBuf};

use ort::session::Session;
use ort::value::{Outlet, Tensor, TensorElementType, ValueType};
use tokenizers::Tokenizer;

use crate::decoder::{
    DecodeError, LabelError, Labels, ModelLimits, Piece, Special, decode, fits, limits_from_config,
    masked_rows,
};
use crate::dependency::{Parse, Parser};

/// A model directory's files, as its Hugging Face repository is cloned.
const CONFIG: &str = "config.json";
const TOKENIZER: &str = "tokenizer.json";
const MODEL: &str = "onnx/model.onnx";

/// The graph's inputs and its output (Definitions, *Model*).
const INPUTS: [&str; 2] = ["input_ids", "attention_mask"];
const LOGITS: &str = "logits";

/// A loaded model: its session, tokenizer and label set.
pub struct OnnxParser {
    session: Session,
    tokenizer: Tokenizer,
    labels: Labels,
    limits: ModelLimits,
    special: Special,
}

/// Why a model could not be loaded; each names the file or library at fault.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("{} does not exist", path.display())]
    MissingFile { path: PathBuf },
    #[error("{} is not a directory", path.display())]
    NotADirectory { path: PathBuf },
    #[error("{} is not a file", path.display())]
    NotAFile { path: PathBuf },
    #[error("cannot read {}: {source}", path.display())]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("{} is not a usable model configuration: {source}", path.display())]
    Config { path: PathBuf, source: LabelError },
    #[error("{} is not a usable tokenizer: {message}", path.display())]
    Tokenizer { path: PathBuf, message: String },
    #[error("{} has no special token {name}", path.display())]
    SpecialToken { path: PathBuf, name: &'static str },
    #[error("cannot load ONNX Runtime: {source}")]
    Runtime { source: ort::LoadDynamicError },
    #[error("{} is not a usable ONNX model: {source}", path.display())]
    Session { path: PathBuf, source: ort::Error },
    #[error("{} does not match {CONFIG}: {problem}", path.display())]
    Graph { path: PathBuf, problem: GraphError },
}

/// Why a model's graph does not fit its `config.json` or the inputs vernier gives it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GraphError {
    #[error("the graph has no input \"{name}\"")]
    NoInput { name: &'static str },
    #[error("the graph's input \"{name}\" is {found}, not a tensor of i64")]
    InputType { name: &'static str, found: String },
    #[error("the graph has an input \"{name}\" besides \"input_ids\" and \"attention_mask\"")]
    OtherInput { name: String },
    #[error("the graph has no output \"logits\"")]
    NoLogits,
    #[error("the graph's output \"logits\" is {found}, not a tensor of f32")]
    LogitsType { found: String },
    #[error("the graph's \"logits\" has {width} values per piece, but there are {labels} labels")]
    Width { width: i64, labels: usize },
}

/// What a graph declares under one of its names: a tensor of an element type with these
/// dimensions (`-1` where a dimension is dynamic), or a value that is not a tensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Declared<'a> {
    Tensor(TensorElementType, &'a [i64]),
    NotTensor,
}

impl<'a> Declared<'a> {
    /// What `outlet` declares.
    fn of(outlet: &'a Outlet) -> Self {
        match outlet.dtype() {
            ValueType::Tensor { ty, shape, .. } => Self::Tensor(*ty, shape),
            ValueType::Sequence(_) | ValueType::Map { .. } | ValueType::Optional(_) => {
                Self::NotTensor
            }
        }
    }

    /// How an error names it: `a tensor of i32`, or `not a tensor`.
    fn describe(self) -> String {
        match self {
            Self::Tensor(ty, _) => format!("a tensor of {ty}"),
            Self::NotTensor => "not a tensor".to_owned(),
        }
    }
}

/// Whether a graph with these inputs and this `logits` output (`None` when it has none) fits
/// `labels` labels: its inputs are exactly `input_ids` and `attention_mask`, both tensors of
/// i64, it has an output `logits` that is a tensor of f32, and the last dimension of `logits` is
/// the label count where the graph states it (a dynamic or unstated last dimension is accepted
/// here and checked on every parse).
fn check_graph(
    inputs: &[(&str, Declared<'_>)],
    logits: Option<Declared<'_>>,
    labels: usize,
) -> Result<(), GraphError> {
    check_inputs(inputs)?;
    let dims = match logits {
        None => return Err(GraphError::NoLogits),
        Some(Declared::Tensor(TensorElementType::Float32, dims)) => dims,
        Some(other) => {
            return Err(GraphError::LogitsType {
                found: other.describe(),
            });
        }
    };
    match dims.last() {
        Some(&width) if width >= 0 && usize::try_from(width) != Ok(labels) => {
            Err(GraphError::Width { width, labels })
        }
        Some(_) | None => Ok(()),
    }
}

/// Whether the graph's inputs are exactly `input_ids` and `attention_mask`, both tensors of i64.
fn check_inputs(inputs: &[(&str, Declared<'_>)]) -> Result<(), GraphError> {
    for name in INPUTS {
        match inputs.iter().find(|(input, _)| *input == name) {
            None => return Err(GraphError::NoInput { name }),
            Some((_, Declared::Tensor(TensorElementType::Int64, _))) => {}
            Some((_, other)) => {
                return Err(GraphError::InputType {
                    name,
                    found: other.describe(),
                });
            }
        }
    }
    match inputs.iter().find(|(input, _)| !INPUTS.contains(input)) {
        Some((name, _)) => Err(GraphError::OtherInput {
            name: (*name).to_owned(),
        }),
        None => Ok(()),
    }
}

/// Why a sentence could not be parsed.
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("the tokenizer failed: {0}")]
    Tokenize(String),
    #[error("inference failed: {0}")]
    Inference(#[from] ort::Error),
    #[error("the model's output is unusable: {0}")]
    Output(String),
    #[error(transparent)]
    Decode(#[from] DecodeError),
}

impl OnnxParser {
    /// Loads the model in `model_dir` with the ONNX Runtime library at `runtime`. The model's
    /// files are checked first, so a missing file is named without the runtime being loaded;
    /// the graph is checked against `config.json` last, from the session's metadata.
    /// Call it at most once per process after a runtime failure: `ort` 2.0.0-rc.13 marks its
    /// library slot filled even when loading fails, so a second load would use an unset slot.
    pub fn load(model_dir: &Path, runtime: &Path) -> Result<Self, LoadError> {
        if !kind_of(model_dir.to_path_buf())?.is_dir() {
            return Err(LoadError::NotADirectory {
                path: model_dir.to_path_buf(),
            });
        }
        let (labels, limits) = read_config(model_dir.join(CONFIG))?;
        let (tokenizer, special) = read_tokenizer(model_dir.join(TOKENIZER))?;
        let model = existing(model_dir.join(MODEL))?;
        // why: `commit` is false only when an environment was committed before; that one stays.
        let _first = ort::init_from(runtime)
            .map_err(|source| LoadError::Runtime { source })?
            .commit();
        let session = Session::builder()
            .and_then(|mut builder| builder.commit_from_file(&model))
            .map_err(|source| LoadError::Session {
                path: model.clone(),
                source,
            })?;
        let inputs: Vec<(&str, Declared<'_>)> = session
            .inputs()
            .iter()
            .map(|outlet| (outlet.name(), Declared::of(outlet)))
            .collect();
        let logits = session
            .outputs()
            .iter()
            .find(|outlet| outlet.name() == LOGITS)
            .map(Declared::of);
        check_graph(&inputs, logits, labels.len()).map_err(|problem| LoadError::Graph {
            path: model,
            problem,
        })?;
        Ok(Self {
            session,
            tokenizer,
            labels,
            limits,
            special,
        })
    }

    /// The sentence's subword pieces, without the special tokens (they span no bytes).
    fn pieces(&self, sentence: &str) -> Result<Vec<Piece>, ParseError> {
        let encoding = self
            .tokenizer
            .encode(sentence, true)
            .map_err(|error| ParseError::Tokenize(error.to_string()))?;
        Ok(encoding
            .get_ids()
            .iter()
            .zip(encoding.get_offsets())
            .filter(|(_, (start, end))| start < end)
            .map(|(&id, &(start, end))| Piece {
                id: i64::from(id),
                range: start..end,
            })
            .collect())
    }

    /// The decoder's logits for `ids`: the masked batch, run `CHUNK_ROWS` rows at a time.
    fn logits(&mut self, ids: &[i64]) -> Result<Vec<f32>, ParseError> {
        let n = ids.len();
        let mut logits = Vec::with_capacity(n * n * self.labels.len());
        for start in (0..n).step_by(CHUNK_ROWS) {
            logits.extend(self.run_rows(ids, start..n.min(start + CHUNK_ROWS))?);
        }
        Ok(logits)
    }

    /// The logits of the batch rows `rows`, positions `1..=n` of each (`ud.py`'s
    /// `e.logits[:, 1:-2, :]`).
    fn run_rows(&mut self, ids: &[i64], rows: Range<usize>) -> Result<Vec<f32>, ParseError> {
        let (n, count, labels) = (ids.len(), rows.len(), self.labels.len());
        let batch = masked_rows(self.special, ids, rows);
        let width = batch.len() / count.max(1);
        let mask = vec![1_i64; batch.len()];
        let outputs = self.session.run(ort::inputs![
            "input_ids" => Tensor::from_array(([count, width], batch))?,
            "attention_mask" => Tensor::from_array(([count, width], mask))?,
        ])?;
        let output = outputs
            .get("logits")
            .ok_or_else(|| ParseError::Output("no \"logits\" output".to_string()))?;
        let (shape, values) = output.try_extract_tensor::<f32>()?;
        check_logits(shape, count, width, labels)?;
        Ok(values
            .chunks_exact(width * labels)
            .flat_map(|row| row.get(labels..(n + 1) * labels).unwrap_or_default())
            .copied()
            .collect())
    }
}

/// Checks that the logits of a run of `rows` rows of width `width` have the shape
/// `[rows, width, labels]`; checked on every run, so a graph whose label dimension is dynamic is
/// held to the number of labels here.
fn check_logits(shape: &[i64], rows: usize, width: usize, labels: usize) -> Result<(), ParseError> {
    let expected = [rows, width, labels].map(|d| i64::try_from(d).unwrap_or(i64::MAX));
    if shape == expected {
        Ok(())
    } else {
        Err(ParseError::Output(format!(
            "logits of shape {shape:?}, expected {expected:?}"
        )))
    }
}

/// Rows of the masked batch per inference run: memory grows with the rows times the square of
/// the row width, so a long sentence is run in chunks.
pub const CHUNK_ROWS: usize = 16;

impl Parser for OnnxParser {
    type Error = ParseError;

    fn parse(&mut self, sentence: &str) -> Result<Parse, ParseError> {
        let pieces = self.pieces(sentence)?;
        if !pieces.is_empty() && !fits(pieces.len(), self.limits.max_positions) {
            return Ok(Parse::TooLong {
                pieces: pieces.len(),
                max: self.limits.max_pieces(),
            });
        }
        let ids: Vec<i64> = pieces.iter().map(|piece| piece.id).collect();
        let logits = self.logits(&ids)?;
        Ok(Parse::Tokens(decode(
            logits,
            &self.labels,
            sentence,
            &pieces,
        )?))
    }
}

/// A path that must be a regular file: missing, not a file, or unreadable metadata each have
/// their own error.
fn existing(path: PathBuf) -> Result<PathBuf, LoadError> {
    if kind_of(path.clone())?.is_file() {
        Ok(path)
    } else {
        Err(LoadError::NotAFile { path })
    }
}

/// What `path` is (following symbolic links), or that it does not exist, or why it cannot be
/// looked at.
fn kind_of(path: PathBuf) -> Result<std::fs::FileType, LoadError> {
    match std::fs::metadata(&path) {
        Ok(metadata) => Ok(metadata.file_type()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            Err(LoadError::MissingFile { path })
        }
        Err(source) => Err(LoadError::Read { path, source }),
    }
}

fn read_config(path: PathBuf) -> Result<(Labels, ModelLimits), LoadError> {
    let path = existing(path)?;
    let json = std::fs::read_to_string(&path).map_err(|source| LoadError::Read {
        path: path.clone(),
        source,
    })?;
    let config = |source| LoadError::Config {
        path: path.clone(),
        source,
    };
    let labels = Labels::from_config(&json).map_err(config)?;
    let limits = limits_from_config(&json).map_err(config)?;
    Ok((labels, limits))
}

fn read_tokenizer(path: PathBuf) -> Result<(Tokenizer, Special), LoadError> {
    let path = existing(path)?;
    let tokenizer = Tokenizer::from_file(&path).map_err(|error| LoadError::Tokenizer {
        path: path.clone(),
        message: error.to_string(),
    })?;
    let id = |name: &'static str| {
        tokenizer
            .token_to_id(name)
            .map(i64::from)
            .ok_or_else(|| LoadError::SpecialToken {
                path: path.clone(),
                name,
            })
    };
    let special = Special {
        cls: id("<s>")?,
        sep: id("</s>")?,
        mask: id("<mask>")?,
    };
    Ok((tokenizer, special))
}

#[cfg(test)]
mod tests {
    use super::*;

    const I64: TensorElementType = TensorElementType::Int64;
    const F32: TensorElementType = TensorElementType::Float32;

    /// The inputs of the tested model: `input_ids` and `attention_mask`, tensors of i64 of
    /// dynamic batch and sequence.
    fn inputs(names: &[&'static str]) -> Vec<(&'static str, Declared<'static>)> {
        names
            .iter()
            .map(|&name| (name, Declared::Tensor(I64, &[-1, -1])))
            .collect()
    }

    /// Given the tested model's graph (i64 inputs in either order, `logits` a tensor of f32 of
    /// `[batch, sequence, 2561]` with dynamic batch and sequence) and 2561 labels, or a dynamic
    /// or unstated label dimension
    /// When the graph is checked
    /// Then it fits (Definitions, *Model*)
    #[test]
    fn a_graph_with_the_contracts_inputs_types_and_width_fits() {
        for names in [INPUTS, ["attention_mask", "input_ids"]] {
            for dims in [&[-1, -1, 2561][..], &[-1, -1, -1], &[]] {
                let logits = Some(Declared::Tensor(F32, dims));
                assert_eq!(
                    check_graph(&inputs(&names), logits, 2561),
                    Ok(()),
                    "{names:?} {dims:?}"
                );
            }
        }
    }

    /// Given a graph that breaks the contract in one way each: an input missing, an input
    /// besides the two, an input of another element type or not a tensor, no `logits`, a
    /// `logits` of another element type or not a tensor, a stated label dimension other than the
    /// label count
    /// When the graph is checked against 4 labels
    /// Then each is refused with its own reason (Definitions, *Model*; M3b criterion 6)
    #[test]
    fn a_graph_that_breaks_the_contract_is_refused() {
        let fitting = Some(Declared::Tensor(F32, &[-1, -1, 4]));
        let int32_ids = vec![
            (
                "input_ids",
                Declared::Tensor(TensorElementType::Int32, &[-1, -1]),
            ),
            ("attention_mask", Declared::Tensor(I64, &[-1, -1])),
        ];
        let listed_mask = vec![
            ("input_ids", Declared::Tensor(I64, &[-1, -1])),
            ("attention_mask", Declared::NotTensor),
        ];
        let cases = [
            (
                inputs(&["input_ids"]),
                fitting,
                GraphError::NoInput {
                    name: "attention_mask",
                },
            ),
            (
                inputs(&["attention_mask"]),
                fitting,
                GraphError::NoInput { name: "input_ids" },
            ),
            (
                inputs(&["input_ids", "attention_mask", "token_type_ids"]),
                fitting,
                GraphError::OtherInput {
                    name: "token_type_ids".to_owned(),
                },
            ),
            (
                int32_ids,
                fitting,
                GraphError::InputType {
                    name: "input_ids",
                    found: "a tensor of i32".to_owned(),
                },
            ),
            (
                listed_mask,
                fitting,
                GraphError::InputType {
                    name: "attention_mask",
                    found: "not a tensor".to_owned(),
                },
            ),
            (inputs(&INPUTS), None, GraphError::NoLogits),
            (
                inputs(&INPUTS),
                Some(Declared::Tensor(TensorElementType::Float16, &[-1, -1, 4])),
                GraphError::LogitsType {
                    found: "a tensor of f16".to_owned(),
                },
            ),
            (
                inputs(&INPUTS),
                Some(Declared::NotTensor),
                GraphError::LogitsType {
                    found: "not a tensor".to_owned(),
                },
            ),
            (
                inputs(&INPUTS),
                Some(Declared::Tensor(F32, &[1, 6, 2561])),
                GraphError::Width {
                    width: 2561,
                    labels: 4,
                },
            ),
            (
                inputs(&INPUTS),
                Some(Declared::Tensor(F32, &[-1, -1, 0])),
                GraphError::Width {
                    width: 0,
                    labels: 4,
                },
            ),
        ];
        for (inputs, logits, expected) in cases {
            assert_eq!(
                check_graph(&inputs, logits, 4),
                Err(expected.clone()),
                "{expected}"
            );
        }
    }

    /// Given a run of 3 rows of width 6 with 4 labels, and the shape `logits` came back in
    /// When the shape is checked, as on every parse
    /// Then only `[3, 6, 4]` passes: a wrong label dimension (what a graph with a dynamic one can
    /// return), a wrong row count or width, or a wrong rank each fail with the shape found and the
    /// shape expected (Definitions, *Model*)
    #[test]
    fn a_parse_checks_the_whole_shape_of_the_logits() {
        assert!(check_logits(&[3, 6, 4], 3, 6, 4).is_ok());
        for shape in [
            &[3, 6, 5][..],
            &[3, 6, 2561],
            &[2, 6, 4],
            &[3, 7, 4],
            &[1, 3, 6, 4],
            &[18, 4],
            &[],
        ] {
            match check_logits(shape, 3, 6, 4) {
                Err(ParseError::Output(said)) => assert_eq!(
                    said,
                    format!("logits of shape {shape:?}, expected [3, 6, 4]")
                ),
                other => panic!("{shape:?}: {other:?}"),
            }
        }
    }
}
