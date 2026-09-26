//! Prose extraction (spec 001 M1): keep only the text a reader reads as prose,
//! each span tied to its byte range in the source.

use std::ops::Range;
use std::path::Path;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};

/// A run of prose text and the byte range it occupies in the source.
///
/// Built only by slicing the source, so `source[range] == text` by construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProseSpan<'a> {
    text: &'a str,
    range: Range<usize>,
}

impl<'a> ProseSpan<'a> {
    /// The span's text, borrowed from the source.
    pub fn text(&self) -> &'a str {
        self.text
    }

    /// The span's byte range in the source.
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

/// How a file's text is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Markdown,
    PlainText,
}

impl SourceFormat {
    /// `.md` and `.markdown` (any letter case) are Markdown; anything else is plain text.
    pub fn from_path(path: &Path) -> Self {
        let is_markdown = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
            });
        if is_markdown {
            Self::Markdown
        } else {
            Self::PlainText
        }
    }
}

/// The prose spans of `source`, read as `format`.
pub fn prose(source: &str, format: SourceFormat) -> Vec<ProseSpan<'_>> {
    match format {
        SourceFormat::Markdown => extract_prose(source),
        SourceFormat::PlainText => plain_text_prose(source),
    }
}

/// The prose blocks of `source`, read as `format`: each block the spans of one paragraph, list
/// item or blockquote line run (Markdown), or of one run of non-blank lines (plain text).
pub fn blocks(source: &str, format: SourceFormat) -> Vec<Vec<ProseSpan<'_>>> {
    match format {
        SourceFormat::Markdown => extract_blocks(source),
        SourceFormat::PlainText => plain_text_blocks(source),
    }
}

/// The blocks of a plain-text file: maximal runs of non-blank lines, one span each, without the
/// final line break.
pub fn plain_text_blocks(text: &str) -> Vec<Vec<ProseSpan<'_>>> {
    let (mut runs, mut run, mut start): (Vec<Range<usize>>, Option<Range<usize>>, usize) =
        (Vec::new(), None, 0);
    for line in text.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let content = content.strip_suffix('\r').unwrap_or(content);
        let range = start..start + content.len();
        start += line.len();
        if content.trim().is_empty() {
            runs.extend(run.take());
        } else {
            run = Some(run.map_or(range.clone(), |open| open.start..range.end));
        }
    }
    runs.extend(run);
    // Every range starts after a '\n' and ends before a '\r' or '\n', so it lies on char boundaries.
    runs.into_iter()
        .filter_map(|range| {
            Some(vec![ProseSpan {
                text: text.get(range.clone())?,
                range,
            }])
        })
        .collect()
}

/// A plain-text file is one prose span covering the whole file.
pub fn plain_text_prose(text: &str) -> Vec<ProseSpan<'_>> {
    vec![ProseSpan {
        text,
        range: 0..text.len(),
    }]
}

/// What an open Markdown element does to the text inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// Its text is prose (paragraphs, list items).
    Prose,
    /// Its text is never prose, whatever encloses it.
    Dropped,
    /// It passes its parent's verdict through (emphasis, quotes, lists).
    Neutral,
}

/// The parser options: tables, math and frontmatter are recognised so they can be dropped.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_MATH
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
}

// why: matched exhaustively, so a new pulldown-cmark tag breaks the build until it gets a role.
fn role(tag: &Tag<'_>) -> Role {
    match tag {
        Tag::Paragraph | Tag::Item => Role::Prose,
        Tag::Heading { .. }
        | Tag::CodeBlock(_)
        | Tag::HtmlBlock
        | Tag::Table(_)
        | Tag::Image { .. }
        | Tag::MetadataBlock(_) => Role::Dropped,
        Tag::BlockQuote(_)
        | Tag::List(_)
        | Tag::FootnoteDefinition(_)
        | Tag::DefinitionList
        | Tag::DefinitionListTitle
        | Tag::DefinitionListDefinition
        | Tag::TableHead
        | Tag::TableRow
        | Tag::TableCell
        | Tag::Emphasis
        | Tag::Strong
        | Tag::Strikethrough
        | Tag::Superscript
        | Tag::Subscript => Role::Neutral,
        // A bare autolink's text is its URL; a written link's text is prose.
        Tag::Link { link_type, .. } => match link_type {
            LinkType::Autolink | LinkType::Email => Role::Dropped,
            LinkType::Inline
            | LinkType::Reference
            | LinkType::ReferenceUnknown
            | LinkType::Collapsed
            | LinkType::CollapsedUnknown
            | LinkType::Shortcut
            | LinkType::ShortcutUnknown
            | LinkType::WikiLink { .. } => Role::Neutral,
        },
    }
}

