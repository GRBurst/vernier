//! Byte offset → 1-based line and column, the column counted in Unicode scalar values (spec 001, Definitions).

/// A 1-based line and 1-based column in a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    line: usize,
    column: usize,
}

impl Position {
    /// The 1-based line number.
    pub fn line(self) -> usize {
        self.line
    }

    /// The 1-based column, counted in Unicode scalar values.
    pub fn column(self) -> usize {
        self.column
    }
}

/// Why a byte offset has no position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PositionError {
    #[error("byte offset {offset} is past the end of the source ({len} bytes)")]
    OutOfBounds { offset: usize, len: usize },
    #[error("byte offset {offset} lies inside a character")]
    NotCharBoundary { offset: usize },
}

/// The start offsets of every line of one source; answers position queries.
#[derive(Debug, Clone)]
pub struct LineIndex<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    /// Indexes the lines of `source`; only `\n` ends a line.
    pub fn new(source: &'a str) -> Self {
        let after_newlines = source.match_indices('\n').map(|(i, _)| i + 1);
        Self {
            source,
            line_starts: std::iter::once(0).chain(after_newlines).collect(),
        }
    }

    /// The position of the character starting at `offset` (or of the end, when `offset` is the length).
    pub fn position(&self, offset: usize) -> Result<Position, PositionError> {
        let len = self.source.len();
        if offset > len {
            return Err(PositionError::OutOfBounds { offset, len });
        }
        let before = self
            .source
            .get(..offset)
            .ok_or(PositionError::NotCharBoundary { offset })?;
        // Invariant: `line_starts[0] == 0 <= offset`, so `line >= 1`; every start follows a '\n',
        // so it is a character boundary no greater than `offset`.
        let line = self.line_starts.partition_point(|&start| start <= offset);
        let line_start = self.line_starts[line - 1];
        let column = before[line_start..].chars().count() + 1;
        Ok(Position { line, column })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// The oracle: walk the characters before `offset` one by one.
    fn scan(source: &str, offset: usize) -> Position {
        source[..offset]
            .chars()
            .fold(Position { line: 1, column: 1 }, |p, c| {
                if c == '\n' {
                    Position {
                        line: p.line + 1,
                        column: 1,
                    }
                } else {
                    Position {
                        line: p.line,
                        column: p.column + 1,
                    }
                }
            })
    }

    fn source() -> impl Strategy<Value = String> {
        let piece = prop::sample::select(vec![
            "a", "Z", " ", "é", "漢", "🦀", "\r\n", "\n", "\r", "\t",
        ]);
        prop::collection::vec(piece, 0..60).prop_map(|p| p.concat())
    }

    fn boundaries(source: &str) -> Vec<usize> {
        source
            .char_indices()
            .map(|(i, _)| i)
            .chain([source.len()])
            .collect()
    }

    proptest! {
        /// Given any source with multi-byte characters and LF, CRLF or lone CR line ends
        /// When every character boundary is converted to a position
        /// Then each position equals the one a character-by-character scan yields
        #[test]
        fn every_boundary_converts_to_the_scanned_position(src in source()) {
            let index = LineIndex::new(&src);
            for offset in boundaries(&src) {
                prop_assert_eq!(index.position(offset), Ok(scan(&src, offset)));
            }
        }

        /// Given any source
        /// When an offset inside a multi-byte character is converted
        /// Then the converter refuses it as not on a character boundary
        #[test]
        fn an_offset_inside_a_character_is_refused(src in source()) {
            let index = LineIndex::new(&src);
            for offset in (0..src.len()).filter(|&o| !src.is_char_boundary(o)) {
                prop_assert_eq!(index.position(offset), Err(PositionError::NotCharBoundary { offset }));
            }
        }

        /// Given any source
        /// When an offset past its end is converted
        /// Then the converter refuses it as out of bounds
        #[test]
        fn an_offset_past_the_end_is_refused(src in source(), excess in 1usize..10) {
            let offset = src.len() + excess;
            prop_assert_eq!(
                LineIndex::new(&src).position(offset),
                Err(PositionError::OutOfBounds { offset, len: src.len() })
            );
        }
    }

    /// Given "ab\r\ncé" (one witness beside the properties)
    /// When the offset of "é" is converted
    /// Then it is line 2, column 2
    #[test]
    fn a_crlf_witness() {
        assert_eq!(
            LineIndex::new("ab\r\ncé").position(5),
            Ok(Position { line: 2, column: 2 })
        );
    }
}
