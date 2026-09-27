//! Maximum spanning forest (spec 001 M3b): Chu-Liu/Edmonds exactly as the parser model's
//! `ud.py` decodes it. `matrix[head][dep]` scores `head` as the head of `dep`, the diagonal
//! `matrix[j][j]` scores `j` as a root; the result `h` has `h[dep] = head`, `h[j] == j` for a root.

/// The head vector of maximum total score among all forests over `matrix` (`ud.py`'s
/// `chu_liu_edmonds`); `matrix` is square with finite scores.
pub fn chu_liu_edmonds(matrix: &[Vec<f32>]) -> Vec<usize> {
    let greedy = greedy(matrix);
    let Some(Contraction { cycle, rest }) = cycle_members(&greedy) else {
        return greedy;
    };
    let reduced = reduced(matrix);
    let sub_heads = chu_liu_edmonds(&contract(&reduced, &cycle, &rest));
    expand(greedy, &cycle, &rest, &sub_heads, &reduced)
}

/// The nodes of the cycle that is contracted, and every other node, each in ascending order.
struct Contraction {
    cycle: Vec<usize>,
    rest: Vec<usize>,
}

/// The cycle of `heads` with the largest representative and the other nodes, or `None` when
/// `heads` is a forest: `ud.py`'s two fixpoint passes, in place as there (pass 1 prunes the
/// nodes no remaining node heads; pass 2 gives each cycle's nodes one representative).
fn cycle_members(heads: &[usize]) -> Option<Contraction> {
    let mut links: Vec<Option<usize>> = heads
        .iter()
        .enumerate()
        .map(|(node, &head)| (node != head).then_some(head))
        .collect();
    to_fixpoint(&mut links, |links, node| {
        links[node].filter(|_| links.contains(&Some(node)))
    });
    to_fixpoint(&mut links, |links, node| {
        links[node].and_then(|head| links[head])
    });
    let top = links.iter().flatten().max().copied()?;
    let (cycle, rest) = (0..heads.len()).partition(|&node| links[node] == Some(top));
    Some(Contraction { cycle, rest })
}

/// Applies `step` to every entry of `links` in order, in place, until a sweep changes nothing.
fn to_fixpoint(
    links: &mut [Option<usize>],
    step: impl Fn(&[Option<usize>], usize) -> Option<usize>,
) {
    loop {
        let before = links.to_vec();
        for node in 0..links.len() {
            links[node] = step(links, node);
        }
        if links == before.as_slice() {
            return;
        }
    }
}

/// Every score minus its column's maximum (`ud.py`'s `z`).
fn reduced(matrix: &[Vec<f32>]) -> Vec<Vec<f32>> {
    let column_max: Vec<f32> = (0..matrix.len())
        .map(|dep| max_of(matrix.iter().map(|row| row[dep])))
        .collect();
    matrix
        .iter()
        .map(|row| {
            row.iter()
                .zip(&column_max)
                .map(|(x, top)| x - top)
                .collect()
        })
        .collect()
}

fn max_of(values: impl IntoIterator<Item = f32>) -> f32 {
    values.into_iter().fold(f32::NEG_INFINITY, f32::max)
}

/// The `(r + 1) × (r + 1)` problem with the cycle as one node `r` after the `r` other nodes: an
/// arc to or from the cycle scores its best arc to or from a cycle node, its root score the best
/// cycle node's root score.
fn contract(reduced: &[Vec<f32>], cycle: &[usize], rest: &[usize]) -> Vec<Vec<f32>> {
    let into_cycle = |head: usize| max_of(cycle.iter().map(|&c| reduced[head][c]));
    let out_of_cycle = |dep: usize| max_of(cycle.iter().map(|&c| reduced[c][dep]));
    let rows = rest.iter().map(|&head| {
        rest.iter()
            .map(|&dep| reduced[head][dep])
            .chain(std::iter::once(into_cycle(head)))
            .collect()
    });
    let cycle_row = rest
        .iter()
        .map(|&dep| out_of_cycle(dep))
        .chain(std::iter::once(max_of(
            cycle.iter().map(|&c| reduced[c][c]),
        )))
        .collect();
    rows.chain(std::iter::once(cycle_row)).collect()
}

/// The heads of the full problem from the contracted problem's `sub_heads`: every other node
/// takes its head from there (a head at the cycle node becomes the cycle node that scores it
/// best), the cycle keeps its greedy heads but for the node the entering arc reaches.
fn expand(
    greedy: Vec<usize>,
    cycle: &[usize],
    rest: &[usize],
    sub_heads: &[usize],
    reduced: &[Vec<f32>],
) -> Vec<usize> {
    let r = rest.len();
    let best_in_cycle =
        |score: &dyn Fn(usize) -> f32| cycle[argmax(cycle.iter().map(|&c| score(c)))];
    let mut heads = greedy;
    for (index, &node) in rest.iter().enumerate() {
        heads[node] = match sub_heads[index] {
            head if head < r => rest[head],
            _ => best_in_cycle(&|c| reduced[c][node]),
        };
    }
    let cycle_head = sub_heads[r];
    let (entered, head) = if cycle_head < r {
        let from = rest[cycle_head];
        (best_in_cycle(&|c| reduced[from][c]), from)
    } else {
        let root = best_in_cycle(&|c| reduced[c][c]);
        (root, root)
    };
    heads[entered] = head;
    heads
}

/// Each column's best head (the first maximum, as numpy's `argmax(axis=0)`).
fn greedy(matrix: &[Vec<f32>]) -> Vec<usize> {
    (0..matrix.len())
        .map(|dep| argmax(matrix.iter().map(|row| row[dep])))
        .collect()
}

