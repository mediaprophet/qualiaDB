# QBS-04 independently driven endpoint extension contract

2026-11-12. Root-approved implementation profile; source implementation now added, independent qualification pending. This worker inspected source and primary standards only; no builds, endpoint traffic, signatures, secret generation or application effects executed. Existing legacy helper/hello bytes remain unchanged. This is an extension inside the current QSession handshake owner, not a new crypto library, key store, bearer or parallel session subsystem.

## Existing owners and precise limits

- net/qdnf/session/handshake/{fragmented,hello}.rs: handshake_over_fragments owns BOTH peers' KEM/DH/signing inputs, compares both computed secrets, generates both Finished values and verifies its own values. It exercises actual Bearer fragmentation but cannot evidence independent peer state or received peer confirmation. hello verifies keys included by its sender; that is possession, not pinned peer identity.
- net/qdnf/crypto/handshake/shares.rs: initiator_share, responder_complete, initiator_complete own validated hybrid exchange over the existing KEM/X25519 adapters. These signatures take raw X25519 secret arrays. EphemeralKeyLease intentionally exposes public/DH operations, not its secret. Add lease-based overloads under THIS owner; do not export the lease's private bytes or duplicate the KEM implementation in the endpoint.
- crypto/network/{kem,entropy,secret_lease}.rs: MlKem768Secret zeroizes on drop; EphemeralKeyLease owns local ephemeral material and terminal cancel/destroy; SecretLease is non-Copy, zeroizing, 64-byte scoped storage. Its unused ephemeral Ed25519 material must never replace managed controller signing.
- net/qdnf/crypto/{schedule,finished}.rs: derive_handshake_keys mixes ML-KEM then X25519 through HKDF-SHA384 with Transcript digest salt; finished_mac/verify_finished use secret-derived HMAC-SHA384 and role separation. HandshakeKeys is Copy and has no Drop: new endpoint must not expose it as installed material. A small profile-specific schedule child retains existing owner/primitives and wraps results in zeroizing state.
- crypto/network/transcript.rs: fixed 4096-byte, length-delimited Transcript; source has complete share codecs (1216-byte initiator and1120-byte responder). Complete shares plus compact intent/negotiation and proof-envelope SHA384 commitments fit without enlarging it.
- crypto/network/cose::sign_pair accepts ONLY ManagedPairedSigning; CryptographicLibrary::begin_paired_signing uses its existing KeyManager, policies, trusted host authentication and SigningControl. Endpoint requests exact pending binding; no direct raw signing or new long-term generator.
- PinAuthority::verify_source/recheck supplies actual full paired pins, exact purpose/audience/epoch, temporal/revocation/generation checks and incarnation-bound receipts. Neither wire keys nor a wire role installs authority. Keep pins mutable BETWEEN endpoint steps; borrow only during short verification/acceptance.
- net/qdnf/bearer/{contract,fragment}.rs: Bearer/RecvMeta, split_handshake, decode_fragment, HandshakeReassembler::admit/buffer exist. 8192 bytes/message, 8 fragments, 2 slots. The convenience fragmented send loses its cursor on partial flight WouldBlock; receive ignores source metadata and accepts the lower decoder's trailing payload tolerance. Use a focused cursor adapter over the existing functions, not that paired helper. Enforce exact payload length and observed source/scope before admission.
- InstalledSessionKeys::new accepts a crate-private confirmed bool; ExecutionPermit::require_keys checks only confirmed, not transcript identity. The endpoint must not call new(...,true) as qualification. Add a focused verified-owner constructor which itself verifies RECEIVED peer Finished before constructing keys, without a bool parameter, and keep the result in a private transcript/operation-bound established wrapper.
- AuthorityOwner current issuance is real, but MutationHandle has no owner incarnation. Bind one concrete owner once at locally trusted adoption; do not accept a new arbitrary owner on every poll. Preserve current issuance and same-owner guard across the final bounded effect. Existing retire_revoked is only safe after local synchronous uses end; it is not a session/IO drain API.