/// Text is prose inside some prose element and no dropped one.
fn keeps(open: &[Open]) -> bool {
    open.iter().any(|e| e.role == Role::Prose) && !open.iter().any(|e| e.role == Role::Dropped)
}

/// The prose blocks of a Markdown document, in source order.
///
/// Every start or end of a block-level element closes the current block, so tight list items
/// (which hold no paragraph) and nested lists split where a reader sees a new block.
pub fn extract_blocks(markdown: &str) -> Vec<Vec<ProseSpan<'_>>> {
    let mut open: Vec<Open> = Vec::new();
    let mut blocks = Blocks::default();
    for (event, range) in Parser::new_ext(markdown, options()).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                let element = Open {
                    role: role(&tag),
                    is_inline: is_inline(&tag),
                };
                blocks.close_unless(element.is_inline);
                open.push(element);
            }
            Event::End(_) => {
                let element = open.pop();
                blocks.close_unless(element.is_some_and(|e| e.is_inline));
            }
            Event::Text(text) if keeps(&open) => blocks.push(span_at(markdown, &text, range)),
            _ => {}
        }
    }
    blocks.finish()
}

/// The prose spans of a Markdown document, in source order.
pub fn extract_prose(markdown: &str) -> Vec<ProseSpan<'_>> {
    extract_blocks(markdown).into_iter().flatten().collect()
}

/// An open Markdown element: what it does to its text, and whether it splits blocks.
#[derive(Debug, Clone, Copy)]
struct Open {
    role: Role,
    is_inline: bool,
}

/// The finished blocks and the one being filled; no block is ever empty.
#[derive(Debug, Default)]
struct Blocks<'a> {
    done: Vec<Vec<ProseSpan<'a>>>,
    current: Vec<ProseSpan<'a>>,
}

impl<'a> Blocks<'a> {
    fn push(&mut self, span: Option<ProseSpan<'a>>) {
        self.current.extend(span);
    }

    fn close_unless(&mut self, is_inline: bool) {
        if !is_inline && !self.current.is_empty() {
            self.done.push(std::mem::take(&mut self.current));
        }
    }

    fn finish(mut self) -> Vec<Vec<ProseSpan<'a>>> {
        self.close_unless(false);
        self.done
    }
}

// why: matched exhaustively, so a new pulldown-cmark tag breaks the build until it is placed.
fn is_inline(tag: &Tag<'_>) -> bool {
    match tag {
        Tag::Emphasis
        | Tag::Strong
        | Tag::Strikethrough
        | Tag::Link { .. }
        | Tag::Image { .. }
        | Tag::Superscript
        | Tag::Subscript => true,
        Tag::Paragraph
        | Tag::Heading { .. }
        | Tag::BlockQuote(_)
        | Tag::CodeBlock(_)
        | Tag::HtmlBlock
        | Tag::List(_)
        | Tag::Item
        | Tag::FootnoteDefinition(_)
        | Tag::DefinitionList
        | Tag::DefinitionListTitle
        | Tag::DefinitionListDefinition
        | Tag::Table(_)
        | Tag::TableHead
        | Tag::TableRow
        | Tag::TableCell
        | Tag::MetadataBlock(_) => false,
    }
}

