# C1 plan — protected channel, conserved credit, recheck-before-disclosure

Prepared while C0 cargo compiles. No production `.rs` routed yet. Execute after C0 focused selectors are green.

Product constraint (HMC review): selective governed streaming of a QBDL-profile HMC. `Session` over `Bearer` stays a framing fixture (`EvidenceClass::Process`). Production I/O goes through `ProtectedChannel`.

## Existing owners to reuse (do not duplicate)

- `ProtectedChannel::transcript()` is the only transcript constructor input.
- `ProtectedChannel::with_runtime_scope` + `ChannelIo::with_current` + `FrameIo::{queue_record,send_pending,receive_record}`.
- `RuntimeAccess::{admit,poll,issue_credit,complete_io}` — one-shot credit (`Ready` → `InFlight`, second issue is `Replay`).
- `LocalEffectControl::recheck` immediately before each disclosure; callback must not sleep.
- `ServiceRecordHeader` is already the 88-byte AEAD AAD layout `check_aad` expects (`QBDL-SCT-V1\0` + transcript + message id + total/offset/chunk/flags).

## Visibility bumps (integrator, tiny)

1. `protected_channel/receive.rs`: `receive_record` `pub(super)` → `pub(crate)` so `qbdl::service` can assemble outside the channel (QBS-09 outer attempt).
2. Do not re-export `ChannelIo`/`FrameIo`. Closures infer them.

## Files

| File | Change |
|---|---|
| NEW `qbdl/service/protected.rs` | `ProtectedSession` over `&mut ProtectedChannel`. Cooperative WouldBlock **outside** `with_current` / runtime scope. |
| `qbdl/service/session.rs` | Comments: carrier framing, not AEAD. Keep `DEADLINE` only for the fixture. `attach` remains caller-transcript fixture. |
| `qbdl/service/provider.rs` | Bind `Credit.transfer_id` to `OpenedTransfer.transfer_id`. Recheck after recv, before payload copy/send. Production leaf path takes `RuntimeAccess` and `issue_credit`. |
| `qbdl/service/client.rs` | Credit/RequestLeaves reuse `open.transfer_id` (stop minting a second transfer id). |
| `qbdl/service/mod.rs` | `mod protected;` after author stop. Fixture `transfer_one_leaf` stays. |
| NEW `qbdl/service/protected_tests.rs` | Handshake pair + runtime parents; wrong transcript; replay credit; expire between frames. |

## Cooperative send (no guards across sleep)

```
loop {
  match channel.with_runtime_scope(access, |mut io, rt| {
    io.with_current(pins, clock, |mut frame, current| {
      frame.queue_record(current, pins, clock, aad, chunk)?;
      frame.send_pending(current, pins, clock)
    })
  }) {
    Ok(()) => return Ok(()),
    Err(QdnfError::WouldBlock) => {
      if clock() >= admitted_deadline { return Expired }
      thread::sleep(IO_POLL); // no ledger, no CurrentAuthorityGuard, no LocalEffectControl
    }
    Err(e) => return Err(from_qdnf(e)),
  }
}
```

`LocalWholeSourceGrant::accept_local_scoped` wraps only the authorize/recheck/issue_credit/queue steps, never the WouldBlock sleep.

## Credit conservation (R03)

- Open admits a `TransferTicket` via `RuntimeAccess::admit` against a `ColdPlan` row for that entry/leaf.
- Wire `credit_serial` is the ticket `CompletionKey` packing already used by runtime (do not invent a second ledger). If the 16-byte key cannot fit the existing u64 serial field, store serial as the runtime `issue_credit` one-shot token already implied by `InFlight` and keep the wire serial as the session-local nonzero id **bound to that ticket**; repeating the serial against an `InFlight` ticket is `Replay`/`Credit`.
- `issue_credit` once per actual leaf. Duplicate Credit with a fresh carrier message id still hits `InFlight` → refuse. Cancel while pending uses `RuntimeAccess::cancel` and retains occupancy until `complete_io`.
- Fixture `Session::next_credit_serial` remains a counter for in-process tests only; production must not treat it as a reservation.

## Recheck (R04)

- After `recv_control` returns Credit and RequestLeaves, `authority.recheck()` then `runtime.issue_credit` then payload send.
- Header `deadline` is the admitted slot deadline, not constant 90.
- Expire after Open before RequestLeaves: recheck/issue_credit fails, no payload.
- Expire between output chunks: `with_current` final `core.check` refuses; assembly aborts; no further plaintext.

## Tests (must refuse before disclosure)

1. RequestLeaves identity mutations — already in `tests.rs` (R02).
2. Repeat the same credit serial after a successful issue → `Credit`/`Replay`; payload not sent twice.
3. Credit `transfer_id` ≠ opened transfer → refuse.
4. `ProtectedSession::attach` uses `channel.transcript()`; a foreign digest in AAD is `Unauthorized`.
5. Revoke/expire after Open, before delayed RequestLeaves — no leaf bytes.
6. WouldBlock on send drops authority (`try_lock` on owner succeeds) then retry under a live grant.

Do not mark QBS-09 complete from the two-process fixture. HTTP waits until this lands.

## Out of this slice

ZIP `.hmc`, QAC adapter (C3), mixed-bundle planner (C4), browser/WebRTC, wrapping with remote grant (follow-on inside C1 if time).
