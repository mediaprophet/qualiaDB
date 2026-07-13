# acquisition-inbox/ — review intake for newly fetched instruments

**This folder is a staging area for human review — NOT the curated corpus.**

A fetch agent (see [`../ACQUISITION_INSTRUCTIONS_GEMINI.md`](../ACQUISITION_INSTRUCTIONS_GEMINI.md))
drops each acquired instrument here as a separate file, with its provenance. Timothy reviews,
then the curated result is promoted into `../un-instruments/` (the real corpus). Nothing here is
trusted, queried, or built until a human has reviewed and promoted it.

- One file per instrument: `<slug>.md` (kebab-case of the official title).
- Each file carries: the **verbatim** source text, `sourceURL`, `retrievalDate`,
  `sourceContentHash`, official title/date, legal form, and proposed tier.
- A run produces `MANIFEST.md` here listing everything fetched (+ anything that failed, with the
  reason — never a fabricated stand-in).
- The fetch agent **must not** write to `../un-instruments/`, `../*.n3`, or any curated file.
