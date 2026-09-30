//! The ONNX parser (spec 001 M3b): a Universal Dependencies model run by ONNX Runtime behind the
//! `Parser` seam. This is shell code — it reads the model's files, loads the runtime library and
//! runs inference; everything between the tokenizer and the tokens is the pure `decoder`.

use std::ops::Range;
use std::path::{Path, PathBuf};

use ort::session::Session;
use ort::value::{Outlet, Tensor, ValueType};
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
    #[error("the graph has an input \"{name}\" besides \"input_ids\" and \"attention_mask\"")]
    OtherInput { name: String },
    #[error("the graph has no output \"logits\"")]
    NoLogits,
    #[error("the graph's output \"logits\" is not a tensor")]
    LogitsNotTensor,
    #[error("the graph's \"logits\" has {width} values per piece, but there are {labels} labels")]
    Width { width: i64, labels: usize },
}

/// What a graph offers under the name `logits`: nothing, something other than a tensor, or a
/// tensor with these dimensions (`-1` where a dimension is dynamic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LogitsOutlet<'a> {
    Missing,
    NotTensor,
    Tensor(&'a [i64]),
}

/// The graph's `logits` output among `outputs`.
fn logits_outlet(outputs: &[Outlet]) -> LogitsOutlet<'_> {
    match outputs.iter().find(|outlet| outlet.name() == LOGITS) {
        None => LogitsOutlet::Missing,
        Some(outlet) => match outlet.dtype() {
            ValueType::Tensor { shape, .. } => LogitsOutlet::Tensor(shape),
            ValueType::Sequence(_) | ValueType::Map { .. } | ValueType::Optional(_) => {
                LogitsOutlet::NotTensor
            }
        },
    }
}

/// Whether a graph with these input names and this `logits` output fits `labels` labels: its
/// inputs are exactly `input_ids` and `attention_mask`, it has a tensor output `logits`, and the
/// last dimension of `logits` is the label count where the graph states it (a dynamic or unstated
/// last dimension is accepted here and checked on every parse).
fn check_graph(inputs: &[&str], logits: LogitsOutlet<'_>, labels: usize) -> Result<(), GraphError> {
    if let Some(name) = INPUTS.into_iter().find(|name| !inputs.contains(name)) {
        return Err(GraphError::NoInput { name });
    }
    if let Some(name) = inputs.iter().find(|name| !INPUTS.contains(name)) {
        return Err(GraphError::OtherInput {
            name: (*name).to_owned(),
        });
    }
    let dims = match logits {
        LogitsOutlet::Missing => return Err(GraphError::NoLogits),
        LogitsOutlet::NotTensor => return Err(GraphError::LogitsNotTensor),
        LogitsOutlet::Tensor(dims) => dims,
    };
    match dims.last() {
        Some(&width) if width >= 0 && usize::try_from(width) != Ok(labels) => {
            Err(GraphError::Width { width, labels })
        }
        Some(_) | None => Ok(()),
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
        if !model_dir.is_dir() {
            return Err(LoadError::MissingFile {
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
        let inputs: Vec<&str> = session.inputs().iter().map(Outlet::name).collect();
        check_graph(&inputs, logits_outlet(session.outputs()), labels.len()).map_err(
            |problem| LoadError::Graph {
                path: model,
                problem,
            },
        )?;
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
        let expected = [count, width, labels].map(|d| i64::try_from(d).unwrap_or(i64::MAX));
        if shape[..] != expected {
            return Err(ParseError::Output(format!(
                "logits of shape {:?}, expected {expected:?}",
                &shape[..]
            )));
        }
        Ok(values
            .chunks_exact(width * labels)
            .flat_map(|row| row.get(labels..(n + 1) * labels).unwrap_or_default())
            .copied()
            .collect())
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

fn existing(path: PathBuf) -> Result<PathBuf, LoadError> {
    if path.is_file() {
        Ok(path)
    } else {
        Err(LoadError::MissingFile { path })
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

    const BOTH: [&str; 2] = ["attention_mask", "input_ids"];

    /// Given the tested model's graph shape (inputs in either order, `logits` of
    /// `[batch, width, 2561]` with dynamic batch and width) and 2561 labels, or a dynamic or
    /// unstated label dimension
    /// When the graph is checked
    /// Then it fits (Definitions, *Model*)
    #[test]
    fn a_graph_with_the_contracts_inputs_and_width_fits() {
        for inputs in [INPUTS, BOTH] {
            for dims in [&[-1, -1, 2561][..], &[-1, -1, -1], &[]] {
                let logits = LogitsOutlet::Tensor(dims);
                assert_eq!(
                    check_graph(&inputs, logits, 2561),
                    Ok(()),
                    "{inputs:?} {dims:?}"
                );
            }
        }
    }

    /// Given a graph that breaks the contract in one way each: an input missing, an input
    /// besides the two, no `logits`, a `logits` that is not a tensor, a stated label dimension
    /// other than the label count
    /// When the graph is checked against 4 labels
    /// Then each is refused with its own reason (Definitions, *Model*; M3b criterion 6)
    #[test]
    fn a_graph_that_breaks_the_contract_is_refused() {
        let fitting = LogitsOutlet::Tensor(&[-1, -1, 4]);
        let cases = [
            (
                &["input_ids"][..],
                fitting,
                GraphError::NoInput {
                    name: "attention_mask",
                },
            ),
            (
                &["attention_mask"],
                fitting,
                GraphError::NoInput { name: "input_ids" },
            ),
            (
                &["input_ids", "attention_mask", "token_type_ids"],
                fitting,
                GraphError::OtherInput {
                    name: "token_type_ids".to_owned(),
                },
            ),
            (&INPUTS, LogitsOutlet::Missing, GraphError::NoLogits),
            (
                &INPUTS,
                LogitsOutlet::NotTensor,
                GraphError::LogitsNotTensor,
            ),
            (
                &INPUTS,
                LogitsOutlet::Tensor(&[1, 6, 2561]),
                GraphError::Width {
                    width: 2561,
                    labels: 4,
                },
            ),
            (
                &INPUTS,
                LogitsOutlet::Tensor(&[-1, -1, 0]),
                GraphError::Width {
                    width: 0,
                    labels: 4,
                },
            ),
        ];
        for (inputs, logits, expected) in cases {
            assert_eq!(
                check_graph(inputs, logits, 4),
                Err(expected.clone()),
                "{expected}"
            );
        }
    }
}
