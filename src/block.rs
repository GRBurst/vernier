//! Block prose (spec 001, Definitions): one block's spans joined into a single string, with a map
//! from every byte of that string back to the source.

use crate::prose::ProseSpan;

/// The prose of one block: its spans joined directly where only inline delimiters separate them
/// and by one space otherwise, every line feed and carriage return read as a space. Both
/// replacements are one byte for one byte, so each span keeps its length and the map back to the
/// source is one shift per span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    text: String,
    pieces: Vec<Piece>,
}

/// Where one span starts in the block text and in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Piece {
    block_start: usize,
    source_start: usize,
}

impl Block {
    /// Joins `spans` (in source order) into one block.
    pub fn from_spans(spans: &[ProseSpan<'_>]) -> Self {
        let mut text = String::new();
        let mut pieces = Vec::with_capacity(spans.len());
        for span in spans {
            if !pieces.is_empty() && !span.joins_previous() {
                text.push(' ');
            }
            pieces.push(Piece {
                block_start: text.len(),
                source_start: span.range().start,
            });
            text.extend(span.text().chars().map(|c| match c {
                '\n' | '\r' => ' ',
                other => other,
            }));
        }
        Self { text, pieces }
    }

    /// The block's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The source offset of `block_offset`: inside a span, the same character in the source; on a
    /// separator (or the text's end), the end of the span before it. A block of no spans has no
    /// source to point into; its only offset, 0, maps to 0.
    pub fn source_offset(&self, block_offset: usize) -> usize {
        let after = self
            .pieces
            .partition_point(|piece| piece.block_start <= block_offset);
        after
            .checked_sub(1)
            .and_then(|k| self.pieces.get(k))
            .map_or(block_offset, |piece| {
                piece.source_start + (block_offset - piece.block_start)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{Thresholds, analyze};
    use crate::prose::{SourceFormat, blocks};
    use crate::words::words;
    use proptest::prelude::*;

    /// The byte ranges in the block text of each span: a separator space lies before every span
    /// after the first that does not join its predecessor.
    fn span_ranges(spans: &[ProseSpan<'_>]) -> Vec<std::ops::Range<usize>> {
        let mut end = 0;
        spans
            .iter()
            .enumerate()
            .map(|(i, span)| {
                let start = if i > 0 && !span.joins_previous() {
                    end + 1
                } else {
                    end
                };
                end = start + span.text().len();
                start..end
            })
            .collect()
    }

    /// The text of the only block of a Markdown source.
    fn block_text(markdown: &str) -> String {
        let all = blocks(markdown, SourceFormat::Markdown);
        assert_eq!(all.len(), 1, "{markdown:?}");
        Block::from_spans(&all[0]).text().to_owned()
    }

    /// A word, how it is wrapped in inline markup, and the punctuation after it.
    fn marked_word() -> impl Strategy<Value = (String, &'static str, &'static str)> {
        (
            "[a-z]{1,8}",
            prop::sample::select(vec!["{}", "*{}*", "**{}**", "[{}](u)"]),
            prop::sample::select(vec!["", ",", "."]),
        )
    }

    /// The same paragraph without markup, and with each word wrapped as drawn.
    fn plain_and_wrapped(words: &[(String, &str, &str)]) -> (String, String) {
        let plain: Vec<String> = words.iter().map(|(w, _, p)| format!("{w}{p}")).collect();
        let wrapped: Vec<String> = words
            .iter()
            .map(|(w, shape, p)| format!("{}{p}", shape.replace("{}", w)))
            .collect();
        (plain.join(" "), wrapped.join(" "))
    }

    fn defaults() -> Thresholds {
        Thresholds {
            max_sentence_len: 25,
            max_mdd: 3.0,
            max_tree_depth: 5,
            max_clauses: 2,
        }
    }

    /// Given spans split only by inline markup or an escape
    /// When each block is built
    /// Then the spans are joined directly, as the reader sees them (W1–W4)
    #[test]
    fn inline_markup_joins_spans_directly() {
        for (markdown, text) in [
            ("**The proposal**, which", "The proposal, which"),
            ("un*believ*able", "unbelievable"),
            ("a [b](u) c", "a b c"),
            ("a\\*b", "a*b"),
        ] {
            assert_eq!(block_text(markdown), text, "{markdown:?}");
        }
    }

    /// Given a word with emphasis inside it (`un*believ*able`)
    /// When its file is analyzed
    /// Then it counts as one word, like the unmarked word
    #[test]
    fn intraword_emphasis_keeps_one_word() {
        let marked = analyze("un*believ*able", SourceFormat::Markdown, &defaults()).totals;
        let plain = analyze("unbelievable", SourceFormat::Markdown, &defaults()).totals;
        assert_eq!(marked.words, 1);
        assert_eq!(marked, plain);
    }

    /// Given spans separated by a character reference, a line break, inline code, inline HTML,
    /// an image or a bare autolink
    /// When each block is built
    /// Then a space separates them: exactly one where only a break or reference lies between
    /// (W5–W7), and the reader's words wherever a dropped construct adds more (W8, W9)
    #[test]
    fn other_boundaries_separate_spans() {
        for (markdown, text) in [("AT&amp;T", "AT T"), ("a  \nb", "a b"), ("a\nb", "a b")] {
            assert_eq!(block_text(markdown), text, "{markdown:?}");
        }
        for (markdown, expected) in [
            ("a `x` b", vec!["a", "b"]),
            ("a <i>b</i> c", vec!["a", "b", "c"]),
            ("x ![alt](i) y", vec!["x", "y"]),
            ("x ![](i) y", vec!["x", "y"]),
            ("see <http://x.y> now", vec!["see", "now"]),
        ] {
            let text = block_text(markdown);
            assert_eq!(words(&text).collect::<Vec<_>>(), expected, "{markdown:?}");
        }
    }

    fn as_block_char(c: char) -> char {
        if c == '\n' || c == '\r' { ' ' } else { c }
    }

    fn word() -> impl Strategy<Value = String> {
        prop::sample::select(vec!["alpha", "café", "Ωmega", "x2", "Straße", "漢字"])
            .prop_map(str::to_owned)
    }

    fn piece() -> impl Strategy<Value = String> {
        let shape = prop::sample::select(vec![
            "{}",
            "*{}*",
            "**{}**",
            "[{}](https://x.example)",
            "`{}`",
            "&amp; {}",
            "{}.",
        ]);
        (word(), shape).prop_map(|(w, s)| s.replace("{}", &w))
    }

    fn paragraph() -> impl Strategy<Value = String> {
        let line = (word(), prop::collection::vec(piece(), 0..5))
            .prop_map(|(w, p)| format!("{w} {}", p.join(" ")));
        let lines = prop::collection::vec(line, 1..4).prop_map(|l| l.join("\n"));
        (lines, prop::sample::select(vec!["", "- ", "> "]))
            .prop_map(|(l, prefix)| format!("{prefix}{l}"))
    }

    /// A Markdown (or plain-text) source of several blocks, LF or CRLF.
    fn source() -> impl Strategy<Value = (String, SourceFormat)> {
        (
            prop::collection::vec(paragraph(), 0..5),
            any::<bool>(),
            prop::sample::select(vec![SourceFormat::Markdown, SourceFormat::PlainText]),
        )
            .prop_map(|(paras, crlf, format)| {
                let lf = format!("{}\n", paras.join("\n\n"));
                (if crlf { lf.replace('\n', "\r\n") } else { lf }, format)
            })
    }

    proptest! {
        /// Given generated Markdown and plain text, LF or CRLF, and each of its blocks
        /// When any offset inside one of the block's spans is mapped to the source
        /// Then the source holds the same character there (a line break read as a space)
        #[test]
        fn every_span_offset_maps_to_its_source_character((src, format) in source()) {
            for spans in blocks(&src, format) {
                let block = Block::from_spans(&spans);
                for range in span_ranges(&spans) {
                    for (i, c) in block.text()[range.clone()].char_indices() {
                        let at = block.source_offset(range.start + i);
                        prop_assert_eq!(src[at..].chars().next().map(as_block_char), Some(c), "{:?} at {}", src, at);
                    }
                }
            }
        }

        /// Given generated sources and each of their blocks
        /// When the offsets of the block text are mapped in order
        /// Then the source offsets never decrease; a separator space stands exactly before each
        /// span that does not join its predecessor and maps to the end of the span before it; a
        /// joined boundary maps to the start of the joining span
        #[test]
        fn the_map_is_monotone_and_separators_map_to_span_ends((src, format) in source()) {
            for spans in blocks(&src, format) {
                let block = Block::from_spans(&spans);
                let mapped: Vec<usize> = (0..=block.text().len()).map(|o| block.source_offset(o)).collect();
                prop_assert!(mapped.windows(2).all(|w| w[0] <= w[1]));
                let ranges = span_ranges(&spans);
                prop_assert_eq!(ranges.last().map_or(0, |r| r.end), block.text().len());
                for (i, (range, span)) in ranges.iter().zip(&spans).enumerate() {
                    if i > 0 && !span.joins_previous() {
                        prop_assert_eq!(&block.text()[range.start - 1..range.start], " ");
                    }
                    let joined_next = spans.get(i + 1).filter(|next| next.joins_previous());
                    let expected = joined_next.map_or(span.range().end, |next| next.range().start);
                    prop_assert_eq!(block.source_offset(range.end), expected);
                }
            }
        }

        /// Given a paragraph of words, each optionally wrapped in emphasis, strong or a link and
        /// optionally followed by a comma or full stop, and the same paragraph without markup
        /// When their blocks are built and the files analyzed
        /// Then both have the same block text and the same totals
        #[test]
        fn inline_markup_changes_neither_block_text_nor_counts(
            words in prop::collection::vec(marked_word(), 1..12)
        ) {
            let (plain, wrapped) = plain_and_wrapped(&words);
            prop_assert_eq!(block_text(&wrapped), block_text(&plain), "{:?}", wrapped);
            let totals = |md: &str| analyze(md, SourceFormat::Markdown, &defaults()).totals;
            prop_assert_eq!(totals(&wrapped), totals(&plain), "{:?}", wrapped);
        }
    }

    /// Given no spans
    /// When a block is built from them
    /// Then its text is empty and its only offset maps to 0
    #[test]
    fn an_empty_block_maps_its_only_offset_to_zero() {
        let block = Block::from_spans(&[]);
        assert_eq!((block.text(), block.source_offset(0)), ("", 0));
    }

    /// Given a plain-text block hard-wrapped with CRLF after a first paragraph
    /// When the block is built
    /// Then its line breaks read as spaces and its first character maps to its source offset
    #[test]
    fn a_crlf_block_witness() {
        let src = "first\r\n\r\nsecond line\r\nwrapped\r\n";
        let all = blocks(src, SourceFormat::PlainText);
        let block = Block::from_spans(&all[1]);
        assert_eq!(block.text(), "second line  wrapped");
        assert_eq!(block.source_offset(0), 9);
        assert_eq!(block.source_offset(13), 22);
    }
}
