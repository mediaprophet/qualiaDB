# QBS-10 protected-object v1 — exact contract candidate

2026-11-12. Root-requested freeze candidate after qbs10-protection-contract-review.md. NOT implemented, executed or security-qualified. This replaces that review's proposed 16 GiB whole-entry ceiling: one entry may be up to 1 TiB, with distinct derived AEAD keys for at most64 segments of16 GiB each. Root approved metadata-only recipient wraps (<=16), encrypted private commitments/gaps, existing central recipient custody, and no-key/no-tag empty entries. Exact bytes/API below await root review before source edits.

## Ownership, profile and confidentiality boundaries

Profile string: qbdl-stream-protected-v1. Suite1 uses existing SHA384, HKDF-SHA384, ChaCha20-Poly1305, ML-KEM-768 and X25519 adapters. Paired proofs remain existing managed ML-DSA-65 + Ed25519 COSE. Legacy QASD plaintext descriptors, COSE binding bytes, KeyVault subgraph envelopes and QSession handshakes remain unchanged. These are application profile codecs, not new crypto primitives or an OAE2/nOAE2 claim.

Public cache objects are ONLY protected context/geometry pages, protected descriptor/root pages, opaque ciphertext leaves/proofs and a recognized publisher paired proof. They carry no recipient records/wraps, recipient IDs, source/header/index/plaintext hashes, original entry names/types/ordinals/offsets, private semantic/policy/consumer hashes, or grant graphs. An independently random opaque scope token is public; it is not a private policy hash. Lengths, object count, generation/predecessor links, access patterns and publisher signing-key linkage remain observable. No padding/anonymity claim is made.

Private recipient authorization, private semantic material and metadata-master wraps travel ONLY through the existing QSession packet-protection owner under a confirmed endpoint and current local authority. TLS/WSS alone, authentication without record encryption, a wire confidentiality flag, or a caller-provided verified boolean does not admit this profile. No public cache/upload API accepts the private delivery type. Trusted host code can copy bytes exposed to it; this is not confinement against that host.

## Common canonical encoding and limits

All unsigned integers are BIG ENDIAN; fixed-width arrays are literal bytes; no alignment padding, text terminators beyond the explicitly written NUL, optional aliases or trailing bytes. Every magic below is12 bytes, including one final NUL. suite is u16=1, flags/reserved are zero unless explicitly defined. Unknown suite/flags reject. SHA384 hashes exact complete canonical encodings with their magic; no truncated digests. All arithmetic and aggregate lengths are checked before allocation/copy. No wire field authorizes memory by itself.

Bounds: original entries0..4096, protected payload objects0..8193, metadata logical length1..8,388,608 bytes, every payload object0..2^40 logical bytes, original source<=2^40 bytes, leaf16384 bytes, tag16, segment2^20 leaves, <=64 segments/object, <=16 attempted recipient wraps/publication (failures count), <=128 rows/page, <=65 pages, <=8192 bytes/private package, <=1024 bytes/private record chunk. Metadata is object ID0 with role0. Payload/gap/empty-entry object IDs are distinct nonzero random u32 values with role1, sorted numerically in public pages; they are not original entry IDs or ordinals. Select IDs through bounded collision checking using existing OS entropy; entropy failure/16 collisions refuses the publication. There may be8193 protected payload rows PLUS the separate metadata row. Gap objects count toward8193. The plaintext source limit remains4096 entries; protected outer limits do not alter plain descriptor constants.

Source interval proof: index/admit.rs::scan bounds each original entry to [64,index_offset) and admit_index sorts by offset/end, rejects overlap among positive-length ranges, and skips empty ranges without moving the occupied frontier. Header [0,64) and suffix index [index_offset,source_length) are fixed. For N<=4096 original rows and M<=N nonempty entry intervals, their complement between header and index has at most M+1 maximal positive-length gaps. Empty entries create no additional gaps. Therefore total protected rows = N + gaps <= N+M+1 <=2*4096+1=8193. This upper bound is reachable with 4096 short positive entries separated by alignment padding, including a gap after the header and before the index; source admission does not require these gaps to be absent. Keep one object per maximal gap, not arbitrary subdivisions. ceil(8193/128)=65 pages; last maximum-size page has one row. Protected Context max=164+65*56=3804; Descriptor max=18+3804+48+2+65*56=7512. Per-page maxima1560/6728, recipient/wrap/package sizes and all crypto domains are unchanged.

