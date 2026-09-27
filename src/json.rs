//! The JSON document (spec 001 M5): `schema_version` 1, each readable file's path, metrics and
//! diagnostics. The output types exist only for this document, so the schema is this file.

use serde::Serialize;

use crate::diagnostic::{CODE, Diagnostic, FlagMessage, TITLE};
use crate::position::Position;
use crate::readability::Readability;
use crate::summary::FileSummary;

/// The version of the document's schema.
pub const SCHEMA_VERSION: u32 = 1;

/// One readable file: its path as printed, its summary and its diagnostics.
#[derive(Debug, Clone)]
pub struct FileReport<'a> {
    pub path: String,
    pub summary: &'a FileSummary,
    pub diagnostics: &'a [Diagnostic],
}

/// The document for `files`, pretty-printed.
pub fn document(files: &[FileReport<'_>]) -> Result<String, serde_json::Error> {
    let document = Document {
        schema_version: SCHEMA_VERSION,
        files: files.iter().map(FileJson::of).collect(),
    };
    serde_json::to_string_pretty(&document)
}

#[derive(Serialize)]
struct Document<'a> {
    schema_version: u32,
    files: Vec<FileJson<'a>>,
}

#[derive(Serialize)]
struct FileJson<'a> {
    path: &'a str,
    metrics: MetricsJson,
    diagnostics: Vec<DiagnosticJson<'a>>,
}