Read existing net/crypto DIRECTORY_INDEX.md and the actual module registries before these source findings. Those indices do not list the newer endpoint code; this report makes no whole-tree absence claim.

## Proposed frozen profile and three flights

New explicit outer profile ASCII qpr-pq-1/endpoint-v1. This distinguishes new wire from legacy share+cert+dual-proof hellos; algorithm suite remains the existing QPR_PQ_1 suite. No fallback to legacy, classical-only, unknown suite or 0-RTT. No change to legacy qsession_domain, dual_sign or previous COSE encodings.

Outer hello magic ASCII QSESSION-ENDPOINT-V1 plus NUL. Following byte is flight kind:1 ClientHello,2 ServerHello,3 InitiatorFinished. These are scoped payload discriminants, not new global QFrame registry tags. ClientHello uses SessionChallenge/QSession; ServerHello and InitiatorFinished use SessionProof/QSession. Every flight uses FRAGMENT framing, including a one-fragment Finished. Unknown phase/kind/reserved values and trailing bytes refuse.

Canonical common intent is fixed ordered bytes: initiator[48], responder[48], service[48], purpose[48], instrument[48], scope[48], operation[16], initiator ML key-ID[48], initiator Ed key-ID[48], responder ML key-ID[48], responder Ed key-ID[48], initiator controller epoch u64be, responder epoch u64be, expires_unix u64be (520 bytes). All full references/operation/epochs are nonzero; parties differ. Each endpoint compares every common field against its locally adopted intent. Local AuthorityBinding uses principal=local endpoint, recipient=peer, exact common purpose/instrument/scope/operation. Service is additionally in the signed intent; it is not inferred from a short ID.

ClientHello body: common intent, fresh nonzero client nonce[32], client offer[8], full existing initiator share[1216]. Offer is suite u16be=1, advertised MTU u16be, max message u16be=8192, max fragments u8=8, zeroRTT u8=0. ClientHello then carries envelope length u16be and paired COSE envelope (<=4608). No long-term public keys/cert chain are imported from the wire.

ServerHello body: SHA384 of exact ClientHello wire[48], echoed client nonce[32], fresh nonzero responder nonce[32], server selection[10], full existing responder share[1120]. Selection is suite u16be=1, responder advertised MTU u16be, selected MTU u16be, max message u16be=8192, max fragments u8=8, zeroRTT u8=0. Then envelope length u16be, paired COSE envelope and received-on-initiator responder Finished[48]. Selection must be the existing negotiate_mtu(client_offer, local responder MTU); neither side accepts downgrades/substitutions. Maximum individual hello stays <8192 even using the full envelope cap.

InitiatorFinished: echoed client nonce[32], responder nonce[32], final pre-initiator-Finished transcript digest[48], initiator Finished[48]. Exact length, no extension fields.

Fresh endpoint instances own only LOCAL secrets. Initiator owns MlKem768Secret + EphemeralKeyLease; responder owns only its EphemeralKeyLease and locally computed encapsulation result. They never receive the other peer's private keys, shared secrets or derived traffic material. Tests compare final opposite directions only via established endpoint observations; there is no aggregate helper that computes both sides' Finished. Application capability remains absent until this side has received+verified the OTHER Finished and successfully sent its own Finished.

## Transcript, pins, managed signing and key separation

Use existing Transcript, ordered append calls with exact labels:

1. endpoint-profile = ASCII profile above; intent = exact common520 bytes; client-nonce =32 bytes; client-offer =8 bytes; initiator-share =1216 bytes. Digest is the ClientHello signature commitment.
2. client-proof = SHA384(exact paired COSE envelope); responder-nonce =32 bytes; server-selection =10 bytes; responder-share =1120 bytes. Digest is the ServerHello signature commitment. The explicit ClientHello wire hash/nonce echoes must match retained sender bytes as well.
3. server-proof = SHA384(exact paired envelope). This is T0 for hybrid schedule and responder Finished. Proof bytes themselves exceed Transcript4096; full SHA384 commitments bind them without truncating security identity.
4. responder-finished = received/locally sent exact48-byte tag. This is T1 for initiator Finished.
5. initiator-finished = received/locally sent exact48-byte tag. This is T2 for traffic derivation and established transcript identity.

