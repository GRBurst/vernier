//! The per-file summary `vernier analyze` prints: prose spans and words (M1), then the surface
//! counts and scores (M2).

use std::path::Path;

use crate::analysis::surface_counts;
use crate::prose::{SourceFormat, prose};
use crate::readability::{Readability, SurfaceCounts, readability};
use crate::words::count_words;

/// How much prose one file holds, and how readable it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FileSummary {
    pub spans: usize,
    pub words: usize,
    pub counts: SurfaceCounts,
    pub readability: Option<Readability>,
}

/// Counts the prose spans of `source` and the words in them, and scores its sentences.
pub fn summarize(source: &str, format: SourceFormat) -> FileSummary {
    let spans = prose(source, format);
    let counts = surface_counts(source, format);
    FileSummary {
        spans: spans.len(),
        words: spans.iter().map(|span| count_words(span.text())).sum(),
        counts,
        readability: readability(counts),
    }
}

/// The summary of one file: M1's line (`notes.md: 3 prose spans, 42 words`), then two indented
/// lines with the surface counts and the scores (or their absence).
pub fn render(path: &Path, summary: &FileSummary) -> String {
    let c = summary.counts;
    format!(
        "{}: {} prose spans, {} words\n  {} sentences, {} syllables, {} complex words\n  {}",
        path.display(),
        summary.spans,
        summary.words,
        c.sentences,
        c.syllables,
        c.complex_words,
        summary
            .readability
            .map_or_else(|| "metrics absent (no sentences)".to_owned(), render_scores)
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
        };
        assert_eq!(
            render(Path::new("notes.md"), &summary),
            "notes.md: 4 prose spans, 100 words\n  4 sentences, 150 syllables, 10 complex words\n  \
             Flesch Reading Ease 54.56, Flesch-Kincaid Grade 11.86, Gunning Fog 14.00, average sentence length 25.00"
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
            "h.md: 0 prose spans, 0 words\n  0 sentences, 0 syllables, 0 complex words\n  metrics absent (no sentences)"
        );
    }
}
