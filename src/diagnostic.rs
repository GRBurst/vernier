//! Diagnostics (spec 001 M5): what a flagged sentence reports, and how it is rendered.

use std::ffi::OsStr;
use std::fmt;
use std::ops::Range;

use annotate_snippets::{AnnotationKind, Level, Renderer, Snippet};

use crate::analysis::{
    FileAnalysis, Flag, SentenceAnalysis, SentenceSyntax, SyntacticFlag, Syntax, Thresholds,
};
use crate::position::{LineIndex, Position, PositionError};

/// The code of every vernier diagnostic.
pub const CODE: &str = "CognitiveOverload";

/// The title of every vernier diagnostic.
pub const TITLE: &str = "Sentence exceeds human working-memory capacity";

/// Whether rendered text carries ANSI colour; the content is the same either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Plain,
    Color,
}

impl Style {
    /// Colour only on a terminal, and only while `NO_COLOR` is unset or empty (no-color.org).
    pub fn for_output(is_terminal: bool, no_color: Option<&OsStr>) -> Self {
        if is_terminal && no_color.is_none_or(OsStr::is_empty) {
            Self::Color
        } else {
            Self::Plain
        }
    }
}

/// What one flagged sentence reports: where it starts and ends, and the flags it raised.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub start: Position,
    pub end: Position,
    pub source_range: Range<usize>,
    /// Surface flags first, then syntactic flags.
    pub flags: Vec<FlagMessage>,
    /// One line per rule metric: its value, the flag it raised and its maximum.
    pub metrics: Vec<String>,
}

/// One diagnostic per sentence of `file` that raised at least one flag, in source order.
pub fn diagnostics(
    source: &str,
    file: &FileAnalysis,
    thresholds: &Thresholds,
) -> Result<Vec<Diagnostic>, PositionError> {
    let index = LineIndex::new(source);
    file.sentences
        .iter()
        .map(|sentence| (sentence, flag_messages(sentence, thresholds)))
        .filter(|(_, flags)| !flags.is_empty())
        .map(|(sentence, flags)| {
            let range = sentence.source_range.clone();
            Ok(Diagnostic {
                start: index.position(range.start)?,
                end: index.position(range.end)?,
                source_range: range,
                flags,
                metrics: metric_lines(sentence, thresholds),
            })
        })
        .collect()
}

/// One flag of a sentence, named, with the measured value and its maximum in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlagMessage {
    pub name: &'static str,
    pub message: String,
}

impl fmt::Display for FlagMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.message)
    }
}

/// `path:line:col: CognitiveOverload: message`, the message joining the flag messages with `; `.
pub fn render_compact(path: &str, diagnostic: &Diagnostic) -> String {
    let messages: Vec<String> = diagnostic.flags.iter().map(ToString::to_string).collect();
    format!(
        "{path}:{}:{}: {CODE}: {}",
        diagnostic.start.line(),
        diagnostic.start.column(),
        messages.join("; ")
    )
}

/// The annotated diagnostic: `warning[CognitiveOverload]`, the sentence's position, and its source
/// lines with the whole sentence underlined.
pub fn render_text(path: &str, source: &str, diagnostic: &Diagnostic, style: Style) -> String {
    let listed: Vec<String> = diagnostic
        .metrics
        .iter()
        .map(|m| format!("- {m}"))
        .collect();
    let group = Level::WARNING
        .primary_title(TITLE)
        .id(CODE)
        .element(
            Snippet::source(source)
                .path(path)
                .annotation(AnnotationKind::Primary.span(diagnostic.source_range.clone())),
        )
        .element(
            Level::NOTE
                .no_name()
                .message(format!("metrics:\n{}", listed.join("\n"))),
        );
    let renderer = match style {
        Style::Plain => Renderer::plain(),
        Style::Color => Renderer::styled(),
    };
    // why: prose lines are often whole paragraphs; the default width (140) would elide the
    // middle of the sentence the diagnostic is about.
    renderer.term_width(RENDER_WIDTH).render(&[group])
}

/// A line width no prose line reaches, so no source line is elided.
const RENDER_WIDTH: usize = 100_000;