A publication uses a fresh32-byte nonzero OS nonce, fresh32-byte nonzero opaque scope token and fresh independent32-byte master per NONEMPTY payload object plus one metadata master. Masters are private managed state, never AEAD keys. No nonempty master may equal another master or the metadata master. All-zero draws/duplicates fail or redraw within a fixed admitted bound; no deterministic entropy fallback. No master/key import or writer-state restore API is available in this profile.

## Public context and geometry

Geometry page G:

    magic = "QBDL-PGE-V1\0"
    suite:u16 | page:u16 | first_row:u32 | row_count:u16 | reserved:u16
    rows[row_count] = object_id:u32 | logical_length:u64

Header24 bytes; maximum1560 bytes. page is0-based; first_row=page*128; all nonlast pages have128 rows; the last has the exact remainder. IDs strictly increase globally. Zero-length rows are allowed only for empty original entries (validated privately later). G contains no context hash, so computing its hash cannot cycle.

Context C:

    magic = "QBDL-PCX-V1\0" | suite:u16 | flags:u16
    publication_nonce:[32] | opaque_scope_token:[32]
    generation:u64 | content_key_epoch:u32 | predecessor_descriptor:[48]
    payload_object_count:u32 | metadata_logical_length:u64
    leaf_bytes:u32=16384 | segment_leaves:u32=1048576
    geometry_page_count:u16 | reserved:u16
    refs[geometry_page_count] = page:u16 | row_count:u16 | encoded_length:u32 | SHA384(G):[48]

Fixed header164 bytes; ref56; maximum3804. page_count=ceil(object_count/128), including zero pages for zero payload rows. generation0 requires an all-zero predecessor; later generations require a nonzero predecessor. content_key_epoch>0. Metadata ID/role are fixed and not encoded as a mutable field. ContextId=SHA384(C). All geometry and declared lengths are frozen BEFORE encryption. Metadata serialization length is calculated from fixed-width row counts and original header/index bytes before any roots are known; the metadata codec below contains no resulting ciphertext root.

## Segment keys, nonce, AAD and empty objects

For an object with role R (u8), ID E (u32), length L (u64), context Ctx[48], generation Gen(u64) and content epoch Epoch(u32): leaf_count=ceil(L/16384); segment(index)=floor(index/1048576). Nonempty index must be<leaf_count, segment<64 and L<=2^40. The segment number is u32 everywhere in crypto framing.

SegmentInfo is exactly116 bytes:

    ASCII "qbdl-stream-protected-v1/segment-key\0" (37 bytes)
    suite:u16 | Ctx:[48] | R:u8 | E:u32 | Gen:u64 | Epoch:u32 | segment:u32 | L:u64

Ksegment = HKDF-SHA384(salt=Ctx, IKM=master32, info=SegmentInfo, out_len=32), using the existing hkdf_sha384. Only Ksegment is passed to AEAD. The master is never passed to AEAD, wrapped as a leaf key, or exported. Derive at most the needed current segment into a scoped zeroizing lease; clear it on segment change, cancellation or failure. Role/id/length/epoch/segment checks are generated from the authenticated frozen context, not independently accepted caller fields.

Nonce is exactly twelve bytes: 00 00 00 00 || u64be(global_leaf_index). The GLOBAL index is used even when a new segment starts; it is not reset to a segment-relative nonce.

LeafAAD is exactly126 bytes:

    ASCII "qbdl-stream-protected-v1/leaf-aad\0" (34 bytes)
    suite:u16 | Ctx:[48] | R:u8 | E:u32 | Gen:u64 | Epoch:u32 | segment:u32
    global_leaf_index:u64 | L:u64 | this_plaintext_length:u32 | final_leaf:u8

this_plaintext_length=min(16384,L-index*16384), positive; final_leaf is1 iff index+1==leaf_count, otherwise0. Ciphertext leaf is ciphertext[this_plaintext_length] || detached_tag[16]. It is hashed by the EXISTING QBS-05 TreeSpec(context=Ctx, entry=E, generation=Gen, representation=Ciphertext, epoch=Epoch, length=L). Metadata uses exactly the same framing with R=0,E=0; payload uses R=1,E>0. Role is implicit in the reserved ID in proof geometry and explicit in encryption/KDF. Each segment encrypts <=2^20 leaves/2^34 plaintext bytes; the1 TiB proof limit permits64 separately derived segment keys. This cap is an engineering usage bound, not a quantified multi-user authenticity theorem.

