# Audit 024 — a review repair narrowed a criterion, and a model path that is a file was "missing"

**Severity:** medium
**Tags:** spec, review, model, cli

## Symptom
M3b criterion 6 read "IF `--model-path` names a directory that is not a usable model …", while before review pass 1 it read "WHEN `--model-path` names a missing or unusable model, THE tool SHALL name it on stderr and exit 2" (`git show f2d5974^:docs/specs/001-vernier/spec.md`, line 132; `8723cc6` before the history was rewritten).
A path that does not exist, or one that is a regular file, is not "a directory", so the criterion no longer covered either, though the code refuses both.
The code's cause for a regular file was false too: `vernier check --model-path README.md …` said `README.md does not exist` (`LoadError::MissingFile`, from `if !model_dir.is_dir()` in `OnnxParser::load`), and a model file that is a directory (`config.json/`) said the same.
Review pass 4 of spec 001 found the narrowing (R4-B4).

## Root cause
Review pass 1's repair rewrote the criterion to state the as-built behaviour and name the cause on stderr, and in doing so replaced "missing or unusable model" with "a directory that is not a usable model"; passes 2 and 3 compared each repair with the text it replaced, which by then was already narrowed, never with the text before review.
The load tested only whether a path is a directory or a file (`is_dir`, `is_file`) and reported every "no" as missing, so a path that exists as the wrong kind, or cannot be looked at, got the cause of one that does not exist.

## Rule
Compare every repair's criterion diff against the criterion's text before the first review pass (`git show <pass-1 commit>^:…/spec.md`), not only against the text it replaces; a repair may add precision, but a case the pre-review text covered stays covered unless the user decides otherwise.
The load looks at a path's metadata once and gives each outcome its own cause: `MissingFile` (does not exist), `NotADirectory` (the model path is not a directory), `NotAFile` (a model file is not a regular file), `Read` (the metadata cannot be read) (`kind_of` in src/onnx.rs).
Tested by `a_missing_or_non_directory_model_path_names_its_cause` (tests/cli.rs, a missing path and a regular file, the exact stderr line; seen saying "does not exist" for the file first), `a_model_path_that_is_a_file_is_not_a_directory` and `a_model_file_that_is_a_directory_is_not_a_file` (tests/onnx.rs).
