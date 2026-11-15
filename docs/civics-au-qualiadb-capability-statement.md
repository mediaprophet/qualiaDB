# civics.au — Capability Statement
### What the platform can do, what it cannot yet do, and where the evidence stands

*Prepared September 2026 · QualiaDB v0.0.40 · For review by policy-makers, economists, demographers, and social-science researchers*

---

## 1. What civics.au is

**civics.au** is a connected set of open proposals for Australian civics infrastructure. It addresses housing insecurity, community resilience, and digital agency through three interlocking layers:

| Layer | Purpose | Current status |
|---|---|---|
| **Physical infrastructure** | Community Grounds — upgraded regional showgrounds with amenities, solar microgrids, connectivity, and workshops | Concept design with detailed financial modelling |
| **Social infrastructure** | The Walkabout Strategy — a supported pathway for people in transition, using mobile housing (homes on wheels) hosted at community grounds | Concept with fit-out and budget planning tools |
| **Digital infrastructure** | Solid Databoxes, verifiable credentials, knowledge banks, and a planning/evidence environment | Open-source reference implementation; planning tools operational in-browser |

The site is a working document, not a deployed service. It is explicitly marked as *"Concept notes · Initial draft for discussion"*.

---

## 2. The digital engine behind the evidence tools

The planning and modelling tools on civics.au are powered by a specialised component of **QualiaDB** called the **WebCivics WASM profile** — a self-contained computational engine that runs entirely inside a standard web browser with no server dependency.

This is not a conventional database. It is a reasoning and computation engine built for:

- **Structured knowledge representation** using W3C standards (RDF, Turtle, SPARQL subsets, SHACL)
- **Formal logic and rules** — including deontic logic (obligations, permissions, prohibitions), temporal reasoning, and epistemic logic (what is known vs. believed)
- **Quantitative analysis** — economics, statistics, financial modelling, welfare metrics
- **Credential and governance plumbing** — verifiable credentials, access control, receipts, and audit trails built on W3C Solid

### What "runs in the browser" means in practice

Every calculation in the modelling lab, evidence views, and planning tools executes locally on the user's own device. No data is sent to a server. No cloud service is required. The engine downloads once (under 1 MB compressed for the WebCivics profile) and operates offline thereafter. This design is deliberate: it ensures that a community council, regional planner, or individual can run the numbers without surrendering data to a third party.

---

## 3. Capabilities relevant to policy and social-science evaluation

### 3.1 Demographic and area-level evidence

| Capability | Status | Detail |
|---|---|---|
| ABS local government area profiles | ✅ Operational | 547 LGAs selectable; population and SEIFA context pre-loaded |
| ABS data integration | ✅ Operational | Draws on published ABS releases for housing, employment, education, and socioeconomic indices |
| Individual-level data | ❌ Not present | The system works with area-level aggregates, never individual records |
| Real-time census data | ❌ Not present | Evidence is drawn from published snapshots, not live feeds |

> **Important limitation:** The evidence view shows area-level public context only. It is explicitly stated on the site that these figures are *"never used to decide whether an individual needs help, deserves a place, or is likely to use the ground."*

### 3.2 Financial and economic modelling

The modelling lab hosts an integrated ten-year financial model with 5,811 verified formulas. It includes:

| Model | What it does | What it does not do |
|---|---|---|
| **Financial model** | Ten-year integrated forecast for community-scale infrastructure — assumptions, scenario drivers, a three-tier portfolio, consolidated forecast and valuation. Every figure recomputes live when an input changes. | Does not constitute financial advice. Status: *"CALCS PASS; PRELIMINARY DRAFT / EVIDENCE GAPS"*. Many inputs are placeholders awaiting market quotes and independent review. |
| **Solar & battery** | Daily energy balance — PV generation, battery storage/delivery, load coverage, surplus. Uses BOM solar exposure data by location. | Engineering first-pass estimator, not a system design. Does not model hourly dispatch, site shading, or tariff structures. |
| **Water & thermal** | Daily energy demand for pumping, RO water production, and heating — with optional solar hot-water and sand-battery thermal store. | Uses standard specific-energy figures. Scenario switches, not product selections. |
| **Site planner** | Sizes a real ground — loads, roof segments, storage, EV, microgrid, datacentre, or greenfield build. Outputs feed all other models. | Does not re-site infrastructure, re-quote vendor costs, or validate legal structures for different jurisdictions. |

### 3.3 Computational economics engine

The underlying engine contains a substantial library of native computational economics capabilities. These are not imported from external packages — they are built into the engine itself and run in the browser. Relevant capabilities include:

**Welfare economics:**
- Gini coefficient, Atkinson index, Theil entropy
- Rawlsian and utilitarian social welfare functions
- Distributional weights for cost-benefit analysis
- Survival-floor allocation model with rights-affecting safeguards
- Poverty headcount, gap, severity (FGT family)

