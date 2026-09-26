//! Test support for the syntactic metrics (spec 001 M3a): hand-built parses in CoNLL-U and a
//! generator of well-formed dependency trees.

use proptest::prelude::*;
use proptest::sample::Index;

use crate::dependency::Token;

/// The tokens of a CoNLL-U sentence: every line starting with a digit, columns ID FORM LEMMA
/// UPOS … HEAD DEPREL (1, 2, 3, 4, 7, 8), separated by tabs or spaces (no form holds a space).
/// Panics on malformed input; test code only.
pub(crate) fn tokens_from_conllu(conllu: &str) -> Vec<Token> {
    conllu
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with(|c: char| c.is_ascii_digit()))
        .map(|line| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            Token {
                id: columns[0].parse().expect("an integer id"),
                form: columns[1].to_owned(),
                lemma: columns[2].to_owned(),
                upostag: columns[3].to_owned(),
                head: columns[6].parse().expect("an integer head"),
                deprel: columns[7].to_owned(),
            }
        })
        .collect()
}

const DEPRELS: [&str; 13] = [
    "nsubj",
    "nsubj:pass",
    "obj",
    "det",
    "obl",
    "advmod",
    "xcomp",
    "advcl",
    "acl",
    "acl:relcl",
    "ccomp",
    "csubj",
    "punct",
];

const FORMS: [&str; 7] = ["the", "proposal", "which", "caused", "42", "%", ","];

const CONTENT_UPOS: [&str; 6] = ["NOUN", "VERB", "PRON", "DET", "NUM", "SYM"];

/// One generated token's random choices: its parent's rank in the order, `PUNCT` or not, deprel,
/// form and content tag.
type Draw = (Index, bool, &'static str, &'static str, &'static str);

fn draw() -> impl Strategy<Value = Draw> {
    (
        any::<Index>(),
        prop::bool::weighted(1.0 / 3.0),
        prop::sample::select(DEPRELS.to_vec()),
        prop::sample::select(FORMS.to_vec()),
        prop::sample::select(CONTENT_UPOS.to_vec()),
    )
}

/// Well-formed trees of 1..=20 tokens: a random order `π` whose first token is the root and whose
/// k-th token is headed by a random earlier one (so every shape and non-projective orders occur);
/// each token is `PUNCT` with probability ⅓, internal and root nodes included.
pub(crate) fn well_formed_tree() -> impl Strategy<Value = Vec<Token>> {
    (1usize..=20)
        .prop_flat_map(|n| {
            (
                Just((1..=n).collect::<Vec<usize>>()).prop_shuffle(),
                prop::collection::vec(draw(), n),
            )
        })
        .prop_map(|(order, draws)| tree_from(&order, &draws))
}

fn tree_from(order: &[usize], draws: &[Draw]) -> Vec<Token> {
    let mut heads = vec![0; order.len()];
    for k in 1..order.len() {
        heads[order[k] - 1] = order[draws[k].0.index(k)];
    }
    heads
        .iter()
        .zip(draws)
        .enumerate()
        .map(|(i, (&head, &(_, is_punct, deprel, form, upos)))| Token {
            id: i + 1,
            form: form.to_owned(),
            lemma: form.to_owned(),
            upostag: if is_punct { "PUNCT" } else { upos }.to_owned(),
            head,
            deprel: if head == 0 { "root" } else { deprel }.to_owned(),
        })
        .collect()
}