For L=0: leaf_count=0, segment_count=0, no master, no nonce, no AEAD invocation, no tag, no wrap and no valid leaf index. Public root MUST equal existing proof::empty_root for the ciphertext TreeSpec. Private metadata marks master_present=0 and master bytes all-zero; its authenticated mapping establishes the empty entry. No tag-only synthetic leaf is introduced. Empty results reach a consumer only through successful metadata/publisher/current-use checks.

## Public protected descriptor and roots

Root page P:

    magic = "QBDL-PRT-V1\0" | suite:u16 | ContextId:[48]
    page:u16 | first_row:u32 | row_count:u16 | reserved:u16
    rows[row_count] = object_id:u32 | ciphertext_root:[48]

Header72 bytes; row52; maximum6728. Same page boundaries and ID sequence as G. Each empty row root is checked immediately against its canonical empty TreeSpec.

Protected descriptor D:

    magic = "QBDL-PCD-V1\0" | suite:u16 | flags:u16
    context_length:u16 | C[context_length]
    metadata_ciphertext_root:[48] | root_page_count:u16
    refs[root_page_count] = page:u16 | row_count:u16 | encoded_length:u32 | SHA384(P):[48]

root_page_count matches geometry_page_count. D maximum7512 bytes; context_length is exact canonical C length. DescriptorId=SHA384(D). Validate all page refs/counts/rows and context equality before constructing a structural candidate. A structural candidate alone carries no publisher authority.

Recognized publisher COSE uses existing Binding(descriptor_digest=DescriptorId, controller_epoch=publisher signing epoch, key_ids=recognized full paired key IDs, purpose=ASCII "qbdl-stream-protected-v1/publisher", audience=opaque_scope_token[32]). The selected pin must already have this exact recognized local purpose/audience. A signature under arbitrary wire keys is insufficient. Verify/recheck its actual PinAuthority receipt at current use; never infer descriptor roots from the private whole-source hash alone.

## Encrypted metadata M

Plain metadata is private key-bearing state, never a public artifact or general Debug/Clone value. Canonical encoding:

    magic = "QBDL-PMD-V1\0" | suite:u16 | flags:u16 | ContextId:[48]
    original_version:u16 | original_flags:u16 | original_length:u64 | original_SHA384:[48]
    original_index_offset:u64 | original_header_length:u32=64 | original_index_length:u32
    closure_manifest_SHA384:[48] | policy_SHA384:[48] | consumer_SHA384:[48]
    mapping_count:u32
    mappings[mapping_count] (108 bytes each):
        object_id:u32 | original_offset:u64 | logical_length:u64
        kind:u8 | original_ordinal:u32 | master_present:u8 | reserved:u16
        plaintext_SHA384:[48] | object_master:[32]
    original_header[64] | original_index[original_index_length]

Header288 bytes. mapping_count exactly equals public payload_object_count; rows use the same globally sorted opaque IDs and exact lengths as G. kind1=original entry, kind2=gap; only these two values. Gap ordinal=0xffffffff and length>0. Entry ordinal is the exact original parsed index ordinal. master_present=1 iff length>0, otherwise0; all-zero or duplicate nonempty masters and equality with the metadata master reject. A nonempty row's hash is exact SHA384 of its original object bytes; empty row hash is SHA384(empty). Validate original header/index with the existing QBDL parser and its limits; no second parser semantics. Original version/flags/index geometry/source digest match the privately authenticated closure/source scope.

Entry rows exactly cover every original index entry. Gap rows exactly cover remaining bytes between original header, entries and suffix index. Header64, suffix index and nonempty row intervals form a complete disjoint partition [0,original_length); empty entries are checked against original index but do not add a coverage interval. Check this with bounded sorted workspace/nonrecursive iteration; refuse overlaps, holes, duplicate ordinals, undeclared bytes, overflow or more than8193 object rows. Gap content is encrypted normally, never exposed as presumed padding. Metadata total288+108*rows+64+index_length MUST equal declared metadata length and be<=8MiB before allocation. Source digest/immutability reuse the actual publisher source owner; roots do not prove whole-source SHA384 until full reconstruction is checked.

