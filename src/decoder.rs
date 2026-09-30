//! Decoding a goeswith UD parser's output (spec 001 M3b): everything between the tokenizer and
//! the tokens that needs no runtime — the label set, the length rule, the masked batch, and the
//! port of the model's `ud.py` from logits to tokens (label masks, the goeswith constraint,
//! Chu-Liu/Edmonds, the single-root fix, the subword merge).

use std::ops::Range;

use serde_json::{Map, Value};

use crate::dependency::Token;
use crate::mst::{argmax, chu_liu_edmonds};

/// What a label may mark: nothing (label 0), a root (on the diagonal), or a relation (off it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    Never,
    Root,
    Relation,
}

/// The model's labels (`UPOS|FEATS|DEPREL`), from `config.json`'s `id2label`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    names: Vec<String>,
    kinds: Vec<LabelKind>,
    goeswith: usize,
}

/// Why `config.json` does not describe a usable model.
#[derive(Debug, thiserror::Error)]
pub enum LabelError {
    #[error("not JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no \"id2label\" object")]
    NoId2Label,
    #[error("\"id2label\" has no label {id}")]
    Gap { id: usize },
    #[error("no \"X|_|goeswith\" label")]
    NoGoeswith,
    #[error("no \"…|root\" label")]
    NoRoot,
    #[error("no relation label besides goeswith")]
    NoRelation,
    #[error("the label {label} is not UPOS|…|DEPREL")]
    Malformed { label: String },
    #[error("no non-negative integer \"{key}\"")]
    MissingKey { key: &'static str },
    #[error(
        "\"max_position_embeddings\" {positions} leaves no subword piece after \"pad_token_id\" \
         {pad}: it must be greater than \"pad_token_id\" + 4"
    )]
    NoPositions { positions: usize, pad: usize },
}

impl Labels {
    /// The labels of a model's `config.json`.
    pub fn from_config(json: &str) -> Result<Self, LabelError> {
        let config: Value = serde_json::from_str(json)?;
        let id2label = config
            .get("id2label")
            .and_then(Value::as_object)
            .ok_or(LabelError::NoId2Label)?;
        let names = (0..id2label.len())
            .map(|id| label_name(id2label, id))
            .collect::<Result<Vec<String>, LabelError>>()?;
        let goeswith = names
            .iter()
            .position(|name| name == GOESWITH)
            .ok_or(LabelError::NoGoeswith)?;
        let kinds: Vec<LabelKind> = (0..names.len()).map(|id| kind(id, &names[id])).collect();
        if !kinds.contains(&LabelKind::Root) {
            return Err(LabelError::NoRoot);
        }
        let is_other_relation = |id: usize| kinds[id] == LabelKind::Relation && id != goeswith;
        if !(0..names.len()).any(is_other_relation) {
            return Err(LabelError::NoRelation);
        }
        Ok(Self {
            names,
            kinds,
            goeswith,
        })
    }

    /// The number of labels.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether there is no label (never, for labels built by `from_config`).
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// The goeswith label `ud.py` constrains.
const GOESWITH: &str = "X|_|goeswith";

/// Label `id` of `id2label`, which must have at least two `|`-separated fields.
fn label_name(id2label: &Map<String, Value>, id: usize) -> Result<String, LabelError> {
    let value = id2label
        .get(&id.to_string())
        .ok_or(LabelError::Gap { id })?;
    match value.as_str() {
        Some(name) if name.contains('|') => Ok(name.to_owned()),
        Some(name) => Err(LabelError::Malformed {
            label: name.to_owned(),
        }),
        None => Err(LabelError::Malformed {
            label: value.to_string(),
        }),
    }
}

/// Label 0 marks nothing, a `…|root` label a root, any other a relation.
fn kind(id: usize, name: &str) -> LabelKind {
    if id == 0 {
        LabelKind::Never
    } else if name.ends_with("|root") {
        LabelKind::Root
    } else {
        LabelKind::Relation
    }
}

/// How many positions a model's input row may have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelLimits {
    /// `max_position_embeddings − pad_token_id − 1`: RoBERTa's position ids start after the pad id.
    pub max_positions: usize,
}

impl ModelLimits {
    /// The most subword pieces a sentence may have to be parsed (see `fits`).
    pub fn max_pieces(&self) -> usize {
        self.max_positions.saturating_sub(ROW_EXTRA)
    }
}

