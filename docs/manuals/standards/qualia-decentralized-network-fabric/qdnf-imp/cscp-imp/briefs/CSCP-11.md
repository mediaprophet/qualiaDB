# Assignment CSCP-11 — Independent review of Wave 1

- Parent: CSCP-11. You did **not** author Wave 1. Do not approve from the author's summary. Read the code.
- Non-goals: do not rewrite the protocol; do not tick workstream boxes; do not edit implementation files. If you find a defect, describe it with file/function and a failing scenario. Integrator applies fixes.
- Allowed writes only:
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/reviews/CSCP-11-wave1.md`
  - `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/cscp-imp/handoffs/CSCP-11.md`

Review against `draft-webcivics-cscp-00.md`:

1. Exclude-then-rank: faster Direct cannot override “do not disclose IP.”
2. Unknown critical TLV fail-closed on every message type.
3. PathEvidence: `observer=2 && validated=1` malformed; decode never yields local validation.
4. Relay lease ≠ custody lease; relay-only forces `export_observations=0`.
5. Transport ACK is not Committed; replay after grant revoke is Denied.
6. Mailbox is not a DHT; Direct locators are not probed under relay-only.
7. `cscp_wss.rs` does not set Internet honesty flags and uses rustls, not plain TCP.
8. Zero-heap: flag any `Vec`/`String`/`Box` on encode/decode/kernel/mailbox hot paths. Cold persist (`receipt_store.rs`) may allocate; say so.
9. Security: TLV duplicate detection, body cap 1024, replay, stale generation.

Be adverse. List residual risks. Recommend accept / accept-with-fixes / reject for Wave 1 local completeness. Wave 2 Internet remains out of scope for “accept.”