/// The rule metrics of a sentence, one line each: words, mean dependency distance, tree depth,
/// subordinate clauses, then one line per center-embedding (or `none`); a syntactic metric of an
/// unparsed sentence is `absent (no parse)`.
pub fn metric_lines(sentence: &SentenceAnalysis, thresholds: &Thresholds) -> Vec<String> {
    let words = sentence.counts.words;
    let long = sentence.flags.contains(&Flag::LongSentence);
    let head = format!(
        "words: {words} ({}max {})",
        raised(long, "LongSentence"),
        thresholds.max_sentence_len
    );
    let syntactic = match &sentence.syntax {
        Syntax::Unparsed => ABSENT_WITHOUT_PARSE
            .map(|name| format!("{name}: absent (no parse)"))
            .to_vec(),
        Syntax::Parsed(syntax) => parsed_lines(syntax, thresholds),
    };
    std::iter::once(head).chain(syntactic).collect()
}

/// The syntactic metrics a diagnostic lists, in order.
const ABSENT_WITHOUT_PARSE: [&str; 4] = [
    "mean dependency distance",
    "tree depth",
    "subordinate clauses",
    "center-embedding",
];

/// `"Flag, "` when the flag was raised, else nothing.
fn raised(is_raised: bool, name: &str) -> String {
    if is_raised {
        format!("{name}, ")
    } else {
        String::new()
    }
}

fn parsed_lines(syntax: &SentenceSyntax, thresholds: &Thresholds) -> Vec<String> {
    let has = |wanted: fn(&SyntacticFlag) -> bool| syntax.flags.iter().any(wanted);
    let metrics = &syntax.metrics;
    let mdd = metrics.mdd().map_or_else(
        || "mean dependency distance: absent (fewer than 2 content tokens)".to_owned(),
        |mdd| {
            let high = has(|f| matches!(f, SyntacticFlag::HighMdd { .. }));
            format!(
                "mean dependency distance: {mdd:.2} ({}max {:.2})",
                raised(high, "HighMdd"),
                thresholds.max_mdd
            )
        },
    );
    let deep = has(|f| matches!(f, SyntacticFlag::DeepTree { .. }));
    let overload = has(|f| matches!(f, SyntacticFlag::ClauseOverload { .. }));
    let lines = [
        mdd,
        format!(
            "tree depth: {} edges ({}max {})",
            metrics.depth,
            raised(deep, "DeepTree"),
            thresholds.max_tree_depth
        ),
        format!(
            "subordinate clauses: {} ({}max {})",
            metrics.clauses,
            raised(overload, "ClauseOverload"),
            thresholds.max_clauses
        ),
    ];
    lines
        .into_iter()
        .chain(center_embedding_lines(syntax))
        .collect()
}

/// One line per center-embedding flag, or `none`.
fn center_embedding_lines(syntax: &SentenceSyntax) -> Vec<String> {
    let embedded: Vec<String> = syntax
        .flags
        .iter()
        .filter_map(|flag| match flag {
            SyntacticFlag::CenterEmbedding(e) => Some(format!(
                "center-embedding: subject \"{}\" separated from verb \"{}\" by {} words (CenterEmbedding)",
                e.subject, e.verb, e.words_between
            )),
            SyntacticFlag::HighMdd { .. }
            | SyntacticFlag::DeepTree { .. }
            | SyntacticFlag::ClauseOverload { .. } => None,
        })
        .collect();
    if embedded.is_empty() {
        vec!["center-embedding: none".to_owned()]
    } else {
        embedded
    }
}

/// The messages of a sentence's flags: its surface flags, then its syntactic flags.
pub fn flag_messages(sentence: &SentenceAnalysis, thresholds: &Thresholds) -> Vec<FlagMessage> {
    let surface = sentence
        .flags
        .iter()
        .map(|flag| surface_message(*flag, sentence, thresholds));
    let syntactic = sentence
        .syntax
        .parsed()
        .into_iter()
        .flat_map(|syntax| syntax.flags.iter())
        .map(|flag| syntactic_message(flag, thresholds));
    surface.chain(syntactic).collect()
}