/// The input limits of a model's `config.json`, which must leave room for at least one subword
/// piece: `max_position_embeddings` > `pad_token_id` + 4.
pub fn limits_from_config(json: &str) -> Result<ModelLimits, LabelError> {
    let config: Value = serde_json::from_str(json)?;
    let positions = count(&config, "max_position_embeddings")?;
    let pad = count(&config, "pad_token_id")?;
    let max_positions = positions
        .checked_sub(pad + 1)
        .filter(|&max| max > ROW_EXTRA)
        .ok_or(LabelError::NoPositions { positions, pad })?;
    Ok(ModelLimits { max_positions })
}

fn count(config: &Value, key: &'static str) -> Result<usize, LabelError> {
    config
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or(LabelError::MissingKey { key })
}

/// Whether a sentence of `pieces` subword pieces can be parsed: its batch rows
/// (`<s> p₁…pₙ </s> pᵢ`) have `pieces + 3` positions.
pub fn fits(pieces: usize, max_positions: usize) -> bool {
    pieces > 0 && pieces + ROW_EXTRA <= max_positions
}

/// A row's positions besides the pieces: `<s>`, `</s>` and the repeated piece.
const ROW_EXTRA: usize = 3;

/// The tokenizer's special token ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Special {
    pub cls: i64,
    pub sep: i64,
    pub mask: i64,
}

/// The model input for `pieces`: one row per piece, row `i` = `<s> p₁ … <mask> … pₙ </s> pᵢ`
/// with piece `i` masked; `n` rows of `n + 3` ids, row-major.
pub fn masked_batch(special: Special, pieces: &[i64]) -> Vec<i64> {
    masked_rows(special, pieces, 0..pieces.len())
}

/// The rows `rows` of `masked_batch(special, pieces)`.
pub fn masked_rows(special: Special, pieces: &[i64], rows: Range<usize>) -> Vec<i64> {
    rows.flat_map(|i| {
        std::iter::once(special.cls)
            .chain(pieces[..i].iter().copied())
            .chain(std::iter::once(special.mask))
            .chain(pieces[i + 1..].iter().copied())
            .chain([special.sep, pieces[i]])
    })
    .collect()
}

/// One subword piece: its token id and its byte range in the sentence (`start < end`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Piece {
    pub id: i64,
    pub range: Range<usize>,
}

/// Why logits could not be decoded into tokens.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    #[error("the sentence has no subword pieces")]
    Empty,
    #[error("expected {expected} logits, got {got}")]
    Shape { expected: usize, got: usize },
    #[error("the bytes {range:?} are not a slice of the sentence")]
    Offset { range: Range<usize> },
}

/// The tokens of `sentence` from the model's `logits` for its `pieces`:
/// `logits[(i·n + j)·L + l]` is label `l`'s score for piece `i` heading piece `j` (`i == j`:
/// `j` is the root), `n` pieces and `L` labels (`ud.py`'s `e.logits[:, 1:-2, :]`).
pub fn decode(
    logits: Vec<f32>,
    labels: &Labels,
    sentence: &str,
    pieces: &[Piece],
) -> Result<Vec<Token>, DecodeError> {
    let n = pieces.len();
    if n == 0 {
        return Err(DecodeError::Empty);
    }
    let expected = n * n * labels.len();
    if logits.len() != expected {
        return Err(DecodeError::Shape {
            expected,
            got: logits.len(),
        });
    }
    let parse = parse_pieces(Scores::new(logits, n, labels.len()), labels);
    tokens(&merge_subwords(&parse, labels), labels, sentence, pieces)
}

/// Label scores of every (head, dependent) pair of `n` pieces over `labels` labels.
#[derive(Debug, Clone)]
struct Scores {
    values: Vec<f32>,
    n: usize,
    labels: usize,
}

impl Scores {
    fn new(values: Vec<f32>, n: usize, labels: usize) -> Self {
        Self { values, n, labels }
    }

    fn cell(&self, head: usize, dep: usize) -> &[f32] {
        let at = (head * self.n + dep) * self.labels;
        &self.values[at..at + self.labels]
    }

    fn cell_mut(&mut self, head: usize, dep: usize) -> &mut [f32] {
        let at = (head * self.n + dep) * self.labels;
        &mut self.values[at..at + self.labels]
    }

    /// Each pair's best label score (`ud.py`'s `m`).
    fn best(&self) -> Vec<Vec<f32>> {
        (0..self.n)
            .map(|head| {
                (0..self.n)
                    .map(|dep| max_of(self.cell(head, dep)))
                    .collect()
            })
            .collect()
    }
}

fn max_of(values: &[f32]) -> f32 {
    values.iter().copied().fold(f32::NEG_INFINITY, f32::max)
}

