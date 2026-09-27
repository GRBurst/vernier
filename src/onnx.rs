//! The ONNX parser (spec 001 M3b): a Universal Dependencies model run by ONNX Runtime behind the
//! `Parser` seam. This is shell code — it reads the model's files, loads the runtime library and
//! runs inference; everything between the tokenizer and the tokens is the pure `decoder`.

use std::ops::Range;
use std::path::{Path, PathBuf};

use ort::session::Session;
use ort::value::Tensor;
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
    /// files are checked first, so a missing file is named without the runtime being loaded.
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
                path: model,
                source,
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
