//! Sentences (spec 001, Definitions): UAX #29 sentence segments of one block, merged forward past a
//! listed abbreviation, trimmed of whitespace, each holding at least one word.

use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::block::Block;
use crate::words::count_words;

/// The committed abbreviation list: a segment ending in one of these is merged with the next.
pub const ABBREVIATIONS: [&str; 13] = [
    "Mr.", "Mrs.", "Ms.", "Dr.", "Prof.", "St.", "e.g.", "i.e.", "etc.", "vs.", "Fig.", "No.",
    "cf.",
];

/// One sentence of a block: its trimmed text and its byte range in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence<'b> {
    text: &'b str,
    source_start: usize,
    source_end: usize,
    block: &'b Block,
    block_start: usize,
}

impl<'b> Sentence<'b> {
    /// The sentence's text, borrowed from its block (line breaks read as spaces).
    pub fn text(&self) -> &'b str {
        self.text
    }

    /// Where the sentence starts and ends in the source file.
    pub fn source_range(&self) -> Range<usize> {
        self.source_start..self.source_end
    }

    /// The source offset of byte `offset_in_sentence` of the text, through the block's map.
    pub fn source_offset(&self, offset_in_sentence: usize) -> usize {
        self.block
            .source_offset(self.block_start + offset_in_sentence)
    }
}

/// The sentences of `block`, in order.
pub fn sentences(block: &Block) -> Vec<Sentence<'_>> {
    let text = block.text();
    let mut carry: Option<usize> = None;
    let mut found = Vec::new();
    for (i, segment) in text.split_sentence_bound_indices() {
        let start = carry.take().unwrap_or(i);
        let end = i + segment.len();
        if text.get(start..end).is_some_and(ends_with_abbreviation) {
            carry = Some(start);
        } else {
            found.extend(sentence(block, start..end));
        }
    }
    found.extend(carry.and_then(|start| sentence(block, start..text.len())));
    found
}

/// Whether the segment's last whitespace-separated token, without leading punctuation such as
/// `(`, is a listed abbreviation; matched case-sensitively, so a sentence ending in "no." ends.
fn ends_with_abbreviation(segment: &str) -> bool {
    segment
        .split_whitespace()
        .last()
        .map(|token| token.trim_start_matches(|c: char| !c.is_alphanumeric()))
        .is_some_and(|token| ABBREVIATIONS.contains(&token))
}