/// The decoded tree over the pieces: `heads[j]` heads piece `j` (`heads[j] == j`: the root),
/// with the label `labels[j]`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PieceParse {
    heads: Vec<usize>,
    labels: Vec<usize>,
}

/// The tree over the pieces: masks, the goeswith constraint, Chu-Liu/Edmonds, one root, and
/// each piece's best label under its head.
fn parse_pieces(mut scores: Scores, labels: &Labels) -> PieceParse {
    mask_labels(&mut scores, labels);
    goeswith_constraint(&mut scores, labels.goeswith);
    let heads = single_root(scores.best());
    let labels = heads
        .iter()
        .enumerate()
        .map(|(dep, &head)| argmax(scores.cell(head, dep).iter().copied()))
        .collect();
    PieceParse { heads, labels }
}

/// Root labels only on the diagonal, relation labels only off it, label 0 never.
fn mask_labels(scores: &mut Scores, labels: &Labels) {
    for head in 0..scores.n {
        for dep in 0..scores.n {
            let cell = scores.cell_mut(head, dep);
            for (score, kind) in cell.iter_mut().zip(&labels.kinds) {
                let allowed = match kind {
                    LabelKind::Never => false,
                    LabelKind::Root => head == dep,
                    LabelKind::Relation => head != dep,
                };
                if !allowed {
                    *score = f32::NEG_INFINITY;
                }
            }
        }
    }
}

/// Goeswith only from a head to its right neighbour, or further right through pieces that
/// already prefer goeswith from the same head (`ud.py`'s `r`); every other goeswith is masked.
fn goeswith_constraint(scores: &mut Scores, goeswith: usize) {
    let best = scores.best();
    let n = scores.n;
    for head in 0..n {
        let blocked = blocked_goeswith(scores, &best, goeswith, head);
        for (dep, is_blocked) in blocked.into_iter().enumerate() {
            if is_blocked {
                scores.cell_mut(head, dep)[goeswith] = f32::NEG_INFINITY;
            }
        }
    }
}

/// `ud.py`'s row `r[head]`: blocked at and left of `head`, open at `head + 1`, and open further
/// right only while the previous piece's best label is goeswith and its best head is `head`.
fn blocked_goeswith(scores: &Scores, best: &[Vec<f32>], goeswith: usize, head: usize) -> Vec<bool> {
    let n = scores.n;
    let mut row: Vec<bool> = (0..n).map(|dep| dep <= head).collect();
    for dep in head + 2..n {
        let previous = dep - 1;
        let chains = argmax(scores.cell(head, previous).iter().copied()) == goeswith
            && argmax(best.iter().map(|scores| scores[previous])) == head;
        row[dep] = !chains || row[previous];
    }
    row
}

/// The maximum spanning tree of `best` with exactly one root: `ud.py`'s fix and, where that
/// still leaves several roots (it can: a node that was no root may become one), a strict pass
/// where only the best former root may be a root.
fn single_root(best: Vec<Vec<f32>>) -> Vec<usize> {
    let heads = chu_liu_edmonds(&best);
    let roots = roots_of(&heads);
    if roots.len() <= 1 {
        return heads;
    }
    let keep = roots[argmax(roots.iter().map(|&r| best[r][r]))];
    let fixed = penalize_roots(best, &roots, keep);
    let heads = chu_liu_edmonds(&fixed);
    if roots_of(&heads).len() <= 1 {
        return heads;
    }
    only_root(fixed, keep)
}

/// `ud.py`'s single-root fix: the former `roots` may only be headed by each other, and only
/// `keep` keeps its root score; every other arc into them loses the matrix's range.
fn penalize_roots(mut best: Vec<Vec<f32>>, roots: &[usize], keep: usize) -> Vec<Vec<f32>> {
    let penalty = min_of(&best) - max_of_matrix(&best);
    for (head, row) in best.iter_mut().enumerate() {
        for &dep in roots {
            if !(roots.contains(&head) && (dep != head || dep == keep)) {
                row[dep] += penalty;
            }
        }
    }
    best
}

/// The maximum spanning tree when no piece but `keep` may be a root. why: a finite penalty
/// cannot rule out a second root; an infinite one can, and every column keeps a finite score
/// (off the diagonal, or `keep`'s root score).
fn only_root(mut best: Vec<Vec<f32>>, keep: usize) -> Vec<usize> {
    for (node, row) in best.iter_mut().enumerate() {
        if node != keep {
            row[node] = f32::NEG_INFINITY;
        }
    }
    chu_liu_edmonds(&best)
}

