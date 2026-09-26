//! Analysis (spec 001 M2): per-sentence and per-file surface counts, scores and flags.

use std::ops::Range;

use crate::block::Block;
use crate::prose::{SourceFormat, blocks, prose};
use crate::readability::{Readability, SurfaceCounts, readability};
use crate::sentence::{Sentence, sentences};
use crate::syllables::{count_syllables, is_complex};
use crate::words::words;

/// The limits a sentence is checked against (built by the shell from the command line).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thresholds {
    /// A sentence with more words than this is a `LongSentence`.
    pub max_sentence_len: usize,
}

/// A rule a sentence breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    /// More words than `Thresholds::max_sentence_len`.
    LongSentence,
}

/// One sentence's position, counts, scores and flags.
#[derive(Debug, Clone, PartialEq)]
pub struct SentenceAnalysis {
    pub source_range: Range<usize>,
    pub counts: SurfaceCounts,
    pub readability: Option<Readability>,
    pub flags: Vec<Flag>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::words::count_words;
    use proptest::prelude::*;

    fn md(source: &str, max_sentence_len: usize) -> FileAnalysis {
        analyze(
            source,
            SourceFormat::Markdown,
            &Thresholds { max_sentence_len },
        )
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