/// The sentence in `range` of the block text, trimmed, if it holds a word.
fn sentence(block: &Block, range: Range<usize>) -> Option<Sentence<'_>> {
    let raw = block.text().get(range.clone())?;
    let text = raw.trim();
    if count_words(text) == 0 {
        return None;
    }
    let start = range.start + (raw.len() - raw.trim_start().len());
    Some(Sentence {
        text,
        source_start: block.source_offset(start),
        source_end: block.source_offset(start + text.len()),
        block,
        block_start: start,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prose::{SourceFormat, blocks};
    use proptest::prelude::*;

    /// The sentence texts of the first block of `source`.
    fn split(source: &str, format: SourceFormat) -> Vec<String> {
        let all = blocks(source, format);
        let block = Block::from_spans(&all[0]);
        sentences(&block)
            .iter()
            .map(|s| s.text().to_owned())
            .collect()
    }

    fn split_md(markdown: &str) -> Vec<String> {
        split(markdown, SourceFormat::Markdown)
    }

    /// Given a second sentence whose verb is in bold (`… was **written** by …`)
    /// When the offset of each word of its text is mapped to the source
    /// Then each points at that word's first character in the source, markup skipped
    #[test]
    fn maps_offsets_in_the_sentence_text_to_the_source() {
        let source = "First one. The report was **written** by the committee.";
        let all = blocks(source, SourceFormat::Markdown);
        let block = Block::from_spans(&all[0]);
        let found = sentences(&block);
        let second = &found[1];
        assert_eq!(second.text(), "The report was written by the committee.");
        for word in ["The", "report", "was", "written", "by", "committee"] {
            let in_text = second.text().find(word).unwrap();
            let in_source = source.find(word).unwrap();
            assert_eq!(second.source_offset(in_text), in_source, "{word}");
        }
        assert_eq!(second.source_offset(0), second.source_range().start);
    }

    /// Given the UAX #29 spike's cases
    /// When they are split into sentences
    /// Then each yields the expected sentences
    #[test]
    fn splits_the_spike_cases() {
        assert_eq!(
            split_md("Dr. Smith reviewed the draft. It was fine."),
            ["Dr. Smith reviewed the draft.", "It was fine."]
        );
        assert_eq!(
            split_md("Use a crate, e.g. Serde, for this."),
            ["Use a crate, e.g. Serde, for this."]
        );
        assert_eq!(
            split_md("The value rose to 3.14 percent in 2024."),
            ["The value rose to 3.14 percent in 2024."]
        );
        assert_eq!(split_md("Wait... Really?"), ["Wait...", "Really?"]);
    }

    /// Given plain text hard-wrapped across two lines, LF or CRLF
    /// When it is split into sentences
    /// Then it is one sentence
    #[test]
    fn hard_wrapped_plain_text_is_one_sentence() {
        for (text, joined) in [
            (
                "This sentence is hard-wrapped\nacross two source lines.\n",
                "This sentence is hard-wrapped across two source lines.",
            ),
            (
                "This sentence is hard-wrapped\r\nacross two source lines.\r\n",
                "This sentence is hard-wrapped  across two source lines.",
            ),
        ] {
            assert_eq!(split(text, SourceFormat::PlainText), [joined]);
        }
    }

    /// Given each abbreviation of the committed list inside a two-sentence text
    /// When it is split into sentences
    /// Then the abbreviation does not end a sentence: there are exactly 2
    #[test]
    fn each_listed_abbreviation_merges() {
        for abbreviation in ABBREVIATIONS {
            let text = format!("We saw {abbreviation} Smith there. He left.");
            assert_eq!(split_md(&text).len(), 2, "{text}: {:?}", split_md(&text));
            let bracketed = format!("We saw ({abbreviation} Smith) there. He left.");
            assert_eq!(
                split_md(&bracketed).len(),
                2,
                "{bracketed}: {:?}",
                split_md(&bracketed)
            );
        }
    }

    /// Given "no." ending a sentence in lower case
    /// When it is split into sentences
    /// Then it ends the sentence, since the list is matched case-sensitively (`No.` only)
    #[test]
    fn lowercase_no_does_not_merge() {
        assert_eq!(
            split_md("He said no. Then he left."),
            ["He said no.", "Then he left."]
        );
    }

    /// Given a paragraph ending in an abbreviation, and one of punctuation only after a sentence
    /// When it is split into sentences
    /// Then the trailing abbreviation stays in the last sentence and no sentence without a word is kept
    #[test]
    fn a_final_abbreviation_and_wordless_segments() {
        assert_eq!(
            split_md("We bought apples, pears, etc."),
            ["We bought apples, pears, etc."]
        );
        assert_eq!(split_md("It works. «!» Go."), ["It works.", "Go."]);
    }

    /// Given a sentence split across emphasis and a link in Markdown
    /// When it is split
    /// Then its source range runs from its first to past its last character in the source
    #[test]
    fn a_source_range_witness() {
        let md = "Intro here. A *second* [one](https://x.example) ends.\n";
        let all = blocks(md, SourceFormat::Markdown);
        let block = Block::from_spans(&all[0]);
        let found = sentences(&block);
        assert_eq!(found.len(), 2);
        assert_eq!(
            &md[found[1].source_range()],
            "A *second* [one](https://x.example) ends."
        );
        assert_eq!(&md[found[0].source_range()], "Intro here.");
    }

    fn piece() -> impl Strategy<Value = String> {
        prop::sample::select(vec![
            "alpha",
            "Beta",
            "café",
            "3.14",
            "x2",
            "Dr.",
            "e.g.",
            "(e.g.",
            "No.",
            "no.",
            "etc.",
            "U.S.",
            "end.",
            "Why?",
            "Wow!",
            "Wait...",
            "\"Quoted.\"",
            "—",
            "...",
            "*em.*",
            "**Strong**",
            "[link.](https://x.example)",
            "`code.`",
            "&amp;",
        ])
        .prop_map(str::to_owned)
    }

    fn source() -> impl Strategy<Value = (String, SourceFormat)> {
        let line = prop::collection::vec(piece(), 1..10).prop_map(|p| p.join(" "));
        let para = prop::collection::vec(line, 1..4).prop_map(|l| l.join("\n"));
        (
            prop::collection::vec(para, 1..4),
            any::<bool>(),
            prop::sample::select(vec![SourceFormat::Markdown, SourceFormat::PlainText]),
        )
            .prop_map(|(paras, crlf, format)| {
                let lf = format!("{}\n", paras.join("\n\n"));
                (if crlf { lf.replace('\n', "\r\n") } else { lf }, format)
            })
    }

    /// Where `inner` starts inside `outer`, which it is a slice of.
    fn offset_in(outer: &str, inner: &str) -> usize {
        inner.as_ptr() as usize - outer.as_ptr() as usize
    }

    proptest! {
        /// Given generated Markdown and plain text
        /// When each block is split into sentences
        /// Then the sentences' words add up to the block's words, and those to its spans' words
        #[test]
        fn sentences_conserve_words((src, format) in source()) {
            for spans in blocks(&src, format) {
                let block = Block::from_spans(&spans);
                let in_sentences: usize = sentences(&block).iter().map(|s| count_words(s.text())).sum();
                let in_spans: usize = spans.iter().map(|s| count_words(s.text())).sum();
                prop_assert_eq!(in_sentences, count_words(block.text()), "{:?}", block.text());
                prop_assert_eq!(count_words(block.text()), in_spans);
            }
        }

        /// Given generated Markdown and plain text
        /// When each block is split into sentences
        /// Then every sentence is trimmed and holds a word, and the sentences are strictly
        /// ordered and disjoint in the block and in the source
        #[test]
        fn sentences_are_trimmed_ordered_and_disjoint((src, format) in source()) {
            for spans in blocks(&src, format) {
                let block = Block::from_spans(&spans);
                let found = sentences(&block);
                for s in &found {
                    prop_assert!(!s.text().is_empty() && s.text().trim() == s.text(), "{:?}", s);
                    prop_assert!(count_words(s.text()) > 0);
                    prop_assert!(s.source_range().start < s.source_range().end);
                    prop_assert!(src.get(s.source_range()).is_some());
                }
                for pair in found.windows(2) {
                    let first_end = offset_in(block.text(), pair[0].text()) + pair[0].text().len();
                    prop_assert!(first_end <= offset_in(block.text(), pair[1].text()), "{:?}", pair);
                    prop_assert!(pair[0].source_range().end <= pair[1].source_range().start, "{:?}", pair);
                }
            }
        }
    }
}
