# Wave 3 reviews — E / Q / M / F / H (expert, read-only, queued 3B)

**Do not launch until 3A+3A2 authors have stopped, integrator has routed parent `mod.rs`, and cargo of those modules is green** (or the review is explicitly source-only if cargo is still held — then say so in the review header).

Five independent reviews, one file each. Review occupies a live slot. **No production writes. No cargo.**

## Allowed writes

- `docs/.../qbdl-streaming/evidence/swarm-wave3-e-review.md`
- `docs/.../qbdl-streaming/evidence/swarm-wave3-q-review.md`
- `docs/.../qbdl-streaming/evidence/swarm-wave3-m-review.md`
- `docs/.../qbdl-streaming/evidence/swarm-wave3-f-review.md`
- `docs/.../qbdl-streaming/evidence/swarm-wave3-h-review.md`

Format: same as `swarm-s2-remote-r2-review-2.md` — `pass: true|false`, `open_bugs: N`, file:line findings.

## E (EMF stream)

Confirm: version dispatch, no v2 fallback on v3/unknown, `select_region` is a chunk not a volume receipt, truncated plane and last digest refuse, two EMF entries distinct, archival ≠ validated field, no `emf_pipeline` / portal / parser copy, no sRGB reduction.

## Q (QAC stream)

Confirm: complete-entry still the transfer unit; `admit_signal` maps live `signal::Error`; UA4Q refuses; partial range still Extent; signed-zero preserved or archival documented; unknown critical is not playable audio; `declared_interval` absent ≠ epoch 0; two QAC entries; no `audio/qac/**` edits.

## M (mux)

Confirm: two QAC + two EMF actually transferred; admit before `present`; blobs not tagged as Qac/Emf; `Schedule::next` only picker; `transfer_one_leaf` reused; M64 delayed; Incomplete refuses before leaf; Describe bootstrap; no Kind 13+.

## F (mixed HMC)

Confirm: ≥2 QAC, ≥2 EMF, ≥2 TenD; E/Q admit + present ids; original bytes preserved; duplicate id refuses; ZIP not sniffed.

## H (HTTP mixed)

Confirm: Component only; two QAC + two EMF 206 bodies equal native and admit as those families; M64 delayed; incomplete is not 200/206.

## Stop

Five review files. `open_bugs: 0` required before integrator treats Wave 3 as closed. List fixes for the integrator; do not patch production code in the review child.
