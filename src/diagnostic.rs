//! Diagnostics (spec 001 M5): what a flagged sentence reports, and how it is rendered.

use std::fmt;
use std::ops::Range;

use crate::analysis::{FileAnalysis, Flag, SentenceAnalysis, SyntacticFlag, Thresholds};
use crate::position::{LineIndex, Position, PositionError};

/// The code of every vernier diagnostic.
pub const CODE: &str = "CognitiveOverload";

/// What one flagged sentence reports: where it starts and ends, and the flags it raised.
#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub start: Position,
    pub end: Position,
    pub source_range: Range<usize>,
    /// Surface flags first, then syntactic flags.
    pub flags: Vec<FlagMessage>,
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

/// The messages of a sentence's flags: its surface flags, then its syntactic flags.
pub fn flag_messages(sentence: &SentenceAnalysis, thresholds: &Thresholds) -> Vec<FlagMessage> {
    let surface = sentence
        .flags
        .iter()
        .map(|flag| surface_message(*flag, sentence, thresholds));
    let syntactic = sentence
        .syntax
        .iter()
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
    use crate::analysis::{SentenceSyntax, analyze};
    use crate::nominalization::NominalizationCount;
    use crate::prose::SourceFormat;
    use crate::readability::SurfaceCounts;
    use crate::syntax::{CenterEmbedding, DependencyDistance, SyntacticMetrics};
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
            syntax: Some(SentenceSyntax {
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
