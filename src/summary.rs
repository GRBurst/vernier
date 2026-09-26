//! The per-file summary `vernier analyze` prints in M1: prose spans and words.

use std::path::Path;

use crate::prose::{SourceFormat, prose};
use crate::words::count_words;

/// How much prose one file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileSummary {
    pub spans: usize,
    pub words: usize,
}

/// Counts the prose spans of `source` and the words in them.
pub fn summarize(source: &str, format: SourceFormat) -> FileSummary {
    let spans = prose(source, format);
    FileSummary {
        spans: spans.len(),
        words: spans.iter().map(|span| count_words(span.text())).sum(),
    }
}

/// The summary line for one file, e.g. `notes.md: 3 prose spans, 42 words`.
pub fn render(path: &Path, summary: &FileSummary) -> String {
    format!(
        "{}: {} prose spans, {} words",
        path.display(),
        summary.spans,
        summary.words
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
            let expected = FileSummary {
                spans: spans.len(),
                words: spans.iter().map(|s| count_words(s.text())).sum(),
            };
            prop_assert_eq!(summarize(&md, SourceFormat::Markdown), expected);
            prop_assert_eq!(expected.words, count_words(&md));
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
        };
        assert_eq!(
            render(Path::new("notes.md"), &summary),
            "notes.md: 3 prose spans, 42 words"
        );
    }
}