fn roots_of(heads: &[usize]) -> Vec<usize> {
    (0..heads.len())
        .filter(|&node| heads[node] == node)
        .collect()
}

fn min_of(matrix: &[Vec<f32>]) -> f32 {
    matrix
        .iter()
        .flatten()
        .copied()
        .fold(f32::INFINITY, f32::min)
}

fn max_of_matrix(matrix: &[Vec<f32>]) -> f32 {
    matrix
        .iter()
        .flatten()
        .copied()
        .fold(f32::NEG_INFINITY, f32::max)
}

/// A word: a run of pieces, its head word (`head == index`: the root) and its label.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Word {
    pieces: Range<usize>,
    head: usize,
    label: usize,
}

/// The words of `parse`: `ud.py`'s `aggregation_strategy="simple"` merge, from the last piece
/// back — a goeswith piece whose head is to its left, with only goeswith pieces after that
/// head, joins the previous piece, and every head index after it moves down by one.
fn merge_subwords(parse: &PieceParse, labels: &Labels) -> Vec<Word> {
    let mut words: Vec<Word> = (0..parse.heads.len())
        .map(|piece| Word {
            pieces: piece..piece + 1,
            head: parse.heads[piece],
            label: parse.labels[piece],
        })
        .collect();
    for i in (1..words.len()).rev() {
        if joins_previous(&words, i, labels) {
            let joined = words.remove(i);
            words[i - 1].pieces.end = joined.pieces.end;
            for word in &mut words {
                word.head -= usize::from(word.head >= i);
            }
        }
    }
    words
}

fn joins_previous(words: &[Word], i: usize, labels: &Labels) -> bool {
    let head = words[i].head;
    head < i
        && words[head + 1..=i]
            .iter()
            .all(|w| labels.is_goeswith(w.label))
}

impl Labels {
    /// Whether label `id`'s relation is `goeswith` (`ud.py` reads the last field).
    fn is_goeswith(&self, id: usize) -> bool {
        self.names[id].rsplit('|').next() == Some("goeswith")
    }
}