Private closure artifacts and the existing signed closure manifest are delivered through the private carrier or already trusted local owners; M contains only their exact private references. Before metadata/plaintext acceptance, existing closure authentication checks the signed full source digest/length, actual policy and consumer membership/reachability, recognized issuer pin and current grant. Merely decrypting M does not evaluate rights. Expected private policy scope is admitted before unwrap; wire M cannot choose its own authorization policy.

## Recipient authorization record A (PRIVATE ONLY)

A is1856 bytes:

    magic = "QBDL-PRA-V1\0" | suite:u16 | flags:u16
    DescriptorId:[48] | ContextId:[48] | recipient_principal:[48] | publisher_principal:[48]
    service:[48] | purpose:[48] | instrument:[48] | scope:[48] | operation:[16]
    closure_manifest_SHA384:[48] | policy_SHA384:[48] | consumer_SHA384:[48]
    original_source_SHA384:[48] | original_source_length:u64
    recipient_key_epoch:u64 | not_before_unix:u64 | expires_unix:u64
    recipient_ML_KEM_768_public:[1184] | recipient_X25519_public:[32]

All required digests/identities, operation and epochs are nonzero; expiry>not_before. Exact expected full source/scope, descriptor/context, parties and operation are compared, never inferred from a compact hash. AuthId=SHA384(A). A recognized issuer paired COSE signs AuthId with purpose ASCII "qbdl-stream-protected-v1/recipient-authority" and audience=publisher_principal[48]. Signing controller epoch belongs to that issuer, separately from recipient_key_epoch. Local trusted pin installation recognizes the issuer role/purpose; the record cannot self-install or self-declare this authority. A recipient self-signature under an otherwise unrecognized pair does not grant disclosure. This proof binds BOTH actual recipient public keys, full source/policy/operation scope and validity; actual current application grant/consent remains a separate required owner.

Recipient key IDs are computed exactly:

    MlId = SHA384(ASCII "qbdl-stream-protected-v1/recipient/ml-kem-768\0" || u16be(1184) || ml_public)
    DhId = SHA384(ASCII "qbdl-stream-protected-v1/recipient/x25519\0" || u16be(32) || dh_public)

These are profile key identifiers, not COSE_Key encodings or a relabel of signing key IDs. ML-KEM public validation uses the existing provider; X25519 low-order/all-zero shared results reject during DH. The receiver derives public keys from its actual managed private material and matches these full IDs and algorithm/epoch policies.

## Hybrid metadata-master wrap (PRIVATE ONLY)

One fresh sender ML-KEM encapsulation plus one fresh OS X25519 ephemeral lease is used per wrap attempt. Existing adapters produce kem_ss32, dh_ss32, kem_ct1088 and eph_public32. Never XOR shared secrets. Existing SecretLease/Zeroizing owners hold intermediates and erase on every exit/unwind. The pair is used only for this one envelope.

WrapContext Wc,1438 bytes:

    magic = "QBDL-PWC-V1\0" | suite:u16
    AuthId:[48] | ContextId:[48] | DescriptorId:[48] | recipient_principal:[48]
    recipient_key_epoch:u64 | metadata_logical_length:u64
    MlId:[48] | DhId:[48] | eph_X25519_public:[32] | ML_KEM_ciphertext:[1088]

KEK = HKDF-SHA384(salt=SHA384(Wc), IKM=kem_ss32||dh_ss32, info=ASCII "qbdl-stream-protected-v1/metadata-wrap-key\0" (43 bytes), out_len=32). Encrypt ONLY metadata_master32 with existing ChaCha20-Poly1305, nonce=[0;12], AAD=exact Wc. The KEK is fresh and one-use; no other value is encrypted under it. Wrap construction happens AFTER D/AuthId are fixed, and wraps are not hashed into C/M/D, so descriptor binding introduces no cycle.

Envelope W,1328 bytes:

    magic = "QBDL-PWR-V1\0" | suite:u16 | flags:u16
    AuthId:[48] | MlId:[48] | DhId:[48]
    eph_X25519_public:[32] | ML_KEM_ciphertext:[1088]
    encrypted_metadata_master:[32] | tag:[16]

