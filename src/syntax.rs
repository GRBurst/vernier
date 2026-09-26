//! Syntactic metrics (spec 001 M3a): pure functions over a validated dependency tree, computed on
//! its content tokens only.

use std::iter::Sum;
use std::ops::Add;

use crate::dependency::{DependencyTree, Token};

/// The summed distances |i − head(i)| between content tokens and their content heads, and how many
/// such dependencies there are; they add up across sentences, so a file's MDD pools its sentences'
/// distances instead of averaging their means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DependencyDistance {
    pub total: usize,
    pub dependencies: usize,
}

impl DependencyDistance {
    /// The mean dependency distance, absent when there is no dependency to divide by.
    pub fn mean(self) -> Option<f64> {
        (self.dependencies > 0).then(|| self.total as f64 / self.dependencies as f64)
    }
}

impl Add for DependencyDistance {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            total: self.total + other.total,
            dependencies: self.dependencies + other.dependencies,
        }
    }
}

impl Sum for DependencyDistance {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::default(), Add::add)
    }
}

/// The distance between every content token and its content head, over the content projection.
pub fn dependency_distance(tree: &DependencyTree) -> DependencyDistance {
    content_tree(tree)
        .dependencies()
        .map(|(dependent, head)| DependencyDistance {
            total: dependent.abs_diff(head),
            dependencies: 1,
        })
        .sum()
}

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

/// The largest number of edges from a projected root down to any content token (0 for a
/// root-only sentence).
pub fn depth(tree: &DependencyTree) -> usize {
    let projection = content_tree(tree);
    (1..=projection.tokens.len())
        .map(|id| projection.edges_to_root(id))
        .max()
        .unwrap_or(0)
}

/// The number of content tokens whose `deprel` is a clausal relation.
pub fn clause_count(tree: &DependencyTree) -> usize {
    content_tree(tree)
        .tokens
        .iter()
        .filter(|c| is_clausal(&c.token.deprel))
        .count()
}

/// Whether `deprel` is a clausal relation (spec 001, *Clausal relation*): its part before any
/// `:` is exactly `advcl`, `acl`, `csubj` or `ccomp`.
pub fn is_clausal(deprel: &str) -> bool {
    matches!(
        deprel.split(':').next(),
        Some("advcl" | "acl" | "csubj" | "ccomp")
    )
}