**Public finance:**
- Progressive taxation (marginal bracket computation)
- Transfer payment modelling
- Fiscal multipliers and Laffer curve analysis
- Survival-floor allocation budgeting

**Spatial economics:**
- Gravity models (inter-location flow analysis)
- Transport cost modelling
- Location-allocation problems
- Spatial autocorrelation (Moran's I)

**Labour and household economics:**
- Labour supply (Cobb-Douglas labour-leisure tradeoff)
- Household production (CES aggregator)
- Human capital modelling

**Econometrics:**
- Ordinary Least Squares (OLS) and Weighted Least Squares (WLS)
- Two-Stage Least Squares (2SLS) instrumental variables
- Logistic regression (maximum likelihood, IRLS)
- Generalised Method of Moments (GMM)
- Calibration records with provenance

**Input-output economics:**
- Leontief inverse and multipliers
- Ghosh supply-side model
- Demand/supply shock propagation

**Further statistical capabilities:**
- Descriptive statistics (mean, variance, covariance, quantiles)
- Hypothesis testing (Mann-Whitney U, Kolmogorov-Smirnov, McNemar, Friedman)
- Time-series analysis (Ljung-Box, ADF proxy)
- Distributions (binomial, Poisson, lognormal, exponential, uniform, Laplace, gamma, beta, Weibull, empirical)
- Bootstrap resampling (basic; block/jackknife still in progress)
- Monte Carlo VaR/CVaR

**Dynamic programming and Markov models:**
- Value function iteration, policy iteration, Bellman equation
- Optimal stopping for finite Markov Decision Processes
- Stationary distribution computation, mean first-passage time

**Environmental and resource economics:**
- Social cost of carbon / quadratic damage functions
- Pollution externality pricing
- Hotelling non-renewable resource extraction

**Game theory and mechanism design:**
- Nash equilibrium computation (finite games)
- Auction design (second-price, first-price, VCG)
- Mechanism design primitives

### 3.4 Governance and credential architecture

| Component | Status | Detail |
|---|---|---|
| **Concession credentials** | Concept architecture | Attribute-based W3C Verifiable Credentials that prove eligibility (e.g. concession status) without exposing identity, medical history, or personal circumstances. Zero-knowledge proofs answer boolean eligibility questions only. Offline operation via CBOR-LD QR/NFC cards. |
| **Solid Databox** | Open-source reference implementation | Organisation-run secure data vaults for governed exchange — records, receipts, credentials, submissions — built on W3C Solid. Each relationship is a separate security domain. Append-only evidence ledger. |
| **Knowledge banks** | Concept | Community cooperatives providing shared infrastructure for data vaults, backups, continuity, and probate under fiduciary duty to members. |
| **Deontic logic engine** | ✅ Implemented and tested | Formal reasoning about obligations, permissions, and prohibitions. Used for credential governance, contract terms, and guardianship workflows. |

> **Concession credential safeguards:** The architecture is explicitly stress-tested against digital redlining (verifiers receive boolean proofs only, never raw attributes), coercion (safe-exit and deadman-switch protocols), fraudulent issuance (governed trust registry), offline denial of service (carried QR/NFC credentials work without connectivity), and algorithmic failure (every automated decision logs its rule ID for appeal and audit).

---

## 4. What the engine explicitly cannot and does not do

This section is as important as the capabilities list.

| Limitation | Detail |
|---|---|
| **No individual-level profiling** | The system works with area-level public data. It does not score, rank, or assess individuals. |
| **No automated decisions about people** | Welfare and poverty metrics carry explicit safety classifications requiring human review. The code documentation states: *"No function in this module performs or recommends an external action on its own."* |
| **No live ABS or government data feeds** | Evidence is loaded from published snapshots, not streaming APIs. |
| **Model status is preliminary** | The financial model carries the status line *"CALCS PASS; PRELIMINARY DRAFT / EVIDENCE GAPS"*. It is a discussion tool, not a certified forecast. |
| **No deployed credential scheme** | The concession credential design is a concept architecture requiring governed trust anchors, issuer accreditation, and legal review before any real-world use. |
| **No cloud dependency** | Deliberate design choice. The system runs locally. This means no centralised analytics, no user tracking, and no third-party data exposure — but it also means no centralised updates or remote data enrichment. |
| **Limited statistical coverage gaps** | Standard errors for regression are not yet computed. Full block bootstrap and jackknife confidence intervals are still needed. Bayesian estimation is not yet available. |
| **Engineering models are estimators** | Solar, battery, water, and thermal calculations are first-pass energy balances — not engineering designs. They require professional verification before any procurement or construction decision. |

---

## 5. Provenance, auditability, and content integrity

A distinctive feature of the system is its approach to provenance:

- **Content hashing:** When the QualiaDB WASM runtime is available, every report entry is stamped with a BLAKE3 content hash (SHA-256 fallback otherwise). An exported result can be verified against exactly the inputs that produced it.
- **Calculation integrity:** All 5,811 formulas in the financial model are verified. Status flags and evidence-gap warnings are generated automatically and travel with the output.
- **Rights-affecting computations:** Welfare, poverty, and public-finance kernels return structured reports containing assumptions and diagnostics alongside scalar results. The report structure is designed so that downstream consumers (including deontic and SHACL validation layers) can audit the basis of a computed allocation.
- **Calibration records:** Econometric models produce provenance records including model name, data hash, parameter count, loss metric, iteration count, and timestamp — so the basis of a fitted model can be traced.

---

## 6. The WebCivics WASM profile — technical scope summary

For readers who wish to understand the boundary of what the engine includes and excludes:

**Included in the WebCivics profile:**
- Structured data parsing (N3, Turtle, RDF)
- Query compilation and execution (SPARQL subset, N-Triples query)
- Schema validation (SHACL property shapes)
- Modal logic suite (deontic, epistemic, paraconsistent, linear temporal, description logic, answer-set programming, linear logic)
- Interaction governance engine
- Conflict-free replicated data types (LWW CRDT)
- Semantic data ingest (YAML-LD-Q42, HCF)
- Community modelling layer (CML graph model)
- Solid protocol record support

**Excluded from the WebCivics profile (available in other QualiaDB builds):**
- GPU-accelerated rendering and spatial visualisation (Portal profile)
- Large language model inference (LLM profile)
- Native daemon services and filesystem access
- Bioinformatics, clinical risk, and organic chemistry engines (Scientific profile)
- WebGPU compute

---

## 7. How this relates to existing public infrastructure

| Existing system | Relationship |
|---|---|
| **ABS / SEIFA** | Consumer of published ABS data; does not replicate or compete with ABS collection infrastructure |
| **myGov / Centrelink** | Complementary concept. Concession credentials could interoperate with existing entitlement systems if trust anchors were established, but the civics.au design does not depend on or integrate with myGov |
| **Services Australia** | The credential architecture is designed so that a government issuer *could* issue verifiable credentials into this ecosystem, but no such integration exists or is assumed |
| **State planning systems** | The modelling lab is an independent evaluation tool. It does not feed into or draw from state planning databases |
| **W3C / Solid** | Builds on existing W3C standards (Solid, Verifiable Credentials, Decentralised Identifiers, ODRL, RDF). Does not create competing standards |

---

## 8. Open questions for reviewers

The civics.au site itself identifies several open questions that are particularly relevant for the intended audience of this document:

1. **Evidence gaps in the financial model.** Which inputs most need independent verification? What would constitute adequate market quotes for the placeholder assumptions?

2. **Trust anchors for credentials.** Which institutions could serve as credential issuers for concession entitlements? What governance framework would be required?

3. **Pilot site selection.** What characteristics should a willing regional community have? What institutional partnerships are prerequisites?

4. **Integration with existing entitlement systems.** Is there a pathway to interoperate with Commonwealth concession and welfare systems without duplicating identity infrastructure?

5. **Community benefit measurement.** The system separates what a site costs to run, what it may save government, and what it means for people's lives. How should these three dimensions be weighted and evaluated in a policy context?

6. **Computational economics validation.** The welfare, public-finance, and econometric kernels are implemented but would benefit from peer review against established implementations (e.g. Stata, R packages) for numerical agreement.

---

## 9. Where to look

| Resource | URL |
|---|---|
| civics.au (development) | [dev.civics.au](https://dev.civics.au) |
| Modelling lab | [dev.civics.au/model.html](https://dev.civics.au/model.html) |
| Planning & outcomes (evidence views) | [dev.civics.au/analysis.html](https://dev.civics.au/analysis.html) |
| Concession credentials concept | [dev.civics.au/concession-card.html](https://dev.civics.au/concession-card.html) |
| Solid Databox | [dev.civics.au/databox.html](https://dev.civics.au/databox.html) |
| Web Civics Databox project | [web.civics.au](https://web.civics.au) |
| Plain-language introduction | [dev.civics.au/plain-terms.html](https://dev.civics.au/plain-terms.html) |
| Review, assumptions & sources | [dev.civics.au/review.html](https://dev.civics.au/review.html) |
| Express interest | [dev.civics.au/eoi.html](https://dev.civics.au/eoi.html) |

---

*This document was prepared from a review of the civics.au website (dev.civics.au) and the QualiaDB source code (v0.0.40). It describes implemented capabilities as verified in the codebase, not marketing claims. Status designations (✅ Operational, Concept architecture, etc.) reflect the system's own capability matrix and self-reported status flags.*
