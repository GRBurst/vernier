//! Analysis (spec 001 M2, M3a): per-sentence and per-file surface counts, scores and flags, and
//! the syntactic metrics and flags of a parsed sentence.

use std::ops::Range;

use crate::block::Block;
use crate::prose::{SourceFormat, blocks, prose};
use crate::readability::{Readability, SurfaceCounts, readability};
use crate::sentence::{Sentence, sentences};
use crate::syllables::{count_syllables, is_complex};
use crate::syntax::{CenterEmbedding, SyntacticMetrics};
use crate::words::words;

/// The limits a sentence is checked against (built by the shell from the command line).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// A sentence with more words than this is a `LongSentence`.
    pub max_sentence_len: usize,
    /// A sentence whose mean dependency distance exceeds this is `HighMdd`.
    pub max_mdd: f64,
    /// A sentence whose tree is deeper than this many edges is `DeepTree`.
    pub max_tree_depth: usize,
    /// A sentence with more clausal dependents than this is `ClauseOverload`.
    pub max_clauses: usize,
}

/// A rule a sentence breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    /// More words than `Thresholds::max_sentence_len`.
    LongSentence,
}

/// A syntactic rule a parsed sentence breaks, with the measured value that broke it.
#[derive(Debug, Clone, PartialEq)]
pub enum SyntacticFlag {
    /// A mean dependency distance above `Thresholds::max_mdd`.
    HighMdd { mdd: f64 },
    /// A tree deeper than `Thresholds::max_tree_depth` edges.
    DeepTree { depth: usize },
    /// More clausal dependents than `Thresholds::max_clauses`.
    ClauseOverload { clauses: usize },
    /// A clausal subtree between a subject and its verb.
    CenterEmbedding(CenterEmbedding),
}

/// A parsed sentence's syntactic metrics and the syntactic rules it breaks.
#[derive(Debug, Clone, PartialEq)]
pub struct SentenceSyntax {
    pub metrics: SyntacticMetrics,
    pub flags: Vec<SyntacticFlag>,
}

/// One sentence's position, counts, scores and flags; `syntax` is `None` when it was not parsed.
#[derive(Debug, Clone, PartialEq)]
pub struct SentenceAnalysis {
    pub source_range: Range<usize>,
    pub counts: SurfaceCounts,
    pub readability: Option<Readability>,
    pub flags: Vec<Flag>,
    pub syntax: Option<SentenceSyntax>,
}

/// One file's sentences, the sum of their counts, and the file's scores computed from that sum.
#[derive(Debug, Clone, PartialEq)]
pub struct FileAnalysis {
    pub spans: usize,
    pub sentences: Vec<SentenceAnalysis>,
    pub totals: SurfaceCounts,
    pub readability: Option<Readability>,
}

/// Analyzes `source`, read as `format`: every sentence's counts, scores and flags, and the file's
/// totals and scores (formulas over the summed counts, never averages of sentence scores).
pub fn analyze(source: &str, format: SourceFormat, thresholds: &Thresholds) -> FileAnalysis {
    let sentences: Vec<SentenceAnalysis> = counted_sentences(source, format)
        .into_iter()
        .map(|(source_range, counts)| SentenceAnalysis {
            source_range,
            counts,
            readability: readability(counts),
            flags: flags(counts, thresholds),
            syntax: None,
        })
        .collect();
    let totals = sentences.iter().map(|s| s.counts).sum();
    FileAnalysis {
        spans: prose(source, format).len(),
        sentences,
        totals,
        readability: readability(totals),
    }
}

/// The summed counts of every sentence of `source`, read as `format`.
pub fn surface_counts(source: &str, format: SourceFormat) -> SurfaceCounts {
    counted_sentences(source, format)
        .into_iter()
        .map(|(_, counts)| counts)
        .sum()
}

