# NLP datasets (NLP-005)

**Status:** `REVIEW` · **Date:** 2026-09-14 · **Owner:** Swarm-E  
**Not a quality gate.** Tiny authored corpora cannot substantiate statistical claims. No F1, no Stanford/Stanza/spaCy comparison, no download.

Canonical layout under this directory. Gold JSON is the annotation source of truth. Sibling `documents/*.txt` bytes must equal each gold `source` string.

## Annotation guidelines (v0)

Single-author (this package). Dual-annotator agreement is **not** available and is recorded as a limitation. Clinical and legal judgments are **blocked** (none in v0).

| Layer | Include | Do not include |
|---|---|---|
| Gazetteer | A `DEFAULT_LEXICON` hit whose source slice equals the gold `surface` | Made-up sites, folded-case mismatches, hypothetical IRIs |
| Dates | Gregorian-valid ISO `YYYY-MM-DD` as `normalize_dates_and_numbers` emits | Invalid calendar dates (`2026-02-31`) |
| Quantities | Number plus a `KNOWN_UNITS` unit; span includes the unit text | Units not on the static list |
| Tokens / sentences | Current `tokenize` / `split_sentences` (honest, including known `Dr.` split) | Desired linguistic behaviour that the splitter does not yet implement |

Offsets are **UTF-8 byte** half-open intervals on the original string. Every gold span must satisfy `source.is_char_boundary(start/end)` and `source[start..end] == surface`.

Held-out **test** documents are listed in `splits/test.txt`. **Split membership** is the `*.txt` id lists (frozen). `gold_sha256` / `split_hashes` record the current annotation revision and **may change** when gold is re-authored (for example after NLP-102). Changing test-set membership is a different event from re-annotating a held-out document.

---

## `qualia-catchment-notes-v0`

| §9.2 field | Record |
|---|---|
| Canonical source | In-repo path `docs/benchmark-datasets/nlp/qualia-catchment-notes-v0/` (documents + gold JSON). No URL retrieval. |
| Retrieval date | 2026-09-14 (authored; not downloaded) |
| License | Project copyright / in-repo. See [`LICENSES.md`](LICENSES.md). |
| Redistribution | **No** third-party text. Do not redistribute as if it were UD/OntoNotes/GUM. Internal project use only. |
| Attribution | Independently authored for Qualia UC-A. Principal name appears only as already present in `DEFAULT_LEXICON`. |
| Checksum | SHA-256 of each gold JSON file (PowerShell `Get-FileHash -Algorithm SHA256`). See table below. |
| Language | `en` (English; Australian English unit/date examples) |
| Domain | Catchment / rainfall measurement notes (`catchment-measurement-notes`) |
| Annotation scheme | `symbolic-lite-spans-v0`: gazetteer surfaces + IRI, ISO dates, quantities+units, tokens, sentences, exact UTF-8 spans |
| Known limitations | Four documents. Not a NER corpus. Independent sites (`East Ridge`, `South Bend`, `West Pond`) are **not** gazetteer hits. Tokenizer currently splits ISO dates into number/punct pieces. Tiny N cannot support a statistical claim. Single annotator. |
| Immutable splits | `splits/train.txt`, `splits/dev.txt`, `splits/test.txt` and `splits/manifest.json`. Seed `42` / `frozen-u64-42`. |
| Train split hash | `8c46f225cb7d1242fc7bff341c2667297207a2c67a8ee4d66071911ca98ab378` |
| Dev split hash | `8d2afa0f356ce9476c6d89d6d89e9cc1f9e4f1bafeb4d92a2d6ff605a77f2ca8` |
| Test split hash | `ad2de8cec5c1efad0384acea3e2b8888a50c3a3bda083079b02b2961fcc18736` |
| Split-hash algorithm | SHA-256 of UTF-8 lines `{doc_id} {gold_sha256}\n` in listed order |
| Allowed use | **bundled** (CI offline; `network: forbidden`) |
| PII / sensitivity | **Restricted** at corpus level because `north-spring-fixture-001` contains the principal name `Timothy Charles Holborn` / DID already in the default lexicon. `east-ridge-weir-001` and `south-bend-gauge-001` are Public per-document (no principal name). Restricted notes must not appear in telemetry or receipts. |
| Retention / deletion | Retained in this repository for the life of the Qualia NLP programme. Delete only by a dated principal decision. No external copies are authorised by this package. |
| Preprocessing | None. UTF-8, no BOM, no trailing newline, LF only. Text was authored, not modified from an upstream dump. |

**Documents**

| Split | `doc_id` | Sensitivity | Role |
|---|---|---|---|
| train | `north-spring-fixture-001` | Restricted | Charter UC-A fixture: North Spring / 12.5 mm / 2026-08-15 |
| train | `west-pond-invalid-date-001` | Public | `2026-02-31` is **not** a date (invalid Gregorian; falls through) |
| dev | `east-ridge-weir-001` | Public | Independent site + `°C` / `km` |
| test | `south-bend-gauge-001` | Public | Held-out independent site + `m/s` |

---

## `qualia-english-notes-v0`

