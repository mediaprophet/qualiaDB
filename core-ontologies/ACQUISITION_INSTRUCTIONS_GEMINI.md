# Acquisition brief — for the fetch agent (Gemini)

**Your job:** acquire the human-rights / legal instruments listed in
[`ACQUISITION_BACKLOG.md`](ACQUISITION_BACKLOG.md) and deposit each as a separate, provenance-stamped
file in [`acquisition-inbox/`](acquisition-inbox/) **for human review**. You are *fetching and
recording source text*, not curating or reasoning over it. Timothy reviews the inbox and promotes
items into the real corpus (`un-instruments/`) himself.

---

## 1. Output — exactly where and how
- Write to **`core-ontologies/acquisition-inbox/`** only. **Never** write to `un-instruments/`,
  any `*.n3`, `PLAN.md`, or any existing/curated file.
- **One file per instrument:** `acquisition-inbox/<slug>.md`, where `<slug>` is the kebab-case of
  the official title (e.g. `convention-on-cluster-munitions-2008.md`).
- **Each file MUST contain, in this order:**
  1. A header block with: `title`, `officialDate`, `sourceURL`, `retrievalDate` (ISO-8601),
     `sourceContentHash` (SHA-256 of the verbatim text you saved), `proposedTier` (A or B — see §3),
     `legalForm` (Treaty / UNGAResolution / Recommendation / Statute / Constitutional / Declaration /
     JudicialReasoning), and `bindingStatus` (Binding / AffirmativeNonBinding).
  2. The **verbatim** instrument text — the full operative + preamble text as published by the
     authoritative source. Preserve article/section structure; do not summarise, paraphrase, or
     translate.
- Produce **`acquisition-inbox/MANIFEST.md`**: a table of every item attempted — slug, source URL,
  retrieval date, proposed tier, and **status** (`fetched` / `failed`). For `failed`, give the
  reason (404, paywalled, no stable source). **Never** invent a stand-in for a document you could
  not fetch.

## 2. Method (per the backlog legend)
- **ICRC** items → the ICRC Drupal **JSON:API** (`?include=field_treaty_content`) — deterministic,
  repeatable; preferred.
- **PD** (public-domain constitutional/historical) → fetch the canonical published text.
- **WEB** → only where there is no stable endpoint; accumulate the rendered text from the official
  page. Prefer deterministic fetchers over scraping wherever a real endpoint exists.
- Always record the exact `sourceURL` you actually used.

## 3. Rules — non-negotiable (these are the project's values)
- **No fabrication, ever.** Save only real, verbatim text from the authoritative source. If you
  cannot get it, mark it `failed` in the manifest — do not write a plausible-looking substitute.
- **Faithful `originalText`.** Do not "clean up" wording. **Strip only** scraper boilerplate
  ("Download: PDF", nav chrome, cookie banners) — never instrument content.
- **Dedupe.** Do **not** re-fetch anything already in `un-instruments/` (≈102 instruments). The
  backlog intro lists known-present items to skip (Statelessness 1954/1961, Refugee 1951/1967,
  Minorities 1992, Right to Development 1986, ILO 105/182, ILO-169, the Geneva Conventions + APs,
  the OHCHR core set). When unsure, check `un-instruments/` for the slug first.
- **Tier honestly (PLAN §1):** Tier A = treaties, UNGA resolutions, regional human-rights charters,
  constitutions + foundational/historical rights documents, AI/digital treaties. Tier B = domestic
  *legislation/statutes* that get amended (e.g. EU AI Act, AU Human Rights Acts) — flag these and do
  **not** mix them into the core set. Declarations/speeches → `bindingStatus: AffirmativeNonBinding`.
- **Watchlist (not yet in force):** for anything not yet adopted/in force (e.g. the BHR treaty), set
  `bindingStatus: AffirmativeNonBinding` + note `NOT YET IN FORCE` — never record it as binding core.
- **You propose; a human disposes.** This is the Curation Prime Directive: a fetch agent only
  *proposes* material for review. You do not assert equivalences, do not curate, do not modify the
  corpus, and you hold no authorship over it.

## 4. Suggested order (solo-capacity sustainable — don't fire-and-forget the whole list)
Per PLAN §7 / §8.4, batch by value × ease:
1. **Constitutional / historical / foundational** (`ACQUISITION_BACKLOG.md` §2.6) + **UNDRIP** (§2.7)
   — high value, public-domain, low friction.
2. **IHL weapons + disarmament** (§2.1, §2.2) — the proven ICRC/UN method.
3. **Regional charters + AI/digital** (§2.3, §2.4) and **business & human rights** (§2.4b).
4. **Environment / space / sea** (§2.5); **Australia** (§2.8, tiering carefully).

Do one batch, write the manifest, and stop for review before the next — so Timothy can check the
provenance and tiering before more is fetched.
