# QBS09/QBS10 channel + retained runtime ledger bridge

2026-11-13. Root approved preparing NEW unrouted bridge children. Routed source frozen under core5809. No parent edits/build here. This follows qbs09-runtime-state-integration.md; both batches must compile together when approved.

## Exact resource provenance and API

EndpointReservation now needs to capture the ledger VALUE incarnation at ORIGINAL actual admission, not first channel/runtime reattach. New resource/identity.rs defines ONE ENDPOINT_BUDGET=128KiB/work16/io24 and admit_bound: locks same original ledger, binds its actual ID once, issues SAME ResourceGovernor::charge_unknown handle, records the ID. No new ledger/caps/authority. Legacy releasing Drop must compare actual value ID before releasing, so replacement at same mutex address with equal slot/generation/budget cannot refund a foreign charge.

RetainedEndpointReservation moves the exact original ID alongside original handle. New retained/identity.rs checks actual held ledger value ID + full original budget/handle before borrowing/releasing. No repair/rebinding after value replacement. Explicit release refusal keeps original handle for retry; restoring the exact original actual value allows release. Lost Drop on wrong owner retains uncertainty/does not refund reconstructed values. AuthorityOwner replacement is still the separately documented trusted-host assumption; this bridge changes resource identity only.

Opaque RuntimeAccess contains private actual original Mutex binding + checked ledger ID captured by RuntimeParents scope, alongside its private mutable legacy runtime borrow. verify_external_reservation compares actual mutex identity, both nonzero original IDs, current actual ledger ID and full live charge through the ALREADY-HELD ledger. It creates no new reservation/permission/keys. Constructor remains private; no parsed wire/booleans/from_caps can construct this view.

NEW protected_channel/runtime_bridge.rs implements:

    ProtectedChannel::with_runtime_scope<'b,'p,T>(
        &mut self,
        access: RuntimeAccess<'b,'p>,
        effect: impl for<'s> FnOnce(ChannelIo<'s,...>,RuntimeAccess<'b,'p>)->Result<T,E>
    )->Result<T,E>

It CONSUMES the opaque access view and forwards that SAME view beside actual ChannelIo, avoiding immutable/mutable aliasing or a second ledger lock. Original charge validation precedes channel mutation/effect; a foreign context refuses and preserves original channel for correct context/close. Actual frame authorization remains short with_current using same current owner/pins; no grant/credit/durability from resource validation. On fatal callback/error/unwind existing channel guard erases state, and runtime scope's restore still puts original runtime state back before unlocking. No raw mutable runtime/parent escapes.

## Exact parent recipe, NOT APPLIED

1. endpoint/resource.rs add:

    mod identity;
    pub(super) use identity::ENDPOINT_BUDGET;

Add private EndpointReservation field:

    incarnation: crate::net::peer::runtime::incarnation::CanonicalIncarnation,

Replace existing public admit BODY ONLY with Self::admit_bound(ledger); preserve exact signature. Replace existing Drop body with self.release_on_drop(). Original algorithm is now ONCE in new resource/identity.rs and fixed full budget shared with validation. Remove obsolete parent ResourceGovernor/ResourceBudget imports only if unused; keep ReservationHandle/ReservationLedger. This step depends on canonical parent incarnation field/helpers in state batch.

2. endpoint/resource/retained.rs add:

    mod identity;

Add identical private incarnation field to RetainedEndpointReservation. Successful original retain_for_channel initializer adds incarnation:self.incarnation. Keep mutable original owner intact on refusal. with_ledger: after successful original ledger try_lock, call self.check_identity(&ledger)? BEFORE callback. release_after_shutdown: FIRST reject handle.is_none as DoubleRelease (preserve existing normal error), then try_lock, then self.check_identity(&ledger)? BEFORE taking/releasing handle. Existing checked handle restoration remains exact. No release/re-admit/new handle. Existing No releasing Drop remains.

3. protected_channel/mod.rs add mod runtime_bridge. In protected_channel/scope.rs make ONLY ChannelIo's core/bearer fields pub(super) to permit construction in its sibling private bridge; FrameIo already has that visibility. No public field/raw key access.

4. Existing endpoint/established/protected_channel/endpoint_tests.rs add:

    #[path="endpoint_runtime_tests.rs"]
    mod runtime_tests;

This NEW test child reuses its private ACTUAL managed received-Finished Fixture, no duplicated raw signing/test owner. resource/identity.rs routes its own NEW identity_tests.rs.

5. NEW (currently unrouted) state/access.rs and state/scope.rs were updated to retain/check actual ledger binding/ID. Actual static size counter automatically includes enlarged RuntimeAccess header. Those files remain uncompiled/unrouted; updated hashes below replace prior state recipe entries. No state/authority/budget duplicate.

## Tests / remaining owner requirement

5 authored NEW tests: dropping original reservation after same-address matching reconstruction never refunds fake; retained borrow/release refuses reconstruction then releases exact restored original; actual received-Finished channel+runtime shares actual held ledger without relock/new charge; foreign actual ledger refuses while original channel remains usable; original channel cannot attach after handshake ledger replacement BEFORE a new runtime registration, even matching slot/generation/full budget. Root runs builds; no tests executed here.

Actual service owner must abort/clear any carrier assembly on OUTER channel/application final authority failure, including failure after inner chunk acceptance. Common ChannelIo::receive_record already owns actual output erase guard through enclosing current check; higher assembly/policy scope must keep its attempt armed until its own final success. A carrier callback receiving chunks does not establish current authority/acceptance by itself. Resume old hints/authority/credit remain forbidden. Native storage/source job handoff and real OS bearer/remote Share remain required subsequent work.

## New bridge files and updated unrouted state hashes

net/qdnf/session/handshake/endpoint/resource/identity.rs 0f7402bdd8231c6b82d41891e40ff19ebdd0d11f382893b53ca5df29d32ce84c
net/qdnf/session/handshake/endpoint/resource/retained/identity.rs 34654f85d80d88481360afdcc0c7946bcb74a616195ae95aafae6ffa007084cb
net/qdnf/session/handshake/endpoint/established/protected_channel/runtime_bridge.rs fb8fb4438aab8cc3fdda0483f0fc028b7f2df29b353327aefb4cc380cb380b87
net/qdnf/session/handshake/endpoint/resource/identity_tests.rs 3cb53985c11a702bef636f1651d4748c8a13d3f7e021d99e25599a31b0545203
net/qdnf/session/handshake/endpoint/established/protected_channel/endpoint_runtime_tests.rs b7c038151951edb785fdaaa9083f0977d1ec293adf3606340dfafca67cd95632
bundle/stream/runtime/state/access.rs ebaf620aec6dec8044f31255d9b96764889ecaf2b0ee4b686d0768f09046f517
bundle/stream/runtime/state/scope.rs 1a997d462a9c6a6cb3f0f3a4969b11dee6e81608728810d3d06a4b6c202a4bf4

All5 NEW bridge files plus3 updated unrouted state files scoped-rustfmt2021 passed;5 bridge tests authored, not run. No routed edits under5809.
