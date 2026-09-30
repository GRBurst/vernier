//! The per-file summary `vernier analyze` prints: prose spans and words (M1), then a table of the
//! file's metrics (M5): surface counts and scores (M2), mean dependency distance (M3), the
//! nominalization ratio and passive voice (M4).

use std::path::Path;

use crate::analysis::{Absence, FileAnalysis, file_nominalizations, surface_counts};
use crate::block::Block;
use crate::nominalization::NominalizationCount;
use crate::prose::{SourceFormat, blocks, prose};
use crate::readability::{Readability, SurfaceCounts, readability};
use crate::words::count_words;

/// How much prose one file holds, and how readable it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FileSummary {
    pub spans: usize,
    pub words: usize,
    pub counts: SurfaceCounts,
    pub readability: Option<Readability>,
    pub nominalizations: NominalizationCount,
    /// The file's mean dependency distance, or why it is absent.
    pub mean_dependency_distance: Result<f64, Absence>,
    /// The file's passive constructions; `None` when no sentence was parsed.
    pub passives: Option<usize>,
}

/// The rows of `analyze`'s table, in order.
pub const METRICS: [&str; 10] = [
    "sentences",
    "syllables",
    "complex words",
    "Flesch Reading Ease",
    "Flesch-Kincaid Grade",
    "Gunning Fog",
    "average sentence length",
    "mean dependency distance",
    "nominalization ratio",
    "passive voice",
];

/// Counts the prose spans of `source` and the words of its block prose (so inline markup inside a
/// word splits no word), and scores its sentences.
pub fn summarize(source: &str, format: SourceFormat) -> FileSummary {
    let counts = surface_counts(source, format);
    FileSummary {
        spans: prose(source, format).len(),
        words: blocks(source, format)
            .iter()
            .map(|spans| count_words(Block::from_spans(spans).text()))
            .sum(),
        counts,
        readability: readability(counts),
        nominalizations: file_nominalizations(source, format),
        mean_dependency_distance: Err(Absence::NoParse),
        passives: None,
    }
}

/// `summary` with the metrics of one parse of the file: the mean dependency distance, the
/// passives and the nominalizations (`NOUN` tokens) of `file`, analyzed with a parser. The
/// sentences are parsed once, for `file`; this adds no parse.
pub fn with_parse(summary: FileSummary, file: &FileAnalysis) -> FileSummary {
    FileSummary {
        mean_dependency_distance: file.mean_dependency_distance(),
        passives: file.passives,
        nominalizations: file.nominalizations,
        ..summary
    }
}

