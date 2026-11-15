# Native service owner follow-ups after resource bridge

2026-11-13. Source review/planning only. No implementation/qualification from this note. Full QBS09/10 and remote governance remain required; no package marked complete. Root owns builds/current carrier/wire integration, service scopes require handoff review.

## Service outer acceptance owns assembly lifetime

Inspected NEW qbdl/carrier/{mod,receive,send,tests}.rs. Bounded contiguous continuation, strict record geometry, high-water/control interleave and error/unwind/Drop erase guard are coherent at parser layer. Every chunk invokes admission before copy; complete messages parse before consume. These callbacks do NOT mint source authority/budget/current pins.

Actual service wrapper must keep an assembly attempt armed OUTSIDE common channel and application outer current checks, then commit only after their final success. An Assembly.accept success followed by outer expiry/revoke must still abort/erase partial assembly; do not publish final consumer/durable effects inside the early consume callback. Common ChannelIo::receive_record owns its actual output guard through its own final current check, but that does not automatically govern a higher carrier assembly or later policy callback. Reuse phaseC's armed outer attempt/poison discipline.

Cancel integration choice must be explicit: Assembly.abort is terminal. A valid Cancel targeting active data does not itself clear/drain active; caller must close/reconnect (safe narrow initial behavior), or implement matched canceled-message drain state with charged retained I/O and exact late-fragment fences. Never reset high_water/reuse old message or transfer IDs. The geometry fixture that continues data after same-transfer Cancel uses permissive parser callbacks and is NOT application cancellation evidence. A real service check must observe actual canonical cancellation before further copy/acceptance. Loss/path/revoke can fail closed; no reliable-carrier application retransmission loop.

PendingMessage only borrows immutable bytes: it cannot erase caller storage. Actual private leased buffer owner must erase on error/cancel/drop, and source/mapping immutable snapshot obligations remain. Sender advances drained only after exact sealed frame is fully accepted by actual channel/adapter. WouldBlock retains SAME pending plaintext association and SAME sealed frame; do not reseal/reset counters. Fresh MessageSequence is one per actual session/direction, not per transfer.

## Required native storage and OS bearer work

Persistent runtime bridge does not make File::sync_all nonblocking. Reviewed bounded storage/source job handoff must retain SAME admission/ticket/page/source/buffers, release parent guards before OS wait, recheck actual cancellation/time/policy at barriers, and drain real I/O before completion/reuse. No fake backend success, new ledger, checkpoint-as-presence or process-restart authority reconstruction. Existing synchronous legacy methods remain honest live-session staging.

Native local_stream is a later separately scoped owner: actual Windows named pipes and Unix-domain transport, real adapter-observed peer/process credentials plus received-Finished QSession identity, bounded caller/admitted buffers/cursors, real shutdown/drain, exact route/scope/MTU and ownership/exclusion. PID/OS credential alone is not cryptographic peer authority. Investigate native pending-I/O memory pinning/Drop safety before choosing implementation, not after. No arbitrary incoming path or silent TCP/IP fallback. In-process IPC remains a unit fixture; Unix8-byte process harness is not useful QBDL transfer. Distinguish Windows, independent Linux/WSL process, physical cross-host and unsupported/permission-refused evidence.

## WSS concrete defect follow-up (root inspection)

Root reviewed existing p2p/connectivity/wss.rs and wss_bearer.rs and reported: fixed client mask[1,2,3,4]; allocating read_ws_binary/recv_datagram; write_all/read_exact losing partial progress on WouldBlock; bearer.shutdown no-op; incoming envelope c/g discarded. Existing adapter MUST NOT be used unchanged for cooperative QBS service. This is a concrete fix scope, not a missing-library claim. No worker edits yet.

Next WSS handoff: retain/reuse actual envelope codec, strict c/g/session-generation association and framing; bounded resumable read/write cursors and aggregate fragmentation/control handling; fresh central-library/OS entropy mask for EACH new client frame while retaining exact encoded frame/mask across partial retries; no unchecked whole-message allocation; real shutdown/drain and budget retention on pending I/O/error/unwind. Existing reliable-carrier semantics, credits/current authority/outer assembly poisoning remain. WSS stays an explicitly labelled transition carrier and TLS is not PQ publisher authority or G01-native independence. Test actual byte boundaries/partial progress/WouldBlock/retry/tamper/oversize/control/close, raw allocation where applicable and actual loopback evidence under honest security class. Root must assign exact source ownership/Cargo routing before code; no speculative edits to this adapter in runtime-state scope.

## Still required authority owner work

Remote Share/Sentinel/contact/consent bridge remains required before remote disclosure. Local admin profile and managed crypto/provider grants do not waive it. AuthorityOwner actual-value incarnation capture/check is follow-up beyond exact mutex affinity; current trusted host must not replace the canonical owner contents while its grants are live. All existing cryptographic key custody/primitives remain under central library and actual pin owner.