Reconstruct Wc from authenticated A, recognized D/C and W; exact field agreement is mandatory. Any mismatch, failed decapsulation/tag, late cancellation/epoch change or expired policy returns no master/capability. Successful unwrap creates a private DECRYPT-ONLY metadata-master handle; it cannot become a writer or export raw bytes. Metadata then conveys scoped payload masters privately. At most16 wrap attempts are consumed by the publication owner, not reset by copying/reopening a seal handle. Rewrap does not revoke masters already learned; excluding a prior recipient requires fresh masters/ciphertext/context generation.

## Private delivery package and actual encrypted carrier

Package Q:

    magic = "QBDL-PDL-V1\0" | DescriptorId:[48] | request_id:[16]
    auth_length:u16=1856 | A[1856]
    proof_length:u16 (1..4608) | recognized_recipient_COSE[proof_length]
    wrap_length:u16=1328 | W[1328]

Maximum7874 bytes; no trailing bytes. One Q contains one recipient wrap; it is never included in public descriptor pages, public/cache HTTP objects, telemetry or plaintext logs. Private closure packages remain separately bounded by existing closure limits and are not added as unbounded Q fields.

The scoped carrier must reuse session::PacketProtection (AEAD, packet number, replay window), initialized ONCE from a consumed EstablishedSession after fresh current checks. Never reconstruct PacketProtection from the same key bytes for each call: that resets its send packet-number space and repeats nonces. A new endpoint-owned child retains actual PacketProtection, existing endpoint authority/pin receipt association and its reservation; no raw key constructor is exposed by the new API. It must recheck current execution/pin/time before each seal/open and again before send/plaintext acceptance, using the same concrete owner guard. Instantiation consumes the only established capability; cancellation/error/unwind drops record keys and reservations.

Private control record clear header H,88 bytes:

    magic = "QBDL-PCT-V1\0" | endpoint_T2:[48] | request_id:[16]
    package_total:u32 | package_offset:u32 | chunk_length:u16 | flags:u16

flags bit0=FINAL, all other bits zero. total1..8192; offsets contiguous from0; every nonfinal chunk1024 bytes, final exact remainder; at most8 chunks. FINAL iff offset+chunk_length==total. H is the exact PacketProtection AAD. Record wire is H || existing PacketProtection output(pn8 || ciphertext[chunk_length] || tag16). Maximum1136 bytes before existing QFrame carriage, under the1280 floor after the bounded frame header; actual integration must assert this with the existing encoder. The current service/carrier message tag is root-owned QBS08 framing, not a worker-invented registry value. T2/request/total/offset bind the private assembly; private recipient/scope fields are inside ciphertext only.

Sender burns packet number before crypto, retains exact encoded ciphertext on WouldBlock and resends only those same bytes when transport requires retry. Do not call seal again for a retry. Receiver authenticates record/tag/observed route/current owner before copying into admitted private caller-buffered assembly. Strict expected request/offset/total and current generation checks reject replay, overlap/trailing/out-of-order records. Use zeroizing assembly/scratch, not the public generic reassembler as an unexamined plaintext store. Reassembly admission uses actual existing runtime reservation/credit. Completed private Q is parsed only under the guard; no generic public plaintext output publication.

Necessary narrow parent repair: PacketProtection::open currently uses a plain local [u8;4096] decrypt scratch. Change that scratch to Zeroizing with identical AEAD/replay behavior so authenticated plaintext is erased on success/error/unwind. Receiver output must itself be a scoped private guard with fresh acceptance checks; zeroizing internal scratch alone does not provide current-use authority. Confidential-carrier composition must be implemented/tested before claiming remote private wrap delivery. Until then only explicit locally protected provisioning is reviewable; there is no authentication-only fallback.

## Managed API and lifecycle freeze

Names below are proposed public interfaces; exact module exports remain root-owned. Wire views are untrusted until their private validated constructors run. No public generic trait may return Allow or supply a verified boolean.