/// Each sentence's source range and counts, in source order.
fn counted_sentences(source: &str, format: SourceFormat) -> Vec<(Range<usize>, SurfaceCounts)> {
    blocks(source, format)
        .iter()
        .flat_map(|spans| {
            let block = Block::from_spans(spans);
            sentences(&block)
                .iter()
                .map(|s| (s.source_range(), sentence_counts(s)))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// One sentence's counts; only its first word is sentence-initial.
fn sentence_counts(sentence: &Sentence<'_>) -> SurfaceCounts {
    words(sentence.text())
        .enumerate()
        .map(|(index, word)| SurfaceCounts {
            words: 1,
            sentences: 0,
            syllables: count_syllables(word),
            complex_words: usize::from(is_complex(word, index == 0)),
        })
        .sum::<SurfaceCounts>()
        + SurfaceCounts {
            sentences: 1,
            ..SurfaceCounts::default()
        }
}

/// The rules a sentence with `counts` breaks.
fn flags(counts: SurfaceCounts, thresholds: &Thresholds) -> Vec<Flag> {
    let is_long = counts.words > thresholds.max_sentence_len;
    is_long.then_some(Flag::LongSentence).into_iter().collect()
}

/// The syntactic rules a sentence with `metrics` breaks: `HighMdd`, `DeepTree` and
/// `ClauseOverload` when a metric exceeds its threshold, then one `CenterEmbedding` per embedding.
pub fn syntactic_flags(metrics: &SyntacticMetrics, thresholds: &Thresholds) -> Vec<SyntacticFlag> {
    let high_mdd = metrics
        .mdd()
        .filter(|&mdd| mdd > thresholds.max_mdd)
        .map(|mdd| SyntacticFlag::HighMdd { mdd });
    let deep_tree =
        (metrics.depth > thresholds.max_tree_depth).then_some(SyntacticFlag::DeepTree {
            depth: metrics.depth,
        });
    let clause_overload =
        (metrics.clauses > thresholds.max_clauses).then_some(SyntacticFlag::ClauseOverload {
            clauses: metrics.clauses,
        });
    let center_embeddings = metrics
        .center_embeddings
        .iter()
        .cloned()
        .map(SyntacticFlag::CenterEmbedding);
    [high_mdd, deep_tree, clause_overload]
        .into_iter()
        .flatten()
        .chain(center_embeddings)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::DependencyDistance;
    use crate::words::count_words;
    use proptest::prelude::*;

    fn md(source: &str, max_sentence_len: usize) -> FileAnalysis {
        analyze(
            source,
            SourceFormat::Markdown,
            &Thresholds {
                max_sentence_len,
                max_mdd: 3.0,
                max_tree_depth: 5,
                max_clauses: 2,
            },
        )
    }

    fn center_embedding() -> impl Strategy<Value = CenterEmbedding> {
        (
            prop::sample::select(vec!["proposal", "man", "it"]),
            prop::sample::select(vec!["caused", "came", "is"]),
            0usize..20,
        )
            .prop_map(|(subject, verb, words_between)| CenterEmbedding {
                subject: subject.to_owned(),
                verb: verb.to_owned(),
                words_between,
            })
    }

    fn syntactic_metrics() -> impl Strategy<Value = SyntacticMetrics> {
        (
            prop_oneof![1 => Just(0usize), 3 => 1usize..30],
            0usize..4,
            0usize..10,
            0usize..6,
            prop::collection::vec(center_embedding(), 0..3),
        )
            .prop_map(|(dependencies, extra, depth, clauses, center_embeddings)| {
                SyntacticMetrics {
                    distance: DependencyDistance {
                        total: dependencies * (1 + extra) + dependencies / 2,
                        dependencies,
                    },
                    depth,
                    clauses,
                    center_embeddings,
                }
            })
    }

    /// A threshold equal to the measured `value` half the time, so the `>` boundary is exercised.
    fn at_or_near(value: usize) -> impl Strategy<Value = usize> {
        prop_oneof![Just(value), 0usize..10]
    }

    /// Metrics with thresholds that equal each metric half the time.
    fn metrics_and_thresholds() -> impl Strategy<Value = (SyntacticMetrics, Thresholds)> {
        syntactic_metrics().prop_flat_map(|m| {
            let mdd = m.mdd().unwrap_or(1.0);
            let max_mdd = prop_oneof![Just(mdd), 0.5f64..6.0];
            (
                Just(m.clone()),
                max_mdd,
                at_or_near(m.depth),
                at_or_near(m.clauses),
            )
                .prop_map(|(m, max_mdd, max_tree_depth, max_clauses)| {
                    let thresholds = Thresholds {
                        max_sentence_len: 25,
                        max_mdd,
                        max_tree_depth,
                        max_clauses,
                    };
                    (m, thresholds)
                })
        })
    }

    /// Given an empty file, and a file of only a heading and a code block
    /// When it is analyzed
    /// Then it has no sentence, zero counts and absent metrics
    #[test]
    fn empty_file_has_absent_metrics() {
        for source in ["", "# Only a heading\n\n```\nlet code = 1;\n```\n"] {
            let file = md(source, 25);
            assert!(file.sentences.is_empty(), "{source:?}");
            assert_eq!(file.totals, SurfaceCounts::default(), "{source:?}");
            assert_eq!(file.readability, None, "{source:?}");
        }
    }

    /// Given "Australia celebrated in Australia. It rained."
    /// When it is analyzed
    /// Then the first sentence counts 4 words, 13 syllables and 2 complex words (the capitalized
    /// non-initial "Australia" is exempt), the second 2 words and 2 syllables, and the file sums them
    #[test]
    fn counts_a_witness_per_sentence_and_per_file() {
        let file = md("Australia celebrated in Australia. It rained.\n", 25);
        let counts: Vec<SurfaceCounts> = file.sentences.iter().map(|s| s.counts).collect();
        let first = SurfaceCounts {
            words: 4,
            sentences: 1,
            syllables: 13,
            complex_words: 2,
        };
        let second = SurfaceCounts {
            words: 2,
            sentences: 1,
            syllables: 2,
            complex_words: 0,
        };
        assert_eq!(counts, [first, second]);
        assert_eq!(file.totals, first + second);
        assert_eq!(file.readability, readability(first + second));
    }

    fn sentence_of(words: usize) -> String {
        let body = vec!["word"; words].join(" ");
        format!("{body}.")
    }

    fn document() -> impl Strategy<Value = String> {
        let word = prop::sample::select(vec![
            "Alpha",
            "beta",
            "café",
            "3.14",
            "celebrated",
            "Dr.",
            "understanding",
        ]);
        let sentence = prop::collection::vec(word, 1..40).prop_map(|w| format!("{}.", w.join(" ")));
        let para = prop::collection::vec(sentence, 1..4).prop_map(|s| s.join(" "));
        prop::collection::vec(para, 0..4).prop_map(|p| p.join("\n\n"))
    }

    proptest! {
        /// Given a sentence of n words and any maximum
        /// When it is analyzed
        /// Then it is flagged LongSentence exactly when n exceeds the maximum
        #[test]
        fn long_sentence_iff_more_words_than_max(n in 1usize..60, max in 0usize..60) {
            let file = md(&sentence_of(n), max);
            prop_assert_eq!(file.sentences.len(), 1);
            prop_assert_eq!(file.sentences[0].counts.words, n);
            prop_assert_eq!(file.sentences[0].flags.contains(&Flag::LongSentence), n > max);
        }

        /// Given any syntactic metrics and thresholds (each threshold equal to its metric half the
        /// time)
        /// When the syntactic flags are computed
        /// Then `HighMdd` carries the MDD exactly when it exceeds `max_mdd`, `DeepTree` the depth
        /// exactly when it exceeds `max_tree_depth`, `ClauseOverload` the clause count exactly when
        /// it exceeds `max_clauses`, and there is one `CenterEmbedding` per embedding, in order
        #[test]
        fn syntactic_flags_follow_the_thresholds((m, t) in metrics_and_thresholds()) {
            let flags = syntactic_flags(&m, &t);
            let high_mdd: Vec<f64> = flags.iter().filter_map(|f| match f {
                SyntacticFlag::HighMdd { mdd } => Some(*mdd),
                _ => None,
            }).collect();
            let expected_mdd: Vec<f64> = m.mdd().filter(|&x| x > t.max_mdd).into_iter().collect();
            prop_assert_eq!(high_mdd, expected_mdd);
            let deep = flags.iter().filter(|f| matches!(f, SyntacticFlag::DeepTree { .. })).collect::<Vec<_>>();
            let expected_deep = (m.depth > t.max_tree_depth).then_some(SyntacticFlag::DeepTree { depth: m.depth });
            prop_assert_eq!(deep, expected_deep.iter().collect::<Vec<_>>());
            let overload = flags.iter().filter(|f| matches!(f, SyntacticFlag::ClauseOverload { .. })).collect::<Vec<_>>();
            let expected_overload = (m.clauses > t.max_clauses).then_some(SyntacticFlag::ClauseOverload { clauses: m.clauses });
            prop_assert_eq!(overload, expected_overload.iter().collect::<Vec<_>>());
            let embedded: Vec<&CenterEmbedding> = flags.iter().filter_map(|f| match f {
                SyntacticFlag::CenterEmbedding(e) => Some(e),
                _ => None,
            }).collect();
            prop_assert_eq!(embedded, m.center_embeddings.iter().collect::<Vec<_>>());
        }

        /// Given generated Markdown paragraphs
        /// When a file is analyzed
        /// Then its totals are the sum of its sentences' counts, its scores are computed from
        /// them, and each sentence's counts follow its words
        #[test]
        fn file_counts_are_the_sum_of_sentence_counts(doc in document(), max in 0usize..40) {
            let file = md(&doc, max);
            let sum: SurfaceCounts = file.sentences.iter().map(|s| s.counts).sum();
            prop_assert_eq!(file.totals, sum);
            prop_assert_eq!(file.totals.sentences, file.sentences.len());
            prop_assert_eq!(file.totals.words, count_words(&doc));
            prop_assert_eq!(file.readability, readability(file.totals));
            for s in &file.sentences {
                let text = &doc[s.source_range.clone()];
                let words: Vec<&str> = words(text).collect();
                prop_assert_eq!(s.counts.syllables, words.iter().map(|w| count_syllables(w)).sum::<usize>());
                let complex = words.iter().enumerate().filter(|(i, w)| is_complex(w, *i == 0)).count();
                prop_assert_eq!(s.counts.complex_words, complex);
                prop_assert_eq!(s.readability, readability(s.counts));
            }
        }
    }
}
