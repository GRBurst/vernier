//! The per-file summary `vernier analyze` prints: prose spans and words (M1), the surface counts
//! and scores (M2), then the nominalization ratio and passive voice (M4).

use std::path::Path;

use crate::analysis::{file_nominalizations, surface_counts};
use crate::block::Block;
use crate::nominalization::NominalizationCount;
use crate::prose::{SourceFormat, blocks, prose};
use crate::readability::{Readability, SurfaceCounts, readability};
use crate::words::count_words;

/// How much prose one file holds, and how readable it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FileSummary {
    pub spans: usize,
    pub words: usize,
    pub counts: SurfaceCounts,
    pub readability: Option<Readability>,
    pub nominalizations: NominalizationCount,
}

/// Counts the prose spans of `source` and the words of its block prose (so inline markup inside a
/// word splits no word), and scores its sentences.
pub fn summarize(source: &str, format: SourceFormat) -> FileSummary {
    let counts = surface_counts(source, format);
    FileSummary {
        spans: prose(source, format).len(),
        words: blocks(source, format)
            .iter()
            .map(|spans| count_words(Block::from_spans(spans).text()))
            .sum(),
        counts,
        readability: readability(counts),
        nominalizations: file_nominalizations(source, format),
    }
}

/// The summary of one file: M1's line (`notes.md: 3 prose spans, 42 words`), then three indented
/// lines with the surface counts, the scores (or their absence), and the nominalization ratio
/// (or its absence) with passive voice, which is absent without a parse.
pub fn render(path: &Path, summary: &FileSummary) -> String {
    let c = summary.counts;
    format!(
        "{}: {} prose spans, {} words\n  {} sentences, {} syllables, {} complex words\n  {}\n  {}, passive voice absent (no parse)",
        path.display(),
        summary.spans,
        summary.words,
        c.sentences,
        c.syllables,
        c.complex_words,
        summary
            .readability
            .map_or_else(|| "metrics absent (no sentences)".to_owned(), render_scores),
        render_nominalizations(summary.nominalizations)
    )
}

fn render_nominalizations(n: NominalizationCount) -> String {
    n.ratio().map_or_else(
        || "nominalization ratio absent (no words)".to_owned(),
        |ratio| {
            format!(
                "nominalization ratio {ratio:.3} ({} of {} words)",
                n.nominalizations, n.words
            )
        },
    )
}

