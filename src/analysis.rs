//! Analysis (spec 001 M2, M3a): per-sentence and per-file surface counts, scores and flags, and
//! the syntactic metrics and flags of a parsed sentence.

use std::convert::Infallible;
use std::ops::Range;

use crate::block::Block;
use crate::dependency::{DependencyTree, Parser, Token, TreeError};
use crate::nominalization::{
    NominalizationCount, Stoplist, parsed_nominalizations, surface_nominalizations,
};
use crate::passive::{Passive, align, passive_heads};
use crate::prose::{SourceFormat, blocks, prose};
use crate::readability::{Readability, SurfaceCounts, readability};
use crate::sentence::{Sentence, sentences};
use crate::syllables::{count_syllables, is_complex};
use crate::syntax::{CenterEmbedding, DependencyDistance, SyntacticMetrics, syntactic_metrics};
use crate::words::words;

/// The limits a sentence is checked against (built by the shell from the command line).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thresholds {
    /// A sentence with more words than this is a `LongSentence`.
    pub max_sentence_len: usize,
    /// A sentence whose mean dependency distance exceeds this is `HighMdd`.
    pub max_mdd: f64,
    /// A sentence whose tree is deeper than this many edges is `DeepTree`.
    pub max_tree_depth: usize,
    /// A sentence with more clausal dependents than this is `ClauseOverload`.
    pub max_clauses: usize,
}

/// A rule a sentence breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    /// More words than `Thresholds::max_sentence_len`.
    LongSentence,
}

/// A syntactic rule a parsed sentence breaks, with the measured value that broke it.
#[derive(Debug, Clone, PartialEq)]
pub enum SyntacticFlag {
    /// A mean dependency distance above `Thresholds::max_mdd`.
    HighMdd { mdd: f64 },
    /// A tree deeper than `Thresholds::max_tree_depth` edges.
    DeepTree { depth: usize },
    /// More clausal dependents than `Thresholds::max_clauses`.
    ClauseOverload { clauses: usize },
    /// A clausal subtree between a subject and its verb.
    CenterEmbedding(CenterEmbedding),
}

/// A parsed sentence's syntactic metrics and the syntactic rules it breaks.
#[derive(Debug, Clone, PartialEq)]
pub struct SentenceSyntax {
    pub metrics: SyntacticMetrics,
    pub flags: Vec<SyntacticFlag>,
    /// The sentence's passive constructions, in token order.
    pub passives: Vec<Passive>,
}

/// What a sentence's parse gave: nothing, because no parser ran, or its syntax.
#[derive(Debug, Clone, PartialEq)]
pub enum Syntax {
    /// No parser ran.
    Unparsed,
    /// The parser gave a tree.
    Parsed(SentenceSyntax),
}

impl Syntax {
    /// The syntax of a parsed sentence, `None` otherwise.
    pub fn parsed(&self) -> Option<&SentenceSyntax> {
        match self {
            Self::Parsed(syntax) => Some(syntax),
            Self::Unparsed => None,
        }
    }
}

/// One sentence's position, counts, scores and flags, and what its parse gave.
#[derive(Debug, Clone, PartialEq)]
pub struct SentenceAnalysis {
    pub source_range: Range<usize>,
    pub counts: SurfaceCounts,
    pub readability: Option<Readability>,
    pub flags: Vec<Flag>,
    pub syntax: Syntax,
    /// Its nominalizations (surface words without a parse, `NOUN` tokens with one) over its words.
    pub nominalizations: NominalizationCount,
}

/// One file's sentences, the sum of their counts, and the file's scores computed from that sum;
/// `dependency_distance` and `passives` sum the sentences' and are `None` when nothing was parsed.
#[derive(Debug, Clone, PartialEq)]
pub struct FileAnalysis {
    pub spans: usize,
    pub sentences: Vec<SentenceAnalysis>,
    pub totals: SurfaceCounts,
    pub readability: Option<Readability>,
    pub dependency_distance: Option<DependencyDistance>,
    pub nominalizations: NominalizationCount,
    pub passives: Option<usize>,
}

