//! Diagnostics (spec 001 M5): what a flagged sentence reports, and how it is rendered.

use std::fmt;

use crate::analysis::{Flag, SentenceAnalysis, SyntacticFlag, Thresholds};

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
    use crate::syntax::CenterEmbedding;

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
}