No signature includes itself and no Finished includes itself. All complete share bytes, party roles, service/purpose/operation/scope/instrument, key IDs/epochs, expiration and negotiation are committed before key confirmation. Full transcripts with labels must be length-tested <=4096; do not rely on window-search identity checks in place of ordered field equality.

Fixed local pin purposes: qpr-pq-1/endpoint-v1/initiator and qpr-pq-1/endpoint-v1/responder. Audience is exactly the OTHER full principal's48 bytes. Use existing COSE common binding with descriptor_digest slot carrying the appropriate transcript commitment under this distinct purpose. Pin receipt principal must equal that flight's expected sender identity; full locally configured key IDs/epoch must match. Each endpoint recognizes its peer's role-scoped pin through actual PinAuthority; no self-supplied certificate trust. The local managed key policy/signing grant is independent and required for each outgoing signature. A prepared commitment is public data, not authority; the host obtains a single-use ManagedPairedSigning for its exact Binding through the existing central facade, and the endpoint checks that binding again before bounded encoding/send.

Key-separation proposal: preserve derive_handshake_keys as the existing hybrid base schedule at T0. Add profile-specific HKDF calls under the schedule owner, using each directional base and T0 as salt for32-byte Finished keys with info qpr-pq-1/endpoint-v1/finished/i2r and .../r2i. Use existing finished_mac/verify_finished with correct role and T0/T1 respectively. After both tags are known, derive32-byte traffic keys from those directional BASE keys and T2 salt using info qpr-pq-1/endpoint-v1/traffic/i2r and .../r2i. Do not use an AEAD traffic key directly as the Finished HMAC key. Base, Finished, KEM and DH arrays are immediately transferred to scoped zeroizing storage and cleared after last use; no Copy output from the endpoint.