impl<'a> FileJson<'a> {
    fn of(report: &'a FileReport<'a>) -> Self {
        Self {
            path: &report.path,
            metrics: MetricsJson::of(report.summary),
            diagnostics: report.diagnostics.iter().map(DiagnosticJson::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct MetricsJson {
    prose_spans: usize,
    words: usize,
    sentences: usize,
    syllables: usize,
    complex_words: usize,
    flesch_reading_ease: Option<f64>,
    flesch_kincaid_grade: Option<f64>,
    gunning_fog: Option<f64>,
    average_sentence_length: Option<f64>,
    mean_dependency_distance: Option<f64>,
    nominalizations: usize,
    nominalization_ratio: Option<f64>,
    passives: Option<usize>,
}

impl MetricsJson {
    fn of(s: &FileSummary) -> Self {
        let score = |pick: fn(&Readability) -> f64| s.readability.as_ref().map(pick);
        Self {
            prose_spans: s.spans,
            words: s.words,
            sentences: s.counts.sentences,
            syllables: s.counts.syllables,
            complex_words: s.counts.complex_words,
            flesch_reading_ease: score(|r| r.flesch_reading_ease),
            flesch_kincaid_grade: score(|r| r.flesch_kincaid_grade),
            gunning_fog: score(|r| r.gunning_fog),
            average_sentence_length: score(|r| r.average_sentence_length),
            mean_dependency_distance: s.mean_dependency_distance,
            nominalizations: s.nominalizations.nominalizations,
            nominalization_ratio: s.nominalizations.ratio(),
            passives: s.passives,
        }
    }
}

#[derive(Serialize)]
struct DiagnosticJson<'a> {
    code: &'static str,
    severity: &'static str,
    message: &'static str,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
    flags: Vec<FlagJson<'a>>,
}

impl<'a> DiagnosticJson<'a> {
    fn of(d: &'a Diagnostic) -> Self {
        let (start, end): (Position, Position) = (d.start, d.end);
        Self {
            code: CODE,
            severity: "warning",
            message: TITLE,
            line: start.line(),
            column: start.column(),
            end_line: end.line(),
            end_column: end.column(),
            flags: d.flags.iter().map(FlagJson::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct FlagJson<'a> {
    name: &'static str,
    message: &'a str,
}

impl<'a> FlagJson<'a> {
    fn of(flag: &'a FlagMessage) -> Self {
        Self {
            name: flag.name,
            message: &flag.message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{Thresholds, analyze};
    use crate::diagnostic::diagnostics;
    use crate::prose::SourceFormat;
    use crate::summary::summarize;
    use proptest::prelude::*;
    use serde_json::Value;

    const THRESHOLDS: Thresholds = Thresholds {
        max_sentence_len: 25,
        max_mdd: 3.0,
        max_tree_depth: 5,
        max_clauses: 2,
    };

    /// The summary and diagnostics of a Markdown source under `t`.
    fn measured(source: &str, t: &Thresholds) -> (FileSummary, Vec<Diagnostic>) {
        let file = analyze(source, SourceFormat::Markdown, t);
        (
            summarize(source, SourceFormat::Markdown),
            diagnostics(source, &file, t).unwrap(),
        )
    }

    fn parsed(files: &[FileReport<'_>]) -> Value {
        serde_json::from_str(&document(files).unwrap()).unwrap()
    }

    /// Given the long-sentence fixture and the sample fixture
    /// When their JSON document is written
    /// Then it has schema_version 1, both files in order, the long sentence's diagnostic at line 3,
    /// column 20 with its code, severity, title and flag, no diagnostic for the sample, and null for
    /// the parse-only metrics
    #[test]
    fn documents_the_fixtures() {
        let long = include_str!("../tests/fixtures/long.md");
        let sample = include_str!("../tests/fixtures/sample.md");
        let (long_summary, long_found) = measured(long, &THRESHOLDS);
        let (sample_summary, sample_found) = measured(sample, &THRESHOLDS);
        let value = parsed(&[
            FileReport {
                path: "long.md".to_owned(),
                summary: &long_summary,
                diagnostics: &long_found,
            },
            FileReport {
                path: "sample.md".to_owned(),
                summary: &sample_summary,
                diagnostics: &sample_found,
            },
        ]);
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["files"][0]["path"], "long.md");
        assert_eq!(value["files"][1]["path"], "sample.md");
        assert_the_long_sentence(&value["files"][0]["diagnostics"][0]);
        assert_eq!(value["files"][1]["diagnostics"], Value::Array(Vec::new()));
        let metrics = &value["files"][1]["metrics"];
        assert_eq!(metrics["sentences"], 4);
        assert_eq!(metrics["mean_dependency_distance"], Value::Null);
        assert_eq!(metrics["passives"], Value::Null);
    }

    /// The long fixture's diagnostic: at 3:20, with code, severity, title and its flag.
    fn assert_the_long_sentence(d: &Value) {
        assert_eq!(
            (d["line"].as_u64(), d["column"].as_u64()),
            (Some(3), Some(20))
        );
        assert_eq!(d["code"], "CognitiveOverload");
        assert_eq!(d["severity"], "warning");
        assert_eq!(
            d["message"],
            "Sentence exceeds human working-memory capacity"
        );
        assert_eq!(d["flags"][0]["name"], "LongSentence");
        assert_eq!(d["flags"][0]["message"], "sentence has 30 words (max 25)");
    }

    fn document_source() -> impl Strategy<Value = String> {
        let word = prop::sample::select(vec![
            "word",
            "Café",
            "decisions",
            "**bold**",
            "implementation",
            "x2",
        ]);
        let sentence = prop::collection::vec(word, 1..10).prop_map(|w| format!("{}.", w.join(" ")));
        let para = prop::collection::vec(sentence, 1..3).prop_map(|s| s.join(" "));
        prop::collection::vec(para, 0..4).prop_map(|p| p.join("\n\n"))
    }

    fn number_or_null(value: Option<f64>) -> Value {
        value.map_or(Value::Null, Value::from)
    }

    proptest! {
        /// Given generated documents and a sentence-length limit
        /// When their JSON document is written
        /// Then every file's metrics are its summary's fields (null exactly when absent) and every
        /// diagnostic's line, column, end line and end column are its start and end positions
        #[test]
        fn the_document_carries_the_summaries_and_positions(
            sources in prop::collection::vec(document_source(), 1..3),
            max in 0usize..8,
        ) {
            let t = Thresholds { max_sentence_len: max, ..THRESHOLDS };
            let measured: Vec<(FileSummary, Vec<Diagnostic>)> = sources.iter().map(|s| measured(s, &t)).collect();
            let reports: Vec<FileReport<'_>> = measured
                .iter()
                .enumerate()
                .map(|(i, (summary, found))| FileReport { path: format!("f{i}.md"), summary, diagnostics: found })
                .collect();
            let value = parsed(&reports);
            prop_assert_eq!(&value["schema_version"], &Value::from(SCHEMA_VERSION));
            prop_assert_eq!(value["files"].as_array().map(Vec::len), Some(reports.len()));
            for (file, report) in value["files"].as_array().unwrap().iter().zip(&reports) {
                let (m, s) = (&file["metrics"], report.summary);
                prop_assert_eq!(&file["path"], &Value::from(report.path.as_str()));
                prop_assert_eq!(&m["prose_spans"], &Value::from(s.spans));
                prop_assert_eq!(&m["words"], &Value::from(s.words));
                prop_assert_eq!(&m["sentences"], &Value::from(s.counts.sentences));
                prop_assert_eq!(&m["syllables"], &Value::from(s.counts.syllables));
                prop_assert_eq!(&m["complex_words"], &Value::from(s.counts.complex_words));
                prop_assert_eq!(&m["flesch_reading_ease"], &number_or_null(s.readability.map(|r| r.flesch_reading_ease)));
                prop_assert_eq!(&m["gunning_fog"], &number_or_null(s.readability.map(|r| r.gunning_fog)));
                prop_assert_eq!(&m["nominalizations"], &Value::from(s.nominalizations.nominalizations));
                prop_assert_eq!(&m["nominalization_ratio"], &number_or_null(s.nominalizations.ratio()));
                prop_assert_eq!(&m["mean_dependency_distance"], &number_or_null(s.mean_dependency_distance));
                prop_assert_eq!(&m["passives"], &s.passives.map_or(Value::Null, Value::from));
                let listed = file["diagnostics"].as_array().unwrap();
                prop_assert_eq!(listed.len(), report.diagnostics.len());
                for (d, expected) in listed.iter().zip(report.diagnostics) {
                    let at = |key: &str| d[key].as_u64().and_then(|n| usize::try_from(n).ok());
                    prop_assert_eq!((at("line"), at("column")), (Some(expected.start.line()), Some(expected.start.column())));
                    prop_assert_eq!((at("end_line"), at("end_column")), (Some(expected.end.line()), Some(expected.end.column())));
                    let names: Vec<&str> = d["flags"].as_array().unwrap().iter().filter_map(|f| f["name"].as_str()).collect();
                    let expected_names: Vec<&str> = expected.flags.iter().map(|f| f.name).collect();
                    prop_assert_eq!(names, expected_names);
                }
            }
        }
    }
}
