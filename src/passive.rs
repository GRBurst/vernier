//! Passive voice (spec 001 M4): the head verb of every `aux:pass` token of a parse, and where its
//! form stands in the sentence text.

use crate::dependency::Token;

/// One passive construction: its verb's form and the verb's first character in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Passive {
    pub verb: String,
    pub source_offset: usize,
}

/// The ids of the tokens that head an `aux:pass` token, ascending, each once.
pub fn passive_heads(tokens: &[Token]) -> Vec<usize> {
    let mut heads: Vec<usize> = tokens
        .iter()
        .filter(|token| token.deprel == "aux:pass" && token.head != 0)
        .map(|token| token.head)
        .collect();
    heads.sort_unstable();
    heads.dedup();
    heads
}

/// Each token's byte offset in `text`, matching the forms in order: after skipping whitespace, a
/// form starts at the cursor, or else at the next later word start (a position after whitespace).
/// A form found at neither gets `None` and leaves the cursor where it was, so one inserted or
/// substituted form costs one position, not the rest (audit 009); no form matches inside a word.
pub fn align(tokens: &[Token], text: &str) -> Vec<Option<usize>> {
    tokens
        .iter()
        .scan(0, |cursor: &mut usize, token| {
            let found = form_at(text, *cursor, &token.form);
            if let Some(at) = found {
                *cursor = at + token.form.len();
            }
            Some(found)
        })
        .collect()
}

/// Where `form` starts in `text`: at `cursor` once its whitespace is skipped, or else at the first
/// later word start.
fn form_at(text: &str, cursor: usize, form: &str) -> Option<usize> {
    let rest = text.get(cursor..)?;
    let at = cursor + (rest.len() - rest.trim_start().len());
    let starts_here = |w: &usize| text.get(*w..).is_some_and(|tail| tail.starts_with(form));
    std::iter::once(at)
        .chain(later_word_starts(text, at))
        .find(starts_here)
}