These are proposed Qualia profile choices, NOT TLS compatibility/security qualification. The reviewed precedent is that received Finished must be validated before application data and uses a derived Finished-specific key ([RFC8446 section4.4.4](https://www.rfc-editor.org/rfc/rfc8446.html#section-4.4.4)); HKDF info binds application/context ([RFC5869 section3.2](https://www.rfc-editor.org/rfc/rfc5869.html#section-3.2)). Existing Qualia crypto owners implement all required primitives. Independent vectors and protocol review remain required.

## Progress, fragmentation and current authority

Each role has its own non-Copy state; endpoint steps send or receive at most one actual Bearer frame, preserving caller-owned output/reassembly/cursor across WouldBlock. Strict expected-phase checks, one active message, exact fragment total/offset/length and MAX_FRAGMENTS admission precede copying. First profile deliberately accepts only contiguous ordered fragments; duplicates/reordering fail closed rather than claiming datagram reliability. No new retransmission or congestion protocol. Client AND server hello flights use1280-byte floor: selection is not authenticated at the initiator until ServerHello is fully reassembled. InitiatorFinished and subsequent traffic use selected MTU<=1500. MTU below1280 refuses. Changed bearer profile/scope epoch/observed source or partial datagram send aborts; reconnect requires a new endpoint, nonces, leases and reassembly generation. Actual source metadata is transport-bound, never claimed by payload.

Caller-owned workspace includes existing bounded reassembler, <=8192-byte inbound/outbound flights, split frame array/cursor, Transcript and COSE workspace. Declare exact size and require the existing parent resource reservation to cover it; do not introduce a second ledger. Lease use is through existing ephemeral/secret owners; transport resource admission remains existing runtime ownership. No hidden heap or await loop.

Proposed local authority adapter binds a reference to ONE std::sync::Mutex<AuthorityOwner> at explicit local grant adoption (not another authority registry). It installs the exact endpoint binding via existing installer and retains its private handle. The same concrete mutex is locked only for short current issuance/terminal effect boundaries; host revocation between ticks is possible through that same owner. A raw copied permit cannot activate an endpoint. Pin owner is supplied per step, but private receipt incarnation validation rejects foreign substitution. Central SigningControl cancellation/rotation remains separate and checked at managed signature publication. Endpoint cancellation is terminal and clears ephemeral/derived material/reassembly; expired or late messages cannot revive it.

Fresh host time, current pin receipt and same-owner issue_permit are checked before each outgoing flight, peer acceptance and final installation; recheck after expensive crypto and immediately before a bounded external send/activation. Final send, installation and established use hold the SAME owner lock across initial issuance/verification, a SECOND fresh clock sample/current issuance, and the bounded effect. Receipt validity is checked against both samples. Lock never spans waiting for a peer. Trusted host clock is not a wire boolean. Local grant adoption is not remote Share/consent/Sentinel qualification.

Installed-key constructor extension must verify the RECEIVED peer tag using the local receive-Finished key and the correct T0/T1 before constructing InstalledSessionKeys, without a confirmation bool argument. Direction mapping is i2r-send/r2i-recv for initiator and reversed for responder. A private established wrapper retains full T2, exact operation/parties/service/purpose and its concrete authority association, and creates SessionBinding only from fresh issuance. No public into_parts/raw-confirmed shortcut. Existing generic require_keys weakness is not silently declared repaired; endpoint integration enforces its additional association. Retiring the local grant waits until this wrapper's synchronous uses end; future application actor transfer requires its actual drain owner and is a separate gate.

## Approved exact source files

- NEW net/qdnf/session/handshake/endpoint/{mod,types,intent,wire,transcript,transport,authority,resource,common,initiator,responder,established,tests,negative_tests,matrix_tests,state_tests,fixture_tests}.rs. If an implementation approaches500 lines split by lifecycle, not a new service subsystem.
- NEW net/qdnf/crypto/handshake/shares/leased.rs: lease-based overloads reuse parent's existing validation, KEM adapter and EphemeralKeyLease DH. Root adds child routing/reexports in shares.rs.
- NEW net/qdnf/crypto/schedule/endpoint.rs: reviewed profile key separation and zeroizing state over existing hybrid schedule/HKDF. Root adds routing in schedule.rs.
- NEW net/qdnf/authority/verified/received_finished.rs: boolean-free constructor verifies received peer Finished; root adds routing in verified.rs. Legacy constructor/callers remain unchanged.
- Root owns handshake/mod.rs routing, all parent reexports, Cargo/features and task status. No build by this worker.

First fixture must independently drive initiator and responder over real ipc_pair with separate central managed key owners, policies, role pins, authority owners and local ephemeral generation. Alternate one-frame steps; delay either peer to prove no shared-call completion. Assert neither endpoint exposes installed keys before received peer Finished, wrong/withheld/reflected/replayed Finished denies, wrong role/identity/audience/service/purpose/operation/scope/key epoch/negotiation/share/proof changes deny, cancellation/revocation/rotation/deadline between ticks denies, foreign pin owner denies, WouldBlock resumes without re-sign/re-key/resending admitted bytes, malformed/overlap/trailing/late fragments do not mutate accepted state, source/epoch change destroys endpoint, secret/public workspace cleanup on error/unwind, 0-RTT rejected, output budgets exact, and legacy tests remain unchanged. Independently encode transcripts and Finished/HKDF vectors; do not label live random KEM agreement a frozen conformance vector.

Exact frame-budget arithmetic at proposal: final Transcript including both Finished values is 3389 bytes (<4096); outer magic is 21 bytes; maximum ClientHello6408 bytes, ServerHello5922 bytes, InitiatorFinished182 bytes (all <=8192). These are layout calculations, not provider execution or protocol qualification. Existing paired COSE wire retains its qbdl-stream-pq-1 binding profile and external AAD unchanged; the endpoint outer profile, transcript labels and role-specific purpose separate this reuse.

## Inspected source fingerprints

Working-tree SHA256 at proposal time, source inspection only:

- `net/qdnf/session/handshake/fragmented.rs`: `b5110f3ddd0dee516dd5f4d5f2916cb1360539c67022b319e3d68a828cacf922`
- `net/qdnf/session/handshake/hello.rs`: `21624209475dd903b1e582b7426e4961bd681d125d06aed890645c8e19913e07`
- `net/qdnf/crypto/handshake/shares.rs`: `ceb9db4e66785fbcc716744ead84caa6f70dedcd10e9f0704ff7595a131a64ea`
- `net/qdnf/crypto/schedule.rs`: `ae0b31e5400ac1ebbe4b8cbf1e93704a447512c622dce6887e9d8e118bfee317`
- `net/qdnf/crypto/finished.rs`: `5d3f9e20cfeb53c66e23ff26f2bb55c28a1b29c5945b98c33a9f65c82f37f75a`
- `crypto/network/transcript.rs`: `6d8c7a1f206c117afb31d43d00997f24ccfdfd0f32a377a6089e205502860806`
- `net/qdnf/authority/verified.rs`: `a7501f57da1fcde143487d5d2cc81fc97446486540fd7f3c4ca04dd46989e3b8`
- `net/qdnf/bearer/fragment.rs`: `21b1ff5c51c341e81807b27b35ce9911b87805c19fbc4d22e7210d039559edba`

## Implementation handoff (source evidence only)

Both roles own only local ephemeral material. Managed COSE signing is required in production. EndpointAuthority retains one concrete Mutex<AuthorityOwner> from local adoption and locks it for current receipt/time checks plus one bounded send or key activation. PinAuthority stays independently mutable between steps; received role proofs create private incarnation-bound receipts. From-received-Finished installation verifies the peer tag again inside the final current-authority guard and accepts no confirmation bool. Both roles expose only EstablishedSession with guarded current use, not raw installed keys as a transferable permit. Initiator Ready means its own final frame was accepted by Bearer; it is NOT remote acknowledgement or proof of responder installation.

EndpointReservation uses the ACTUAL parent ReservationLedger transient reservation (128KiB, work16, IO24) and retains its handle until endpoint/established state closes. COUNTED_ENDPOINT_BYTES statically counts workspace (including reassembler/frame arrays), BOTH endpoint state representations, central ManagedPairedSigning's retained provider key representation, and24KiB local transient allowance; compile-time assertion requires <=128KiB. This conservative combined size bound is not a provider stack/heap benchmark: FIPS/KDF provider-internal temporary storage and managed library's existing long-term store remain separately owned/provider qualification. Cached fips204-0.4.6 PrivateKey stores its precomputed s1/s2/t0 polynomials, so its actual size_of is included rather than assuming serialized4032 bytes.

A failure/unwind guard destroys endpoint ephemeral/derived leases and scrubs workspace even when a host catches the panic and retains the endpoint. EstablishedSession current-use failure/unwind drops installed keys and releases its parent reservation. Mutex poisoning fails closed; owner retirement cannot pretend to succeed through a poisoned lock. No asynchronous session-drain shortcut is provided. No tests/builds were run by this worker; rustfmt parsing/formatting passed. Eleven authored tests cover independently alternating separate endpoint steps through actual ipc_pair and central key owners, received Finished/current permit recheck, cancellation, panic cleanup, early install, real reservation refusal, tampered proof/share/negotiation/intent, observed source/scope, trailing/duplicate/out-of-order fragments, wrong/reflected Finished, real signed wrong-role/commitment proofs, revocation/foreign pin during setup, actual valid signed flight then local revocation before any send, WouldBlock cursor preservation, and second-clock expiry before send/installation. The actual Intent codec, offer/selection helpers and transcript::client are compared with independently generated endpoint-v1.json bytes/digest; fixture shares/proofs are explicitly synthetic. Authored coverage is not execution evidence or protocol qualification. Rotation, broader transport/provider resource measurements, remote policy integration and independent full captured-flight verification remain release gates.

Root parent routing: handshake/mod.rs pub mod endpoint; crypto/handshake/shares.rs pub(crate) mod leased; crypto/schedule.rs pub(crate) mod endpoint; authority/verified.rs mod received_finished. Existing parent routers/Cargo/legacy helper were not edited by this worker.

Workspace reuse calls the existing lose_interface/reconnect path and preserves its checked generation increment; it does not replace reassembly with generation1. The24KiB local transient allowance covers the existing single-slot clearing value plus bounded profile/codec temporaries, separately from provider internals.

## Endpoint implementation source snapshot

SHA256 after negative-matrix and independent fixture test additions; source/format evidence only, no worker build.

- `net/qdnf/session/handshake/endpoint/authority.rs`: `ff37ec66f6d2f5cd36010c667267386a03bbb13306261e54d9699c87f009dd3a`
- `net/qdnf/session/handshake/endpoint/common.rs`: `2cfb918091f5f1f30296fa6c166eca71512caa7e94552a100ae0c06a830e5fe7`
- `net/qdnf/session/handshake/endpoint/established.rs`: `0d959c06d924363406ce2117d3a2aa963da133c56665bd449cb3a28948fa4eb2`
- `net/qdnf/session/handshake/endpoint/fixture_tests.rs`: `4ed2400079a7d9d8175c03c25bc2fee84d403a8b094b46a09baa4308b9f01ecd`
- `net/qdnf/session/handshake/endpoint/initiator.rs`: `d135405383d5937d010d366e85b9461062eb6285f9f554b5dbac4234d174db43`
- `net/qdnf/session/handshake/endpoint/intent.rs`: `51977d1c7022f89f0bccce303a418c1f99146fe41dcf277301779a65b33c04e4`
- `net/qdnf/session/handshake/endpoint/matrix_tests.rs`: `3dc3aecc89f927c71ffc045d4a36ecb4a40a977aa8db50db14fa1add8c402c93`
- `net/qdnf/session/handshake/endpoint/mod.rs`: `703c4f137e4879580a7645d12b9c97cf1a5282fd493a2912ef6394d230943acb`
- `net/qdnf/session/handshake/endpoint/negative_tests.rs`: `441670a687179f93b26b47347ae048d94237a624d854cabf0b5b339f467491df`
- `net/qdnf/session/handshake/endpoint/resource.rs`: `24826d3107a12fca9c457f7f91d157f4fe8a41c0de531c06b6d720e883889f76`
- `net/qdnf/session/handshake/endpoint/responder.rs`: `4157b2bd561b6e099b4394d3b69544e60ea7d919a48d7fc9677cae17a4b9b343`
- `net/qdnf/session/handshake/endpoint/state_tests.rs`: `3ea0dd33049a671f66f774d1ce7dfdb1078fd22ae6902159e85aaf6d945ea1b5`
- `net/qdnf/session/handshake/endpoint/tests.rs`: `1f3e2ae06f400902a12f2266b9fdeec7b4bebb681741ed65507e768a2697fe9d`
- `net/qdnf/session/handshake/endpoint/transcript.rs`: `0fbc751e830eaef69f8b1203046194995e27fb69e1269726cc1f871ae70a4fe7`
- `net/qdnf/session/handshake/endpoint/transport.rs`: `04ca01903f8c7ce9329f73f0b289d7e6cc4db71e3ed50ec9884d8b091513fbc7`
- `net/qdnf/session/handshake/endpoint/types.rs`: `8b96210f2a97ae5af804a7baaf6ac7475fdb4125f109fd34b858f0a7ea62b2c2`
- `net/qdnf/session/handshake/endpoint/wire.rs`: `ffc5beb0cd768652d20d9535c8fb6422c5d587c48698ae97d9b5f89480f78739`
- `net/qdnf/crypto/handshake/shares/leased.rs`: `5bffb9bb7c3d2fa0f3dd926f3dd09d72e3dcd3e6aa8b19df2a323bed4daef0f9`
- `net/qdnf/crypto/schedule/endpoint.rs`: `f320f30ef99aa83ae992a9c4097a8c08ec5dd7193eb1c58c4061eb94c4e062b3`
- `net/qdnf/authority/verified/received_finished.rs`: `322b353c04840aacbc8126c69ab85224e6be30fe1f18027d43f4b3b83c7cd897`