/// The span at `range`, if the parsed text is exactly the source there.
///
/// Only a character reference (`&amp;`) decodes to something other than its source;
/// its source spelling is no prose word, so it yields no span.
fn span_at<'a>(source: &'a str, parsed: &str, range: Range<usize>) -> Option<ProseSpan<'a>> {
    let text = source.get(range.clone()).filter(|slice| *slice == parsed)?;
    Some(ProseSpan { text, range })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given paths with Markdown and other extensions
    /// When the source format is detected
    /// Then only `.md` and `.markdown`, in any letter case, are Markdown
    #[test]
    fn detects_markdown_by_extension() {
        for md in ["a.md", "dir/b.markdown", "README.MD", "c.Markdown"] {
            assert_eq!(
                SourceFormat::from_path(Path::new(md)),
                SourceFormat::Markdown,
                "{md}"
            );
        }
        for txt in ["a.txt", "b", "c.md.bak", ".md", "d.mdx"] {
            assert_eq!(
                SourceFormat::from_path(Path::new(txt)),
                SourceFormat::PlainText,
                "{txt}"
            );
        }
    }

    proptest::proptest! {
        /// Given any plain text, Markdown syntax included
        /// When its prose is taken
        /// Then it is exactly one span covering the whole text
        #[test]
        fn plain_text_is_one_span(text in ".*") {
            let spans = prose(&text, SourceFormat::PlainText);
            proptest::prop_assert_eq!(spans.len(), 1);
            proptest::prop_assert_eq!(spans[0].range(), 0..text.len());
            proptest::prop_assert_eq!(spans[0].text(), text.as_str());
        }
    }

    fn texts(markdown: &str) -> Vec<&str> {
        extract_prose(markdown)
            .iter()
            .map(ProseSpan::text)
            .collect()
    }

    /// Given a paragraph, a tight and a loose list, and a blockquote
    /// When prose is extracted
    /// Then the text of each is returned, each span with its byte range in the source
    #[test]
    fn keeps_paragraph_list_item_and_blockquote_text() {
        let md = "One para.\n\n- tight item\n- two\n\n* loose\n\n  second\n\n> quoted\n> lines\n";
        assert_eq!(
            texts(md),
            [
                "One para.",
                "tight item",
                "two",
                "loose",
                "second",
                "quoted",
                "lines"
            ]
        );
        assert!(
            extract_prose(md)
                .iter()
                .all(|s| md.get(s.range()) == Some(s.text()))
        );
    }

    fn block_texts(markdown: &str) -> Vec<Vec<&str>> {
        extract_blocks(markdown)
            .iter()
            .map(|block| block.iter().map(ProseSpan::text).collect())
            .collect()
    }

    /// Given a tight list, whose items hold no paragraph
    /// When blocks are extracted
    /// Then each item is its own block
    #[test]
    fn tight_list_items_are_separate_blocks() {
        assert_eq!(
            block_texts("- one item\n- two *items*\n"),
            [vec!["one item"], vec!["two ", "items"]]
        );
    }

    /// Given a list item holding a nested list
    /// When blocks are extracted
    /// Then the parent's text, the nested item and the next item are three blocks
    #[test]
    fn nested_list_is_its_own_block() {
        assert_eq!(
            block_texts("- parent text\n  - child text\n- next\n"),
            [vec!["parent text"], vec!["child text"], vec!["next"]]
        );
    }

    /// Given a paragraph with inline markup and line breaks, then a blockquote
    /// When blocks are extracted
    /// Then inline markup and line breaks split no block, and the blockquote is its own block
    #[test]
    fn inline_markup_splits_no_block() {
        assert_eq!(
            block_texts(
                "See *this* and [that](https://x.example)\nwrapped.\n\n> quoted\n> lines\n"
            ),
            [
                vec!["See ", "this", " and ", "that", "wrapped."],
                vec!["quoted", "lines"]
            ]
        );
    }

    /// Given plain text with hard-wrapped lines, a whitespace-only line and CRLF line ends
    /// When its blocks are taken
    /// Then blank lines separate blocks, each one span without its final line break
    #[test]
    fn plain_text_blank_lines_separate_blocks() {
        let text = "a line\r\nwrapped here\r\n\r\n \t\n\nnext block\n";
        let blocks = plain_text_blocks(text);
        let texts: Vec<Vec<&str>> = blocks
            .iter()
            .map(|b| b.iter().map(ProseSpan::text).collect())
            .collect();
        assert_eq!(texts, [vec!["a line\r\nwrapped here"], vec!["next block"]]);
        assert!(
            blocks
                .iter()
                .flatten()
                .all(|s| text.get(s.range()) == Some(s.text()))
        );
        assert_eq!(
            plain_text_blocks(" \n\t\n"),
            Vec::<Vec<ProseSpan<'_>>>::new()
        );
    }

    /// Every word of the extracted prose, for "is this sentinel gone?" checks.
    fn joined(markdown: &str) -> String {
        texts(markdown).join(" ")
    }

    /// Asserts a construct is dropped both at the top level and inside a list item,
    /// where an enclosing prose element is open.
    fn assert_dropped(construct: &str) {
        let top = format!("keep\n\n{construct}\n\nkeep\n");
        let nested = format!("- keep\n\n{}\n", indent(construct));
        for md in [top, nested] {
            let prose = joined(&md);
            assert!(!prose.contains("gone"), "{md:?} leaked into {prose:?}");
            assert!(prose.contains("keep"), "{md:?} lost its prose: {prose:?}");
        }
    }

    fn indent(block: &str) -> String {
        block.lines().map(|l| format!("  {l}\n")).collect()
    }

    /// Given a fenced and an indented code block
    /// When prose is extracted
    /// Then no code text is returned
    #[test]
    fn drops_code_blocks() {
        assert_dropped("```\ngone\n```");
        assert_dropped("    gone");
    }

    /// Given inline code inside a paragraph
    /// When prose is extracted
    /// Then the code text is not returned
    #[test]
    fn drops_inline_code() {
        assert_dropped("keep `gone` keep");
    }

    /// Given an HTML block and inline HTML
    /// When prose is extracted
    /// Then no markup is returned
    #[test]
    fn drops_html() {
        assert_dropped("<div>gone</div>");
        assert_dropped("keep <span class=\"gone\"> keep");
    }

    /// Given YAML and TOML frontmatter
    /// When prose is extracted
    /// Then the frontmatter is not returned
    #[test]
    fn drops_frontmatter() {
        for md in [
            "---\ntitle: gone\n---\n\nkeep\n",
            "+++\ntitle = \"gone\"\n+++\n\nkeep\n",
        ] {
            assert_eq!(texts(md), ["keep"]);
        }
    }

    /// Given inline and display math
    /// When prose is extracted
    /// Then no math is returned
    #[test]
    fn drops_math() {
        assert_dropped("keep $gone$ keep");
        assert_dropped("$$\ngone\n$$");
    }

    /// Given a table
    /// When prose is extracted
    /// Then no cell text is returned
    #[test]
    fn drops_tables() {
        assert_dropped("| gone | gone |\n|---|---|\n| gone | gone |");
    }

    /// Given ATX and setext headings
    /// When prose is extracted
    /// Then no heading text is returned
    #[test]
    fn drops_headings() {
        assert_dropped("# gone");
        assert_dropped("gone\n====");
    }

    /// Given an image with alt text
    /// When prose is extracted
    /// Then the alt text is not returned
    #[test]
    fn drops_image_alt_text() {
        assert_dropped("keep ![gone](gone.png) keep");
    }

    /// Given an inline, a reference and a collapsed link
    /// When prose is extracted
    /// Then the link text is kept and no URL is returned
    #[test]
    fn keeps_link_text_but_not_its_url() {
        let md = "See [the guide](https://gone.example) and [ref][r] and [ref].\n\n[r]: https://gone.example\n[ref]: https://gone.example\n";
        let prose = joined(md);
        assert!(
            prose.contains("the guide") && prose.contains("ref"),
            "{prose:?}"
        );
        assert!(!prose.contains("gone"), "{prose:?}");
    }

    /// Given a bare URL and a bare e-mail autolink
    /// When prose is extracted
    /// Then neither contributes any text
    #[test]
    fn a_bare_autolink_contributes_no_text() {
        assert_dropped("keep <https://gone.example> keep");
        assert_dropped("keep <gone@example.org> keep");
    }

    /// Given a character reference such as `&amp;`
    /// When prose is extracted
    /// Then it contributes no span, so no span's text differs from its source slice
    #[test]
    fn a_character_reference_contributes_no_span() {
        assert_eq!(
            texts("keep &amp; keep &#169; keep\n"),
            ["keep ", " keep ", " keep"]
        );
    }

    mod generated {
        use super::*;
        use proptest::prelude::*;
        use unicode_segmentation::UnicodeSegmentation;

        /// A generated document and the words its prose constructs hold.
        #[derive(Debug, Clone)]
        struct Doc {
            markdown: String,
            kept: Vec<String>,
        }

        fn word() -> impl Strategy<Value = String> {
            prop::sample::select(vec!["alpha", "café", "naïve", "Ωmega", "x2", "Straße"])
                .prop_map(str::to_owned)
        }

        /// One inline piece: its Markdown and the prose words it contributes.
        fn inline() -> impl Strategy<Value = (String, Vec<String>)> {
            let kept = word().prop_flat_map(|w| {
                prop::sample::select(vec![
                    "{}",
                    "*{}*",
                    "**{}**",
                    "[{}](https://gone.example)",
                    "\\*{}",
                    "&amp; {}",
                ])
                .prop_map(move |shape| (shape.replace("{}", &w), vec![w.clone()]))
            });
            let dropped = prop::sample::select(vec![
                "`gone`",
                "$gone$",
                "![gone](gone.png)",
                "<https://gone.example>",
                "<gone@example.org>",
                "<span data-x=\"gone\">",
            ])
            .prop_map(|md| (md.to_owned(), Vec::new()));
            prop_oneof![3 => kept, 1 => dropped]
        }

        /// A paragraph line; it starts with a plain word so no piece turns it into another block.
        fn line() -> impl Strategy<Value = (String, Vec<String>)> {
            (word(), prop::collection::vec(inline(), 0..6)).prop_map(|(first, pieces)| {
                let mut kept = vec![first.clone()];
                let mut md = first;
                for (piece, words) in pieces {
                    md = format!("{md} {piece}");
                    kept.extend(words);
                }
                (md, kept)
            })
        }

        fn block() -> impl Strategy<Value = (String, Vec<String>)> {
            let dropped = prop::sample::select(vec![
                "# gone heading",
                "```\ngone code\n```",
                "<div>gone</div>",
                "| gone | gone |\n|---|---|\n| gone | gone |",
                "$$\ngone\n$$",
            ])
            .prop_map(|md| (md.to_owned(), Vec::new()));
            prop_oneof![
                2 => line(),
                1 => line().prop_map(|(md, kept)| (format!("- {md}"), kept)),
                1 => line().prop_map(|(md, kept)| (format!("> {md}"), kept)),
                1 => dropped,
            ]
        }

        fn doc() -> impl Strategy<Value = Doc> {
            let frontmatter = prop::sample::select(vec![
                "",
                "---\ntitle: gone\n---\n\n",
                "+++\ntitle = 1\ngone = 2\n+++\n\n",
            ]);
            (
                frontmatter,
                prop::collection::vec(block(), 0..8),
                any::<bool>(),
            )
                .prop_map(|(front, blocks, crlf)| {
                    let (mds, kept): (Vec<String>, Vec<Vec<String>>) = blocks.into_iter().unzip();
                    let lf = format!("{front}{}\n", mds.join("\n\n"));
                    let markdown = if crlf { lf.replace('\n', "\r\n") } else { lf };
                    Doc {
                        markdown,
                        kept: kept.concat(),
                    }
                })
        }

        proptest! {
            /// Given generated Markdown mixing prose with every dropped construct, LF or CRLF
            /// When prose is extracted
            /// Then the source slice at every span's byte range equals the span's text
            #[test]
            fn every_span_is_its_source_slice(doc in doc()) {
                for span in extract_prose(&doc.markdown) {
                    prop_assert_eq!(doc.markdown.get(span.range()), Some(span.text()));
                }
            }

            /// Given generated Markdown
            /// When its blocks are extracted
            /// Then no block is empty and, flattened, they are exactly the extracted prose spans
            #[test]
            fn blocks_flatten_to_the_prose_spans(doc in doc()) {
                let blocks = extract_blocks(&doc.markdown);
                prop_assert!(blocks.iter().all(|b| !b.is_empty()), "{:?}", doc.markdown);
                prop_assert_eq!(blocks.concat(), extract_prose(&doc.markdown));
            }

            /// Given generated plain text of non-blank lines separated by blank ones, LF or CRLF
            /// When its blocks are taken
            /// Then there is one single-span block per run of lines, each span its source slice
            #[test]
            fn plain_text_has_one_block_per_line_run(
                runs in prop::collection::vec(prop::collection::vec("[a-z][a-z .]{0,10}", 1..4), 0..5),
                crlf in any::<bool>(),
            ) {
                let eol = if crlf { "\r\n" } else { "\n" };
                let text = runs.iter().map(|r| r.join(eol)).collect::<Vec<_>>().join(&format!("{eol} {eol}"));
                let blocks = plain_text_blocks(&text);
                prop_assert_eq!(blocks.len(), runs.len());
                for (block, run) in blocks.iter().zip(&runs) {
                    prop_assert_eq!(block.len(), 1);
                    prop_assert_eq!(block[0].text(), run.join(eol));
                    prop_assert_eq!(text.get(block[0].range()), Some(block[0].text()));
                }
            }

            /// Given generated Markdown
            /// When prose is extracted
            /// Then the spans come in source order and never overlap
            #[test]
            fn spans_are_ordered_and_disjoint(doc in doc()) {
                let spans = extract_prose(&doc.markdown);
                for pair in spans.windows(2) {
                    prop_assert!(pair[0].range().end <= pair[1].range().start, "{:?}", pair);
                }
            }

            /// Given generated Markdown whose dropped constructs hold only the word "gone"
            /// When prose is extracted
            /// Then no span holds "gone", and every word placed in prose is returned
            #[test]
            fn returns_exactly_the_prose_words(doc in doc()) {
                let prose: Vec<&str> = extract_prose(&doc.markdown).iter().flat_map(|s| s.text().unicode_words()).collect();
                prop_assert!(!prose.contains(&"gone"), "{:?} → {:?}", doc.markdown, prose);
                let mut expected: Vec<&str> = doc.kept.iter().map(String::as_str).collect();
                let mut actual = prose;
                expected.sort_unstable();
                actual.sort_unstable();
                prop_assert_eq!(expected, actual, "{:?}", doc.markdown);
            }
        }
    }
}