/// Why a file could not be analyzed with a parser.
#[derive(Debug, thiserror::Error)]
pub enum AnalysisError<E: std::error::Error> {
    #[error("the parser failed: {0}")]
    Parse(#[source] E),
    #[error("the parse of the sentence at byte {sentence_start} is not a tree: {source}")]
    Malformed {
        sentence_start: usize,
        source: TreeError,
    },
}

/// Analyzes `source`, read as `format`: every sentence's counts, scores and flags, and the file's
/// totals and scores (formulas over the summed counts, never averages of sentence scores).
pub fn analyze(source: &str, format: SourceFormat, thresholds: &Thresholds) -> FileAnalysis {
    let Ok(file) = analyze_with::<Infallible>(source, format, thresholds, |_, _| Ok(None));
    file
}

/// Analyzes `source` like `analyze`, and also parses every sentence's text with `parser` to add
/// its syntactic metrics and flags, and the file's summed dependency distance.
pub fn analyze_parsed<P: Parser>(
    source: &str,
    format: SourceFormat,
    thresholds: &Thresholds,
    parser: &P,
) -> Result<FileAnalysis, AnalysisError<P::Error>> {
    let file = analyze_with(source, format, thresholds, |sentence, stoplist| {
        parse_sentence(sentence, thresholds, stoplist, parser).map(Some)
    })?;
    let parsed = file.sentences.iter().filter_map(|s| s.syntax.parsed());
    let dependency_distance = parsed.clone().map(|syntax| syntax.metrics.distance).sum();
    let passives = parsed.map(|syntax| syntax.passives.len()).sum();
    Ok(FileAnalysis {
        dependency_distance: Some(dependency_distance),
        passives: Some(passives),
        ..file
    })
}

/// The summed counts of every sentence of `source`, read as `format`.
pub fn surface_counts(source: &str, format: SourceFormat) -> SurfaceCounts {
    let Ok(counts) = each_sentence::<_, Infallible>(source, format, |s| Ok(sentence_counts(s)));
    counts.into_iter().sum()
}

/// What a parse adds to one sentence: its syntax, and its nominalizations among `NOUN` tokens.
struct Parsed {
    syntax: SentenceSyntax,
    nominalizations: usize,
}

/// The summed surface nominalizations and words of every sentence of `source`, read as `format`;
/// the same count `analyze` gives the file.
pub fn file_nominalizations(source: &str, format: SourceFormat) -> NominalizationCount {
    let stoplist = Stoplist::committed();
    let Ok(counts) = each_sentence::<_, Infallible>(source, format, |s| {
        Ok(NominalizationCount {
            nominalizations: surface_nominalizations(words(s.text()), &stoplist),
            words: sentence_counts(s).words,
        })
    });
    counts.into_iter().sum()
}

/// The analysis of `source`, with `parse` giving each sentence's parse (or `None` when there is
/// no parser); the file's dependency distance and passives are left `None` for the caller to fill.
fn analyze_with<E>(
    source: &str,
    format: SourceFormat,
    thresholds: &Thresholds,
    parse: impl Fn(&Sentence<'_>, &Stoplist) -> Result<Option<Parsed>, E>,
) -> Result<FileAnalysis, E> {
    let stoplist = Stoplist::committed();
    let sentences = each_sentence(source, format, |sentence| {
        let counts = sentence_counts(sentence);
        let parsed = parse(sentence, &stoplist)?;
        let nominalizations = parsed.as_ref().map_or_else(
            || surface_nominalizations(words(sentence.text()), &stoplist),
            |p| p.nominalizations,
        );
        Ok(SentenceAnalysis {
            source_range: sentence.source_range(),
            counts,
            readability: readability(counts),
            flags: flags(counts, thresholds),
            syntax: parsed.map_or(Syntax::Unparsed, |p| Syntax::Parsed(p.syntax)),
            nominalizations: NominalizationCount {
                nominalizations,
                words: counts.words,
            },
        })
    })?;
    let totals = sentences.iter().map(|s| s.counts).sum();
    Ok(FileAnalysis {
        spans: prose(source, format).len(),
        totals,
        readability: readability(totals),
        dependency_distance: None,
        nominalizations: sentences.iter().map(|s| s.nominalizations).sum(),
        sentences,
        passives: None,
    })
}

/// The parse of one sentence's text: its syntactic metrics, the syntactic rules it breaks, its
/// passives and its nominalizations.
fn parse_sentence<P: Parser>(
    sentence: &Sentence<'_>,
    thresholds: &Thresholds,
    stoplist: &Stoplist,
    parser: &P,
) -> Result<Parsed, AnalysisError<P::Error>> {
    let tokens = parser
        .parse(sentence.text())
        .map_err(AnalysisError::Parse)?;
    let tree = DependencyTree::new(tokens).map_err(|source| AnalysisError::Malformed {
        sentence_start: sentence.source_range().start,
        source,
    })?;
    let metrics = syntactic_metrics(&tree);
    Ok(Parsed {
        nominalizations: parsed_nominalizations(tree.tokens(), stoplist),
        syntax: SentenceSyntax {
            flags: syntactic_flags(&metrics, thresholds),
            metrics,
            passives: passives(sentence, tree.tokens()),
        },
    })
}

/// Each passive head of `tokens`, at its verb's first character in the source, or at the
/// sentence's first character when its form is not found in the text.
fn passives(sentence: &Sentence<'_>, tokens: &[Token]) -> Vec<Passive> {
    let offsets = align(tokens, sentence.text());
    passive_heads(tokens)
        .into_iter()
        .map(|head| Passive {
            verb: tokens[head - 1].form.clone(),
            source_offset: offsets[head - 1].map_or(sentence.source_range().start, |at| {
                sentence.source_offset(at)
            }),
        })
        .collect()
}

/// `f` applied to every sentence of `source`, in source order, stopping at the first error.
fn each_sentence<T, E>(
    source: &str,
    format: SourceFormat,
    mut f: impl FnMut(&Sentence<'_>) -> Result<T, E>,
) -> Result<Vec<T>, E> {
    let per_block = blocks(source, format)
        .iter()
        .map(|spans| {
            let block = Block::from_spans(spans);
            sentences(&block)
                .iter()
                .map(&mut f)
                .collect::<Result<Vec<T>, E>>()
        })
        .collect::<Result<Vec<Vec<T>>, E>>()?;
    Ok(per_block.into_iter().flatten().collect())
}

/// One sentence's counts; only its first word is sentence-initial.
fn sentence_counts(sentence: &Sentence<'_>) -> SurfaceCounts {
    words(sentence.text())
        .enumerate()
        .map(|(index, word)| SurfaceCounts {
            words: 1,
            sentences: 0,
            syllables: count_syllables(word),
            complex_words: usize::from(is_complex(word, index == 0)),
        })
        .sum::<SurfaceCounts>()
        + SurfaceCounts {
            sentences: 1,
            ..SurfaceCounts::default()
        }
}

/// The rules a sentence with `counts` breaks.
fn flags(counts: SurfaceCounts, thresholds: &Thresholds) -> Vec<Flag> {
    let is_long = counts.words > thresholds.max_sentence_len;
    is_long.then_some(Flag::LongSentence).into_iter().collect()
}

/// The syntactic rules a sentence with `metrics` breaks: `HighMdd`, `DeepTree` and
/// `ClauseOverload` when a metric exceeds its threshold, then one `CenterEmbedding` per embedding.
pub fn syntactic_flags(metrics: &SyntacticMetrics, thresholds: &Thresholds) -> Vec<SyntacticFlag> {
    let high_mdd = metrics
        .mdd()
        .filter(|&mdd| mdd > thresholds.max_mdd)
        .map(|mdd| SyntacticFlag::HighMdd { mdd });
    let deep_tree =
        (metrics.depth > thresholds.max_tree_depth).then_some(SyntacticFlag::DeepTree {
            depth: metrics.depth,
        });
    let clause_overload =
        (metrics.clauses > thresholds.max_clauses).then_some(SyntacticFlag::ClauseOverload {
            clauses: metrics.clauses,
        });
    let center_embeddings = metrics
        .center_embeddings
        .iter()
        .cloned()
        .map(SyntacticFlag::CenterEmbedding);
    [high_mdd, deep_tree, clause_overload]
        .into_iter()
        .flatten()
        .chain(center_embeddings)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nominalization::{parsed_nominalizations, surface_lemma};
    use crate::testing::{
        EXAMPLE_CONLLU, EXAMPLE_TEXT, NOMZ_CONLLU, NOMZ_TEXT, PASSIVE_CONLLU, PASSIVE_TEXT,
        tokens_from_conllu,
    };
    use crate::words::count_words;
    use proptest::prelude::*;

    fn thresholds(max_sentence_len: usize) -> Thresholds {
        Thresholds {
            max_sentence_len,
            max_mdd: 3.0,
            max_tree_depth: 5,
            max_clauses: 2,
        }
    }

    fn md(source: &str, max_sentence_len: usize) -> FileAnalysis {
        analyze(
            source,
            SourceFormat::Markdown,
            &thresholds(max_sentence_len),
        )
    }

    #[derive(Debug, PartialEq, Eq, thiserror::Error)]
    #[error("no parse for {0:?}")]
    struct Unparsable(String);

    /// Knows the parses of a fixed table of sentence texts and fails on any other text.
    struct FixtureParser(Vec<(String, Vec<Token>)>);

    impl Parser for FixtureParser {
        type Error = Unparsable;

        fn parse(&self, sentence: &str) -> Result<Vec<Token>, Unparsable> {
            self.0
                .iter()
                .find(|(text, _)| text == sentence)
                .map(|(_, tokens)| tokens.clone())
                .ok_or_else(|| Unparsable(sentence.to_owned()))
        }
    }

    /// Splits at whitespace and heads every token by the next; the last is the root.
    struct ChainParser;

    impl Parser for ChainParser {
        type Error = Unparsable;

        fn parse(&self, sentence: &str) -> Result<Vec<Token>, Unparsable> {
            let forms: Vec<&str> = sentence.split_whitespace().collect();
            let n = forms.len();
            Ok(forms
                .iter()
                .enumerate()
                .map(|(i, form)| Token {
                    id: i + 1,
                    form: (*form).to_owned(),
                    lemma: (*form).to_owned(),
                    upostag: "X".to_owned(),
                    head: if i + 1 == n { 0 } else { i + 2 },
                    deprel: if i + 1 == n { "root" } else { "dep" }.to_owned(),
                })
                .collect())
        }
    }

    /// Fails on every sentence.
    struct FailingParser;

    impl Parser for FailingParser {
        type Error = Unparsable;

        fn parse(&self, sentence: &str) -> Result<Vec<Token>, Unparsable> {
            Err(Unparsable(sentence.to_owned()))
        }
    }

    fn example_parser(text: &str) -> FixtureParser {
        FixtureParser(vec![(text.to_owned(), tokens_from_conllu(EXAMPLE_CONLLU))])
    }

    fn proposal_caused_8() -> SyntacticFlag {
        SyntacticFlag::CenterEmbedding(CenterEmbedding {
            subject: "proposal".to_owned(),
            verb: "caused".to_owned(),
            words_between: 8,
        })
    }

    /// Given the M3a example sentence and its UDPipe 2 parse (MDD 32/12 = 8/3, depth 4, 1 clause)
    /// When it is analyzed with `max_mdd` 2.5, and with 3.0
    /// Then it is flagged HighMdd (carrying 8/3) and CenterEmbedding at 2.5, only CenterEmbedding
    /// at 3.0, and never DeepTree or ClauseOverload
    #[test]
    fn example_is_center_embedded_and_high_mdd_only_below_8_3() {
        let parser = example_parser(EXAMPLE_TEXT);
        let flags_at = |max_mdd: f64| {
            let t = Thresholds {
                max_mdd,
                ..thresholds(25)
            };
            let file = analyze_parsed(EXAMPLE_TEXT, SourceFormat::Markdown, &t, &parser).unwrap();
            assert_eq!(file.sentences.len(), 1);
            file.sentences[0].syntax.parsed().unwrap().flags.clone()
        };
        assert_eq!(
            flags_at(2.5),
            [
                SyntacticFlag::HighMdd { mdd: 32.0 / 12.0 },
                proposal_caused_8()
            ]
        );
        assert_eq!(flags_at(3.0), [proposal_caused_8()]);
    }

    /// Given the example sentence with its subject in bold (`**The proposal**, which …`), and a
    /// parser that knows only the sentence's prose text (block prose joins the bold span and the
    /// comma directly, so the text is the example sentence itself) and fails on anything else
    /// When it is analyzed with that parser
    /// Then the parser is given the prose, not the markup, and the center-embedding is reported
    #[test]
    fn parses_the_prose_not_the_markup() {
        let source = EXAMPLE_TEXT.replacen("The proposal", "**The proposal**", 1);
        let parser = example_parser(EXAMPLE_TEXT);
        let file = analyze_parsed(&source, SourceFormat::Markdown, &thresholds(25), &parser);
        let syntax = file.unwrap().sentences[0].syntax.parsed().cloned().unwrap();
        assert!(syntax.flags.contains(&proposal_caused_8()), "{syntax:?}");
    }

    /// Given a file and a parser that fails on every sentence
    /// When the file is analyzed with it
    /// Then the analysis fails with the parser's error
    #[test]
    fn a_parser_failure_fails_the_analysis() {
        let result = analyze_parsed(
            "One sentence here.",
            SourceFormat::Markdown,
            &thresholds(25),
            &FailingParser,
        );
        assert!(
            matches!(&result, Err(AnalysisError::Parse(Unparsable(text))) if text == "One sentence here."),
            "{result:?}"
        );
    }

    /// Given a file whose second sentence (at byte 17) the parser returns as a cycle
    /// When the file is analyzed with it
    /// Then the analysis fails as malformed, naming the sentence's start and the tree error
    #[test]
    fn a_malformed_parse_fails_the_analysis_at_its_sentence() {
        let cycle = tokens_from_conllu("1 Broken broken X _ _ 2 dep\n2 one. one X _ _ 1 dep");
        let parser = FixtureParser(vec![
            (
                "Fine words here.".to_owned(),
                ChainParser.parse("Fine words here.").unwrap(),
            ),
            ("Broken one.".to_owned(), cycle),
        ]);
        let result = analyze_parsed(
            "Fine words here. Broken one.",
            SourceFormat::Markdown,
            &thresholds(25),
            &parser,
        );
        assert!(
            matches!(
                &result,
                Err(AnalysisError::Malformed {
                    sentence_start: 17,
                    source: TreeError::Cycle { .. }
                })
            ),
            "{result:?}"
        );
    }

    /// Given an empty file
    /// When it is analyzed without a parser, and with one
    /// Then its dependency distance is absent without a parser and zero with one
    #[test]
    fn dependency_distance_is_absent_without_a_parser_and_zero_with_one() {
        assert_eq!(md("", 25).dependency_distance, None);
        let parsed =
            analyze_parsed("", SourceFormat::Markdown, &thresholds(25), &ChainParser).unwrap();
        assert_eq!(
            parsed.dependency_distance,
            Some(DependencyDistance::default())
        );
    }

    /// Given the NOMZ sentence
    /// When it is analyzed without a parse
    /// Then it has 4 nominalizations of 14 words (commission, decisions, implementation,
    /// consideration), as does the file, and passive voice is absent
    #[test]
    fn counts_surface_nominalizations_of_the_nomz_sentence() {
        let file = md(NOMZ_TEXT, 25);
        let expected = NominalizationCount {
            nominalizations: 4,
            words: 14,
        };
        assert_eq!(file.sentences.len(), 1);
        assert_eq!(file.sentences[0].nominalizations, expected);
        assert_eq!(file.nominalizations, expected);
        assert_eq!(file.passives, None);
    }

    /// Given the NOMZ sentence and its UDPipe 2 parse
    /// When it is analyzed with that parse
    /// Then it has 3 nominalizations of 14 words: `commission` is a VERB, `decisions` counts by
    /// its lemma `decision`, and the words stay M2's
    #[test]
    fn counts_parsed_nominalizations_of_the_nomz_sentence() {
        let parser = FixtureParser(vec![(
            NOMZ_TEXT.to_owned(),
            tokens_from_conllu(NOMZ_CONLLU),
        )]);
        let file =
            analyze_parsed(NOMZ_TEXT, SourceFormat::Markdown, &thresholds(25), &parser).unwrap();
        let expected = NominalizationCount {
            nominalizations: 3,
            words: 14,
        };
        assert_eq!(file.sentences[0].nominalizations, expected);
        assert_eq!(file.nominalizations, expected);
    }

    /// Given the PASSIVE sentence, as plain text and with its first verb in bold, and its parse
    /// When it is analyzed with that parse
    /// Then it reports `written` and `rejected` at their first characters in the source, and the
    /// file counts 2 passives
    #[test]
    fn reports_passives_at_their_verbs() {
        let parser = FixtureParser(vec![(
            PASSIVE_TEXT.to_owned(),
            tokens_from_conllu(PASSIVE_CONLLU),
        )]);
        let bold = PASSIVE_TEXT.replacen("written", "**written**", 1);
        for source in [PASSIVE_TEXT.to_owned(), bold] {
            let file =
                analyze_parsed(&source, SourceFormat::Markdown, &thresholds(25), &parser).unwrap();
            let passives = &file.sentences[0].syntax.parsed().unwrap().passives;
            let expected = ["written", "rejected"].map(|verb| Passive {
                verb: verb.to_owned(),
                source_offset: source.find(verb).unwrap(),
            });
            assert_eq!(passives, &expected, "{source:?}");
            assert_eq!(file.passives, Some(2));
        }
    }

    /// Chains each sentence's whitespace tokens like `ChainParser`, tags them `NOUN` with their
    /// surface lemma, makes every token at an even index an `aux:pass` of the token two places
    /// on, and upper-cases the second token's form, so one passive verb cannot be found in the
    /// text and falls back to the sentence start.
    struct PassiveChainParser;

    impl Parser for PassiveChainParser {
        type Error = Unparsable;

        fn parse(&self, sentence: &str) -> Result<Vec<Token>, Unparsable> {
            let mut tokens = ChainParser.parse(sentence)?;
            let n = tokens.len();
            for (i, token) in tokens.iter_mut().enumerate() {
                token.upostag = "NOUN".to_owned();
                token.lemma = surface_lemma(&token.form);
                if i % 2 == 0 && i + 2 < n {
                    token.deprel = "aux:pass".to_owned();
                }
                if i == 1 {
                    token.form = token.form.to_uppercase();
                }
            }
            Ok(tokens)
        }
    }

    /// Paragraphs of sentences mixing nominalizations (plain, plural, possessive, stoplisted,
    /// capitalized) with other words.
    fn nominal_document() -> impl Strategy<Value = String> {
        let word = prop::sample::select(vec![
            "the",
            "decisions",
            "implementation",
            "committee's",
            "Consideration",
            "cities",
            "rations",
            "was",
            "written",
            "we",
            "review",
            "activities",
        ]);
        let sentence = prop::collection::vec(word, 1..15).prop_map(|w| format!("{}.", w.join(" ")));
        let para = prop::collection::vec(sentence, 1..4).prop_map(|s| s.join(" "));
        prop::collection::vec(para, 0..4).prop_map(|p| p.join("\n\n"))
    }

    /// The analysis without anything that only a parse provides.
    fn surface_only(file: &FileAnalysis) -> FileAnalysis {
        FileAnalysis {
            sentences: file
                .sentences
                .iter()
                .map(|s| SentenceAnalysis {
                    syntax: Syntax::Unparsed,
                    ..s.clone()
                })
                .collect(),
            dependency_distance: None,
            passives: None,
            ..file.clone()
        }
    }

    fn center_embedding() -> impl Strategy<Value = CenterEmbedding> {
        (
            prop::sample::select(vec!["proposal", "man", "it"]),
            prop::sample::select(vec!["caused", "came", "is"]),
            0usize..20,
        )
            .prop_map(|(subject, verb, words_between)| CenterEmbedding {
                subject: subject.to_owned(),
                verb: verb.to_owned(),
                words_between,
            })
    }

    fn any_metrics() -> impl Strategy<Value = SyntacticMetrics> {
        (
            prop_oneof![1 => Just(0usize), 3 => 1usize..30],
            0usize..4,
            0usize..10,
            0usize..6,
            prop::collection::vec(center_embedding(), 0..3),
        )
            .prop_map(|(dependencies, extra, depth, clauses, center_embeddings)| {
                SyntacticMetrics {
                    distance: DependencyDistance {
                        total: dependencies * (1 + extra) + dependencies / 2,
                        dependencies,
                    },
                    depth,
                    clauses,
                    center_embeddings,
                }
            })
    }

    /// A threshold equal to the measured `value` half the time, so the `>` boundary is exercised.
    fn at_or_near(value: usize) -> impl Strategy<Value = usize> {
        prop_oneof![Just(value), 0usize..10]
    }

    /// Metrics with thresholds that equal each metric half the time.
    fn metrics_and_thresholds() -> impl Strategy<Value = (SyntacticMetrics, Thresholds)> {
        any_metrics().prop_flat_map(|m| {
            let mdd = m.mdd().unwrap_or(1.0);
            let max_mdd = prop_oneof![Just(mdd), 0.5f64..6.0];
            (
                Just(m.clone()),
                max_mdd,
                at_or_near(m.depth),
                at_or_near(m.clauses),
            )
                .prop_map(|(m, max_mdd, max_tree_depth, max_clauses)| {
                    let thresholds = Thresholds {
                        max_sentence_len: 25,
                        max_mdd,
                        max_tree_depth,
                        max_clauses,
                    };
                    (m, thresholds)
                })
        })
    }

    /// Given an empty file, and a file of only a heading and a code block
    /// When it is analyzed
    /// Then it has no sentence, zero counts and absent metrics
    #[test]
    fn empty_file_has_absent_metrics() {
        for source in ["", "# Only a heading\n\n```\nlet code = 1;\n```\n"] {
            let file = md(source, 25);
            assert!(file.sentences.is_empty(), "{source:?}");
            assert_eq!(file.totals, SurfaceCounts::default(), "{source:?}");
            assert_eq!(file.readability, None, "{source:?}");
        }
    }

    /// Given "Australia celebrated in Australia. It rained."
    /// When it is analyzed
    /// Then the first sentence counts 4 words, 13 syllables and 2 complex words (the capitalized
    /// non-initial "Australia" is exempt), the second 2 words and 2 syllables, and the file sums them
    #[test]
    fn counts_a_witness_per_sentence_and_per_file() {
        let file = md("Australia celebrated in Australia. It rained.\n", 25);
        let counts: Vec<SurfaceCounts> = file.sentences.iter().map(|s| s.counts).collect();
        let first = SurfaceCounts {
            words: 4,
            sentences: 1,
            syllables: 13,
            complex_words: 2,
        };
        let second = SurfaceCounts {
            words: 2,
            sentences: 1,
            syllables: 2,
            complex_words: 0,
        };
        assert_eq!(counts, [first, second]);
        assert_eq!(file.totals, first + second);
        assert_eq!(file.readability, readability(first + second));
    }

    fn sentence_of(words: usize) -> String {
        let body = vec!["word"; words].join(" ");
        format!("{body}.")
    }

    fn document() -> impl Strategy<Value = String> {
        let word = prop::sample::select(vec![
            "Alpha",
            "beta",
            "café",
            "3.14",
            "celebrated",
            "Dr.",
            "understanding",
        ]);
        let sentence = prop::collection::vec(word, 1..40).prop_map(|w| format!("{}.", w.join(" ")));
        let para = prop::collection::vec(sentence, 1..4).prop_map(|s| s.join(" "));
        prop::collection::vec(para, 0..4).prop_map(|p| p.join("\n\n"))
    }

    proptest! {
        /// Given a sentence of n words and any maximum
        /// When it is analyzed
        /// Then it is flagged LongSentence exactly when n exceeds the maximum
        #[test]
        fn long_sentence_iff_more_words_than_max(n in 1usize..60, max in 0usize..60) {
            let file = md(&sentence_of(n), max);
            prop_assert_eq!(file.sentences.len(), 1);
            prop_assert_eq!(file.sentences[0].counts.words, n);
            prop_assert_eq!(file.sentences[0].flags.contains(&Flag::LongSentence), n > max);
        }

        /// Given any syntactic metrics and thresholds (each threshold equal to its metric half the
        /// time)
        /// When the syntactic flags are computed
        /// Then `HighMdd` carries the MDD exactly when it exceeds `max_mdd`, `DeepTree` the depth
        /// exactly when it exceeds `max_tree_depth`, `ClauseOverload` the clause count exactly when
        /// it exceeds `max_clauses`, and there is one `CenterEmbedding` per embedding, in order
        #[test]
        fn syntactic_flags_follow_the_thresholds((m, t) in metrics_and_thresholds()) {
            let flags = syntactic_flags(&m, &t);
            let high_mdd: Vec<f64> = flags.iter().filter_map(|f| match f {
                SyntacticFlag::HighMdd { mdd } => Some(*mdd),
                _ => None,
            }).collect();
            let expected_mdd: Vec<f64> = m.mdd().filter(|&x| x > t.max_mdd).into_iter().collect();
            prop_assert_eq!(high_mdd, expected_mdd);
            let deep = flags.iter().filter(|f| matches!(f, SyntacticFlag::DeepTree { .. })).collect::<Vec<_>>();
            let expected_deep = (m.depth > t.max_tree_depth).then_some(SyntacticFlag::DeepTree { depth: m.depth });
            prop_assert_eq!(deep, expected_deep.iter().collect::<Vec<_>>());
            let overload = flags.iter().filter(|f| matches!(f, SyntacticFlag::ClauseOverload { .. })).collect::<Vec<_>>();
            let expected_overload = (m.clauses > t.max_clauses).then_some(SyntacticFlag::ClauseOverload { clauses: m.clauses });
            prop_assert_eq!(overload, expected_overload.iter().collect::<Vec<_>>());
            let embedded: Vec<&CenterEmbedding> = flags.iter().filter_map(|f| match f {
                SyntacticFlag::CenterEmbedding(e) => Some(e),
                _ => None,
            }).collect();
            prop_assert_eq!(embedded, m.center_embeddings.iter().collect::<Vec<_>>());
        }

        /// Given generated Markdown paragraphs and a parser that chains each sentence's words
        /// When the file is analyzed with and without the parser
        /// Then both have the same sentences, positions, counts and surface flags; without it
        /// nothing is parsed; with it every sentence has the metrics of its text's parse and the
        /// flags of those metrics, and the file's dependency distance is the sum of the sentences'
        #[test]
        fn parsing_adds_syntax_and_changes_nothing_else(doc in document(), max in 0usize..40) {
            let t = thresholds(max);
            let plain = analyze(&doc, SourceFormat::Markdown, &t);
            let parsed = analyze_parsed(&doc, SourceFormat::Markdown, &t, &ChainParser).unwrap();
            prop_assert_eq!(surface_only(&parsed), plain.clone());
            prop_assert_eq!(plain.dependency_distance, None);
            prop_assert!(plain.sentences.iter().all(|s| s.syntax == Syntax::Unparsed));
            let mut sum = DependencyDistance::default();
            for s in &parsed.sentences {
                let syntax = s.syntax.parsed().expect("every sentence is parsed");
                let text = &doc[s.source_range.clone()];
                let tree = DependencyTree::new(ChainParser.parse(text).unwrap()).unwrap();
                prop_assert_eq!(&syntax.metrics, &syntactic_metrics(&tree));
                prop_assert_eq!(&syntax.flags, &syntactic_flags(&syntax.metrics, &t));
                sum = sum + syntax.metrics.distance;
            }
            prop_assert_eq!(parsed.dependency_distance, Some(sum));
        }

        /// Given generated paragraphs with nominalizations
        /// When the file is analyzed without a parse
        /// Then each sentence counts the nominalizations among its words over its M2 words, the
        /// file's count is their sum over the file's words, and passive voice is absent
        #[test]
        fn file_nominalizations_are_the_sum_of_the_sentences(doc in nominal_document()) {
            let file = md(&doc, 25);
            let stoplist = Stoplist::committed();
            for s in &file.sentences {
                let text = &doc[s.source_range.clone()];
                prop_assert_eq!(s.nominalizations.words, s.counts.words);
                prop_assert_eq!(s.nominalizations.nominalizations, surface_nominalizations(words(text), &stoplist));
            }
            let sum: NominalizationCount = file.sentences.iter().map(|s| s.nominalizations).sum();
            prop_assert_eq!(file.nominalizations, sum);
            prop_assert_eq!(file.nominalizations.words, file.totals.words);
            prop_assert_eq!(file.passives, None);
        }

        /// Given generated paragraphs with nominalizations, and a parser that marks passives
        /// When the file is analyzed with and without it
        /// Then without it passive voice is absent; with it the file counts the sentences'
        /// passives, each at its verb in the source or at the sentence start when the verb's form
        /// is not in the text (both occur), and nominalizations come from the parse over M2 words
        #[test]
        fn parsed_passives_and_nominalizations_add_up(doc in nominal_document()) {
            prop_assert_eq!(md(&doc, 25).passives, None);
            let file = analyze_parsed(&doc, SourceFormat::Markdown, &thresholds(25), &PassiveChainParser).unwrap();
            let mut passives = 0;
            for s in &file.sentences {
                let syntax = s.syntax.parsed().expect("parsed");
                passives += syntax.passives.len();
                for p in &syntax.passives {
                    prop_assert!(s.source_range.contains(&p.source_offset), "{:?} {:?}", p, s.source_range);
                    prop_assert!(
                        doc[p.source_offset..].starts_with(&p.verb) || p.source_offset == s.source_range.start,
                        "{:?}", p
                    );
                }
                let tokens = PassiveChainParser.parse(&doc[s.source_range.clone()]).unwrap();
                prop_assert_eq!(s.nominalizations.nominalizations, parsed_nominalizations(&tokens, &Stoplist::committed()));
                prop_assert_eq!(s.nominalizations.words, s.counts.words);
            }
            prop_assert_eq!(file.passives, Some(passives));
            let sum: NominalizationCount = file.sentences.iter().map(|s| s.nominalizations).sum();
            prop_assert_eq!(file.nominalizations, sum);
            prop_assert_eq!(file.nominalizations.words, file.totals.words);
        }

        /// Given generated Markdown paragraphs
        /// When a file is analyzed
        /// Then its totals are the sum of its sentences' counts, its scores are computed from
        /// them, and each sentence's counts follow its words
        #[test]
        fn file_counts_are_the_sum_of_sentence_counts(doc in document(), max in 0usize..40) {
            let file = md(&doc, max);
            let sum: SurfaceCounts = file.sentences.iter().map(|s| s.counts).sum();
            prop_assert_eq!(file.totals, sum);
            prop_assert_eq!(file.totals.sentences, file.sentences.len());
            prop_assert_eq!(file.totals.words, count_words(&doc));
            prop_assert_eq!(file.readability, readability(file.totals));
            for s in &file.sentences {
                let text = &doc[s.source_range.clone()];
                let words: Vec<&str> = words(text).collect();
                prop_assert_eq!(s.counts.syllables, words.iter().map(|w| count_syllables(w)).sum::<usize>());
                let complex = words.iter().enumerate().filter(|(i, w)| is_complex(w, *i == 0)).count();
                prop_assert_eq!(s.counts.complex_words, complex);
                prop_assert_eq!(s.readability, readability(s.counts));
            }
        }
    }
}
