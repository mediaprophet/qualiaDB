# Wave 3 reviews — M / F / H (expert, read-only, queued 3B)

**Do not launch until Wave 3A authors have stopped, integrator has routed parent `mod.rs`, and cargo of those modules is green** (or the review is explicitly source-only if cargo is still held — then say so in the review header).

Three independent reviews, one file each. Review occupies a live slot: run them after M/F/H stop, sequentially or as a single child writing three files. **No production writes. No cargo.**

## Allowed writes

- `docs/.../qbdl-streaming/evidence/swarm-wave3-m-review.md`
- `docs/.../qbdl-streaming/evidence/swarm-wave3-f-review.md`
- `docs/.../qbdl-streaming/evidence/swarm-wave3-h-review.md`

Format: same as `swarm-s2-remote-r2-review-2.md` / `swarm-s2-write-leaf-drop-review.md` — header with scope, `pass: true|false`, `open_bugs: N`, findings with file:line, acceptance gaps.

## M review (mux)

Confirm: `Schedule::next` is the only unit picker; `transfer_one_leaf` reused; no Kind 13+; grant not held across WouldBlock; Describe bootstrap test exists; blobs are not tagged as Qac/Emf/TenD; Incomplete refuses before leaf; shared dep not sent twice; cancel retains occupancy per `Schedule`.

## F review (mixed HMC)

Confirm: ≥2 QAC, ≥2 EMF, ≥2 TenD; bytes admit through format views; `include_bytes!` of candidate fixtures or public `compile_mesh_to_10d`; no edits under `audio/qac` / `q42/field_*`; original bytes preserved; duplicate id refuses; missing mandatory is Incomplete; ZIP `.hmc` not sniffed.

## H review (HTTP mixed)

Confirm: Component only; serve order follows `Schedule::next`; 206 bodies equal native slices; two QAC ids are distinct objects; M64 delayed; incomplete is not 200/206; no `wss*` / `cscp_h2` / `HttpRangeSource` wrap; JSON not an object; status is not a grant.

## Stop

Three review files. `open_bugs: 0` required before integrator treats Wave 3 as closed. Do not patch production code in the review child; list fixes for the integrator.