fn surface_message(
    flag: Flag,
    sentence: &SentenceAnalysis,
    thresholds: &Thresholds,
) -> FlagMessage {
    match flag {
        Flag::LongSentence => FlagMessage {
            name: "LongSentence",
            message: format!(
                "sentence has {} words (max {})",
                sentence.counts.words, thresholds.max_sentence_len
            ),
        },
    }
}

fn syntactic_message(flag: &SyntacticFlag, thresholds: &Thresholds) -> FlagMessage {
    match flag {
        SyntacticFlag::HighMdd { mdd } => FlagMessage {
            name: "HighMdd",
            message: format!(
                "mean dependency distance {mdd:.2} (max {:.2})",
                thresholds.max_mdd
            ),
        },
        SyntacticFlag::DeepTree { depth } => FlagMessage {
            name: "DeepTree",
            message: format!(
                "dependency tree depth {depth} edges (max {})",
                thresholds.max_tree_depth
            ),
        },
        SyntacticFlag::ClauseOverload { clauses } => FlagMessage {
            name: "ClauseOverload",
            message: format!(
                "{clauses} subordinate clauses (max {})",
                thresholds.max_clauses
            ),
        },
        SyntacticFlag::CenterEmbedding(e) => FlagMessage {
            name: "CenterEmbedding",
            message: format!(
                "subject \"{}\" separated from verb \"{}\" by {} words",
                e.subject, e.verb, e.words_between
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{analyze, syntactic_flags};
    use crate::dependency::DependencyTree;
    use crate::nominalization::NominalizationCount;
    use crate::prose::SourceFormat;
    use crate::readability::SurfaceCounts;
    use crate::syntax::syntactic_metrics;
    use crate::syntax::{CenterEmbedding, DependencyDistance, SyntacticMetrics};
    use crate::testing::{EXAMPLE_CONLLU, tokens_from_conllu};
    use proptest::prelude::*;

    const THRESHOLDS: Thresholds = Thresholds {
        max_sentence_len: 25,
        max_mdd: 3.0,
        max_tree_depth: 5,
        max_clauses: 2,
    };

    /// Given each syntactic flag with its measured value
    /// When it is described
    /// Then the message names the flag, the value and, for a threshold rule, the maximum
    #[test]
    fn describes_each_syntactic_flag_with_its_value() {
        let embedding = CenterEmbedding {
            subject: "proposal".to_owned(),
            verb: "caused".to_owned(),
            words_between: 8,
        };
        let cases = [
            (
                SyntacticFlag::HighMdd { mdd: 32.0 / 12.0 },
                "HighMdd: mean dependency distance 2.67 (max 3.00)",
            ),
            (
                SyntacticFlag::DeepTree { depth: 6 },
                "DeepTree: dependency tree depth 6 edges (max 5)",
            ),
            (
                SyntacticFlag::ClauseOverload { clauses: 3 },
                "ClauseOverload: 3 subordinate clauses (max 2)",
            ),
            (
                SyntacticFlag::CenterEmbedding(embedding),
                "CenterEmbedding: subject \"proposal\" separated from verb \"caused\" by 8 words",
            ),
        ];
        for (flag, expected) in cases {
            assert_eq!(syntactic_message(&flag, &THRESHOLDS).to_string(), expected);
        }
    }

    fn sentence(
        source_range: Range<usize>,
        words: usize,
        flags: Vec<Flag>,
        syntactic: Vec<SyntacticFlag>,
    ) -> SentenceAnalysis {
        let metrics = SyntacticMetrics {
            distance: DependencyDistance::default(),
            depth: 0,
            clauses: 0,
            center_embeddings: Vec::new(),
        };
        SentenceAnalysis {
            source_range,
            counts: SurfaceCounts {
                words,
                sentences: 1,
                ..Default::default()
            },
            readability: None,
            flags,
            syntax: Syntax::Parsed(SentenceSyntax {
                metrics,
                flags: syntactic,
                passives: Vec::new(),
            }),
            nominalizations: NominalizationCount::default(),
        }
    }

    fn file_of(sentences: Vec<SentenceAnalysis>) -> FileAnalysis {
        FileAnalysis {
            spans: 1,
            sentences,
            totals: SurfaceCounts::default(),
            readability: None,
            dependency_distance: None,
            nominalizations: NominalizationCount::default(),
            passives: None,
        }
    }

    /// Given three sentences: the first flagged LongSentence and DeepTree, the second unflagged,
    /// the third flagged HighMdd only
    /// When the diagnostics are built
    /// Then there are two, one per flagged sentence: the first with both flags (surface first),
    /// the second at line 3 with its syntactic flag
    #[test]
    fn one_diagnostic_per_flagged_sentence_with_all_its_flags() {
        let source = "First sentence.\nFine one.\nThird one.\n";
        let file = file_of(vec![
            sentence(
                0..15,
                30,
                vec![Flag::LongSentence],
                vec![SyntacticFlag::DeepTree { depth: 6 }],
            ),
            sentence(16..25, 2, vec![], vec![]),
            sentence(26..36, 2, vec![], vec![SyntacticFlag::HighMdd { mdd: 3.5 }]),
        ]);
        let found = diagnostics(source, &file, &THRESHOLDS).unwrap();
        let names: Vec<Vec<&str>> = found
            .iter()
            .map(|d| d.flags.iter().map(|f| f.name).collect())
            .collect();
        assert_eq!(names, [vec!["LongSentence", "DeepTree"], vec!["HighMdd"]]);
        assert_eq!(found[0].source_range, 0..15);
        assert_eq!((found[1].start.line(), found[1].start.column()), (3, 1));
        assert_eq!((found[1].end.line(), found[1].end.column()), (3, 11));
    }

    /// Given the sentence flagged LongSentence and DeepTree
    /// When its diagnostic is rendered compact
    /// Then one line carries its position, the code and both flag messages joined by `; `
    #[test]
    fn compact_joins_the_flags_of_one_sentence() {
        let file = file_of(vec![sentence(
            0..15,
            30,
            vec![Flag::LongSentence],
            vec![SyntacticFlag::DeepTree { depth: 6 }],
        )]);
        let found = diagnostics("First sentence.\n", &file, &THRESHOLDS).unwrap();
        assert_eq!(
            render_compact("f.md", &found[0]),
            "f.md:1:1: CognitiveOverload: LongSentence: sentence has 30 words (max 25); \
             DeepTree: dependency tree depth 6 edges (max 5)"
        );
    }

    /// `text` without its ANSI escape sequences (`ESC [ … m`).
    fn strip_ansi(text: &str) -> String {
        let mut plain = String::new();
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                chars.by_ref().find(|&d| d == 'm');
            } else {
                plain.push(c);
            }
        }
        plain
    }

    /// Given every combination of terminal or not and `NO_COLOR` unset, empty or set
    /// When the output style is chosen
    /// Then colour is chosen only on a terminal with `NO_COLOR` unset or empty
    #[test]
    fn colour_only_on_a_terminal_without_no_color() {
        let cases = [
            (true, None, Style::Color),
            (true, Some(""), Style::Color),
            (true, Some("1"), Style::Plain),
            (true, Some("0"), Style::Plain),
            (false, None, Style::Plain),
            (false, Some(""), Style::Plain),
            (false, Some("1"), Style::Plain),
        ];
        for (is_terminal, no_color, expected) in cases {
            let style = Style::for_output(is_terminal, no_color.map(OsStr::new));
            assert_eq!(style, expected, "{is_terminal} {no_color:?}");
        }
    }

    /// Given a 30-word sentence on a line of about 190 characters (the `long.md` fixture)
    /// When its diagnostic is rendered as text
    /// Then the whole line is shown, not elided, under the header and position
    #[test]
    fn a_long_line_is_shown_whole() {
        let source = include_str!("../tests/fixtures/long.md");
        let t = Thresholds {
            max_sentence_len: 25,
            ..THRESHOLDS
        };
        let file = analyze(source, SourceFormat::Markdown, &t);
        let found = diagnostics(source, &file, &t).unwrap();
        let text = render_text("long.md", source, &found[0], Style::Plain);
        let line = source.lines().nth(2).unwrap();
        assert!(line.len() > 140, "{}", line.len());
        assert!(text.lines().any(|l| l.ends_with(line)), "{text}");
        assert!(!text.contains("..."), "{text}");
        assert_eq!(text.lines().nth(1).map(str::trim), Some("--> long.md:3:20"));
    }

    /// A sentence of `words` words with the flags `analyze` would give it under `t`, parsed into
    /// `metrics` or not parsed.
    fn measured(
        words: usize,
        metrics: Option<SyntacticMetrics>,
        t: &Thresholds,
    ) -> SentenceAnalysis {
        SentenceAnalysis {
            source_range: 0..1,
            counts: SurfaceCounts {
                words,
                sentences: 1,
                ..Default::default()
            },
            readability: None,
            flags: (words > t.max_sentence_len)
                .then_some(Flag::LongSentence)
                .into_iter()
                .collect(),
            syntax: metrics.map_or(Syntax::Unparsed, |metrics| {
                Syntax::Parsed(SentenceSyntax {
                    flags: syntactic_flags(&metrics, t),
                    metrics,
                    passives: Vec::new(),
                })
            }),
            nominalizations: NominalizationCount::default(),
        }
    }

    /// Given the UDPipe 2 parse of the M3a example sentence (13 words, MDD 32/12, depth 4,
    /// 1 clause, one center-embedding) and `max_mdd` 2.5
    /// When its metric lines are listed
    /// Then each rule metric shows its value, the flag it raised and its maximum
    #[test]
    fn lists_the_rule_metrics_of_the_parsed_example() {
        let t = Thresholds {
            max_mdd: 2.5,
            ..THRESHOLDS
        };
        let tree = DependencyTree::new(tokens_from_conllu(EXAMPLE_CONLLU)).unwrap();
        let sentence = measured(13, Some(syntactic_metrics(&tree)), &t);
        assert_eq!(
            metric_lines(&sentence, &t),
            [
                "words: 13 (max 25)",
                "mean dependency distance: 2.67 (HighMdd, max 2.50)",
                "tree depth: 4 edges (max 5)",
                "subordinate clauses: 1 (max 2)",
                "center-embedding: subject \"proposal\" separated from verb \"caused\" by 8 words (CenterEmbedding)",
            ]
        );
    }

    /// Given an unparsed 30-word sentence
    /// When its diagnostic is rendered as text
    /// Then a `= metrics:` footer lists the words with LongSentence and each syntactic metric as
    /// absent (no parse)
    #[test]
    fn the_text_footer_lists_the_metrics() {
        let source = include_str!("../tests/fixtures/long.md");
        let file = analyze(source, SourceFormat::Markdown, &THRESHOLDS);
        let found = diagnostics(source, &file, &THRESHOLDS).unwrap();
        let text = render_text("long.md", source, &found[0], Style::Plain);
        let footer: Vec<&str> = text
            .lines()
            .skip_while(|l| !l.contains("= metrics:"))
            .map(str::trim)
            .collect();
        assert_eq!(
            footer,
            [
                "= metrics:",
                "- words: 30 (LongSentence, max 25)",
                "- mean dependency distance: absent (no parse)",
                "- tree depth: absent (no parse)",
                "- subordinate clauses: absent (no parse)",
                "- center-embedding: absent (no parse)",
            ],
            "{text}"
        );
    }

    fn metrics_or_none() -> impl Strategy<Value = Option<SyntacticMetrics>> {
        let embedding = (0usize..12).prop_map(|words_between| CenterEmbedding {
            subject: "proposal".to_owned(),
            verb: "caused".to_owned(),
            words_between,
        });
        let metrics = (
            0usize..20,
            1usize..4,
            0usize..9,
            0usize..5,
            prop::collection::vec(embedding, 0..3),
        )
            .prop_map(|(dependencies, per, depth, clauses, center_embeddings)| {
                SyntacticMetrics {
                    distance: DependencyDistance {
                        total: dependencies * per,
                        dependencies,
                    },
                    depth,
                    clauses,
                    center_embeddings,
                }
            });
        prop::option::weighted(0.7, metrics)
    }

    /// Paragraphs of sentences of 1–12 words, some multi-byte, some with inline markup, LF or CRLF.
    fn document() -> impl Strategy<Value = String> {
        let word = prop::sample::select(vec![
            "word", "Café", "漢字", "**bold**", "*it*", "x2", "naïve",
        ]);
        let sentence = prop::collection::vec(word, 1..12).prop_map(|w| format!("{}.", w.join(" ")));
        let para = prop::collection::vec(sentence, 1..4).prop_map(|s| s.join(" "));
        (prop::collection::vec(para, 0..4), any::<bool>()).prop_map(|(p, crlf)| {
            let lf = format!("{}\n", p.join("\n\n"));
            if crlf { lf.replace('\n', "\r\n") } else { lf }
        })
    }

    proptest! {
        /// Given any sentence, parsed or not, and thresholds that equal its metrics half the time
        /// When its metric lines are listed
        /// Then the words line names LongSentence exactly when the words exceed the maximum; an
        /// unparsed sentence has its four syntactic lines absent; a parsed one names HighMdd,
        /// DeepTree and ClauseOverload exactly when it raised them, and lists one center-embedding
        /// line per CenterEmbedding flag
        #[test]
        fn metric_lines_name_exactly_the_raised_flags(
            words in 1usize..40,
            metrics in metrics_or_none(),
            same in any::<[bool; 4]>(),
            other in (0usize..40, 0.5f64..4.0, 0usize..9, 0usize..5),
        ) {
            let mdd = metrics.as_ref().and_then(SyntacticMetrics::mdd);
            let t = Thresholds {
                max_sentence_len: if same[0] { words } else { other.0 },
                max_mdd: match (same[1], mdd) { (true, Some(m)) => m, _ => other.1 },
                max_tree_depth: match (same[2], &metrics) { (true, Some(m)) => m.depth, _ => other.2 },
                max_clauses: match (same[3], &metrics) { (true, Some(m)) => m.clauses, _ => other.3 },
            };
            let sentence = measured(words, metrics, &t);
            let lines = metric_lines(&sentence, &t);
            prop_assert_eq!(lines[0].contains("LongSentence"), words > t.max_sentence_len, "{}", lines[0]);
            match &sentence.syntax {
                Syntax::Unparsed => {
                    prop_assert_eq!(lines.len(), 5);
                    prop_assert!(lines[1..].iter().all(|l| l.ends_with("absent (no parse)")), "{:?}", lines);
                }
                Syntax::Parsed(syntax) => {
                    let raised = |name: &str| syntax.flags.iter().any(|f| syntactic_message(f, &t).name == name);
                    prop_assert_eq!(lines[1].contains("HighMdd"), raised("HighMdd"), "{}", lines[1]);
                    prop_assert_eq!(lines[2].contains("DeepTree"), raised("DeepTree"), "{}", lines[2]);
                    prop_assert_eq!(lines[3].contains("ClauseOverload"), raised("ClauseOverload"), "{}", lines[3]);
                    let embedded = syntax.flags.iter().filter(|f| matches!(f, SyntacticFlag::CenterEmbedding(_))).count();
                    let listed = lines[4..].iter().filter(|l| l.contains("(CenterEmbedding)")).count();
                    prop_assert_eq!(listed, embedded, "{:?}", lines);
                }
            }
        }

        /// Given generated documents with every sentence flagged
        /// When each diagnostic is rendered as text, plain and in colour
        /// Then it opens with `warning[CognitiveOverload]` and the title, then ` --> path:line:col`
        /// at the diagnostic's start; plain has no escape codes, and colour differs from plain only
        /// by them
        #[test]
        fn text_opens_with_the_code_and_the_position(doc in document()) {
            let t = Thresholds { max_sentence_len: 0, ..THRESHOLDS };
            let file = analyze(&doc, SourceFormat::Markdown, &t);
            for d in diagnostics(&doc, &file, &t).unwrap() {
                let plain = render_text("f.md", &doc, &d, Style::Plain);
                let lines: Vec<&str> = plain.lines().collect();
                prop_assert_eq!(lines[0], format!("warning[{CODE}]: {TITLE}"));
                prop_assert_eq!(lines[1].trim(), format!("--> f.md:{}:{}", d.start.line(), d.start.column()));
                prop_assert!(!plain.contains('\x1b'));
                let coloured = render_text("f.md", &doc, &d, Style::Color);
                prop_assert!(coloured.contains('\x1b'));
                prop_assert_eq!(strip_ansi(&coloured), plain);
            }
        }

        /// Given one-line paragraphs of capitalized ASCII sentences, every sentence flagged
        /// When each diagnostic is rendered as text
        /// Then the run of `^` starts under the sentence's first character and is as long as the
        /// sentence
        #[test]
        fn the_underline_covers_a_one_line_sentence(
            sentences in prop::collection::vec(("[A-Z][a-z]{0,5}q", prop::collection::vec("[a-z]{0,6}q", 0..7)), 1..5)
        ) {
            // Capitalized starts: UAX #29 does not break before a lower-case word; the final `q`
            // keeps every word off the abbreviation list (`Dr.`, `vs.` would merge two sentences).
            let line = sentences
                .iter()
                .map(|(first, rest)| format!("{}.", std::iter::once(first).chain(rest).cloned().collect::<Vec<_>>().join(" ")))
                .collect::<Vec<_>>()
                .join(" ");
            let doc = format!("# Title\n\n{line}\n");
            let t = Thresholds { max_sentence_len: 0, ..THRESHOLDS };
            let file = analyze(&doc, SourceFormat::Markdown, &t);
            let found = diagnostics(&doc, &file, &t).unwrap();
            prop_assert_eq!(found.len(), sentences.len());
            for d in found {
                let text = render_text("f.md", &doc, &d, Style::Plain);
                let shown = text.lines().find(|l| l.ends_with(line.as_str())).expect("the source line");
                let text_at = shown.len() - line.len();
                let marker = text.lines().find(|l| l.contains('^')).expect("a marker line");
                let first = marker.find('^').expect("a caret");
                let run = marker[first..].chars().take_while(|&c| c == '^').count();
                prop_assert_eq!(first, text_at + d.start.column() - 1, "{}", text);
                prop_assert_eq!(run, d.source_range.len(), "{}", text);
            }
        }

        /// Given generated documents with a limit of 0 words, so every sentence is flagged
        /// When each diagnostic is rendered compact
        /// Then the line splits into the path, the diagnostic's start line and column, the code,
        /// and the diagnostic's flag messages
        #[test]
        fn compact_lines_carry_position_code_and_flags(doc in document()) {
            let t = Thresholds { max_sentence_len: 0, ..THRESHOLDS };
            let file = analyze(&doc, SourceFormat::Markdown, &t);
            for d in diagnostics(&doc, &file, &t).unwrap() {
                let line = render_compact("dir/f.md", &d);
                let rest = line.strip_prefix("dir/f.md:").expect("the path first");
                let mut parts = rest.splitn(4, ": ");
                let position = parts.next().unwrap_or_default();
                prop_assert_eq!(position, format!("{}:{}", d.start.line(), d.start.column()));
                prop_assert_eq!(parts.next(), Some(CODE));
                let message = parts.collect::<Vec<_>>().join(": ");
                let expected: Vec<String> = d.flags.iter().map(ToString::to_string).collect();
                prop_assert_eq!(message.split("; ").map(str::to_owned).collect::<Vec<_>>(), expected);
            }
        }

        /// Given generated documents and any sentence-length limit
        /// When their diagnostics are built
        /// Then there is one per flagged sentence, in source order, with that sentence's flag
        /// messages, starting and ending at the positions of the sentence's first and end offsets
        #[test]
        fn diagnostics_follow_the_flagged_sentences(doc in document(), max in 0usize..12) {
            let t = Thresholds { max_sentence_len: max, ..THRESHOLDS };
            let file = analyze(&doc, SourceFormat::Markdown, &t);
            let found = diagnostics(&doc, &file, &t).unwrap();
            let flagged: Vec<&SentenceAnalysis> = file.sentences.iter().filter(|s| !flag_messages(s, &t).is_empty()).collect();
            prop_assert_eq!(found.len(), flagged.len());
            let index = LineIndex::new(&doc);
            for (d, s) in found.iter().zip(flagged) {
                prop_assert_eq!(&d.source_range, &s.source_range);
                prop_assert_eq!(&d.flags, &flag_messages(s, &t));
                prop_assert_eq!(d.start, index.position(s.source_range.start).unwrap());
                prop_assert_eq!(d.end, index.position(s.source_range.end).unwrap());
            }
        }
    }
}