/// The summary of one file: M1's line (`notes.md: 3 prose spans, 42 words`), then a table with a
/// `metric`/`value` header and one row per metric, in the order of `METRICS`, where an absent
/// metric reads `absent (…)`.
pub fn render(path: &Path, summary: &FileSummary) -> String {
    let head = format!(
        "{}: {} prose spans, {} words",
        path.display(),
        summary.spans,
        summary.words
    );
    let header = table_row("metric", "value");
    let rows = METRICS
        .iter()
        .zip(values(summary))
        .map(|(name, value)| table_row(name, &value));
    std::iter::once(head)
        .chain(std::iter::once(header))
        .chain(rows)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The width of the table's name column: the longest metric name.
const NAME_WIDTH: usize = "mean dependency distance".len();

fn table_row(name: &str, value: &str) -> String {
    format!("  {name:<NAME_WIDTH$}  {value}")
}

/// Each metric's value, in the order of `METRICS`.
fn values(summary: &FileSummary) -> [String; METRICS.len()] {
    let c = summary.counts;
    let score = |pick: fn(&Readability) -> f64| {
        summary.readability.as_ref().map_or_else(
            || "absent (no sentences)".to_owned(),
            |r| format!("{:.2}", pick(r)),
        )
    };
    [
        c.sentences.to_string(),
        c.syllables.to_string(),
        c.complex_words.to_string(),
        score(|r| r.flesch_reading_ease),
        score(|r| r.flesch_kincaid_grade),
        score(|r| r.gunning_fog),
        score(|r| r.average_sentence_length),
        present_or_absent(
            summary
                .mean_dependency_distance
                .map(|mdd| format!("{mdd:.2}")),
        ),
        render_nominalizations(summary.nominalizations),
        present_or_absent(
            summary
                .passives
                .map(|n| n.to_string())
                .ok_or(Absence::NoParse),
        ),
    ]
}

/// A metric's rendered value, or `absent (<reason>)`.
fn present_or_absent(value: Result<String, Absence>) -> String {
    value.unwrap_or_else(|absence| format!("absent ({absence})"))
}

fn render_nominalizations(n: NominalizationCount) -> String {
    n.ratio().map_or_else(
        || "absent (no words)".to_owned(),
        |ratio| format!("{ratio:.3} ({} of {} words)", n.nominalizations, n.words),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn nominal_paragraphs() -> impl Strategy<Value = String> {
        let word = prop::sample::select(vec![
            "the",
            "**decisions**",
            "implementation's",
            "cities",
            "rations",
            "we",
            "*Movement*.",
        ]);
        let para = prop::collection::vec(word, 1..10).prop_map(|w| format!("{}.", w.join(" ")));
        prop::collection::vec(para, 0..4).prop_map(|p| p.join("\n\n"))
    }

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

        /// Given Markdown paragraphs with nominalizations, markup and possessives
        /// When the file is summarized
        /// Then its nominalization count is the file's count from `analyze`, over the file's words
        #[test]
        fn nominalizations_are_the_analyzed_files(md in nominal_paragraphs()) {
            let summary = summarize(&md, SourceFormat::Markdown);
            let thresholds = crate::analysis::Thresholds { max_sentence_len: 25, max_mdd: 3.0, max_tree_depth: 5, max_clauses: 2 };
            let file = crate::analysis::analyze(&md, SourceFormat::Markdown, &thresholds);
            prop_assert_eq!(summary.nominalizations, file.nominalizations);
            prop_assert_eq!(summary.nominalizations.words, summary.counts.words);
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

    /// A paragraph of words, each drawn as two halves and wrapped in emphasis, strong or a link
    /// as a whole or around one half (intraword markup), optionally followed by a comma or full
    /// stop; paired with the same paragraph without markup.
    fn marked_and_stripped() -> impl Strategy<Value = (String, String)> {
        let shape = prop::sample::select(vec![
            "{a}{b}",
            "*{a}{b}*",
            "**{a}{b}**",
            "[{a}{b}](u)",
            "{a}*{b}*",
            "*{a}*{b}",
            "{a}**{b}**",
        ]);
        let word = (
            "[a-z]{1,6}",
            "[a-z]{1,6}",
            shape,
            prop::sample::select(vec!["", ",", "."]),
        )
            .prop_map(|(a, b, shape, p)| {
                let marked = shape.replace("{a}", &a).replace("{b}", &b);
                (format!("{marked}{p}"), format!("{a}{b}{p}"))
            });
        let para = prop::collection::vec(word, 1..10).prop_map(|words| {
            let (marked, stripped): (Vec<String>, Vec<String>) = words.into_iter().unzip();
            (marked.join(" "), stripped.join(" "))
        });
        prop::collection::vec(para, 1..4).prop_map(|paras| {
            let (marked, stripped): (Vec<String>, Vec<String>) = paras.into_iter().unzip();
            (marked.join("\n\n"), stripped.join("\n\n"))
        })
    }

    proptest! {
        /// Given paragraphs with inline markup around and inside words, and the same paragraphs
        /// stripped of it
        /// When both files are summarized
        /// Then they have the same word count, and it equals the sentences' summed words
        #[test]
        fn inline_markup_changes_no_word_count((marked, stripped) in marked_and_stripped()) {
            let (m, s) = (summarize(&marked, SourceFormat::Markdown), summarize(&stripped, SourceFormat::Markdown));
            prop_assert_eq!(m.words, s.words, "{:?}", marked);
            prop_assert_eq!(m.words, m.counts.words, "{:?}", marked);
        }
    }

    /// Given two paragraphs with bold, italic, link and intraword (`un*believ*able`) markup, and
    /// the same paragraphs without it
    /// When both are summarized
    /// Then both count 19 words
    #[test]
    fn markup_inside_a_word_keeps_one_word() {
        let marked = "The **proposal**, which the *executive* committee rejected after \
                      [extensive](https://x.y) deliberation, caused un*believ*able delays.\n\n\
                      A second **paragraph** with _emphasis_ here.\n";
        let stripped = "The proposal, which the executive committee rejected after extensive \
                        deliberation, caused unbelievable delays.\n\nA second paragraph with \
                        emphasis here.\n";
        let words = |md: &str| summarize(md, SourceFormat::Markdown).words;
        assert_eq!((words(marked), words(stripped)), (19, 19));
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
            nominalizations: NominalizationCount::default(),
            mean_dependency_distance: Err(Absence::NoParse),
            passives: None,
        };
        assert_eq!(
            render(Path::new("notes.md"), &summary).lines().next(),
            Some("notes.md: 3 prose spans, 42 words")
        );
    }

    /// The value in the table row of `name`, if the row is there.
    fn row<'a>(rendered: &'a str, name: &str) -> Option<&'a str> {
        rendered.lines().skip(1).find_map(|line| {
            let rest = line.trim_start().strip_prefix(name)?;
            rest.starts_with("  ").then(|| rest.trim())
        })
    }

    /// Given a summary of 100 words, 4 sentences, 150 syllables and 10 complex words
    /// When it is rendered
    /// Then a table follows M1's line: a `metric`/`value` header, then one row per metric with
    /// the counts, the four scores to 2 decimals, and the absent syntactic metrics
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
            nominalizations: NominalizationCount {
                nominalizations: 5,
                words: 100,
            },
            mean_dependency_distance: Err(Absence::NoParse),
            passives: None,
        };
        assert_eq!(
            render(Path::new("notes.md"), &summary),
            "notes.md: 4 prose spans, 100 words\n\
             \x20 metric                    value\n\
             \x20 sentences                 4\n\
             \x20 syllables                 150\n\
             \x20 complex words             10\n\
             \x20 Flesch Reading Ease       54.56\n\
             \x20 Flesch-Kincaid Grade      11.86\n\
             \x20 Gunning Fog               14.00\n\
             \x20 average sentence length   25.00\n\
             \x20 mean dependency distance  absent (no parse)\n\
             \x20 nominalization ratio      0.050 (5 of 100 words)\n\
             \x20 passive voice             absent (no parse)"
        );
    }

    /// Given a file without sentences
    /// When its summary is rendered
    /// Then the counts are 0 and each score row says the scores are absent
    #[test]
    fn renders_absent_metrics() {
        let rendered = render(
            Path::new("h.md"),
            &summarize("# Only a heading\n", SourceFormat::Markdown),
        );
        assert_eq!(
            rendered.lines().next(),
            Some("h.md: 0 prose spans, 0 words")
        );
        for name in ["sentences", "syllables", "complex words"] {
            assert_eq!(row(&rendered, name), Some("0"), "{rendered}");
        }
        for name in [
            "Flesch Reading Ease",
            "Flesch-Kincaid Grade",
            "Gunning Fog",
            "average sentence length",
        ] {
            assert_eq!(
                row(&rendered, name),
                Some("absent (no sentences)"),
                "{rendered}"
            );
        }
        assert_eq!(
            row(&rendered, "nominalization ratio"),
            Some("absent (no words)"),
            "{rendered}"
        );
        assert_eq!(
            row(&rendered, "passive voice"),
            Some("absent (no parse)"),
            "{rendered}"
        );
    }

    /// Given an empty file and a file of only a heading
    /// When it is summarized and rendered
    /// Then it has no nominalization ratio, and its row says so
    #[test]
    fn empty_file_has_no_nominalization_ratio() {
        for source in ["", "# Only a heading\n"] {
            let summary = summarize(source, SourceFormat::Markdown);
            assert_eq!(summary.nominalizations.ratio(), None, "{source:?}");
            let rendered = render(Path::new("e.md"), &summary);
            assert_eq!(
                row(&rendered, "nominalization ratio"),
                Some("absent (no words)"),
                "{source:?}"
            );
        }
    }

    /// Parses the NOMZ sentence into UDPipe's tokens and every other sentence into a chain of
    /// its words (each headed by the next), tagged `NOUN` with lemma `_`.
    struct NounParser;

    impl crate::dependency::Parser for NounParser {
        type Error = std::convert::Infallible;

        fn parse(&mut self, sentence: &str) -> Result<crate::dependency::Parse, Self::Error> {
            if sentence == crate::testing::NOMZ_TEXT {
                let tokens = crate::testing::tokens_from_conllu(crate::testing::NOMZ_CONLLU);
                return Ok(crate::dependency::Parse::Tokens(tokens));
            }
            let forms: Vec<&str> = sentence.split_whitespace().collect();
            let tokens = (1..=forms.len())
                .map(|id| crate::dependency::Token {
                    id,
                    form: forms[id - 1].to_owned(),
                    lemma: "_".to_owned(),
                    upostag: "NOUN".to_owned(),
                    head: if id == forms.len() { 0 } else { id + 1 },
                    deprel: if id == forms.len() { "root" } else { "dep" }.to_owned(),
                })
                .collect();
            Ok(crate::dependency::Parse::Tokens(tokens))
        }
    }

    const THRESHOLDS: crate::analysis::Thresholds = crate::analysis::Thresholds {
        max_sentence_len: 25,
        max_mdd: 3.0,
        max_tree_depth: 5,
        max_clauses: 2,
    };

    // why: a test helper; clippy's allow-unwrap-in-tests covers only `#[test]` items (audit 003).
    #[allow(clippy::unwrap_used)]
    fn parsed(source: &str) -> FileAnalysis {
        crate::analysis::analyze_parsed(
            source,
            SourceFormat::Markdown,
            &THRESHOLDS,
            &mut NounParser,
        )
        .unwrap()
    }

    proptest! {
        /// Given paragraphs with nominalizations, markup and possessives, and their analysis
        /// with a parser
        /// When the summary takes the parse
        /// Then it keeps every surface field and takes the parse's mean dependency distance,
        /// passives and nominalizations (P14)
        #[test]
        fn with_parse_takes_the_parse_metrics(md in nominal_paragraphs()) {
            let surface = summarize(&md, SourceFormat::Markdown);
            let file = parsed(&md);
            let summary = with_parse(surface, &file);
            prop_assert_eq!(summary, FileSummary {
                mean_dependency_distance: file.mean_dependency_distance(),
                passives: file.passives,
                nominalizations: file.nominalizations,
                ..surface
            });
        }
    }

    /// Given the NOMZ sentence, whose `NOUN` tokens hold 3 nominalizations and whose surface
    /// words hold 4 (`commission` is a verb there)
    /// When its summary takes the parse
    /// Then the summary counts 3, and the parse's distance and passives are present
    #[test]
    fn with_parse_counts_the_parsed_nominalizations() {
        let surface = summarize(crate::testing::NOMZ_TEXT, SourceFormat::Markdown);
        let summary = with_parse(surface, &parsed(crate::testing::NOMZ_TEXT));
        assert_eq!(surface.nominalizations.nominalizations, 4);
        assert_eq!(summary.nominalizations.nominalizations, 3);
        assert!(summary.mean_dependency_distance.is_ok());
        assert_eq!(summary.passives, Some(0));
    }

    /// Given the PASSIVE sentence and a parser that finds every sentence too long for the model
    /// When its summary takes that parse and is rendered
    /// Then no sentence was parsed, so passive voice is absent, not zero, and its row says
    /// `absent (no parse)` (M4)
    #[test]
    fn passive_voice_is_absent_when_every_sentence_is_too_long() {
        let source = crate::testing::PASSIVE_TEXT;
        let file = crate::analysis::analyze_parsed(
            source,
            SourceFormat::Markdown,
            &THRESHOLDS,
            &mut crate::testing::EveryTooLong,
        )
        .unwrap();
        let summary = with_parse(summarize(source, SourceFormat::Markdown), &file);
        assert_eq!(summary.passives, None);
        let rendered = render(Path::new("p.md"), &summary);
        assert_eq!(
            row(&rendered, "passive voice"),
            Some("absent (no parse)"),
            "{rendered}"
        );
    }

    /// Given `Go.` and `Stop!`, and `Yes no.` (two content tokens), each parsed without a
    /// content dependency
    /// When the summary takes that parse and is rendered
    /// Then the file was parsed, so its mean dependency distance reads `absent (no content
    /// dependency)`, not `absent (no parse)` (File metrics, M3a)
    #[test]
    fn a_parsed_file_without_content_dependency_names_that_reason() {
        for source in ["Go.\n\nStop!\n", "Yes no.\n"] {
            let file = crate::analysis::analyze_parsed(
                source,
                SourceFormat::Markdown,
                &THRESHOLDS,
                &mut crate::testing::ProjectedRoots,
            )
            .unwrap();
            let summary = with_parse(summarize(source, SourceFormat::Markdown), &file);
            let rendered = render(Path::new("go.md"), &summary);
            assert_eq!(
                row(&rendered, "mean dependency distance"),
                Some("absent (no content dependency)"),
                "{rendered}"
            );
        }
    }

    /// Given the NOMZ sentence
    /// When it is summarized and rendered
    /// Then the nominalization row gives the ratio 4/14 to 3 decimals and its counts
    #[test]
    fn renders_the_nominalization_ratio() {
        let summary = summarize(crate::testing::NOMZ_TEXT, SourceFormat::Markdown);
        let rendered = render(Path::new("n.md"), &summary);
        assert_eq!(
            row(&rendered, "nominalization ratio"),
            Some("0.286 (4 of 14 words)")
        );
    }

    proptest! {
        /// Given generated paragraphs, with nominalizations and markup
        /// When their summary is rendered
        /// Then M1's line is followed by the header and exactly one row per metric, in order,
        /// each holding the summary's value or its absence
        #[test]
        fn the_table_has_one_row_per_metric(md in nominal_paragraphs()) {
            let summary = summarize(&md, SourceFormat::Markdown);
            let rendered = render(Path::new("f.md"), &summary);
            let lines: Vec<&str> = rendered.lines().collect();
            prop_assert_eq!(lines.len(), 2 + METRICS.len(), "{}", rendered);
            prop_assert!(lines[1].trim_start().starts_with("metric"));
            for (line, name) in lines[2..].iter().zip(METRICS) {
                prop_assert!(line.trim_start().starts_with(name), "{} vs {}", line, name);
            }
            let c = summary.counts;
            for (name, count) in [("sentences", c.sentences), ("syllables", c.syllables), ("complex words", c.complex_words)] {
                let expected = count.to_string();
                prop_assert_eq!(row(&rendered, name), Some(expected.as_str()));
            }
            let fre = summary.readability.map_or("absent (no sentences)".to_owned(), |r| format!("{:.2}", r.flesch_reading_ease));
            prop_assert_eq!(row(&rendered, "Flesch Reading Ease"), Some(fre.as_str()));
            let fog = summary.readability.map_or("absent (no sentences)".to_owned(), |r| format!("{:.2}", r.gunning_fog));
            prop_assert_eq!(row(&rendered, "Gunning Fog"), Some(fog.as_str()));
            prop_assert_eq!(row(&rendered, "mean dependency distance"), Some("absent (no parse)"));
            prop_assert_eq!(row(&rendered, "passive voice"), Some("absent (no parse)"));
            let n = summary.nominalizations;
            let ratio = n.ratio().map_or("absent (no words)".to_owned(), |r| format!("{r:.3} ({} of {} words)", n.nominalizations, n.words));
            prop_assert_eq!(row(&rendered, "nominalization ratio"), Some(ratio.as_str()));
        }
    }
}
