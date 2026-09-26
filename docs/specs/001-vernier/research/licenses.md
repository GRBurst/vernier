# License research — UD parsing & CMUdict for `vernier`

All sources accessed **2026-09-26** (UTC) via direct HTTP fetch of the primary source. Quotes are verbatim.
Not legal advice. Items marked **[interpretation]** are my reading, not a quoted fact.

## TL;DR

| Artifact | License | Commercial? | UD labels? | Runs from Rust? |
|---|---|---|---|---|
| UDPipe 1 models UD 2.5 (`english-ewt-ud-2.5-191206.udpipe`) | CC BY-NC-SA 4.0 | **No** | yes | yes (UDPipe 1 C++ via FFI) |
| UDPipe 2 models UD 2.12/2.15/2.17 | CC BY-NC-SA 4.0 | **No** | yes | **no** (TF checkpoints + Python; REST only in practice) |
| UDPipe 1 software | MPL-2.0 | yes | – | – |
| `udpipe-rs` crate 0.2.0 | MIT OR Apache-2.0 (bundles MPL-2.0 UDPipe C++) | yes | – | yes |
| UD_English-EWT | CC BY-SA 4.0 | yes (ShareAlike) | yes | – |
| UD_English-GUM / LinES / ParTUT | CC BY-NC-SA 4.0 | **No** | yes | – |
| Stanza code / HF `stanza-en` models | Apache-2.0 (declared), default `combined` trained incl. GUM (NC) | declared yes; provenance caveat | yes | no official ONNX/Rust path |
| Trankit | Apache-2.0 code; model table lists treebank license (EWT = CC BY-SA 4.0) | see treebank | yes | no official ONNX/Rust path |
| spaCy `en_core_web_sm` 3.8.0 | MIT | yes | **no** (ClearNLP labels) | no |
| `KoichiYasuoka/roberta-base-english-ud-goeswith` (+ ONNX export by `ghotriw`) | MIT (declared), trained on EWT+GUM+ParTUT+LinES+Atis | declared yes; provenance caveat | **yes** (full UD incl. `acl:relcl`, `aux:pass`) | **yes** via `ort` (ONNX file exists) + port of `ud.py` decoding |
| CMUdict | BSD-2-Clause-style + "completely unrestricted" | yes | – | – |

---

## 1. UDPipe 1 English models (UD 2.5, `udpipe-ud-2.5-191206`)

**Verdict:** CC BY-NC-SA 4.0. **Commercial use not allowed.** Redistribution is allowed only under the same license, non-commercially, with attribution. Having the tool fetch the file from the original LINDAT URL at the user's request is not redistribution by vernier **[interpretation]**, but the user is then bound by NC-SA, so vernier must not present it as "free for any use" and should show the license before downloading.

- LINDAT item page (handle 11234/1-3131), which lists `english-ewt-ud-2.5-191206.udpipe`, `english-gum-…`, `english-lines-…`, `english-partut-…`:
  > "This item is Publicly Available and licensed under: Creative Commons - Attribution-NonCommercial-ShareAlike 4.0 International (CC BY-NC-SA 4.0)"
  URL: https://lindat.mff.cuni.cz/repository/xmlui/handle/11234/1-3131
- UDPipe 1 model docs:
  > "Universal Dependencies 2.5 Models are distributed under the CC BY-NC-SA licence. The models are based solely on Universal Dependencies 2.5 treebanks. … The latest version is 191206."
  URL: https://ufal.mff.cuni.cz/udpipe/1/models
- UDPipe README:
  > "the linguistic models are free for non-commercial use and distributed under the CC BY-NC-SA (http://creativecommons.org/licenses/by-nc-sa/4.0/) license, although for some models the original data used to create the model may impose additional licensing conditions."
  URL: https://raw.githubusercontent.com/ufal/udpipe/master/README
- CC BY-NC-SA 4.0 legal code, definition:
  > "NonCommercial means not primarily intended for or directed towards commercial advantage or monetary compensation."
  URL: https://creativecommons.org/licenses/by-nc-sa/4.0/legalcode.en