| §9.2 field | Record |
|---|---|
| Canonical source | In-repo path `docs/benchmark-datasets/nlp/qualia-english-notes-v0/` |
| Retrieval date | 2026-09-14 (authored; not downloaded) |
| License | Project copyright / in-repo. See [`LICENSES.md`](LICENSES.md). |
| Redistribution | **No** third-party text. |
| Attribution | Independently authored for Qualia UC-B. |
| Checksum | SHA-256 of each gold JSON file. See table below. |
| Language | `en` |
| Domain | English general notes (`english-general-notes`) |
| Annotation scheme | `symbolic-lite-spans-v0`: token and sentence gold; decimals, abbreviations, curly quotes, Unicode enders |
| Known limitations | Eight documents. `Dr.` and `e.g.` **do** split under the current splitter (honest gold; NLP-102). U+2026 `…` does **not** end a sentence. Zero gazetteer hits is success (do not score catchment IRI recall). Single annotator. Not a UD treebank. |
| Immutable splits | `splits/train.txt`, `splits/dev.txt`, `splits/test.txt` and `splits/manifest.json`. Seed `42` / `frozen-u64-42`. |
| Train split hash | `7fed513098b0b5dcfd21a0ca92a5f5517dccbdfd6678b3f96b23f023749a5563` |
| Dev split hash | `1b6f6945d25cb4aa51c787758461537a39f432e437e16c7a11355a1154c89ddb` |
| Test split hash | `020c1caadff0ccbbba230b52e4547070c71f64354b19d8f51527560240ed0f0b` |
| Split-hash algorithm | Same as catchment (SHA-256 of `{doc_id} {gold_sha256}\n` lines) |
| Allowed use | **bundled** |
| PII / sensitivity | **Public**. No principal DID, no sanctuary content. `Smith` is a generic surname not in `DEFAULT_LEXICON`. |
| Retention / deletion | Same as catchment: in-repo for programme life; delete only by principal decision. |
| Preprocessing | None. UTF-8, no BOM, no trailing newline, LF only. |

**Documents**

| Split | `doc_id` | Phenomena |
|---|---|---|
| train | `notes-plain-en-001` | Two ASCII sentences, no domain lexicon |
| train | `notes-eg-en-001` | `e.g.` currently **does** split into `See e.` / `g.` / `later notes.` (honest gold; NLP-102) |
| train | `notes-punct-en-001` | ASCII `?` and `!` end sentences |
| train | `notes-fullwidth-q-en-001` | Fullwidth `？` (U+FF1F) ends a sentence |
| dev | `notes-decimal-en-001` | `12.5 mm.` must **not** split on the decimal point |
| dev | `notes-newline-en-001` | ASCII newline ends a sentence |
| test | `notes-abbrev-en-001` | `Dr.` currently **does** split (honest gold) |
| test | `notes-unicode-en-001` | Curly quotes; `你好。世界！`; U+2026 does not end |

---

## Gold file checksums (SHA-256)

Computed with PowerShell `Get-FileHash -Algorithm SHA256` on the gold JSON bytes. Also listed in [`checksums.sha256`](checksums.sha256).

| File | SHA-256 |
|---|---|
| `qualia-catchment-notes-v0/gold/north-spring-fixture-001.json` | `577886c3817728ac426fbf216c8bc1dd0a693501addff8259de85363526473a7` |
| `qualia-catchment-notes-v0/gold/east-ridge-weir-001.json` | `7f7dc8ddc8c4d0d8e41ae17c44f4c39e8d4918283e861a2a878d80c306497814` |
| `qualia-catchment-notes-v0/gold/south-bend-gauge-001.json` | `4f687d49834035e71b8f5eb12a45fff3b7f04bff1df832cf51cadb7039ec5edc` |
| `qualia-catchment-notes-v0/gold/west-pond-invalid-date-001.json` | `13f4b028726a938cdf21765c5c77617e52c0a846a6bb1d858bc059f94094e670` |
| `qualia-english-notes-v0/gold/notes-plain-en-001.json` | `bc6cab10463eb6a4898e0935b1d269183cb2c17957cfb49f77e92d7e06088dad` |
| `qualia-english-notes-v0/gold/notes-decimal-en-001.json` | `02575dac3d88dfc6148e5ba0adc6fadb21dd4fce626e6bf2a13d1c0d773726f5` |
| `qualia-english-notes-v0/gold/notes-abbrev-en-001.json` | `2f47cbf356d0ea94deb40251b28799c2b574379c1c314b44176c1e41a76360f0` |
| `qualia-english-notes-v0/gold/notes-unicode-en-001.json` | `4e317e463f5e8dc1c70fde122fde9e44e31d43c18f61509f3119404eef8878fe` |
| `qualia-english-notes-v0/gold/notes-eg-en-001.json` | `4edd2d860e26578c97eddd64b92347ee467dfdd23cdf38a54ee2121284639def` |
| `qualia-english-notes-v0/gold/notes-punct-en-001.json` | `28a229da9f92805cca58897a325233ba6975d789ed7ff9727a09a0f08c26eb34` |
| `qualia-english-notes-v0/gold/notes-fullwidth-q-en-001.json` | `48b8f12aca198ececfde35aad391242ba4b3e0d0358524429221d48d3b8a66a7` |
| `qualia-english-notes-v0/gold/notes-newline-en-001.json` | `67a0661afe899a629b7f3649084ec4190b7ef4315cf901c2077c0b5aafe643fd` |

---

## Unavailable (not present, not downloaded)

These rows stay **UNEVALUATED**. NLP-006 must not mark their `release_required` gates passing. See [`LICENSES.md`](LICENSES.md).

| Dataset | Status | Reason |
|---|---|---|
| Universal Dependencies English EWT | **unavailable** | Not in this checkout; separately licensed; not downloaded |
| Universal Dependencies English GUM | **unavailable** | Not present; not downloaded |
| OntoNotes (coref / NER) | **unavailable** | License required; not present; not downloaded |
| CorefUD / GUM coref | **unavailable** | Not present; not downloaded |
| FrameNet | **unavailable** | Not present; not downloaded |
| PropBank | **unavailable** | Not present; not downloaded |
| Licensed temporal / geo sets | **unavailable** | Not present; not downloaded |
| Stanza / spaCy / CoreNLP model packages | **unavailable** | Weights not present; download forbidden |

Optional UC-C keyword-triple smoke (`qualia-keyword-triples-v0`) was **not** authored in this package.
