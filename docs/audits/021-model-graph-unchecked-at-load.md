# Audit 021 — a model whose graph did not match its config.json passed the load

**Severity:** medium
**Tags:** model, onnx, exit-code, spec, review

## Symptom
A copy of the tested model whose `config.json` kept four labels loaded, and then every file failed at its first sentence: `cannot parse the sentence at s.md:1:1: the model's output is unusable: logits of shape [3, 6, 2561], expected [3, 6, 4]`, once per file, exit 2.
The model directory was never named, and M3b criterion 6 ("name the model directory and the cause, process no file") was broken for a model that is not usable.
Review pass 3 of spec 001 found it (R3-B2).

## Root cause
Definitions *Model* listed the three files and what `config.json` and `tokenizer.json` must hold, but nothing about the graph, so the load checked the files it could parse without ONNX Runtime and trusted the graph.
The graph's contract (the inputs `input_ids` and `attention_mask`, an output `logits` with one value per label) lived only in `run_rows`, which checks it on every parse, long after the point where M3b criterion 6 promises to refuse the model.

## Rule
Everything a parse relies on about the model is part of Definitions *Model* and is checked at load, from the files or the session's metadata; a parse-time check is a backstop for what the metadata cannot state (a dynamic dimension), never the only check.
`OnnxParser::load` checks that the graph's inputs are exactly `input_ids` and `attention_mask`, that it has a tensor output `logits`, and that the last dimension of `logits`, where the graph states it, equals the label count (`check_graph`, `LoadError::Graph`).
Tested by `a_graph_that_breaks_the_contract_is_refused` (src/onnx.rs, every reason), `a_model_whose_labels_do_not_match_its_logits_is_refused` (tests/onnx.rs, seen loading first) and `a_model_whose_labels_do_not_match_its_graph_processes_no_file` (tests/model.rs), the last two on a four-label copy of `VERNIER_TEST_MODEL`.