impl ContentTree<'_> {
    /// The number of edges from content token `id` up to its projected root.
    fn edges_to_root(&self, id: usize) -> usize {
        std::iter::successors(Some(id), |&k| {
            Some(self.tokens[k - 1].head).filter(|&h| h != 0)
        })
        .skip(1)
        .count()
    }

    /// Every (content id, content head) pair whose head is not the root.
    fn dependencies(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.tokens
            .iter()
            .enumerate()
            .filter(|(_, c)| c.head != 0)
            .map(|(k, c)| (k + 1, c.head))
    }
}

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
    use crate::testing::{EXAMPLE_CONLLU, tokens_from_conllu, well_formed_tree};
    use proptest::prelude::*;

    fn tree(tokens: Vec<Token>) -> DependencyTree {
        DependencyTree::new(tokens).expect("a well-formed tree")
    }

    fn word(id: usize, head: usize) -> Token {
        Token {
            id,
            form: format!("w{id}"),
            lemma: format!("w{id}"),
            upostag: "X".to_owned(),
            head,
            deprel: if head == 0 { "root" } else { "dep" }.to_owned(),
        }
    }

    /// n words, each headed by the next; the last is the root.
    fn chain(n: usize) -> DependencyTree {
        tree(
            (1..=n)
                .map(|i| word(i, if i == n { 0 } else { i + 1 }))
                .collect(),
        )
    }

    /// n words, all headed by the first, which is the root.
    fn star(n: usize) -> DependencyTree {
        tree((1..=n).map(|i| word(i, usize::from(i != 1))).collect())
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    /// Given the UDPipe 2 parse of the M3a example sentence (13 content tokens, 3 `PUNCT`)
    /// When its dependency distance is measured
    /// Then the total is 32 over 12 dependencies, an MDD of 32/12
    #[test]
    fn example_sentence_has_mdd_32_over_12() {
        let distance = dependency_distance(&tree(tokens_from_conllu(EXAMPLE_CONLLU)));
        assert_eq!(
            distance,
            DependencyDistance {
                total: 32,
                dependencies: 12
            }
        );
        assert_eq!(distance.mean(), Some(32.0 / 12.0));
    }

    /// Given a one-word sentence `1 Go root`, and the same with a `PUNCT` token `2 ! →1`
    /// When its dependency distance is measured
    /// Then the MDD is absent
    #[test]
    fn single_word_sentence_has_no_mdd() {
        let bare = "1 Go go VERB _ _ 0 root";
        let punctuated = "1 Go go VERB _ _ 0 root\n2 ! ! PUNCT _ _ 1 punct";
        for conllu in [bare, punctuated] {
            let distance = dependency_distance(&tree(tokens_from_conllu(conllu)));
            assert_eq!(distance.mean(), None, "{conllu:?}");
        }
    }

    /// Given a chain of 2 words (MDD 1) and a star of 4 words (MDD 2)
    /// When the file MDD is taken over both sentences
    /// Then it is the pooled 7/4, not the mean of the sentence MDDs 1.5
    #[test]
    fn file_mdd_pools_distances_not_means() {
        let file: DependencyDistance = [chain(2), star(4)].iter().map(dependency_distance).sum();
        assert_eq!(file.mean(), Some(7.0 / 4.0), "{file:?}");
    }

    /// Given the UDPipe 2 parse of the M3a example sentence
    /// When its depth is measured
    /// Then it is 4 edges (caused → proposal → rejected → committee → the)
    #[test]
    fn example_sentence_has_depth_4() {
        assert_eq!(depth(&tree(tokens_from_conllu(EXAMPLE_CONLLU))), 4);
    }

    /// Given a root-only sentence `1 Go root`, and the same with a `PUNCT` token `2 ! →1`
    /// When its depth is measured
    /// Then it is 0
    #[test]
    fn root_only_sentence_has_depth_0() {
        let bare = "1 Go go VERB _ _ 0 root";
        let punctuated = "1 Go go VERB _ _ 0 root\n2 ! ! PUNCT _ _ 1 punct";
        for conllu in [bare, punctuated] {
            assert_eq!(depth(&tree(tokens_from_conllu(conllu))), 0, "{conllu:?}");
        }
    }

    /// Given the UDPipe 2 parse of the M3a example sentence
    /// When its clauses are counted
    /// Then there is 1 (`rejected`, `acl:relcl`)
    #[test]
    fn example_sentence_has_one_clause() {
        assert_eq!(clause_count(&tree(tokens_from_conllu(EXAMPLE_CONLLU))), 1);
    }

    /// Given relations whose base is not one of the four clausal ones, or merely resembles one
    /// When they are classified
    /// Then none is clausal
    #[test]
    fn non_clausal_relations_are_not_clausal() {
        for deprel in [
            "xcomp",
            "advmod",
            "obj",
            "nsubj",
            "obl",
            "aclx",
            "nsubj:pass",
            "punct",
        ] {
            assert!(!is_clausal(deprel), "{deprel}");
        }
    }

    fn distance_counts() -> impl Strategy<Value = DependencyDistance> {
        (prop_oneof![1 => Just(0usize), 3 => 1usize..100], 0usize..10).prop_map(
            |(dependencies, extra)| DependencyDistance {
                total: dependencies + dependencies * extra,
                dependencies,
            },
        )
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

        /// Given any well-formed tree with N content tokens
        /// When its dependency distance is measured
        /// Then the MDD is absent exactly when there is no dependency, and always when N < 2;
        /// otherwise it is at least 1; with one projected root there are N − 1 dependencies
        #[test]
        fn mdd_is_absent_below_two_content_tokens_and_at_least_one(tokens in well_formed_tree()) {
            let t = tree(tokens);
            let projection = content_tree(&t);
            let n = projection.tokens.len();
            let roots = projection.tokens.iter().filter(|c| c.head == 0).count();
            let distance = dependency_distance(&t);
            prop_assert_eq!(distance.mean().is_none(), distance.dependencies == 0);
            if n < 2 {
                prop_assert_eq!(distance.mean(), None);
            }
            if let Some(mdd) = distance.mean() {
                prop_assert!(mdd >= 1.0, "{}", mdd);
            }
            if roots == 1 {
                prop_assert_eq!(distance.dependencies, n - 1);
            }
        }

        /// Given a chain of n ≥ 2 words and a star of n ≥ 2 words rooted at the first
        /// When their MDD is measured
        /// Then the chain's is 1 and the star's is n / 2
        #[test]
        fn chain_mdd_is_one_and_star_mdd_is_half_its_length(n in 2usize..40) {
            let chain_mdd = dependency_distance(&chain(n)).mean().expect("n ≥ 2");
            let star_mdd = dependency_distance(&star(n)).mean().expect("n ≥ 2");
            prop_assert!(close(chain_mdd, 1.0), "{}", chain_mdd);
            prop_assert!(close(star_mdd, n as f64 / 2.0), "{}", star_mdd);
        }

        /// Given any well-formed tree with N ≥ 1 content tokens
        /// When its depth is measured
        /// Then it is at most N − 1
        #[test]
        fn depth_is_at_most_one_less_than_the_content_tokens(tokens in well_formed_tree()) {
            let n = tokens.iter().filter(|t| !t.is_punct()).count();
            prop_assume!(n >= 1);
            let d = depth(&tree(tokens));
            prop_assert!(d < n, "depth {} with {} content tokens", d, n);
        }

        /// Given a chain of n ≥ 1 words
        /// When its depth is measured
        /// Then it is n − 1, so the bound N − 1 is tight
        #[test]
        fn chain_depth_is_one_less_than_its_length(n in 1usize..40) {
            prop_assert_eq!(depth(&chain(n)), n - 1);
        }

        /// Given each clausal base relation, bare and with any subtype
        /// When it is classified
        /// Then it is clausal
        #[test]
        fn clausal_bases_are_clausal_with_any_subtype(
            base in prop::sample::select(vec!["advcl", "acl", "csubj", "ccomp"]),
            subtype in "[a-z]{1,8}",
        ) {
            prop_assert!(is_clausal(base));
            let with_subtype = format!("{base}:{subtype}");
            prop_assert!(is_clausal(&with_subtype), "{}", with_subtype);
        }

        /// Given any well-formed tree
        /// When its clauses are counted
        /// Then the count is the number of content tokens whose deprel is one of the generator's
        /// clausal relations (advcl, acl, acl:relcl, ccomp, csubj)
        #[test]
        fn clause_count_counts_content_tokens_with_a_clausal_relation(tokens in well_formed_tree()) {
            let clausal = ["advcl", "acl", "acl:relcl", "ccomp", "csubj"];
            let expected = tokens
                .iter()
                .filter(|t| !t.is_punct() && clausal.contains(&t.deprel.as_str()))
                .count();
            prop_assert_eq!(clause_count(&tree(tokens)), expected);
        }

        /// Given any three dependency distances
        /// When they are added
        /// Then addition is associative and commutative, zero is neutral, and the MDD of a sum is
        /// the pooled total over the pooled dependencies
        #[test]
        fn dependency_distances_form_a_commutative_monoid(
            a in distance_counts(), b in distance_counts(), c in distance_counts()
        ) {
            prop_assert_eq!((a + b) + c, a + (b + c));
            prop_assert_eq!(a + b, b + a);
            prop_assert_eq!(a + DependencyDistance::default(), a);
            prop_assert_eq!([a, b, c].into_iter().sum::<DependencyDistance>(), a + b + c);
            let pooled = (a.dependencies + b.dependencies > 0)
                .then(|| (a.total + b.total) as f64 / (a.dependencies + b.dependencies) as f64);
            prop_assert_eq!((a + b).mean(), pooled);
        }
    }
}