fn render_scores(r: Readability) -> String {
    format!(
        "Flesch Reading Ease {:.2}, Flesch-Kincaid Grade {:.2}, Gunning Fog {:.2}, average sentence length {:.2}",
        r.flesch_reading_ease, r.flesch_kincaid_grade, r.gunning_fog, r.average_sentence_length
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn nominal_paragraphs() -> impl Strategy<Value = String> {
        let word = prop::sample::select(vec![
            "the",
            "**decisions**",
            "implementation's",
            "cities",
            "rations",
            "we",
            "*Movement*.",
        ]);
        let para = prop::collection::vec(word, 1..10).prop_map(|w| format!("{}.", w.join(" ")));
        prop::collection::vec(para, 0..4).prop_map(|p| p.join("\n\n"))
    }

    fn paragraphs() -> impl Strategy<Value = String> {
        let word = prop::sample::select(vec!["alpha", "beta", "café", "x2"]);
        let para = prop::collection::vec(word, 1..8).prop_map(|w| w.join(" "));
        prop::collection::vec(para, 0..5).prop_map(|p| p.join("\n\n"))
    }

    proptest! {
        /// Given Markdown paragraphs
        /// When the file is summarized
        /// Then it counts the extracted spans and the words in them
        #[test]
        fn counts_the_extracted_spans_and_their_words(md in paragraphs()) {
            let spans = prose(&md, SourceFormat::Markdown);
            let (expected_spans, expected_words) = (
                spans.len(),
                spans.iter().map(|s| count_words(s.text())).sum::<usize>(),
            );
            let summary = summarize(&md, SourceFormat::Markdown);
            prop_assert_eq!((summary.spans, summary.words), (expected_spans, expected_words));
            prop_assert_eq!(expected_words, count_words(&md));
        }

        /// Given Markdown paragraphs
        /// When the file is summarized
        /// Then its counts are its sentences' summed counts, their words are the file's words,
        /// and its scores are computed from those counts
        #[test]
        fn scores_the_summed_sentence_counts(md in paragraphs()) {
            let summary = summarize(&md, SourceFormat::Markdown);
            prop_assert_eq!(summary.counts, surface_counts(&md, SourceFormat::Markdown));
            prop_assert_eq!(summary.counts.words, summary.words);
            prop_assert_eq!(summary.readability, readability(summary.counts));
        }

        /// Given Markdown paragraphs with nominalizations, markup and possessives
        /// When the file is summarized
        /// Then its nominalization count is the file's count from `analyze`, over the file's words
        #[test]
        fn nominalizations_are_the_analyzed_files(md in nominal_paragraphs()) {
            let summary = summarize(&md, SourceFormat::Markdown);
            let thresholds = crate::analysis::Thresholds { max_sentence_len: 25, max_mdd: 3.0, max_tree_depth: 5, max_clauses: 2 };
            let file = crate::analysis::analyze(&md, SourceFormat::Markdown, &thresholds);
            prop_assert_eq!(summary.nominalizations, file.nominalizations);
            prop_assert_eq!(summary.nominalizations.words, summary.counts.words);
        }

        /// Given Markdown paragraphs
        /// When a code block and a heading are added
        /// Then the word count does not change
        #[test]
        fn code_and_headings_add_no_words(md in paragraphs()) {
            let more = format!("# Title words\n\n{md}\n\n```\nlet code = words;\n```\n");
            prop_assert_eq!(
                summarize(&more, SourceFormat::Markdown).words,
                summarize(&md, SourceFormat::Markdown).words
            );
        }

        /// Given Markdown paragraphs
        /// When every word is wrapped in emphasis
        /// Then the word count does not change
        #[test]
        fn emphasis_hides_no_words(md in paragraphs()) {
            let emphasize = |para: &str| para.split(' ').map(|w| format!("*{w}*")).collect::<Vec<_>>().join(" ");
            let emphasized = md.split("\n\n").map(emphasize).collect::<Vec<_>>().join("\n\n");
            prop_assert_eq!(
                summarize(&emphasized, SourceFormat::Markdown).words,
                summarize(&md, SourceFormat::Markdown).words
            );
        }
    }

    /// A paragraph of words, each drawn as two halves and wrapped in emphasis, strong or a link
    /// as a whole or around one half (intraword markup), optionally followed by a comma or full
    /// stop; paired with the same paragraph without markup.
    fn marked_and_stripped() -> impl Strategy<Value = (String, String)> {
        let shape = prop::sample::select(vec![
            "{a}{b}",
            "*{a}{b}*",
            "**{a}{b}**",
            "[{a}{b}](u)",
            "{a}*{b}*",
            "*{a}*{b}",
            "{a}**{b}**",
        ]);
        let word = (
            "[a-z]{1,6}",
            "[a-z]{1,6}",
            shape,
            prop::sample::select(vec!["", ",", "."]),
        )
            .prop_map(|(a, b, shape, p)| {
                let marked = shape.replace("{a}", &a).replace("{b}", &b);
                (format!("{marked}{p}"), format!("{a}{b}{p}"))
            });
        let para = prop::collection::vec(word, 1..10).prop_map(|words| {
            let (marked, stripped): (Vec<String>, Vec<String>) = words.into_iter().unzip();
            (marked.join(" "), stripped.join(" "))
        });
        prop::collection::vec(para, 1..4).prop_map(|paras| {
            let (marked, stripped): (Vec<String>, Vec<String>) = paras.into_iter().unzip();
            (marked.join("\n\n"), stripped.join("\n\n"))
        })
    }

    proptest! {
        /// Given paragraphs with inline markup around and inside words, and the same paragraphs
        /// stripped of it
        /// When both files are summarized
        /// Then they have the same word count, and it equals the sentences' summed words
        #[test]
        fn inline_markup_changes_no_word_count((marked, stripped) in marked_and_stripped()) {
            let (m, s) = (summarize(&marked, SourceFormat::Markdown), summarize(&stripped, SourceFormat::Markdown));
            prop_assert_eq!(m.words, s.words, "{:?}", marked);
            prop_assert_eq!(m.words, m.counts.words, "{:?}", marked);
        }
    }

    /// Given two paragraphs with bold, italic, link and intraword (`un*believ*able`) markup, and
    /// the same paragraphs without it
    /// When both are summarized
    /// Then both count 19 words
    #[test]
    fn markup_inside_a_word_keeps_one_word() {
        let marked = "The **proposal**, which the *executive* committee rejected after \
                      [extensive](https://x.y) deliberation, caused un*believ*able delays.\n\n\
                      A second **paragraph** with _emphasis_ here.\n";
        let stripped = "The proposal, which the executive committee rejected after extensive \
                        deliberation, caused unbelievable delays.\n\nA second paragraph with \
                        emphasis here.\n";
        let words = |md: &str| summarize(md, SourceFormat::Markdown).words;
        assert_eq!((words(marked), words(stripped)), (19, 19));
    }

    /// Given a summary of 3 spans and 42 words for `notes.md`
    /// When it is rendered
    /// Then the line reads `notes.md: 3 prose spans, 42 words`
    #[test]
    fn renders_one_line_per_file() {
        let summary = FileSummary {
            spans: 3,
            words: 42,
            counts: SurfaceCounts::default(),
            readability: None,
            nominalizations: NominalizationCount::default(),
        };
        assert_eq!(
            render(Path::new("notes.md"), &summary).lines().next(),
            Some("notes.md: 3 prose spans, 42 words")
        );
    }

    /// Given a summary of 100 words, 4 sentences, 150 syllables and 10 complex words
    /// When it is rendered
    /// Then the second line gives the counts and the third the four scores to 2 decimals
    #[test]
    fn renders_counts_and_scores() {
        let counts = SurfaceCounts {
            words: 100,
            sentences: 4,
            syllables: 150,
            complex_words: 10,
        };
        let summary = FileSummary {
            spans: 4,
            words: 100,
            counts,
            readability: readability(counts),
            nominalizations: NominalizationCount {
                nominalizations: 5,
                words: 100,
            },
        };
        assert_eq!(
            render(Path::new("notes.md"), &summary),
            "notes.md: 4 prose spans, 100 words\n  4 sentences, 150 syllables, 10 complex words\n  \
             Flesch Reading Ease 54.56, Flesch-Kincaid Grade 11.86, Gunning Fog 14.00, average sentence length 25.00\n  \
             nominalization ratio 0.050 (5 of 100 words), passive voice absent (no parse)"
        );
    }

    /// Given a file without sentences
    /// When its summary is rendered
    /// Then the third line says the metrics are absent
    #[test]
    fn renders_absent_metrics() {
        let summary = summarize("# Only a heading\n", SourceFormat::Markdown);
        assert_eq!(
            render(Path::new("h.md"), &summary),
            "h.md: 0 prose spans, 0 words\n  0 sentences, 0 syllables, 0 complex words\n  metrics absent (no sentences)\n  \
             nominalization ratio absent (no words), passive voice absent (no parse)"
        );
    }

    /// Given an empty file and a file of only a heading
    /// When it is summarized and rendered
    /// Then it has no nominalization ratio, and the fourth line says so
    #[test]
    fn empty_file_has_no_nominalization_ratio() {
        for source in ["", "# Only a heading\n"] {
            let summary = summarize(source, SourceFormat::Markdown);
            assert_eq!(summary.nominalizations.ratio(), None, "{source:?}");
            assert_eq!(
                render(Path::new("e.md"), &summary).lines().nth(3),
                Some("  nominalization ratio absent (no words), passive voice absent (no parse)"),
                "{source:?}"
            );
        }
    }

    /// Given the NOMZ sentence
    /// When it is summarized and rendered
    /// Then the fourth line gives the ratio 4/14 to 3 decimals and its counts
    #[test]
    fn renders_the_nominalization_ratio() {
        let summary = summarize(crate::testing::NOMZ_TEXT, SourceFormat::Markdown);
        assert_eq!(
            render(Path::new("n.md"), &summary).lines().nth(3),
            Some("  nominalization ratio 0.286 (4 of 14 words), passive voice absent (no parse)")
        );
    }
}