- Note: the EWT model is NC even though EWT itself is CC BY-SA. That is ÚFAL's choice of license for its models.
- Note: `udpipe-rs` (see §3) already has a `download` feature that fetches from `https://lindat.mff.cuni.cz/repository/xmlui/bitstream/handle/11234/1-3131` (src line 31 of crate 0.2.0). That is prior art for fetching from the original URL.

## 2. UDPipe 2 models (UD 2.12 / 2.15 / 2.17)

**Verdict:** CC BY-NC-SA 4.0 (no commercial use). **Not runnable from Rust in practice.** They are TensorFlow checkpoints that need the Python UDPipe 2 server, mBERT/RobeCzech embeddings, and UDPipe 1 for tokenization. The supported ways to use them are the LINDAT REST service or a self-hosted Python REST server. There is no official ONNX export, and I found no Rust path.

- > "Universal Dependencies 2.15 Models are distributed under the CC BY-NC-SA licence. The models are based solely on Universal Dependencies 2.15 treebanks, and additionally use multilingual BERT and RobeCzech . The models require UDPipe 2 ." (the 2.12 and 2.17 sections use the same wording)
  URL: https://ufal.mff.cuni.cz/udpipe/2/models
- LINDAT 2.15 item: "licensed under: Creative Commons - Attribution-NonCommercial-ShareAlike 4.0 International (CC BY-NC-SA 4.0)". The file is `udpipe2-ud-2.15-241121.tar.gz` (8.53 GB). The preview shows TF checkpoint files `options.json`, `weights.index`, `weights.data-00000-of-00001`.
  URL: https://lindat.mff.cuni.cz/repository/xmlui/handle/11234/1-5797
- UDPipe 2 README (branch `udpipe-2`):
  > "UDPipe 2 is Python-only and tested only in Linux, … is meant as a research tool, not as a user-friendly UDPipe 1 replacement, … requires a GPU for reasonable performance, … does not perform tokenization by itself – it uses UDPipe 1 for that."
  > "Then you can run UDPipe 2 as a local REST server"
  URL: https://raw.githubusercontent.com/ufal/udpipe/udpipe-2/README.md
- requirements.txt: `tensorflow_gpu~=1.15.4 # Note that for just inference, you can use TF 2.* too.`, `ufal.chu_liu_edmonds`, `ufal.udpipe>=1.3,<2`
  URL: https://raw.githubusercontent.com/ufal/udpipe/udpipe-2/requirements.txt
- The REST service page repeats the same "free for non-commercial use … CC BY-NC-SA" text, so calling the REST API does not avoid NC. URL: https://lindat.mff.cuni.cz/services/udpipe/

## 3. UDPipe 1 software + Rust bindings

**Verdict:** UDPipe 1 is **MPL-2.0**, and the repo is active (GitHub `pushed_at` 2026-06-11). One Rust crate exists: **`udpipe-rs`** (latest 0.2.0, 2026-01-25). It is licensed `MIT OR Apache-2.0`, **bundles the UDPipe C++ sources** (MPL-2.0) and compiles them with `cc` in `build.rs`. Its GitHub repo was pushed 2026-09-21 (5 stars, 9 open issues, not archived), and `Cargo.toml` on HEAD already says `version = "1.0.0"` (unreleased). `udpipe-sys`, `udpipe` and `ufal-udpipe` **do not exist** on crates.io ("crate `udpipe-sys` does not exist"). The crates.io search for "udpipe" also returns `corpipe-rs` (unrelated, coreference).