/// The index of the first maximum of `values`; a NaN never wins; 0 when there is none.
pub fn argmax(values: impl IntoIterator<Item = f32>) -> usize {
    let mut best: Option<(usize, f32)> = None;
    for (index, value) in values.into_iter().enumerate() {
        if !value.is_nan() && best.is_none_or(|(_, top)| value > top) {
            best = Some((index, value));
        }
    }
    best.map_or(0, |(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::test_runner::{Config, TestCaseError, TestRunner};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Whether following `heads` from every node reaches a root (`h[r] == r`) without a cycle.
    fn is_forest(heads: &[usize]) -> bool {
        let n = heads.len();
        heads.iter().all(|&h| h < n)
            && (0..n).all(|start| {
                let mut node = start;
                for _ in 0..n {
                    if heads[node] == node {
                        return true;
                    }
                    node = heads[node];
                }
                heads[node] == node
            })
    }

    fn score(matrix: &[Vec<f32>], heads: &[usize]) -> f32 {
        heads
            .iter()
            .enumerate()
            .map(|(dep, &head)| matrix[head][dep])
            .sum()
    }

    /// The maximum score over all forests, by enumerating every head assignment.
    fn brute_force_max(matrix: &[Vec<f32>]) -> f32 {
        let n = matrix.len();
        let total = n.pow(u32::try_from(n).unwrap());
        (0..total)
            .map(|code| {
                (0..n)
                    .scan(code, |rest, _| {
                        let head = *rest % n;
                        *rest /= n;
                        Some(head)
                    })
                    .collect::<Vec<usize>>()
            })
            .filter(|heads| is_forest(heads))
            .map(|heads| score(matrix, &heads))
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// Square matrices of 1–6 nodes with scores in -10..10; in half of them the arcs of a cycle
    /// through 2..=n random nodes get +15, so the greedy heads often contain a cycle.
    fn matrices() -> impl Strategy<Value = Vec<Vec<f32>>> {
        (1usize..=6)
            .prop_flat_map(|n| {
                (
                    prop::collection::vec(prop::collection::vec(-10.0f32..10.0, n), n),
                    any::<bool>(),
                    Just((0..n).collect::<Vec<usize>>()).prop_shuffle(),
                    2usize..=n.max(2),
                )
            })
            .prop_map(|(mut matrix, biased, order, len)| {
                if biased && order.len() >= len {
                    let cycle = &order[..len];
                    for (k, &dep) in cycle.iter().enumerate() {
                        matrix[cycle[(k + 1) % len]][dep] += 15.0;
                    }
                }
                matrix
            })
    }

    /// Given the matrix [[0, 5, 1], [5, 0, 1], [1, 1, 3]], whose greedy heads [1, 0, 2] make 0
    /// and 1 each other's head
    /// When the maximum spanning forest is decoded
    /// Then it is a forest whose score is the maximum over all forests
    #[test]
    fn breaks_the_cycle_of_the_witness() {
        let matrix = vec![
            vec![0.0, 5.0, 1.0],
            vec![5.0, 0.0, 1.0],
            vec![1.0, 1.0, 3.0],
        ];
        assert_eq!(greedy(&matrix), [1, 0, 2]);
        let heads = chu_liu_edmonds(&matrix);
        assert!(is_forest(&heads), "{heads:?}");
        assert!(
            score(&matrix, &heads) >= brute_force_max(&matrix) - 1e-3,
            "{heads:?}"
        );
    }

    /// Given values with ties, a NaN first, and none at all
    /// When their argmax is taken
    /// Then it is the first maximum, never the NaN, and 0 for none
    #[test]
    fn argmax_takes_the_first_maximum_and_never_a_nan() {
        assert_eq!(argmax([1.0, 3.0, 3.0, 2.0]), 1);
        assert_eq!(argmax([f32::NAN, -1.0, -2.0]), 1);
        assert_eq!(argmax([f32::NEG_INFINITY, f32::NEG_INFINITY]), 0);
        assert_eq!(argmax(std::iter::empty()), 0);
    }

    /// Given generated matrices of 1–6 nodes (a quarter or more with a cycle in the greedy heads)
    /// When the maximum spanning forest is decoded
    /// Then it has one head per node, each in range, and is a forest (P1); its score is within
    /// 1e-3 of the brute-force maximum over all forests (P2); it equals the greedy heads whenever
    /// those are a forest (P3)
    #[test]
    fn decodes_a_maximum_spanning_forest() {
        let cyclic = AtomicUsize::new(0);
        let cases = AtomicUsize::new(0);
        let mut runner = TestRunner::new(Config::default());
        let result = runner.run(&matrices(), |matrix| {
            cases.fetch_add(1, Ordering::Relaxed);
            let greedy_heads = greedy(&matrix);
            let heads = chu_liu_edmonds(&matrix);
            let n = matrix.len();
            if !is_forest(&greedy_heads) {
                cyclic.fetch_add(1, Ordering::Relaxed);
            }
            prop_assert_eq!(heads.len(), n);
            prop_assert!(is_forest(&heads), "P1: {:?} on {:?}", heads, matrix);
            let best = brute_force_max(&matrix);
            prop_assert!(
                score(&matrix, &heads) >= best - 1e-3,
                "P2: {:?} scores {} < {} on {:?}",
                heads,
                score(&matrix, &heads),
                best,
                matrix
            );
            if is_forest(&greedy_heads) {
                prop_assert_eq!(&heads, &greedy_heads, "P3 on {:?}", matrix);
            }
            Ok::<(), TestCaseError>(())
        });
        if let Err(failure) = result {
            panic!("{failure}");
        }
        let (cyclic, cases) = (cyclic.into_inner(), cases.into_inner());
        assert!(
            cyclic * 4 >= cases,
            "P4: only {cyclic} of {cases} cases had a greedy cycle"
        );
    }
}
