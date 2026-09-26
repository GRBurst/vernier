//! Syntactic metrics (spec 001 M3a): pure functions over a validated dependency tree, computed on
//! its content tokens only.

use crate::dependency::{DependencyTree, Token};

/// A content token of the projection, with its head renumbered among content tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ContentToken<'t> {
    token: &'t Token,
    /// The content id (1-based) of the nearest non-`PUNCT` ancestor, 0 for a root.
    head: usize,
}

/// The content projection (spec 001, *Content token*): `PUNCT` dropped, the rest renumbered
/// 1..N in sentence order, each re-attached to its nearest non-`PUNCT` ancestor. Index k holds
/// content id k + 1. A token whose ancestors are all `PUNCT` becomes a root.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ContentTree<'t> {
    tokens: Vec<ContentToken<'t>>,
}

// why: the metrics that read the projection land in the next tasks of plan-M3a (T3–T6).
#[allow(dead_code)]
fn content_tree(tree: &DependencyTree) -> ContentTree<'_> {
    let tokens = tree.tokens();
    let content_ids = content_ids(tokens);
    let renumber = |original: usize| original.checked_sub(1).map_or(0, |i| content_ids[i]);
    ContentTree {
        tokens: tokens
            .iter()
            .filter(|token| !token.is_punct())
            .map(|token| ContentToken {
                token,
                head: renumber(nearest_content_ancestor(tokens, token.head)),
            })
            .collect(),
    }
}

/// Each token's rank among the non-`PUNCT` tokens (1-based), 0 for a `PUNCT` token.
fn content_ids(tokens: &[Token]) -> Vec<usize> {
    tokens
        .iter()
        .scan(0, |rank, token| {
            if token.is_punct() {
                Some(0)
            } else {
                *rank += 1;
                Some(*rank)
            }
        })
        .collect()
}

/// The original id of the first non-`PUNCT` token from `head` up the head chain, or 0 when the
/// chain reaches the root first.
fn nearest_content_ancestor(tokens: &[Token], head: usize) -> usize {
    let mut at = head;
    while at != 0 && tokens[at - 1].is_punct() {
        at = tokens[at - 1].head;
    }
    at
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{tokens_from_conllu, well_formed_tree};
    use proptest::prelude::*;

    fn tree(tokens: Vec<Token>) -> DependencyTree {
        DependencyTree::new(tokens).expect("a well-formed tree")
    }

    /// The original id of the first non-`PUNCT` token on `id`'s head chain, walked one step at a
    /// time, or 0 when the chain reaches the root first.
    fn naive_content_ancestor(tokens: &[Token], id: usize) -> usize {
        let mut chain = std::iter::successors(Some(tokens[id - 1].head), |&h| {
            (h != 0).then(|| tokens[h - 1].head)
        });
        chain
            .find(|&h| h == 0 || !tokens[h - 1].is_punct())
            .unwrap_or(0)
    }

    /// Given `1 a →2, 2 , PUNCT →3, 3 ; PUNCT →4, 4 b root` (a chain of two `PUNCT` tokens)
    /// When the content projection is taken
    /// Then it holds a and b as content tokens 1 and 2, and a is re-attached to b
    #[test]
    fn reattaches_across_a_chain_of_punctuation() {
        let t = tree(tokens_from_conllu(
            "
            1 a a X _ _ 2 dep
            2 , , PUNCT _ _ 3 punct
            3 ; ; PUNCT _ _ 4 punct
            4 b b X _ _ 0 root",
        ));
        let projection = content_tree(&t);
        let shape: Vec<(&str, usize)> = projection
            .tokens
            .iter()
            .map(|c| (c.token.form.as_str(), c.head))
            .collect();
        assert_eq!(shape, [("a", 2), ("b", 0)]);
    }

    proptest! {
        /// Given any well-formed tree with N non-`PUNCT` tokens
        /// When the content projection is taken
        /// Then it holds exactly those N tokens in sentence order, and every head is in 0..=N and
        /// differs from the token's own content id
        #[test]
        fn projection_keeps_content_tokens_in_order(tokens in well_formed_tree()) {
            let t = tree(tokens.clone());
            let projection = content_tree(&t);
            let content: Vec<&Token> = tokens.iter().filter(|t| !t.is_punct()).collect();
            let kept: Vec<&Token> = projection.tokens.iter().map(|c| c.token).collect();
            prop_assert_eq!(kept, content);
            let n = projection.tokens.len();
            for (k, c) in projection.tokens.iter().enumerate() {
                prop_assert!(c.head <= n && c.head != k + 1, "token {} head {}", k + 1, c.head);
            }
        }

        /// Given any well-formed tree
        /// When the content projection is taken
        /// Then each content token's head is the renumbered id of the first non-`PUNCT` token on
        /// its original head chain, or 0 when there is none
        #[test]
        fn projection_reattaches_to_the_nearest_content_ancestor(tokens in well_formed_tree()) {
            let t = tree(tokens.clone());
            let projection = content_tree(&t);
            let renumbered = |original: usize| {
                projection.tokens.iter().position(|c| c.token.id == original).map_or(0, |k| k + 1)
            };
            for c in &projection.tokens {
                let expected = renumbered(naive_content_ancestor(&tokens, c.token.id));
                prop_assert_eq!(c.head, expected, "token {}", c.token.id);
            }
        }
    }
}
