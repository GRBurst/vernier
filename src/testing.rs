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

/// The spec's M3a example sentence parsed by UDPipe 2 (LINDAT REST service, model
/// `english-ewt-ud-2.17-251125`, fetched 2026-09-27); used as test data only.
pub(crate) const EXAMPLE_CONLLU: &str = "\
# generator = UDPipe 2, https://lindat.mff.cuni.cz/services/udpipe (fetched 2026-09-27)
# udpipe_model = english-ewt-ud-2.17-251125
# udpipe_model_licence = CC BY-NC-SA
# text = The proposal, which the executive committee rejected after extensive deliberation, caused significant delays.
1	The	the	DET	DT	Definite=Def|PronType=Art	2	det	_	_
2	proposal	proposal	NOUN	NN	Number=Sing	13	nsubj	_	SpaceAfter=No
3	,	,	PUNCT	,	_	8	punct	_	_
4	which	which	PRON	WDT	PronType=Rel	8	obj	_	_
5	the	the	DET	DT	Definite=Def|PronType=Art	7	det	_	_
6	executive	executive	ADJ	JJ	Degree=Pos	7	amod	_	_
7	committee	committee	NOUN	NN	Number=Sing	8	nsubj	_	_
8	rejected	reject	VERB	VBD	Mood=Ind|Number=Sing|Person=3|Tense=Past|VerbForm=Fin	2	acl:relcl	_	_
9	after	after	ADP	IN	_	11	case	_	_
10	extensive	extensive	ADJ	JJ	Degree=Pos	11	amod	_	_
11	deliberation	deliberation	NOUN	NN	Number=Sing	8	obl	_	SpaceAfter=No
12	,	,	PUNCT	,	_	2	punct	_	_
13	caused	cause	VERB	VBD	Mood=Ind|Number=Sing|Person=3|Tense=Past|VerbForm=Fin	0	root	_	_
14	significant	significant	ADJ	JJ	Degree=Pos	15	amod	_	_
15	delays	delay	NOUN	NNS	Number=Plur	13	obj	_	SpaceAfter=No
16	.	.	PUNCT	.	_	13	punct	_	SpaceAfter=No
";

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
