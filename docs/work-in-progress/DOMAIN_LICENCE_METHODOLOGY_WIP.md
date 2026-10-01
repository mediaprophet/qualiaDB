# Domain licence methodology — WebCivics WASM export licensing WIP

**Status:** work-in-progress · **Branch:** `0.0.40` · **Not standards** · **No code yet**
**Owner:** Timothy (licensor / issuer) · **This doc:** implementation planning only — used to decide what to build
**Cite:** `webizen-lite-wasm` (`wasm-ontology` capability profile) · `asset_envelope/licence.rs` · `client-core/domains.rs` · `node_identity.rs` · `wallet/` (XEC) · `net/qdnf/authority/permit.rs` · `semantic_instruments/` · Permissive Commons spec §4 (`docs/manuals/webizen_permissive_commons.md`)

**Purpose:** Establish a licensing methodology that issues **signed licence instruments bound to particular domain names**, governing use of the WebCivics WASM export (`webizen-lite-wasm`, the `wasm-ontology` capability profile). The instrument must support multiple licence classes — commercial organisations (development/client work), non-profits and community bodies (e.g. `.org` domains), evaluation/preview, and personal/sovereign use — with eCash as the initial payment/activation rail.

Two discovery/attestation mechanisms are required by the author:

1. **Declared domains** — the codebase can *declare* which domain names a licence covers (compiled-in declarations for client-pinned builds, plus the signed instrument's own subject list, plus an on-disk registry).
2. **Blockchain discovery** — a licence can be *found and audited on nominated blockchains* by its digest: the canonical licence hash is anchored on-chain (XEC `OP_RETURN`, in or alongside the payment tx), so a verifier can search a nominated chain for the anchor and confirm the licence exists, when it was issued, and that it has not been revoked.

---

## 1. Problem (today)

- `webizen-lite-wasm` ships under **CC BY-NC-ND 4.0** (crate `LICENSE`). That is fine for read-only public ontology sites (e.g. `ns.webcivics.net`) but gives a commercial organisation **no right** to run the binary, and gives a non-profit only the ambiguous NC reading.
- There is no instrument that lets the rights holder grant a particular domain additional rights, no signed artefact a licensee can hold, and no payment or activation mechanism.
- The codebase already has most of the machinery: Ed25519 node identity and signing, a domain/agent model (QDP), a fail-closed licence-policy vocabulary for assets, a semantic-instrument manifest format, an eCash (XEC) wallet, and a permit/authority pattern in QDNF. What is missing is the **domain licence as a first-class artefact** and the gate that consults it.

### 1.1 Framing (important)

The licence instrument is a **grant above the base licence**, not a replacement:

- Base: the binary remains CC BY-NC-ND 4.0 for the public at large.
- Grant: the issuer signs a `DomainLicence` that names domain(s), licensee, class, capability scope, and validity window — granting the rights that class carries (e.g. commercial operation, client-specific derivative builds, support terms).
- Honest bound: a WASM artefact can be copied; the licence is a **compliance and provenance instrument** (fail-closed policy gate + signed evidence), not DRM. Legal enforceability rests on the signed grant, the publication record, and the terms — not on technical prevention of copying.

---

## 2. Licence classes (proposal)

Aligned with `domains.rs::AgentType` and the Permissive Commons entity-pricing model (§4.3: natural persons free, commercial actors gated):

| Class token | Audience | Rights granted | Payment |
|---|---|---|---|
| `community` | Non-profit / community / humanitarian (`.org`, `HumanitarianService`, `Group` cooperatives) | Run + serve the WASM export; no commercial rights | Gratis grant (signed, zero-cost) |
| `community-steward` | A local custodian (the "inn" — pub, library, co-op) administering services for member stores in a small community (§2-A) | Run + serve **and administer delegated subdomains/mailboxes for named members**; no commercial resale of the licence itself | Gratis or nominal token — the town is the beneficiary, not the customer |
| `personal` | Natural persons, sovereign self-hosting | Run for personal use | Gratis |
| `evaluation` | Any org, time-boxed | Run, capped window (e.g. 90 days), no redistribution | Gratis or token |
| `commercial` | Commercial organisations Tim does dev work for | Run + serve + client-scoped derivative builds per terms | eCash payment evidence required |
| `internal` | Issuer's own deployments / dev | Unrestricted scope | None |

Open decision: whether `community`/`community-steward` requires evidence of non-profit status (signed attestation in the request) or is self-declared with the issuer's signature acting as acceptance. **Recommendation:** self-declared at request; the signed licence is itself the issuer's acceptance, keeping the flow paste-simple.

---

## 2-A. The community "inn" model (steward-managed domains)

**Rationale (author's framing):** small towns have a resource problem, and digitally savvy actors tend to make it worse — extracting value outward rather than building local capacity. Historically the pub functioned as the town's "inn of court": the trusted physical place where community information was held, notices posted, and disputes heard. The digital equivalent is a **trusted local custodian operating one domain and administering the community's stores/services under it** — rather than every shopkeeper being forced to become their own sysadmin, or surrendering to a platform.

Concretely, in the existing model:

- The inn is a **group/steward-owned domain** — `DomainOwner::Group` (the deferred M:N agreement placeholder finally gets its use-case: the inn's operating agreement, e.g. "the pub's committee, 2-of-3 stewards").
- Each member store gets a **subdomain or purpose address** under the inn's domain: `bakery.towninn.example`, `bakery@towninn.example`, with `parent` set to the inn's domain. Member stores are entries, not sysadmins.
- The licence must therefore cover **delegated administration**: the steward (licence holder) runs the WASM export and manages subdomains/mailboxes *on behalf of* members who never touch the tooling. This is the Permissive Commons **custodial governance** pattern (§1.1 of the commons spec) applied to licensing.
- Guard rail to write into the class terms: the steward licence is **non-extractive** — the inn administers for members at cost/gratis; it may not itself resell licences or charge members rent for what the grant made free. That is what distinguishes "inn" from "platform".

Instrument impact — `subject` gains an optional delegation block:

```json
"subject": {
  "domains": ["towninn.example"],
  "domain_did": "did:web:towninn.example",
  "delegation": {
    "covers_subdomains": true,
    "members": ["bakery.towninn.example", "grocer.towninn.example"],
    "steward_agreement": "did:q42:agreement:…"   // the inn's M:N operating agreement
  }
}
```

`covers_subdomains` + `members` are alternatives (wildcard policy vs explicit roster — decision §7); `steward_agreement` chains into `AgreementDID` / `SuspendedTransactionQueue` M:N machinery when the inn is stewarded by more than one person.

---

## 3. The instrument: `DomainLicence`

A signed, self-contained document — plain JSON (canonical form signed), renderable as a semantic instrument and optionally as yaml-ld-q42 for graph ingest.

```json
{
  "format": "qualia-domain-licence/0.1",
  "licence_id": "did:q42:licence:<hash>",
  "issuer": {
    "did": "did:q42:…(issuer)",
    "pubkey_hex": "<ed25519 verifying key>"
  },
  "subject": {
    "domains": ["example.org", "app.example.org"],
    "domain_did": "did:web:example.org"     // optional QDP agent binding
  },
  "licensee": {
    "did": "did:…",                          // org/person DID; may be empty for community
    "agent_type": "org|person|ai|service|content|group",
    "label": "Example Org"
  },
  "licence_class": "community|personal|evaluation|commercial|internal",
  "capability_scope": ["wasm-ontology"],      // which profiles/toolsets this unlocks
  "not_before_unix": 0,
  "expires_unix": 0,                          // 0 = no expiry
  "payment_evidence": {
    "rail": "xec|none",
    "txid": "…",                              // eCash txid, or cashu token hash later
    "amount_sats": 0
  },
  "chain_anchors": [                          // nominated chains carrying the licence digest
    {
      "chain": "xec",                         // nominated chain identifier
      "txid": "…",                            // anchor transaction (may equal payment tx)
      "op_return_hex": "…",                   // e.g. "QUALIA-LIC " + sha256(canonical licence)
      "digest": "sha256:<hex>"                // canonical licence digest being anchored
    }
  ],
  "terms_uri": "https://webcivics.net/licence/terms/commercial-1",
  "signature_hex": "<ed25519 over canonical bytes>"
}
```

Design notes:

- **Canonical signing** — sign over a defined canonical serialisation (sorted keys, no whitespace) so any node can verify. Reuse the `signature_hex`/`pubkey_hex` convention already used by `social_connect` invites. The **canonical digest** (`sha256` over the same bytes, excluding `signature_hex` and `chain_anchors`) is the licence's stable identity — the value that gets hashed into quins and anchored on-chain.
- **Domain binding** — `subject.domains` is the operative scope. The verifier compares the serving origin (`window.location.hostname` in browser WASM) against this list. Wildcard policy is a decision (§7).
- **Domain DID** — where the licensee runs QDP (`/.well-known/QDP` agent profile), bind `domain_did` so the licence chains into the existing domain-agent model rather than floating on DNS strings alone.
- **Payment evidence** — v1 records the eCash (XEC) txid or a token reference as *evidence the issuer verified at issuance*. The runtime verifier does **not** re-verify payment — it trusts the issuer signature. This keeps the WASM verifier dependency-free and matches "at this stage, via ecash tokens".
- **Chain anchors** — each entry points to an on-chain commitment of the licence digest on a *nominated* chain. For XEC this is naturally an `OP_RETURN` output; when the anchor tx **is** the payment tx, one transaction proves both payment and licence existence. See §4-B.
- **Fail-closed** — unknown `format`, unknown `licence_class`, bad signature, expired window, or domain mismatch → not licensed. No silent downgrade; the unlicensed state is exactly today's CC BY-NC-ND posture.
- **Quin encoding (optional, later)** — a grant can be compiled to deontic quins (`OP_PERMIT` in `deontic_logic.rs`, domain hash in `context`, expiry in `metadata`) for in-engine evaluation. Not required for v1; record it as a bridge task.

---

## 4. Issuance flow

1. **Request** — applicant submits domain(s), entity type, intended use. Web form or `q42 licence-request` CLI path; issuer-side review is human (Tim) for `commercial`, semi-automatic for `community`.
2. **Payment** (commercial) — applicant pays XEC to the issuer wallet (`wallet/` module exists: BIP32 derivation, P2PKH signing). Issuer records txid as `payment_evidence`.
3. **Issue** — issuer tool builds the `DomainLicence`, signs with the issuer Ed25519 key (reuse `NodeIdentity` pattern, but a **dedicated licence-issuer key** — not the node key — is recommended so node keys can rotate without invalidating licences).
4. **Deliver** — licence file (`.qlic.json`) pasted/installed by the licensee; optionally published at the domain's `/.well-known/` or QDP profile for transparency.
5. **Anchor** — the issuer writes the canonical licence digest to the nominated chain(s). For XEC the cheapest path is an `OP_RETURN` output reading e.g. `QUALIA-LIC <sha256-hex>` — either in the payment tx itself (one tx = payment + anchor) or a separate self-spend for gratis licences. The `txid` goes into `chain_anchors`; the signed file is then re-issued or the anchor is appended and re-signed (decision §7 — anchoring must happen *before* final signature if `chain_anchors` is inside the signed bytes, or keep anchors outside the signed envelope).
6. **Revocation (decision)** — v1: expiry + issuer-side revocation list URI checked only when online. A later option is an on-chain revoke marker (a second `OP_RETURN` spending/flagging the anchor). Keep optional; the licence states `revocation_uri` or omits it.

### 4-A. Declared domains

A domain becomes "licensed" through any of three declared paths — the codebase should support all three:

| Path | Mechanism | Use case |
|---|---|---|
| **Compiled-in declaration** | A build-time constant / config list in the crate (`licensed_domains` for the `wasm-ontology` profile, or a per-client build flag) | Client-specific WASM builds where the binary itself declares its licensed domains; simplest possible gate — no file needed on the licensee side |
| **Signed instrument** | `subject.domains` inside `DomainLicence` (§3) | General path; the licence file travels with the deployment |
| **Local registry** | An on-disk/registry declaration (`domains` store or `licence_registry.json`) mapping domain → `licence_id`/digest | Desktop/daemon side: declares which domains this node has licences for, and where to fetch/verify them |
| **Deployed artefact** | Licence file served from the licensed domain itself at `/.well-known/qualia-licence.json` | Static hosts (CF Pages / GitHub Pages); anyone can fetch and verify — the site self-declares its licence |
| **DNS TXT** | `_qualia-licence.<domain>` TXT carrying the licence digest (and optionally the licence body or its URI) | Zero-hosting declaration; published via the existing `cloudflare.rs` `_qdp` TXT machinery — also works for GitHub Pages custom domains via registrar DNS |

The effective licensed set for a given runtime = union of these, each element still requiring a valid signature/anchor where applicable. Compiled-in declarations signed *by the build itself* (i.e. baked into the issuer-signed binary) need no separate file.

### 4-B. Blockchain discovery

- **Nominated chains** — the licence names which chains carry anchors (`chain_anchors[].chain`); the verifier's config nominates which chains it is willing to search (allowlist: `"xec"` first; extensible to others the wallet stack grows). A licence anchored on an un-nominated chain is simply not found — fail-closed, not an error.
- **Lookup** — `licence lookup <digest|txid>` (CLI / native) queries the nominated chain's indexer (XEC: Chronik-style API) for the anchor `OP_RETURN`, extracts the committed digest, and compares it to the candidate licence file's canonical digest. Match ⇒ the licence was anchored at that block height/time by the issuer.
- **What the chain proves** — existence at a time (block height), binding to the payment when the anchor tx is the payment tx, and a public audit trail without any issuer server. It does **not** replace signature verification — the digest confirms *this file* is the anchored one, the Ed25519 signature confirms *the issuer* wrote it.
- **Placement** — chain lookup is a **native/CLI/issuer-side audit path**, not a WASM runtime dependency. The browser gate remains offline signature + domain check; an optional host-side online check may consult anchors for `commercial` class (decision §7).

---

## 5. Verification path (WASM gate)

- `webizen-lite-wasm` gains an internal licence check: load licence (embedded at build time for issuer-signed bundles, or supplied via a `licence` parameter / `/.well-known/qualia-licence.json` fetch by the host page), verify signature against the **pinned issuer verifying key compiled into the binary**, check `capability_scope` covers the active profile, check domain match and validity window.
- Expose an MCP tool `licence_status` returning `{ licensed, class, subject_domains, expires_unix, reason }` — read-only introspection, never a bypass path.
- Gating posture (decision): (a) hard gate — gated tools return a licence-required error when unlicensed; or (b) watermark/notice — tools run but report `licensed: false`. **Recommendation:** (a) for `commercial` scope claims, with `community`/`evaluation` issued freely so the friction is a signature, not a payment wall.
- Non-goals for v1: no network callback to an issuer server (local-first, Sanctuary-compatible), no DRM, no telemetry.

### 5-A. The inn deployment profile — static hosting, non-technical steward

The community-inn use-case (§2-A) sets the operational bar: **the pub manager is not tech-savvy**. Consequences that shape the design:

- **Static hosting only.** The inn's site + WASM export must run off **Cloudflare Pages or GitHub Pages** — no server, no daemon, no admin console. This is already the product path: `deploy_static_site_cf_pages` (Domains pane, "Publish Static Site (GitHub + CF Pages)") + `cloudflare.rs` (zone lookup, `_qdp` TXT publish) exist. The licence methodology must not require any server-side component — and doesn't: signature + hostname + window checks are all client-side.
- **Licence rides the same publish step.** The issuer generates the `.qlic.json`; the deploy flow places it at `/.well-known/qualia-licence.json` in the same site bundle and (optionally) publishes the `_qualia-licence` TXT digest in the same Cloudflare call that already writes `_qdp`. The steward sees one button and a "Licensed ✓" — never a JSON file.
- **Issuer does the crypto, steward does the approving.** All signing (base licence, roster amendments, anchors) is done by the issuer key, which the steward never holds. Member join/leave: a simple roster form in Webizen Studio produces an unsigned amendment → issuer signs (or a held co-sign flow) → steward clicks **Publish**. Extends decision §7-14: the steward needs *no keys at all* — the licence covers administration, the issuer retains signature authority.
- **GitHub Pages parity.** No CF API needed: licence file committed to the repo, DNS TXT at the registrar. The `deploy_static_site_cf_pages` path covers GitHub repo creation already; a pure-GitHub-Pages variant is a fallback, not a separate design.
- **Degraded honesty.** `licence_status` should also power a small site-visible badge ("Licensed by WebCivics · class: community-steward · verify") — the audit trail for the community is that anyone can fetch the well-known file and check the signature/digest themselves.

---

## 6. Where the code lives (proposal)

| Component | Location | Notes |
|---|---|---|
| `DomainLicence` model, canonical codec, sign/verify, digest | `crates/qualia-core-db/src/domain_licence/` (new dir-lib: `mod.rs` + `model.rs` + `codec.rs` + `verify.rs` + `anchor.rs`) | Cold-construction tier (alloc OK at build/verify boundary); follows `asset_envelope` conventions: fail-closed parse, `<500` lines/file |
| Domain declaration registry | `domain_licence/registry.rs` + declared-domain plumbing in `client-core/domains.rs` | Union of compiled-in list, signed instrument subjects, and on-disk registry |
| Issuer CLI | `crates/qualia-cli` `licence issue|verify|status|lookup|anchor` | Reuses Ed25519 signing; dedicated issuer key file under app meta dir |
| WASM gate + `licence_status` tool | `crates/webizen-lite-wasm/src` + `wasm-ontology` profile in core-db | Pinned issuer pubkey const; compiled-in domain declaration option; domain from host-supplied param (JS passes hostname — WASM cannot read `window.location` without a JS shim; decision in §7) |
| eCash payment + anchor writer | `crates/qualia-client-core/src/wallet/` (XEC) | Payment tx with `OP_RETURN` licence digest; txid recorded as `payment_evidence` + `chain_anchors` |
| Chain lookup / audit | `qualia-cli licence lookup` → Chronik-style indexer client | Search nominated chains for digest anchor; compare to file |
| Static-host deploy + DNS TXT | `api/domains.rs::deploy_static_site_cf_pages` + `cloudflare.rs` (TXT publish) | Licence file into the site bundle at `/.well-known/`; `_qualia-licence` TXT digest beside the existing `_qdp` publish |
| Member roster UX | `webizen-studio` Domains pane (steward form) → signed amendment → republish | Steward never touches JSON or keys; one publish action |
| Semantic-instrument packaging (optional) | `semantic_instruments/manifest.rs` (`licence`, `domain` fields already exist) | Licence renders as an instrument for catalogue display |

---

## 7. Open decisions for the author

1. **Wildcard subdomains** — does `example.org` cover `*.example.org`? Recommend explicit list + optional `*.` prefix entries, exact-match default (fail-closed).
2. **Hostname source** — WASM can't see `window.location` natively; host JS must pass the claimed domain. That makes the domain check *policy-level* only — acceptable, since enforceability rests on the signed grant, but the doc should not overclaim.
3. **Dedicated issuer key vs node key** — recommend a dedicated `licence_issuer` keypair so node rotation doesn't invalidate licences; publish verifying key at a well-known URI.
4. **Revocation** — expiry-only v1, or a signed revocation list the verifier may fetch when online (advisory, never blocking offline use of a valid licence)? On-chain revoke markers are a later option.
5. **eCash rail depth** — txid evidence only (v1) vs Cashu bearer token embedded in the licence (later, if blind-signed bearer semantics are wanted).
6. **Community verification** — self-declared org type, or require a signed attestation in the request?
7. **Quin bridge** — defer to a later task, or compile grants to deontic quins in v1 for in-engine evaluation?
8. **Anchor scope** — is `chain_anchors` inside the signed bytes (requires anchoring before final signature; anchors become immutable licence content) or outside the signed envelope (file can gain anchors post-issuance; digest then covers the signed portion only)? Recommend **outside** — lets a gratis licence gain an anchor later without re-issuing.
9. **Anchor-vs-payment coupling** — single tx carrying payment + `OP_RETURN` digest (cheap, binds them) vs separate anchor tx (cleaner for gratis/renewals)? Recommend: combined for `commercial`, standalone self-spend optional otherwise.
10. **Nominated-chain set** — v1 = `"xec"` only? Which indexer endpoint is canonical, and is it a build constant or verifier config?
11. **Declaration precedence** — if the compiled-in declaration and a signed licence disagree for the same domain (e.g. stale baked list vs. newer file), which wins? Recommend: signed instrument wins; compiled list is a floor, not an override.
12. **Delegation enumeration** (inn model) — explicit `members` roster vs `covers_subdomains: true` wildcard? Roster is auditable but needs re-issue on join/leave; wildcard is flexible but covers *any* subdomain including ones the issuer never vetted. Recommend: roster for v1 (fail-closed), wildcard reserved for `internal`/`commercial` classes.
13. **Member join/leave** — does a roster change re-issue the licence (new digest, new anchor) or is the roster maintained as a separately-signed amendment list referenced by `licence_id`? Recommend: amendments signed by the same issuer key and chained by digest, so the base licence + amendment chain both verify.
14. **Custody model** — does the inn hold member keys, or do member stores keep their own DIDs with the steward holding only an *admin grant*? Custodial key-holding is simpler but concentrates compromise risk and brushes the selfhood/personhood line if members are natural persons. Recommend: members keep DIDs where feasible; the steward's licence covers administration, not identity custody. **And for the steward itself (§5-A): the steward holds no keys either — the issuer signs all amendments; the steward only approves content.**
15. **Discovery precedence on static hosts** — order of checks for a verifier: embedded licence → `/.well-known/qualia-licence.json` on the serving origin → `_qualia-licence` DNS TXT → compiled-in declaration? Recommend: first valid wins; all are fail-closed so precedence only affects latency, not correctness.
16. **Amendment signing** — steward co-sign flow (issuer + steward keys) vs issuer-only signing with steward approval as an *unsigned* step? Recommend issuer-only v1: fewer keys in the pub, matches #14.

---

## 8. Phased work register

| Phase | Deliverable | Depends on |
|---|---|---|
| L0 | This methodology doc — decisions in §7 resolved | — |
| L1 | `domain_licence` core lib: model (incl. `delegation` block), canonical codec, sign/verify, canonical digest, fail-closed tests (bad sig, wrong domain, expired, unknown class, delegation-scope violation) | L0 |
| L2 | Issuer CLI (`licence issue/verify/status`) + dedicated issuer key management + domain declaration registry | L1 |
| L3 | `webizen-lite-wasm` gate + `licence_status` MCP tool + pinned pubkey + compiled-in declaration path | L1 |
| L4 | eCash payment tx with `OP_RETURN` licence digest; `chain_anchors` written; `licence anchor` CLI | L2 + wallet |
| L5 | `licence lookup` — search nominated chains by digest/txid; audit report (found / height / matches file) | L4 |
| L6 | Inn deployment profile: licence file + `_qualia-licence` TXT in the `deploy_static_site_cf_pages` publish path; steward roster form → signed amendment → republish; `licence_status` badge | L1, L3 |
| L7 | Optional: QDP publication, semantic-instrument packaging, deontic-quin bridge, on-chain revoke markers | L3 |

---

## 9. Relationship to existing licence machinery

- `asset_envelope/licence.rs` governs **dataset/asset** redistribution (CC classes, obligation union). `DomainLicence` governs **permission to run/serve a binary from a domain**. They share the fail-closed posture and class vocabulary style; they are different artefacts — do not merge, do cross-reference (`terms_uri` may point at the same published terms).
- Permissive Commons `EnforceBilateralMicroCommons` / TSL: a `commercial` domain licence is conceptually a bilateral micro-commons grant; if/when assets served under a licensed domain carry commons quins, the licence id can be cited in the quin `context` field. Deferred — record only.