- CryptographicLibrary::generate_managed_recipient_pair(ml_id, dh_id, security) returns public arrays/IDs only and stores private keys directly in EXISTING KeyManager. ML uses existing KeyAlgorithm::Kyber/fips203 ML-KEM-768 generator; add explicit X25519 variant routed through existing generator and network X25519/OS primitives. Reject implicit replacement; actual paired rotation generates both valid successors and invalidates existing operation generations/recipient recognition. Generic KeyRotator is not a valid ML-KEM pair rotator.
- Strict managed policy uses existing AccessPolicy, AuthenticationMethod, KeyOperation and host-authenticated context. Require explicit Decrypt policy for ML-KEM and Derive policy for X25519; absent/expired/wrong auth/IP/date/key policies deny. Factor the existing strict Sign evaluator into an internal requested-operation helper while retaining the Sign wrapper and legacy checker behavior.
- Reuse/extract SigningControl's concrete mutex generation/cancellation fence as ManagedOperationControl, preserving SigningControl compatibility. A control constructor is not authority. Single-use ManagedRecipientUnwrap borrows the actual KeyManager for its bounded call, checks exact stored IDs/types/metadata/times/serialized lengths/derived public IDs and both policies, computes via existing providers, then takes the actual control lock and second fresh time/current checks before returning only a private decrypt handle. No get_key/get_key_with_access clone, private Key return, public master export or borrowed-key clone is used. Cached handles carry scoped state/fences, not an indefinite manager borrow that prevents revocation.
- Add distinct reviewed object-purpose values to network KeyPurpose (proposed ObjectContent=9, ObjectRecipient=10), retaining all existing values. ObjectContent covers scoped masters/derived AEAD keys; ObjectRecipient covers ephemeral wrap shared/KEK material. Exact domain bytes still separate uses. Do not relabel QSessionAead or ControllerSign grants as object authority.
- ManagedObjectPublication is non-Copy/non-Clone, generated only by the existing central owner and a current whole-source publication admission. It owns one attempt's frozen geometry and private master state in explicitly admitted caller workspace. Begin object -> monotonic global leaf seal -> object finish -> metadata finish -> descriptor sign -> bounded recipient wraps; no backward transition or imported counter. Public callbacks receive only ciphertext/roots, never masters. Existing central generator supplies bounded cold master generation; retain scoped SecretLease state and zeroize temporary Key buffers before drop. No long-term storage registry is created for ephemeral content keys.
- Each seal validates capacity/context/index first, burns expected leaf index BEFORE encryption or sink callback, poisons before callbacks, and erases on any error/unwind/late cancellation. Failed sinks never authorize re-encryption. Private metadata/master buffers remain private and are zeroized after ciphertext and required wraps are durably handed off or operation aborts. Crash recovery may replay exact persisted ciphertext only; otherwise start a wholly new attempt. Durable restart under the same master/counter is unsupported.
- A managed decrypt-only object handle binds context, ID, role, length, generation, epoch, recognized publisher receipt and private whole-source authorization. The provider adapter must not accept arbitrary raw arrays as keys. Current-use path verifies exact ciphertext proof/root first, derives the required segment, decrypts into private scratch, validates the tag, and samples fresh time/rechecks actual owners again before one bounded consumer callback. Whole scratch is zeroized on error, cancellation, panic and callback completion. No asynchronous borrowed plaintext or permission snapshot escapes. A failed tag poisons that handle; repeated new unwrap admissions remain governed by actual shared runtime limits, not a claimed global cryptographic attempt cap.

Source ownership correction required for policy composition: existing LocalWholeSourceGrant retains a concrete BoundClosure built from plaintext CandidateIdentity/SourceCommit. Do NOT fabricate those public plaintext types for protected D. Add a narrow POLICY-OWNED protected-source binding/current-use child or share its private grant core, accepting authenticated D plus privately authenticated M/source/closure references and exact coverage. It must retain the same actual AuthorityOwner and actual issuer/publisher/recipient receipts; role/scoped local trusted adoption remains explicit. Existing Sentinel/labels/consent/remote Share integration is still not claimed until its actual owners are connected.

New-algorithm dispatch review: append X25519 without renumbering existing serialized variants. Existing generic SignatureEngine::sign_data and verify_signature have unmatched-algorithm Ed25519 fallbacks. Explicitly refuse X25519 and Kyber in both; NTRU is also a KEM alias in the generator and should likewise refuse signing rather than enter this fallback. Regression tests preserve all real signing algorithms/default bytes. Generator dispatch/helper, policy helper and scoped owner children are the concrete minimal changes; no new vault or crypto subsystem.

## Acceptance gates and source claims

