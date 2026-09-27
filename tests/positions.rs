//! Criterion 6 of spec 001 M5: every diagnostic's line and column point at its sentence's first
//! character, and are the same in the text, compact and JSON formats.

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

/// Markdown with multi-byte words before sentence starts, markup at sentence starts, list items,
/// blockquotes and hard-wrapped lines, LF or CRLF.
fn document_source() -> impl Strategy<Value = String> {
    let word = prop::sample::select(vec!["word", "Café", "漢字", "naïve", "x2", "*it*"]);
    let start = prop::sample::select(vec!["The", "**The**", "Émile", "*Une*", "A"]);
    let sentence = (start, prop::collection::vec(word, 0..6))
        .prop_map(|(start, rest)| format!("{start} {}.", rest.join(" ")));
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

/// The (line, column) at the start of a compact line.
fn compact_position(line: &str) -> Option<(usize, usize)> {
    let mut parts = line.strip_prefix("f.md:")?.splitn(3, ':');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

proptest! {
    /// Given generated Markdown in which every sentence is flagged
    /// When each diagnostic is rendered as text, compact and JSON
    /// Then all three give the same line and column, and the source character there is the first
    /// character of the sentence's text
    #[test]
    fn every_format_points_at_the_sentence_start(source in document_source()) {
        let file = analyze(&source, SourceFormat::Markdown, &EVERY_SENTENCE);
        let found = diagnostics(&source, &file, &EVERY_SENTENCE).unwrap();
        let firsts: Vec<char> = blocks(&source, SourceFormat::Markdown)
            .iter()
            .flat_map(|spans| {
                let block = Block::from_spans(spans);
                sentences(&block).iter().filter_map(|s| s.text().chars().next()).collect::<Vec<_>>()
            })
            .collect();
        prop_assert_eq!(found.len(), firsts.len());
        let summary = summarize(&source, SourceFormat::Markdown);
        let report = FileReport { path: "f.md".to_owned(), summary: &summary, diagnostics: &found };
        let json: serde_json::Value = serde_json::from_str(&document(&[report]).unwrap()).unwrap();
        for (i, (d, first)) in found.iter().zip(firsts).enumerate() {
            let text = text_position(&render_text("f.md", &source, d, Style::Plain));
            let compact = compact_position(&render_compact("f.md", d));
            let entry = &json["files"][0]["diagnostics"][i];
            let in_json = entry["line"].as_u64().zip(entry["column"].as_u64())
                .and_then(|(l, c)| Some((usize::try_from(l).ok()?, usize::try_from(c).ok()?)));
            prop_assert_eq!(text, compact, "{:?}", source);
            prop_assert_eq!(compact, in_json, "{:?}", source);
            let (line, column) = text.unwrap();
            prop_assert_eq!(char_at(&source, line, column), Some(first), "{:?} at {}:{}", source, line, column);
        }
    }
}