- UDPipe README: "UDPipe is a free software distributed under the Mozilla Public License 2.0 (http://www.mozilla.org/MPL/2.0/)"; LICENSE file starts "Mozilla Public License Version 2.0"; GitHub API `license.spdx_id = "MPL-2.0"`. URL: https://github.com/ufal/udpipe
- crates.io API: `udpipe-rs` versions 0.1.0 to 0.2.0, all with `"license":"MIT OR Apache-2.0"`. Crate size jumps from 24 KB (0.1.3) to about 1 MB (0.1.4+), which is when the vendored sources were added. URL: https://crates.io/api/v1/crates/udpipe-rs
- Contents of the published crate 0.2.0 (downloaded and listed): 324 files under `vendor/udpipe/src/`, plus `vendor/udpipe/LICENSE` (MPL-2.0) and `vendor/udpipe/releases/LICENSE.CC-BY-NC-SA-4`. `.gitmodules`: `url = https://github.com/ufal/udpipe.git`.
- udpipe-rs README: "This crate is dual-licensed under MIT OR Apache-2.0. UDPipe itself is licensed under the [Mozilla Public License 2.0]". URL: https://github.com/ccostello97/udpipe-rs
- **[interpretation]** MPL-2.0 is file-level copyleft. Linking it into an MIT/Apache binary is fine. Obligations: keep the MPL notices, and make the source of MPL files (plus any changes to them) available. The crate's own SPDX string leaves out the MPL-2.0 part, so `cargo-deny` will not see it. vernier's license inventory has to add MPL-2.0 by hand. The crate is also FFI, so it involves `unsafe`, which touches vernier's D9 (ADR needed).

## 4. English UD treebanks (does a self-trained model inherit?)

**Verdict:** **EWT = CC BY-SA 4.0** (commercial OK, ShareAlike). **GUM = CC BY-NC-SA 4.0** (no commercial use), and so are LinES and ParTUT. Atis = CC BY-SA 4.0. A model trained **only on EWT** would carry at most a BY-SA obligation. Any model that includes GUM, LinES or ParTUT would carry NC. Whether a trained model counts as "Adapted Material" under CC 4.0 is **legally unsettled** **[interpretation]**. The CC definition covers material "derived from or based upon the Licensed Material … in a manner requiring permission under the Copyright and Similar Rights", so the question is whether model weights need that permission.

- EWT README:
  > "The annotations and database rights of the Universal Dependencies English Web Treebank are licensed under a Creative Commons Attribution-ShareAlike 4.0 International License."  …  "License: CC BY-SA 4.0"
  URL: https://github.com/UniversalDependencies/UD_English-EWT/blob/master/README.md
- GUM LICENSE.txt:
  > "The treebank is licensed under the Creative Commons License Attribution-NonCommercial-ShareAlike 4.0 International." … "wikiHow texts are made available under a CC-BY-NC-SA license (non-commercial, share alike), as are the fiction texts, meaning that commercial and/or non-open source use of those texts is prohibited."
  README: "License: CC BY-NC-SA 4.0". URL: https://github.com/UniversalDependencies/UD_English-GUM
- LinES README.txt: "License: CC BY-NC-SA 4.0"; ParTUT README.md: "License: CC BY-NC-SA 4.0"; Atis README.md: "License: CC BY-SA 4.0" (github.com/UniversalDependencies/UD_English-{LinES,ParTUT,Atis}).
- These match UDPipe's UD 2.5 training table (`training/ud-2.5/langs_sizes_licenses`): `en_ewt … License: CC BY-SA 4.0`, `en_gum … CC BY-NC-SA 4.0`, `en_lines … CC BY-NC-SA 4.0`, `en_partut … CC BY-NC-SA 4.0`.

## 5. Alternatives with UD labels