Proposed NEW files: crypto/network/object_stream/{mod,types,context,segment,aad,recipient,wrap,tests}.rs; specialized_libs/cryptographic_library/key_management/managed_object/{mod,recipient,publication,seal,open,tests}.rs; library/managed_object.rs; key_management/managed_control.rs; bundle/stream/protection/{mod,context,descriptor,metadata,delivery,authenticated,current_use,tests}.rs; endpoint/protected_channel.rs and focused tests. Minimal existing-parent claims: key_management/{types,generator,manager,managed_signing}.rs; access/managed_policy.rs; signing.rs; crypto/network/types.rs; policy-owned protected-source/shared-grant adapter; session/packet_protection.rs zeroizing scratch only. Root owns every router/export/Cargo/registry and must approve these exact claims before implementation. Keep focused implementation files<500 lines and cold provisioning separate from zero-heap leaf operations.

Required tests: independent canonical codec hashes/lengths; full segment boundary indices1048575/1048576 and maximum67108863, no counter reset, same master/different segment-role-context-epoch separation; empty no-key semantics; exact source/gap coverage; confidential descriptor inspection;16 wrap cap and private-carrier-only delivery; real managed KEM/DH custody/policies/algorithm/refusal; recipient record key/scope mismatch; received wrap tag failure and late cancel/rotation; packet-owner one-shot initialization/retry/replay; producer sink failure/unwind and crash-new-attempt behavior; full scratch/master erasure; zero callback before current authority; actual resource reservation and no hot allocations. Independent Node/OpenSSL AEAD/HKDF/ML-KEM vectors supplement actual owner tests; they do not replace them. Root runs builds.

Standards rationale: [RFC 8439 §2.8 and §4](https://www.rfc-editor.org/rfc/rfc8439.html#section-2.8) supplies the existing AEAD/tag/nonce rules; [RFC 5869](https://www.rfc-editor.org/rfc/rfc5869.html) supplies existing extract/expand and context binding. Segmentation, hybrid wrap, disclosure policy and confidentiality composition are Qualia profile decisions requiring independent review; no theorem, universal security level, physical device qualification or complete remote governance claim follows from these codecs.


## Phase A implementation boundary (2026-11-12)

The new crypto/network/object_stream codec is deliberately structural. Inventory authenticates neither publisher nor source. The protected bundle compositor MUST (1) verify recognized current publisher paired proof over the exact protected descriptor, (2) check every zero-length object root with existing bundle::stream::proof::empty_root and ciphertext TreeSpec binding, (3) match all geometry/root pages and exact private original-source coverage, (4) verify private metadata source/closure/policy/consumer commitments and original bytes, (5) verify recognized recipient paired proof binding both KEM keys and full scope, and (6) recheck actual current application authority before disclosure. Metadata cannot be empty. The crypto codec must not import bundle proof ownership merely to hide these remaining composition checks.

Central managed operations now use the existing KeyManager, KeyGenerator, KeyStorage, policies and provider adapters. ManagedOperationControl is the extracted existing SigningControl fence; SigningControl remains a compatibility alias. New X25519 is appended to KeyAlgorithm and routes existing network X25519, while generic signing explicitly refuses X25519/Kyber/NTRU. Generic real signing profiles and prior signature bytes are unchanged. Recipient provisioning returns public keys only; unwrap borrows stored private key bytes and checks their algorithms, metadata, full public IDs, mandatory Decrypt/Derive policies, trusted local authentication context, time, cancellation and generation before and after cryptography.

ManagedObjectPublication owns one attempt in caller-charged PublicationWorkspace (size_of is the actual budget minimum). It generates context nonce/scope and private masters through existing entropy/key generation, permits monotonic leaf sealing, never uses a master directly as an AEAD key, and destroys all attempt masters on failure, cancel, unwind or incomplete sealer Drop. Metadata master rows, wrapping and unwrapped metadata stages remain crate-private, with no public master import/export. Provider Write/Encrypt admission is NOT application disclosure authority. Runtime reservation accounting, immutable source traversal, canonical private metadata serialization/validation, publisher signatures, authorized wrap composition and existing authenticated PacketProtection delivery remain subsequent integration, not completed Phase A behavior. A delivery view or caller ProviderGrant is never itself proof of recognized remote authority.

Source tests authored in this wave cover maximum codec bounds, cross-bindings, segment boundaries, genuine managed ML-KEM/X25519 recipient agreement, explicit key policy failures, wrong algorithms/IDs, cancellation/generation/expiry at point of use, tampering, secret/output erasure and panic cleanup. They require the coordinated actual-source build; source inspection/rustfmt are not runtime test evidence.
