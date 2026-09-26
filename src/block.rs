//! Block prose (spec 001, Definitions): one block's spans joined into a single string, with a map
//! from every byte of that string back to the source.

use crate::prose::ProseSpan;

/// The prose of one block: its spans joined by one space, every line feed and carriage return
/// read as a space. Both replacements are one byte for one byte, so each span keeps its length
/// and the map back to the source is one shift per span.
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
            if !pieces.is_empty() {
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
    use crate::prose::{SourceFormat, blocks};
    use proptest::prelude::*;

    /// The byte ranges in the block text of each span (a separator space lies between two).
    fn span_ranges(spans: &[ProseSpan<'_>]) -> Vec<std::ops::Range<usize>> {
        let mut start = 0;
        spans
            .iter()
            .map(|span| {
                let range = start..start + span.text().len();
                start = range.end + 1;
                range
            })
            .collect()
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
        /// Then the source offsets never decrease, and a separator maps to the end of the span before it
        #[test]
        fn the_map_is_monotone_and_separators_map_to_span_ends((src, format) in source()) {
            for spans in blocks(&src, format) {
                let block = Block::from_spans(&spans);
                let mapped: Vec<usize> = (0..=block.text().len()).map(|o| block.source_offset(o)).collect();
                prop_assert!(mapped.windows(2).all(|w| w[0] <= w[1]));
                for (range, span) in span_ranges(&spans).iter().zip(&spans) {
                    prop_assert_eq!(block.source_offset(range.end), span.range().end);
                }
            }
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
