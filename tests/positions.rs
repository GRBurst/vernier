//! Criterion 6 of spec 001 M5: every diagnostic's line and column point at its sentence's first
//! character, and are the same in the text, compact and JSON formats; its JSON end points just
//! past the sentence's last character, where the text format's underline ends.

use proptest::prelude::*;
use vernier::analysis::{Thresholds, analyze};
use vernier::block::Block;
use vernier::diagnostic::{Style, diagnostics, render_compact, render_text};
use vernier::json::{FileReport, document};
use vernier::prose::{SourceFormat, blocks};
use vernier::sentence::sentences;
use vernier::summary::summarize;

/// Every sentence flagged: no sentence has fewer than 1 word.
const EVERY_SENTENCE: Thresholds = Thresholds {
    max_sentence_len: 0,
    max_mdd: 3.0,
    max_tree_depth: 5,
    max_clauses: 2,
};

/// Markdown with multi-byte words before sentence starts, markup at sentence starts and ends, list
/// items, blockquotes and hard-wrapped lines, LF or CRLF.
fn document_source() -> impl Strategy<Value = String> {
    let word = prop::sample::select(vec!["word", "Café", "漢字", "naïve", "x2", "*it*"]);
    let start = prop::sample::select(vec!["The", "**The**", "Émile", "*Une*", "A"]);
    let end = prop::sample::select(vec![".", " *end.*", " **x.**"]);
    let sentence = (start, prop::collection::vec(word, 0..6), end)
        .prop_map(|(start, rest, end)| format!("{start} {}{end}", rest.join(" ")));
    let line = prop::collection::vec(sentence, 1..4).prop_map(|s| s.join(" "));
    let block = (
        prop::collection::vec(line, 1..3),
        prop::sample::select(vec!["", "- ", "> "]),
    )
        .prop_map(|(lines, prefix)| format!("{prefix}{}", lines.join("\n")));
    (prop::collection::vec(block, 0..4), any::<bool>()).prop_map(|(blocks, crlf)| {
        let lf = format!("{}\n", blocks.join("\n\n"));
        if crlf { lf.replace('\n', "\r\n") } else { lf }
    })
}

/// The character at 1-based `line`, `column` (in Unicode scalar values) of `source`.
fn char_at(source: &str, line: usize, column: usize) -> Option<char> {
    source.split('\n').nth(line - 1)?.chars().nth(column - 1)
}

/// The (line, column) of the ` --> path:line:col` line of a text diagnostic.
fn text_position(text: &str) -> Option<(usize, usize)> {
    let at = text.lines().nth(1)?.trim().strip_prefix("--> f.md:")?;
    let (line, column) = at.split_once(':')?;
    Some((line.parse().ok()?, column.parse().ok()?))
}

/// The first and one-past-last column of the underline of a text diagnostic, if it has one line.
fn underline(text: &str) -> Option<(usize, usize)> {
    let mut carets = text.lines().filter(|l| l.contains('^'));
    let marks = carets.next()?.split_once("| ")?.1;
    if carets.next().is_some() {
        return None;
    }
    let first = marks.chars().position(|c| c == '^')? + 1;
    Some((first, first + marks.chars().filter(|&c| c == '^').count()))
}

/// Whether the text format's underline columns are scalar columns on `line`: one sentence line,
/// every character one column wide, short enough not to be cut.
fn comparable(source: &str, line: usize, end_line: usize) -> bool {
    source
        .split('\n')
        .nth(line - 1)
        .is_some_and(|l| line == end_line && l.chars().count() < 100 && !l.contains(['漢', '字']))
}

/// A JSON number as a `usize`.
fn number(entry: &serde_json::Value, key: &str) -> Option<usize> {
    usize::try_from(entry[key].as_u64()?).ok()
}

/// The (line, column) at the start of a compact line.
fn compact_position(line: &str) -> Option<(usize, usize)> {
    let mut parts = line.strip_prefix("f.md:")?.splitn(3, ':');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

proptest! {
    /// Given generated Markdown in which every sentence is flagged
    /// When each diagnostic is rendered as text, compact and JSON
    /// Then all three give the same line and column, and the source character there is the first
    /// character of the sentence's text; the source character before the JSON end is its last,
    /// and a one-line underline of the text format runs from the column to that end
    #[test]
    fn every_format_points_at_the_sentence_start(source in document_source()) {
        let file = analyze(&source, SourceFormat::Markdown, &EVERY_SENTENCE);
        let found = diagnostics(&source, &file, &EVERY_SENTENCE).unwrap();
        let ends: Vec<(char, char)> = blocks(&source, SourceFormat::Markdown)
            .iter()
            .flat_map(|spans| {
                let block = Block::from_spans(spans);
                sentences(&block)
                    .iter()
                    .filter_map(|s| s.text().chars().next().zip(s.text().chars().next_back()))
                    .collect::<Vec<_>>()
            })
            .collect();
        prop_assert_eq!(found.len(), ends.len());
        let summary = summarize(&source, SourceFormat::Markdown);
        let report = FileReport { path: "f.md".to_owned(), summary: &summary, diagnostics: &found };
        let json: serde_json::Value = serde_json::from_str(&document(&[report]).unwrap()).unwrap();
        for (i, (d, (first, last))) in found.iter().zip(ends).enumerate() {
            let rendered = render_text("f.md", &source, d, Style::Plain);
            let text = text_position(&rendered);
            let compact = compact_position(&render_compact("f.md", d));
            let entry = &json["files"][0]["diagnostics"][i];
            let in_json = number(entry, "line").zip(number(entry, "column"));
            let end = number(entry, "end_line").zip(number(entry, "end_column"));
            prop_assert_eq!(text, compact, "{:?}", source);
            prop_assert_eq!(compact, in_json, "{:?}", source);
            let (line, column) = text.unwrap();
            prop_assert_eq!(char_at(&source, line, column), Some(first), "{:?} at {}:{}", source, line, column);
            let (end_line, end_column) = end.unwrap();
            prop_assert_eq!(char_at(&source, end_line, end_column - 1), Some(last), "{:?} to {}:{}", source, end_line, end_column);
            if comparable(&source, line, end_line) {
                prop_assert_eq!(underline(&rendered), Some((column, end_column)), "{:?}\n{}", source, rendered);
            }
        }
    }
}
