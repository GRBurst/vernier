//! Dependency trees (spec 001 M3a): the tokens a parser returns for one sentence, and the
//! validated tree every syntactic metric is computed from.

/// One token of a Universal Dependencies parse; `head` 0 marks the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub id: usize,
    pub form: String,
    pub lemma: String,
    pub upostag: String,
    pub head: usize,
    pub deprel: String,
}

impl Token {
    /// Whether the token is punctuation (`upostag` `PUNCT`), which no metric counts.
    pub fn is_punct(&self) -> bool {
        self.upostag == "PUNCT"
    }
}

/// What a parser makes of one sentence: its tokens, or nothing because the sentence is longer
/// than the model can take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parse {
    /// The tokens, ids 1..n in order, head 0 = root.
    Tokens(Vec<Token>),
    /// The sentence has `pieces` subword pieces; the model takes at most `max`.
    TooLong { pieces: usize, max: usize },
}

/// A dependency parser: the seam a real Universal Dependencies model fills (spec 001 M3b).
/// `&mut self`: a model's inference session is used, and changed, by every parse.
pub trait Parser {
    type Error: std::error::Error;

    /// The parse of one pre-segmented sentence.
    fn parse(&mut self, sentence: &str) -> Result<Parse, Self::Error>;
}

/// Why a token list is not a dependency tree.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TreeError {
    #[error("the sentence has no tokens")]
    Empty,
    #[error("token {position} has id {id}; ids must run 1..n in order")]
    IdOutOfOrder { position: usize, id: usize },
    #[error("token {id} has head {head}, outside 0..=n")]
    HeadOutOfRange { id: usize, head: usize },
    #[error("several roots: {roots:?}")]
    SeveralRoots { roots: Vec<usize> },
    #[error("token {id} lies on a cycle")]
    Cycle { id: usize },
}

/// A token list that is a tree: ids 1..n in order, every head in 0..=n, one root, no cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyTree {
    tokens: Vec<Token>,
}

impl DependencyTree {
    /// Checks that `tokens` form a tree, reporting the first failed check in the order of
    /// `TreeError`'s variants.
    pub fn new(tokens: Vec<Token>) -> Result<Self, TreeError> {
        non_empty(&tokens)?;
        ids_in_order(&tokens)?;
        heads_in_range(&tokens)?;
        single_root(&tokens)?;
        acyclic(&tokens)?;
        Ok(Self { tokens })
    }

    /// The tokens, ids 1..n in order.
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
}

fn non_empty(tokens: &[Token]) -> Result<(), TreeError> {
    if tokens.is_empty() {
        Err(TreeError::Empty)
    } else {
        Ok(())
    }
}

fn ids_in_order(tokens: &[Token]) -> Result<(), TreeError> {
    match tokens.iter().enumerate().find(|(i, t)| t.id != i + 1) {
        Some((i, t)) => Err(TreeError::IdOutOfOrder {
            position: i + 1,
            id: t.id,
        }),
        None => Ok(()),
    }
}

fn heads_in_range(tokens: &[Token]) -> Result<(), TreeError> {
    match tokens.iter().find(|t| t.head > tokens.len()) {
        Some(t) => Err(TreeError::HeadOutOfRange {
            id: t.id,
            head: t.head,
        }),
        None => Ok(()),
    }
}

/// Rejects more than one root; no root at all is left to `acyclic`, which then finds a cycle.
fn single_root(tokens: &[Token]) -> Result<(), TreeError> {
    let roots: Vec<usize> = tokens
        .iter()
        .filter(|t| t.head == 0)
        .map(|t| t.id)
        .collect();
    if roots.len() > 1 {
        Err(TreeError::SeveralRoots { roots })
    } else {
        Ok(())
    }
}

fn acyclic(tokens: &[Token]) -> Result<(), TreeError> {
    match tokens.iter().find_map(|t| cycle_reached_from(tokens, t.id)) {
        Some(id) => Err(TreeError::Cycle { id }),
        None => Ok(()),
    }
}

