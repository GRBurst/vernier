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
/// form must start at the cursor. A form that does not gets `None` and leaves the cursor where it
/// was, so one odd form costs one position, not the rest.
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

/// Where `form` starts in `text` once the whitespace at `cursor` is skipped, if it starts there.
fn form_at(text: &str, cursor: usize, form: &str) -> Option<usize> {
    let rest = text.get(cursor..)?;
    let at = cursor + (rest.len() - rest.trim_start().len());
    text.get(at..)?.starts_with(form).then_some(at)
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

    proptest! {
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
        /// into the token list
        /// When the tokens are aligned to the text
        /// Then every form is found where it starts, at strictly increasing offsets; the inserted
        /// form gets `None` and every other offset is unchanged
        #[test]
        fn aligns_forms_in_order(
            words in prop::collection::vec(("[a-z']{1,6}", 1usize..=3), 1..12),
            insert in prop::option::weighted(0.8, any::<Index>()),
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
            if let Some(position) = insert {
                let at = position.index(tokens.len() + 1);
                let mut with_absent = tokens.clone();
                with_absent.insert(at, token(0, "ZQ", 0, "dep"));
                let mut shifted = align(&with_absent, &text);
                prop_assert_eq!(shifted.remove(at), None);
                prop_assert_eq!(shifted, offsets);
            }
        }
    }
}
