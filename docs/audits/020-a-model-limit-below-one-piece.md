# Audit 020 — a model whose limit is below one piece passed the load

**Severity:** low
**Tags:** model, config, spec, review

## Symptom
With `max_position_embeddings` = `pad_token_id` + 2 or + 3, the model loaded, and every sentence was reported too long for the model with `max 0`, although M3b criterion 1 states the limit as `max_position_embeddings − pad_token_id − 4`, which is then negative.
Review pass 3 of spec 001 found it (R3-B3).

## Root cause
Definitions *Model* and `limits_from_config` required `max_position_embeddings` > `pad_token_id` + 1: room for one position, not for a row, which also holds `<s>`, `</s>` and the repeated piece (`ROW_EXTRA` = 3).
`ModelLimits::max_pieces` then hid the gap with `saturating_sub`, turning an impossible limit into 0 instead of refusing the model; the bound was never checked against the formula it feeds.

## Rule
A bound read from a model's configuration is checked against every formula that uses it: `limits_from_config` accepts only `max_position_embeddings` > `pad_token_id` + 4 (a limit of at least 1 piece), else the model is unusable (M3b criterion 6, exit 2).
A `saturating_sub` on a value derived from input is a sign of an unchecked precondition: validate at the boundary instead.
Tested by `a_limit_below_one_piece_is_refused` (src/decoder.rs, the law over positions and pad ids) and `a_model_without_room_for_a_piece_is_named_and_exits_2` (tests/cli.rs), both seen failing first.