**Stanza.** Code is Apache-2.0: "Stanza is released under the Apache License, Version 2.0." (https://github.com/stanfordnlp/stanza README). The HF model repo `stanfordnlp/stanza-en` declares `license: apache-2.0` (https://huggingface.co/stanfordnlp/stanza-en). Labels are UD. **Caveat:** the default English package is `"en": "combined"` (`stanza/resources/default_packages.py`), and its training data is:
> "en_combined is currently EWT, GUM, PUD, Pronouns, and handparsed" … `train_treebanks = ["UD_English-EWT", "UD_English-GUM", "UD_English-GUMReddit"]`
(https://github.com/stanfordnlp/stanza/blob/main/stanza/utils/datasets/prepare_tokenizer_treebank.py). So an Apache label sits on top of NC training data. The EWT-only package (`ewt`) avoids this. Stanza runs on PyTorch in Python. I found no official ONNX export or Rust path.

**Trankit.** Code is Apache-2.0 (GitHub API; LICENSE "Apache License Version 2.0"; last push 2025-07-22). Models are trained on UD v2.5 with XLM-RoBERTa. Trankit publishes no separate model license. Instead, its docs list a "Treebank License" column per pipeline, and the default English pipeline is UD_English-EWT (CC BY-SA 4.0): "to initialize a pipeline that is trained on the default treebank UD_English-EWT , we can use the code name english". URL: https://trankit.readthedocs.io/en/latest/pkgnames.html. Labels are UD. It runs on Python/PyTorch, with no official ONNX.

**spaCy.** `en_core_web_sm` 3.8.0 meta: `"license":"MIT"`, sources `"OntoNotes 5","ClearNLP Constituent-to-Dependency Conversion","WordNet 3.0"`. Parser labels include `dobj, pobj, nsubjpass, auxpass, relcl, npadvmod, attr, prep` (https://raw.githubusercontent.com/explosion/spacy-models/master/meta/en_core_web_sm-3.8.0.json). **Confirmed: not UD** (ClearNLP style: `nsubjpass` rather than `nsubj:pass`, `auxpass` rather than `aux:pass`, `relcl` rather than `acl:relcl`). A UD-trained spaCy pipeline does exist, `explosion/en_udv25_englishewt_trf`, but it is `license: cc-by-sa-4.0` and runs in Python.

**ONNX-exported UD parser (usable from Rust via `ort`).**
- `KoichiYasuoka/roberta-base-english-ud-goeswith`: `license: "mit"`, base `FacebookAI/roberta-base`. The labels in config `id2label` are full UD: `acl acl:relcl … aux aux:pass … nsubj nsubj:pass … obl:agent … root …`. **Caveat:** `maker.py` trains on `["UD_English-EWT","UD_English-GUM","UD_English-ParTUT","UD_English-Lines","UD_English-Atis"]`, so 3 of the 5 are NC-SA. URL: https://huggingface.co/KoichiYasuoka/roberta-base-english-ud-goeswith
- `ghotriw/roberta-base-english-ud-goeswith-onnx`: "This is an ONNX export of the KoichiYasuoka/roberta-base-english-ud-goeswith model", `license: mit`, contains `onnx/model.onnx`. It inherits the same provenance caveat. The decoding (the `ud.py` goeswith/head selection) would have to be ported to Rust. URL: https://huggingface.co/ghotriw/roberta-base-english-ud-goeswith-onnx
- `KoichiYasuoka/modernbert-{base,large}-english-ud-{triangular,embeds,square}`: `license: "apache-2.0"`, but `maker.py` again trains on `["EWT","GUM","Atis","ParTUT","LinES"]`. They are PyTorch only (no ONNX file).
- **I found no permissively licensed UD English parser trained only on non-NC data and shipped as ONNX.** The cleanest route: train or fine-tune your own on EWT (+Atis) only and export to ONNX. The result carries at most a CC BY-SA question, not NC.

## 6. CMUdict

**Verdict:** **BSD-2-Clause-style license, plus an explicit statement that research or commercial use is "completely unrestricted".** Committing a 500-word sample with syllable counts to an MIT/Apache-2.0 repo is allowed. Put the CMU copyright notice, the two conditions and the disclaimer next to the sample (e.g. in a header or a `LICENSE-CMUDICT` file), and acknowledge the origin.

- LICENSE (github.com/cmusphinx/cmudict):
  > "Copyright (C) 1993-2015 Carnegie Mellon University. All rights reserved. Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met: 1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer. The contents of this file are deemed to be source code. 2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution." (followed by the standard "AS IS" disclaimer)
  URL: https://github.com/cmusphinx/cmudict/blob/master/LICENSE
- README:
  > "Use of this dictionary for any research or commercial purpose is completely unrestricted. If you make use of or redistribute this material we request that you acknowledge its origin in your descriptions."
  URL: https://github.com/cmusphinx/cmudict/blob/master/README
- GitHub's detector reports `NOASSERTION` because the text is a modified BSD-2 that is not verbatim. Syllable counts come from counting the stress digits (0/1/2) on vowel phones in `cmudict.dict`. That is a derived value, and the notice still applies to the word/pronunciation list **[interpretation]**.