/// The CoNLL-U tokens of `words`: the form is the sentence's bytes from the first piece's start
/// to the last piece's end, the lemma `_`, UPOS and DEPREL the label's first and last fields.
fn tokens(
    words: &[Word],
    labels: &Labels,
    sentence: &str,
    pieces: &[Piece],
) -> Result<Vec<Token>, DecodeError> {
    words
        .iter()
        .enumerate()
        .map(|(index, word)| {
            let range =
                pieces[word.pieces.start].range.start..pieces[word.pieces.end - 1].range.end;
            let form = sentence
                .get(range.clone())
                .ok_or(DecodeError::Offset { range })?;
            let name = &labels.names[word.label];
            let fields = || name.split('|');
            Ok(Token {
                id: index + 1,
                form: form.to_owned(),
                lemma: "_".to_owned(),
                upostag: fields().next().unwrap_or_default().to_owned(),
                head: if word.head == index { 0 } else { word.head + 1 },
                deprel: fields().next_back().unwrap_or_default().to_owned(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dependency::DependencyTree;
    use proptest::prelude::*;

    /// A small label set: label 0, two root labels, two relations, goeswith.
    const CONFIG: &str = r#"{
        "max_position_embeddings": 514,
        "pad_token_id": 1,
        "id2label": {
            "0": "-|_|dep",
            "1": "NOUN|_|root",
            "2": "VERB|Mood=Ind|root",
            "3": "NOUN|Number=Sing|nsubj",
            "4": "ADJ|_|amod",
            "5": "X|_|goeswith"
        }
    }"#;
    const ROOT: usize = 1;
    const NSUBJ: usize = 3;
    const GOESWITH: usize = 5;

    fn labels() -> Labels {
        Labels::from_config(CONFIG).unwrap()
    }

    fn special() -> Special {
        Special {
            cls: 0,
            sep: 2,
            mask: 50264,
        }
    }

    /// Given the configuration of the plan's witness
    /// When its labels are read
    /// Then the kinds are label 0, relation, root, relation and goeswith is label 3
    #[test]
    fn reads_the_label_kinds_and_goeswith() {
        let config = r#"{"id2label": {"0": "-|_|dep", "1": "NOUN|_|nsubj", "2": "VERB|_|root",
            "3": "X|_|goeswith"}}"#;
        let labels = Labels::from_config(config).unwrap();
        assert_eq!(
            labels.kinds,
            [
                LabelKind::Never,
                LabelKind::Relation,
                LabelKind::Root,
                LabelKind::Relation
            ]
        );
        assert_eq!(labels.goeswith, 3);
        assert_eq!(labels.len(), 4);
    }

    /// Given configurations that are not JSON, lack `id2label`, have a gap, lack goeswith, a
    /// root or another relation, or hold a label without `|`
    /// When their labels are read
    /// Then each fails with its own error
    #[test]
    fn refuses_unusable_label_sets() {
        type Check = fn(&LabelError) -> bool;
        let cases: [(&str, Check); 7] = [
            ("{", |e| matches!(e, LabelError::Json(_))),
            ("{}", |e| matches!(e, LabelError::NoId2Label)),
            (
                r#"{"id2label": {"0": "-|_|dep", "1": "A|_|root", "3": "X|_|goeswith"}}"#,
                |e| matches!(e, LabelError::Gap { id: 2 }),
            ),
            (
                r#"{"id2label": {"0": "-|_|dep", "1": "A|_|root", "2": "B|_|obj"}}"#,
                |e| matches!(e, LabelError::NoGoeswith),
            ),
            (
                r#"{"id2label": {"0": "-|_|dep", "1": "B|_|obj", "2": "X|_|goeswith"}}"#,
                |e| matches!(e, LabelError::NoRoot),
            ),
            (
                r#"{"id2label": {"0": "-|_|dep", "1": "A|_|root", "2": "X|_|goeswith"}}"#,
                |e| matches!(e, LabelError::NoRelation),
            ),
            (
                r#"{"id2label": {"0": "-|_|dep", "1": "root", "2": "X|_|goeswith"}}"#,
                |e| matches!(e, LabelError::Malformed { label } if label == "root"),
            ),
        ];
        for (config, expected) in cases {
            let result = Labels::from_config(config);
            assert!(result.as_ref().is_err_and(expected), "{config}: {result:?}");
        }
    }

    /// Given the model's `max_position_embeddings` 514 and `pad_token_id` 1, and broken limits
    /// When the limits are read
    /// Then 512 positions remain; a missing key or no piece left is refused
    #[test]
    fn reads_the_models_position_limit() {
        assert_eq!(
            limits_from_config(CONFIG).unwrap(),
            ModelLimits { max_positions: 512 }
        );
        assert!(matches!(
            limits_from_config(r#"{"pad_token_id": 1}"#),
            Err(LabelError::MissingKey {
                key: "max_position_embeddings"
            })
        ));
        assert!(matches!(
            limits_from_config(r#"{"max_position_embeddings": 2, "pad_token_id": -1}"#),
            Err(LabelError::MissingKey {
                key: "pad_token_id"
            })
        ));
        assert!(matches!(
            limits_from_config(r#"{"max_position_embeddings": 2, "pad_token_id": 1}"#),
            Err(LabelError::NoPositions {
                positions: 2,
                pad: 1
            })
        ));
    }

    /// Given `max_position_embeddings` from 0 to 11 above `pad_token_id` (0 to 3)
    /// When the limits are read
    /// Then they are accepted exactly when `max_position_embeddings` > `pad_token_id` + 4, and
    /// the limit is then `max_position_embeddings − pad_token_id − 4`, at least 1; a model
    /// whose limit would be 0 or less is unusable (Definitions, *Model*; M3b criterion 1)
    #[test]
    fn a_limit_below_one_piece_is_refused() {
        for pad in 0..4_usize {
            for positions in pad..pad + 12 {
                let json =
                    format!(r#"{{"max_position_embeddings": {positions}, "pad_token_id": {pad}}}"#);
                match limits_from_config(&json) {
                    Ok(limits) => {
                        assert!(positions > pad + 4, "{positions} {pad}");
                        assert_eq!(limits.max_pieces(), positions - pad - 4);
                        assert!(limits.max_pieces() >= 1);
                    }
                    Err(LabelError::NoPositions { .. }) => {
                        assert!(positions <= pad + 4, "{positions} {pad}");
                    }
                    Err(other) => panic!("{positions} {pad}: {other:?}"),
                }
            }
        }
    }

    /// Given the model's 512 positions
    /// When the length rule is applied to 509 and to 510 pieces
    /// Then 509 fits and 510 does not, and 509 is the most pieces the model takes
    #[test]
    fn the_models_limit_is_509_pieces() {
        assert!(fits(509, 512));
        assert!(!fits(510, 512));
        assert_eq!(ModelLimits { max_positions: 512 }.max_pieces(), 509);
    }

    /// Scores of `n` pieces, all `-5` but the given `(head, dep, label)` cells at `10`.
    fn logits(n: usize, high: &[(usize, usize, usize)]) -> Vec<f32> {
        let width = labels().len();
        let mut logits = vec![-5.0; n * n * width];
        for &(head, dep, label) in high {
            logits[(head * n + dep) * width + label] = 10.0;
        }
        logits
    }

    fn pieces_of(ranges: &[Range<usize>]) -> Vec<Piece> {
        ranges
            .iter()
            .map(|range| Piece {
                id: 7,
                range: range.clone(),
            })
            .collect()
    }

    /// Given `unbelievable` as the pieces `un`, `believ`, `able`, the first a root, the second
    /// goeswith headed by the first, the third goeswith headed by the second
    /// When the logits are decoded
    /// Then one token `unbelievable` remains, the root, with the root label's UPOS
    #[test]
    fn merges_goeswith_pieces_into_one_word() {
        let sentence = "unbelievable";
        let pieces = pieces_of(&[0..2, 2..8, 8..12]);
        let logits = logits(3, &[(0, 0, ROOT), (0, 1, GOESWITH), (1, 2, GOESWITH)]);
        let tokens = decode(logits, &labels(), sentence, &pieces).unwrap();
        let expected = Token {
            id: 1,
            form: "unbelievable".to_owned(),
            lemma: "_".to_owned(),
            upostag: "NOUN".to_owned(),
            head: 0,
            deprel: "root".to_owned(),
        };
        assert_eq!(tokens, [expected]);
    }

    /// Given four one-letter pieces whose greedy heads form the cycle 0 → 1 → 3 → 0 (piece 0
    /// heads piece 1 by goeswith) and piece 0 heads piece 2 by goeswith, so that Chu-Liu/Edmonds
    /// breaks the cycle by making piece 1 the root
    /// When the logits are decoded
    /// Then piece 2 stays its own word (a goeswith of piece 0), because piece 1 between them is
    /// no goeswith: `ud.py` merges only across goeswith pieces
    #[test]
    fn a_goeswith_piece_after_a_non_goeswith_piece_stays_a_word() {
        let sentence = "abcd";
        let pieces = pieces_of(&[0..1, 1..2, 2..3, 3..4]);
        let mut logits = logits(
            4,
            &[
                (3, 0, NSUBJ),
                (0, 1, GOESWITH),
                (0, 2, GOESWITH),
                (1, 3, NSUBJ),
            ],
        );
        logits[(4 + 1) * CONFIG_LABELS + ROOT] = 9.0;
        let tokens = decode(logits, &labels(), sentence, &pieces).unwrap();
        let summary: Vec<(&str, usize, &str)> = tokens
            .iter()
            .map(|t| (t.form.as_str(), t.head, t.deprel.as_str()))
            .collect();
        assert_eq!(
            summary,
            [
                ("a", 4, "nsubj"),
                ("b", 0, "root"),
                ("c", 1, "goeswith"),
                ("d", 2, "nsubj")
            ]
        );
    }

    /// Given `Ann sleeps`, `sleeps` the root and `Ann` its nsubj
    /// When the logits are decoded
    /// Then the tokens are `Ann` (NOUN, nsubj of 2) and `sleeps` (the root)
    #[test]
    fn decodes_a_two_word_sentence() {
        let sentence = "Ann sleeps";
        let pieces = pieces_of(&[0..3, 4..10]);
        let logits = logits(2, &[(1, 1, 2), (1, 0, NSUBJ)]);
        let tokens = decode(logits, &labels(), sentence, &pieces).unwrap();
        let summary: Vec<(&str, &str, usize, &str)> = tokens
            .iter()
            .map(|t| {
                (
                    t.form.as_str(),
                    t.upostag.as_str(),
                    t.head,
                    t.deprel.as_str(),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [("Ann", "NOUN", 2, "nsubj"), ("sleeps", "VERB", 0, "root")]
        );
    }

    /// Given no pieces, logits of the wrong length, and a piece that splits a character
    /// When they are decoded
    /// Then each fails with its own error
    #[test]
    fn refuses_inputs_it_cannot_decode() {
        let labels = labels();
        assert_eq!(
            decode(Vec::new(), &labels, "", &[]),
            Err(DecodeError::Empty)
        );
        let pieces = pieces_of(&[0..1, 1..2]);
        assert_eq!(
            decode(vec![0.0; 5], &labels, "ab", &pieces),
            Err(DecodeError::Shape {
                expected: 24,
                got: 5
            })
        );
        let pieces = [Piece { id: 7, range: 0..1 }];
        assert_eq!(
            decode(logits(1, &[(0, 0, ROOT)]), &labels, "é", &pieces),
            Err(DecodeError::Offset { range: 0..1 })
        );
    }

    /// Given head scores whose maximum spanning forest has the roots 1, 3 and 4, where
    /// `ud.py`'s fix (keep 3, the best root score) still leaves two roots, 2 and 3
    /// When one root is enforced
    /// Then the result is a tree with the single root 3
    #[test]
    fn enforces_one_root_where_the_ud_py_fix_leaves_two() {
        let best = vec![
            vec![0.0, 0.0, 3.0, 3.0, 2.0],
            vec![4.0, 11.245_302, 9.0, 1.0, 5.0],
            vec![5.0, 1.0, 9.0, 4.0, 2.0],
            vec![8.0, 8.0, 5.0, 11.516_386, 4.0],
            vec![7.0, 9.0, 9.0, 8.0, 6.681_225_3],
        ];
        assert_eq!(roots_of(&chu_liu_edmonds(&best)), [1, 3, 4]);
        let fixed = penalize_roots(best.clone(), &[1, 3, 4], 3);
        assert_eq!(roots_of(&chu_liu_edmonds(&fixed)), [2, 3]);
        let heads = single_root(best);
        assert_eq!(roots_of(&heads), [3]);
        let tokens: Vec<Token> = heads
            .iter()
            .enumerate()
            .map(|(dep, &head)| Token {
                id: dep + 1,
                form: "w".to_owned(),
                lemma: "_".to_owned(),
                upostag: "X".to_owned(),
                head: if head == dep { 0 } else { head + 1 },
                deprel: "dep".to_owned(),
            })
            .collect();
        assert!(DependencyTree::new(tokens).is_ok(), "{heads:?}");
    }

    /// A sentence cut into 1–8 pieces of 1–3 characters (some multi-byte), with or without a
    /// space before each, and the pieces' byte ranges.
    fn sentence_and_pieces() -> impl Strategy<Value = (String, Vec<Piece>)> {
        let piece = (
            prop::collection::vec(prop::sample::select(vec!['a', 'é', '漢', 'b']), 1..=3),
            any::<bool>(),
        );
        prop::collection::vec(piece, 1..=8).prop_map(|parts| {
            let mut sentence = String::new();
            let mut pieces = Vec::new();
            for (i, (chars, spaced)) in parts.into_iter().enumerate() {
                if i > 0 && spaced {
                    sentence.push(' ');
                }
                let start = sentence.len();
                sentence.extend(chars);
                pieces.push(Piece {
                    id: 7,
                    range: start..sentence.len(),
                });
            }
            (sentence, pieces)
        })
    }

    /// Logits for `n` pieces in -5..5, with some cells' goeswith, some cells' root labels and
    /// two diagonals' root labels raised by 8, each bias present in about half the cases.
    fn biased_logits(n: usize) -> impl Strategy<Value = Vec<f32>> {
        let width = CONFIG_LABELS;
        (
            prop::collection::vec(-5.0f32..5.0, n * n * width),
            prop::collection::vec(any::<bool>(), n * n),
            prop::collection::vec(any::<bool>(), n * n),
            (0..n, 0..n),
            any::<[bool; 3]>(),
        )
            .prop_map(move |(mut logits, gw, root, (a, b), on)| {
                for cell in 0..n * n {
                    if on[0] && gw[cell] {
                        logits[cell * width + GOESWITH] += 8.0;
                    }
                    if on[1] && root[cell] {
                        logits[cell * width + ROOT] += 8.0;
                    }
                }
                if on[2] {
                    for d in [a, b] {
                        logits[(d * n + d) * width + ROOT] += 8.0;
                    }
                }
                logits
            })
    }

    const CONFIG_LABELS: usize = 6;

    fn case() -> impl Strategy<Value = (String, Vec<Piece>, Vec<f32>)> {
        sentence_and_pieces().prop_flat_map(|(sentence, pieces)| {
            let n = pieces.len();
            (Just(sentence), Just(pieces), biased_logits(n))
        })
    }

    /// The piece groups `ud.py`'s merge must give for `parse`, restated from its rule: piece
    /// `i ≥ 1` joins the previous piece iff it is labelled `…|goeswith`, its head lies to its
    /// left, and every piece after that head up to `i` is labelled `…|goeswith`.
    fn expected_groups(parse: &PieceParse, labels: &Labels) -> Vec<Range<usize>> {
        let is_goeswith = |p: usize| labels.names[parse.labels[p]].ends_with("|goeswith");
        let joins = |i: usize| {
            let head = parse.heads[i];
            head < i && (head + 1..=i).all(is_goeswith)
        };
        let mut groups: Vec<Range<usize>> = Vec::new();
        for i in 0..parse.heads.len() {
            match groups.last_mut() {
                Some(last) if joins(i) => last.end = i + 1,
                _ => groups.push(i..i + 1),
            }
        }
        groups
    }

    proptest! {
        /// Given any piece ids
        /// When the masked batch is built
        /// Then it has n rows of n + 3 ids; row i is `<s>`, the pieces with piece i masked,
        /// `</s>`, piece i (P5); and any rows of it are those rows of the whole batch
        #[test]
        fn the_masked_batch_masks_one_piece_per_row(
            pieces in prop::collection::vec(3i64..50000, 0..12),
            cut in any::<(prop::sample::Index, prop::sample::Index)>(),
        ) {
            let n = pieces.len();
            let batch = masked_batch(special(), &pieces);
            prop_assert_eq!(batch.len(), n * (n + 3));
            for (i, row) in batch.chunks(n + 3).enumerate() {
                let mut expected = vec![0];
                expected.extend(pieces.iter().enumerate().map(|(k, &p)| if k == i { 50264 } else { p }));
                expected.extend([2, pieces[i]]);
                prop_assert_eq!(row, expected.as_slice());
            }
            if n > 0 {
                let (a, b) = (cut.0.index(n + 1), cut.1.index(n + 1));
                let (lo, hi) = (a.min(b), a.max(b));
                let rows = masked_rows(special(), &pieces, lo..hi);
                prop_assert_eq!(rows.as_slice(), &batch[lo * (n + 3)..hi * (n + 3)]);
            }
        }

        /// Given any piece count and position limit (half of the limits within 5 of the count)
        /// When the length rule is applied
        /// Then a sentence fits exactly when it has pieces and they need at most the limit's
        /// positions with `<s>`, `</s>` and the repeated piece (P6)
        #[test]
        fn fits_iff_the_row_has_room(
            (pieces, max) in (0usize..1100).prop_flat_map(|n| (Just(n), prop_oneof![n..n + 6, 0usize..1100]))
        ) {
            prop_assert_eq!(fits(pieces, max), pieces > 0 && pieces + 3 <= max);
            prop_assert!(fits(max.saturating_sub(3), max) || max < 4);
            let most = ModelLimits { max_positions: max }.max_pieces();
            prop_assert_eq!(fits(pieces, max), pieces > 0 && pieces <= most);
        }

        /// Given any finite logits for 1–8 pieces (biased toward goeswith, root labels and two
        /// strong roots) and piece ranges on character boundaries
        /// When they are decoded
        /// Then the tokens form a tree (P7) with exactly one root (P8); every token carries
        /// the UPOS and DEPREL of a label other than label 0, and `root` exactly when it is the
        /// root (P9); the tokens are the pieces merged by `ud.py`'s rule, in order, each form
        /// its bytes of the sentence (P10); and no goeswith points right (P11)
        #[test]
        fn decodes_a_tree_of_merged_words((sentence, pieces, logits) in case()) {
            let labels = labels();
            let parse = parse_pieces(Scores::new(logits.clone(), pieces.len(), labels.len()), &labels);
            let tokens = decode(logits, &labels, &sentence, &pieces).unwrap();
            prop_assert_eq!(tokens.iter().filter(|t| t.head == 0).count(), 1, "P8: {:?}", tokens);
            prop_assert!(DependencyTree::new(tokens.clone()).is_ok(), "P7: {:?}", tokens);
            let allowed: Vec<(&str, &str)> = labels.names[1..]
                .iter()
                .map(|n| (n.split('|').next().unwrap(), n.rsplit('|').next().unwrap()))
                .collect();
            for t in &tokens {
                prop_assert!(allowed.contains(&(t.upostag.as_str(), t.deprel.as_str())), "P9: {:?}", t);
                prop_assert_eq!(t.deprel == "root", t.head == 0, "P9: {:?}", t);
                prop_assert!(!(t.deprel == "goeswith" && t.head > t.id), "P11: {:?}", t);
            }
            let groups = expected_groups(&parse, &labels);
            prop_assert_eq!(tokens.len(), groups.len(), "P10: {:?} {:?}", tokens, parse);
            for (t, g) in tokens.iter().zip(&groups) {
                let range = pieces[g.start].range.start..pieces[g.end - 1].range.end;
                prop_assert_eq!(&t.form, &sentence[range], "P10: {:?}", tokens);
            }
        }
    }
}
