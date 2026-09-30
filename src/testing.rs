//! Test support for the syntactic metrics (spec 001 M3a): hand-built parses in CoNLL-U and a
//! generator of well-formed dependency trees.

use proptest::prelude::*;
use proptest::sample::Index;

use crate::dependency::{Parse, Parser, Token};

/// The tokens of a CoNLL-U sentence: every line starting with a digit, except multiword ranges
/// (`7-8`) and empty nodes (`8.1`), columns ID FORM LEMMA
/// UPOS … HEAD DEPREL (1, 2, 3, 4, 7, 8), separated by tabs or spaces (no form holds a space).
/// Panics on malformed input; test code only.
pub(crate) fn tokens_from_conllu(conllu: &str) -> Vec<Token> {
    conllu
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with(|c: char| c.is_ascii_digit()))
        .filter(|line| {
            let id = line.split_whitespace().next().unwrap_or_default();
            !id.contains(['-', '.'])
        })
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

/// The spec's M3a example sentence.
pub(crate) const EXAMPLE_TEXT: &str = "The proposal, which the executive committee rejected after extensive deliberation, caused significant delays.";

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

/// The M4 nominalization witness sentence.
pub(crate) const NOMZ_TEXT: &str = "We commission a review of the committee's decisions because their implementation needs careful consideration.";

/// `NOMZ_TEXT` parsed by UDPipe 2 (LINDAT REST service, `tokenizer=presegmented&tagger&parser`,
/// model `english-ewt-ud-2.17-251125`, fetched 2026-09-27), with its `7-8` multiword line.
pub(crate) const NOMZ_CONLLU: &str = "\
# generator = UDPipe 2, https://lindat.mff.cuni.cz/services/udpipe
# udpipe_model = english-ewt-ud-2.17-251125
# udpipe_model_licence = CC BY-NC-SA
# text = We commission a review of the committee's decisions because their implementation needs careful consideration.
1	We	we	PRON	PRP	Case=Nom|Number=Plur|Person=1|PronType=Prs	2	nsubj	_	_
2	commission	commission	VERB	VBP	Mood=Ind|Number=Plur|Person=1|Tense=Pres|VerbForm=Fin	0	root	_	_
3	a	a	DET	DT	Definite=Ind|PronType=Art	4	det	_	_
4	review	review	NOUN	NN	Number=Sing	2	obj	_	_
5	of	of	ADP	IN	_	9	case	_	_
6	the	the	DET	DT	Definite=Def|PronType=Art	7	det	_	_
7-8	committee's	_	_	_	_	_	_	_	_
7	committee	committee	NOUN	NN	Number=Sing	9	nmod:poss	_	_
8	's	's	PART	POS	_	7	case	_	_
9	decisions	decision	NOUN	NNS	Number=Plur	4	nmod	_	_
10	because	because	SCONJ	IN	_	13	mark	_	_
11	their	their	PRON	PRP$	Case=Gen|Number=Plur|Person=3|Poss=Yes|PronType=Prs	12	nmod:poss	_	_
12	implementation	implementation	NOUN	NN	Number=Sing	13	nsubj	_	_
13	needs	need	VERB	VBZ	Mood=Ind|Number=Sing|Person=3|Tense=Pres|VerbForm=Fin	2	advcl	_	_
14	careful	careful	ADJ	JJ	Degree=Pos	15	amod	_	_
15	consideration	consideration	NOUN	NN	Number=Sing	13	obj	_	SpaceAfter=No
16	.	.	PUNCT	.	_	2	punct	_	SpaceAfter=No
";

/// The M4 passive witness sentence.
pub(crate) const PASSIVE_TEXT: &str =
    "The report was written by the committee after the proposal had been rejected.";

/// `PASSIVE_TEXT` parsed by UDPipe 2 (same service, model and date as `NOMZ_CONLLU`).
pub(crate) const PASSIVE_CONLLU: &str = "\
# generator = UDPipe 2, https://lindat.mff.cuni.cz/services/udpipe
# udpipe_model = english-ewt-ud-2.17-251125
# udpipe_model_licence = CC BY-NC-SA
# text = The report was written by the committee after the proposal had been rejected.
1	The	the	DET	DT	Definite=Def|PronType=Art	2	det	_	_
2	report	report	NOUN	NN	Number=Sing	4	nsubj:pass	_	_
3	was	be	AUX	VBD	Mood=Ind|Number=Sing|Person=3|Tense=Past|VerbForm=Fin	4	aux:pass	_	_
4	written	write	VERB	VBN	Tense=Past|VerbForm=Part|Voice=Pass	0	root	_	_
5	by	by	ADP	IN	_	7	case	_	_
6	the	the	DET	DT	Definite=Def|PronType=Art	7	det	_	_
7	committee	committee	NOUN	NN	Number=Sing	4	obl:agent	_	_
8	after	after	SCONJ	IN	_	13	mark	_	_
9	the	the	DET	DT	Definite=Def|PronType=Art	10	det	_	_
10	proposal	proposal	NOUN	NN	Number=Sing	13	nsubj:pass	_	_
11	had	have	AUX	VBD	Mood=Ind|Number=Sing|Person=3|Tense=Past|VerbForm=Fin	13	aux	_	_
12	been	be	AUX	VBN	Tense=Past|VerbForm=Part	13	aux:pass	_	_
13	rejected	reject	VERB	VBN	Tense=Past|VerbForm=Part|Voice=Pass	4	advcl	_	SpaceAfter=No
14	.	.	PUNCT	.	_	4	punct	_	SpaceAfter=No
";

/// Finds every sentence too long for the model: 600 subword pieces, the tested model's maximum
/// 509 (spec 001 M3b).
pub(crate) struct EveryTooLong;

impl Parser for EveryTooLong {
    type Error = std::convert::Infallible;

    fn parse(&mut self, _sentence: &str) -> Result<Parse, Self::Error> {
        Ok(Parse::TooLong {
            pieces: 600,
            max: 509,
        })
    }
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