/// A token on the cycle that the head chain from `id` runs into, or `None` if the chain reaches
/// the root. A chain that has not reached 0 after n steps has entered a cycle and stays on it.
fn cycle_reached_from(tokens: &[Token], id: usize) -> Option<usize> {
    let mut at = id;
    for _ in 0..tokens.len() {
        if at == 0 {
            return None;
        }
        at = tokens[at - 1].head;
    }
    (at != 0).then_some(at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{tokens_from_conllu, well_formed_tree};
    use proptest::prelude::*;
    use proptest::sample::Index;

    /// The ids of `x` and of every token whose head chain passes through `x`.
    fn subtree(tokens: &[Token], x: usize) -> Vec<usize> {
        let passes_through = |mut id: usize| {
            for _ in 0..=tokens.len() {
                if id == x {
                    return true;
                }
                if id == 0 {
                    return false;
                }
                id = tokens[id - 1].head;
            }
            false
        };
        tokens
            .iter()
            .map(|t| t.id)
            .filter(|&id| passes_through(id))
            .collect()
    }

    fn non_roots(tokens: &[Token]) -> Vec<usize> {
        tokens
            .iter()
            .filter(|t| t.head != 0)
            .map(|t| t.id)
            .collect()
    }

    /// Whether following heads from `id` comes back to `id`.
    fn lies_on_a_cycle(tokens: &[Token], id: usize) -> bool {
        let mut h = tokens[id - 1].head;
        for _ in 0..tokens.len() {
            if h == id {
                return true;
            }
            if h == 0 {
                return false;
            }
            h = tokens[h - 1].head;
        }
        false
    }

    /// Given an empty token list
    /// When a tree is built from it
    /// Then it is rejected as `Empty`
    #[test]
    fn rejects_an_empty_sentence() {
        assert_eq!(DependencyTree::new(vec![]), Err(TreeError::Empty));
    }

    /// Given `1 a →2, 2 b →1, 3 c root` (a two-token cycle beside a root), and `1 a →2, 2 b →1`
    /// (no root at all)
    /// When a tree is built from them
    /// Then both are rejected as a `Cycle` through token 1 or 2
    #[test]
    fn rejects_a_cycle_beside_a_root_and_a_rootless_list() {
        let beside_root = "
            1 a a X _ _ 2 dep
            2 b b X _ _ 1 dep
            3 c c X _ _ 0 root";
        let rootless = "
            1 a a X _ _ 2 dep
            2 b b X _ _ 1 dep";
        for conllu in [beside_root, rootless] {
            let result = DependencyTree::new(tokens_from_conllu(conllu));
            assert!(
                matches!(result, Err(TreeError::Cycle { id: 1 | 2 })),
                "{conllu:?}: {result:?}"
            );
        }
    }

    proptest! {
        /// Given any generated well-formed tree
        /// When a tree is built from its tokens
        /// Then it is accepted and keeps the tokens
        #[test]
        fn accepts_every_well_formed_tree(tokens in well_formed_tree()) {
            let tree = DependencyTree::new(tokens.clone());
            prop_assert_eq!(tree.map(|t| t.tokens().to_vec()), Ok(tokens));
        }

        /// Given a well-formed tree with one token's head moved past the last id
        /// When a tree is built from it
        /// Then it is rejected as `HeadOutOfRange` naming that token and head
        #[test]
        fn rejects_a_head_out_of_range(mut tokens in well_formed_tree(), at in any::<Index>(), k in 0usize..5) {
            let n = tokens.len();
            let token = &mut tokens[at.index(n)];
            token.head = n + 1 + k;
            let expected = TreeError::HeadOutOfRange { id: token.id, head: token.head };
            prop_assert_eq!(DependencyTree::new(tokens), Err(expected));
        }

        /// Given a well-formed tree of at least 2 tokens with a non-root's head set to 0
        /// When a tree is built from it
        /// Then it is rejected as `SeveralRoots` naming both roots
        #[test]
        fn rejects_several_roots(mut tokens in well_formed_tree(), at in any::<Index>()) {
            let candidates = non_roots(&tokens);
            prop_assume!(!candidates.is_empty());
            let x = candidates[at.index(candidates.len())];
            tokens[x - 1].head = 0;
            let roots: Vec<usize> = tokens.iter().filter(|t| t.head == 0).map(|t| t.id).collect();
            prop_assert_eq!(roots.len(), 2);
            prop_assert_eq!(DependencyTree::new(tokens), Err(TreeError::SeveralRoots { roots }));
        }

        /// Given a well-formed tree of at least 2 tokens with a non-root x re-headed to a token of
        /// its own subtree (x itself included)
        /// When a tree is built from it
        /// Then it is rejected as a `Cycle` naming a token on the cycle
        #[test]
        fn rejects_a_cycle(mut tokens in well_formed_tree(), at in any::<Index>(), to in any::<Index>()) {
            let candidates = non_roots(&tokens);
            prop_assume!(!candidates.is_empty());
            let x = candidates[at.index(candidates.len())];
            let below = subtree(&tokens, x);
            tokens[x - 1].head = below[to.index(below.len())];
            match DependencyTree::new(tokens.clone()) {
                Err(TreeError::Cycle { id }) => prop_assert!(lies_on_a_cycle(&tokens, id), "{}", id),
                other => prop_assert!(false, "expected a cycle, got {:?}", other),
            }
        }

        /// Given a well-formed tree of at least 2 tokens with the ids of two tokens swapped
        /// When a tree is built from it
        /// Then it is rejected as `IdOutOfOrder` at the first of the two positions
        #[test]
        fn rejects_ids_out_of_order(mut tokens in well_formed_tree(), a in any::<Index>(), b in any::<Index>()) {
            let n = tokens.len();
            prop_assume!(n >= 2);
            let i = a.index(n);
            let j = (i + 1 + b.index(n - 1)) % n;
            let (id_i, id_j) = (tokens[i].id, tokens[j].id);
            tokens[i].id = id_j;
            tokens[j].id = id_i;
            let (first, second) = (i.min(j), i.max(j));
            let expected = TreeError::IdOutOfOrder { position: first + 1, id: second + 1 };
            prop_assert_eq!(DependencyTree::new(tokens), Err(expected));
        }
    }
}