/// The positions after `from` that follow a whitespace character.
fn later_word_starts(text: &str, from: usize) -> impl Iterator<Item = usize> + '_ {
    text.get(from..)
        .unwrap_or_default()
        .match_indices(char::is_whitespace)
        .map(move |(i, space)| from + i + space.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{
        NOMZ_CONLLU, NOMZ_TEXT, PASSIVE_CONLLU, PASSIVE_TEXT, tokens_from_conllu,
    };
    use proptest::prelude::*;
    use proptest::sample::Index;
    use std::collections::BTreeSet;

    fn token(id: usize, form: &str, head: usize, deprel: &str) -> Token {
        Token {
            id,
            form: form.to_owned(),
            lemma: form.to_owned(),
            upostag: "X".to_owned(),
            head,
            deprel: deprel.to_owned(),
        }
    }

    fn forms_as_tokens(forms: &[String]) -> Vec<Token> {
        forms
            .iter()
            .enumerate()
            .map(|(i, form)| token(i + 1, form, 0, "dep"))
            .collect()
    }

    /// Given the UDPipe 2 parse of "The report was written by the committee after the proposal
    /// had been rejected."
    /// When its passive heads are found and aligned to the text
    /// Then they are `written` (4) and `rejected` (13), at their first characters in the text
    #[test]
    fn finds_and_places_the_passive_witness() {
        let tokens = tokens_from_conllu(PASSIVE_CONLLU);
        assert_eq!(passive_heads(&tokens), [4, 13]);
        let offsets = align(&tokens, PASSIVE_TEXT);
        assert_eq!(offsets[3], PASSIVE_TEXT.find("written"));
        assert_eq!(offsets[12], PASSIVE_TEXT.find("rejected"));
        let found: Vec<usize> = offsets.iter().flatten().copied().collect();
        assert_eq!(found.len(), tokens.len());
        assert!(found.windows(2).all(|w| w[0] < w[1]), "{found:?}");
    }

    /// Given `it has been being written`, where two `aux:pass` tokens share the head `written`
    /// When its passive heads are found
    /// Then `written` is reported once
    #[test]
    fn a_head_with_two_passive_auxiliaries_is_reported_once() {
        let tokens = [
            token(1, "it", 5, "nsubj:pass"),
            token(2, "has", 5, "aux"),
            token(3, "been", 5, "aux:pass"),
            token(4, "being", 5, "aux:pass"),
            token(5, "written", 0, "root"),
        ];
        assert_eq!(passive_heads(&tokens), [5]);
    }

    /// Given the PASSIVE parse with the form of `was` changed to `WAS` (a substituted form)
    /// When its tokens are aligned to the text
    /// Then only `WAS` is missed, and `written` and `rejected` are still found at their verbs
    #[test]
    fn a_substituted_form_costs_one_position() {
        let mut tokens = tokens_from_conllu(PASSIVE_CONLLU);
        tokens[2].form = "WAS".to_owned();
        let offsets = align(&tokens, PASSIVE_TEXT);
        assert_eq!(offsets[2], None);
        assert_eq!(offsets[3], PASSIVE_TEXT.find("written"));
        assert_eq!(offsets[12], PASSIVE_TEXT.find("rejected"));
        assert_eq!(
            offsets.iter().filter(|o| o.is_none()).count(),
            1,
            "{offsets:?}"
        );
    }

    /// Given the forms `'` `'` over the text `' ' ` with the first form substituted by `ZQ`
    /// When the tokens are aligned to the text
    /// Then the second form matches the first word: the known limit of audit 009 — after a miss
    /// the cursor stays (so an inserted token costs nothing), and a next form that is a prefix of
    /// the substituted word matches that word
    #[test]
    fn the_known_limit_of_a_substitution() {
        let tokens = [token(1, "ZQ", 0, "dep"), token(2, "'", 0, "dep")];
        assert_eq!(align(&tokens, "' ' "), [None, Some(0)]);
    }

    /// Given the NOMZ parse, whose multiword token `committee's` is split into `committee` + `'s`
    /// When its tokens are aligned to the text
    /// Then every token is found, `'s` right after `committee`
    #[test]
    fn aligns_the_parts_of_a_multiword_token() {
        let tokens = tokens_from_conllu(NOMZ_CONLLU);
        let offsets = align(&tokens, NOMZ_TEXT);
        assert!(offsets.iter().all(Option::is_some), "{offsets:?}");
        let committee = NOMZ_TEXT.find("committee");
        assert_eq!(offsets[6], committee);
        assert_eq!(offsets[7], committee.map(|at| at + "committee".len()));
    }

    fn any_tokens() -> impl Strategy<Value = Vec<Token>> {
        (1usize..15).prop_flat_map(|n| {
            prop::collection::vec(
                (
                    0..=n,
                    prop::sample::select(vec!["aux:pass", "aux", "nsubj", "aux:passx"]),
                ),
                n,
            )
            .prop_map(|drawn| {
                drawn
                    .into_iter()
                    .enumerate()
                    .map(|(i, (head, deprel))| token(i + 1, "w", head, deprel))
                    .collect()
            })
        })
    }

    /// What is done to one token of an otherwise faithful token list.
    #[derive(Debug, Clone)]
    enum Edit {
        None,
        /// A form absent from the text is inserted before the token at this index.
        Insert(Index),
        /// A token gets a form absent from the text; it is picked by this index among the tokens
        /// whose next form is not a prefix of their own (the precondition of audit 009).
        Substitute(Index),
    }

    fn edit() -> impl Strategy<Value = Edit> {
        prop_oneof![
            1 => Just(Edit::None),
            2 => any::<Index>().prop_map(Edit::Insert),
            2 => any::<Index>().prop_map(Edit::Substitute),
        ]
    }

    proptest! {
        /// Given words joined by spaces and one word's token replaced by a piece of a word that
        /// occurs in the text only inside words, never at a word start
        /// When the tokens are aligned to the text
        /// Then that piece gets `None` and every other offset is unchanged
        #[test]
        fn never_matches_inside_a_word(
            words in prop::collection::vec("[a-z]{2,6}", 1..10),
            which in any::<Index>(),
            cut in any::<Index>(),
        ) {
            let text = words.join(" ");
            let k = which.index(words.len());
            let word = &words[k];
            let inner = &word[1 + cut.index(word.len() - 1)..];
            prop_assume!(!words.iter().any(|w| w.starts_with(inner)));
            let faithful = forms_as_tokens(&words);
            let mut tokens = faithful.clone();
            tokens[k].form = inner.to_owned();
            let mut offsets = align(&tokens, &text);
            let expected = align(&faithful, &text);
            prop_assert_eq!(offsets[k], None, "{:?} in {:?}", inner, text);
            offsets[k] = expected[k];
            prop_assert_eq!(offsets, expected);
        }

        /// Given any token list with random heads and deprels
        /// When its passive heads are found
        /// Then they ascend strictly and are exactly the non-root heads of `aux:pass` tokens
        #[test]
        fn passive_heads_are_the_heads_of_passive_auxiliaries(tokens in any_tokens()) {
            let heads = passive_heads(&tokens);
            prop_assert!(heads.windows(2).all(|w| w[0] < w[1]), "{:?}", heads);
            let expected: BTreeSet<usize> = tokens
                .iter()
                .filter(|t| t.deprel == "aux:pass" && t.head != 0)
                .map(|t| t.head)
                .collect();
            prop_assert_eq!(heads.into_iter().collect::<BTreeSet<_>>(), expected);
        }

        /// Given forms joined by 1–3 spaces, and optionally one form absent from the text inserted
        /// into the token list or substituted for one token's form (only where the next form is
        /// not a prefix of the substituted one; see `the_known_limit_of_a_substitution`)
        /// When the tokens are aligned to the text
        /// Then every form is found where it starts, at strictly increasing offsets; the inserted
        /// or substituted form gets `None` and every other offset is unchanged
        #[test]
        fn aligns_forms_in_order(
            words in prop::collection::vec(("[a-z']{1,6}", 1usize..=3), 1..12),
            edit in edit(),
        ) {
            let forms: Vec<String> = words.iter().map(|(f, _)| f.clone()).collect();
            let text: String = words
                .iter()
                .map(|(form, gap)| format!("{form}{}", " ".repeat(*gap)))
                .collect();
            let tokens = forms_as_tokens(&forms);
            let offsets = align(&tokens, &text);
            let found: Vec<usize> = offsets.iter().flatten().copied().collect();
            prop_assert_eq!(found.len(), forms.len(), "{:?} in {:?}", forms, text);
            prop_assert!(found.windows(2).all(|w| w[0] < w[1]), "{:?}", found);
            for (at, form) in found.iter().zip(&forms) {
                prop_assert!(text[*at..].starts_with(form.as_str()));
            }
            match edit {
                Edit::None => {}
                Edit::Insert(position) => {
                    let at = position.index(tokens.len() + 1);
                    let mut with_absent = tokens.clone();
                    with_absent.insert(at, token(0, "ZQ", 0, "dep"));
                    let mut shifted = align(&with_absent, &text);
                    prop_assert_eq!(shifted.remove(at), None);
                    prop_assert_eq!(shifted, offsets);
                }
                Edit::Substitute(position) => {
                    let unambiguous: Vec<usize> = (0..forms.len())
                        .filter(|&i| forms.get(i + 1).is_none_or(|next| !forms[i].starts_with(next.as_str())))
                        .collect();
                    let at = unambiguous[position.index(unambiguous.len())];
                    let mut substituted = tokens.clone();
                    substituted[at].form = "ZQ".to_owned();
                    let mut aligned = align(&substituted, &text);
                    prop_assert_eq!(aligned[at], None);
                    aligned[at] = offsets[at];
                    prop_assert_eq!(aligned, offsets);
                }
            }
        }
    }
}
