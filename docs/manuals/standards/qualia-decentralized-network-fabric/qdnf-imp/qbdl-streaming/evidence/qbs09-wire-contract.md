# QBS09 native service wire/domain freeze — proposed for root approval

2026-11-13. No service codec/registry implementation. This replaces the approximate per-variant section of qbs09-native-service-contract.md; common channel/state source ownership remains separate. Root owns interface/schema/registry changes. Candidate plaintext whole-source profile only; local-admin demonstration does not complete remote Share/Sentinel authority or protected-object QBS10.

## Collision/source checks

Inspected net/qdnf/registries.rs, net/peer/host/services.rs, programme interfaces.md and schemas/qasd-plain-v1.cddl. Exact rg searches across core source and QDNF standards found no existing QBS1/QBDL-SCT-V1/q42:QBDLStream/1 allocation; only prior pause note proposes QBS1. Existing QFrame SessionDatagram64, NextProtocol::QSession5 and their canonical header remain unchanged; service kind tags below are INSIDE this new service namespace, not new global QFrame codes. Existing protected RecordHeader domain QBDL-PCT-V1 plus NUL and8192 limit stay unchanged. SHA384/AEAD/proofs remain their current owners, not new primitives.

Propose magic ASCII QBS1, schema u16be1, profile u16be1 and ServiceId::from_iri(b"q42:QBDLStream/1") through the existing service identity owner. Profile1 means PlainWholeSourceV1; negotiated carrier mode1 ReliableOrdered requires actual adapter/session admission, never a wire assertion. Unknown versions/profiles/modes/tags/flags/classes/statuses fail Unsupported. Root approval is required before these integer bytes are encoded.

## Service prefix208, exact offsets

|Offsets|Bytes|
|---|---|
|0..4|QBS1|
|4..6|u16 schema1|
|6..8|u16 variant below|
|8..10|u16 profile1|
|10..12|flags0|
|12..16|reserved0|
|16..64|full descriptor digest48|
|64..112|full context digest48|
|112..120|publication generation u64|
|120..128|selected host policy generation u64|
|128..136|wire cancellation fence u64 (v1 only0 live,1 terminal)|
|136..152|transfer ID16, nonzero; never reused in this session|
|152..156|entry IDu32|
|156|representation0 plaintext|
|157..160|reserved0|
|160..168|representation epochu64=0|
|168..176|leaf indexu64|
|176..180|countu32|
|180..184|exact body lengthu32|
|184..192|credit serialu64, nonwrapping|
|192..200|credit CEILING bytesu64 for ONE exact response|
|200..208|deadline Unix secondsu64|

All numeric fields big-endian; exact total208+body, no trailing bytes. Descriptor/context always nonzero and selected out of band before disclosure, including Describe; no wildcard/object enumeration. Publication generation may0 per actual descriptor contract. Policy/credit/deadline are claims compared with local admitted state, never a way to assign generations or reserve capacity. Deadline cannot exceed actual endpoint Intent/host grant and is mapped to independent capped local monotonic lifetime <=30,000ms; fresh host time/current policy is authoritative.

Canonical per-variant unused fields: metadata/negotiation/Close have entry/leaf/count0; Select/Open/OpenedGrant have entry ID but leaf/count0; leaf Credit/RequestLeaves/LeafData have count1 and exact selected leaf; Resume prefix leaf/count0 and body list; Cancel/Outcome preserve the exact original request association. Entry ID0 remains valid for a selected entry; variant determines whether the field is used. All unused credit fields0. Describe metadata transfers also have Credit/serial but entry/leaf/count0. A bound transfer's descriptor/context/generation/policy/deadline/entry/leaf values cannot change between Credit/request/response. Grant/Outcome/Cancel are distinct messages, not extra bulk responses under the same credit.

Cancellation fence is protocol association, not a serialized OperationTable CancelEpoch/MutationHandle. Live data/credits use0. First Cancel uses1 and invokes actual canonical local cancellation once; identical repeated terminal Cancel may be acknowledged without another cancel/effect. Late live0 records refuse on terminal state. No value>1 in v1, no reopening canceled transfer, no adoption of peer-provided local generations. Actual local ticket/cancel epoch remains separately checked at every effect/drain.

## Two fixed body records reused by variants

Limits64 offsets: max_record_bytesu32@0; max_page_payloadu32@4; max_cose_bytesu32@8; max_index_bytesu32@12; max_metadata_bytesu32@16; max_closure_bytesu32@20; max_entriesu32@24; max_inflightu16@28; reassembly_lanesu16@30; window_bytesu64@32; max_cold_rowsu32@40; carrier_modeu16@44; flagsu16=0@46; max_logical_extentu64@48; request_lifetime_msu32@56; reservedu32=0@60. Nonzero relevant limits only. Negotiate minima against ACTUAL admitted capacity. Required v1 lanes2 (one data, one control); inflight1..actual runtime maximum, not64 cold rows. Maxima: record65776, page65536, COSE4608, index/metadata each<=8388608, closure1048576, entries4096, cold rows64, extent1099511627776, lifetime30000. Window<=profile8MiB/32MiB and actual parent headroom; these maxima do not allocate memory.

ObjectSelector32 offsets: classu8@0; three zero bytes@1; numberu32@4; TOTAL object lengthu64@8; object offsetu64@16; this chunk lengthu32@24; reservedu32=0@28. Positive chunk length<=65536 and checked offset+length<=total on response. Request total0 means unknown only for the first body request; it supplies bounded requested capacity rather than authority. Once source/manifest commits total, requests/responses must match it exactly. Requests allow a requested capacity, response may use a smaller exact final chunk; it consumes the ENTIRE one-response credit, never leaves reusable leftover bytes.

Object classes:1 QASD body(max65536);2 publication paired COSE(max4608);3 signed closure manifest(max2048);4 closure artifact exact original bytes;5 closure paired COSE(max4608);6 original header(exact64);7 original suffix index(total<=admitted index ceiling, split chunks<=65536);8 context page(max65536);9 entry page(max65536). Classes1/2/3/5/6 number0/offset0 and one complete response;7 number0 and exact contiguous bounded chunks;8/9 number0..31 and offset0/one complete page. Class4 number is canonical full-digest-sorted node ordinal in the ACTUAL decoded signed manifest,0..15; each artifact can be chunked, and manifest+all originals<=1048576. No names, paths, locators or arbitrary object keys on wire.

Closure graph already exists in policy/manifest.rs signed manifest57-byte sorted node rows plus two-byte edge indices. Do NOT invent a second graph serialization. The service needs a narrow policy-owned manifest transport iterator to expose those already-decoded exact digest/kind/length/edge records; current ManifestView::row is private. Proposed NEW policy/manifest/transport.rs child for that bounded typed projection, root routing only. Then actual bounded_closure::ClosureCandidate and authenticate_closure rebuild/check the exact graph from received originals. Raw graph/artifact presence is not policy admission.

## Exact variants/tags

|Tag|Variant|Body and exact length|
|---|---|---|
|1|Describe|Limits64 then ObjectSelector32, exactly96|
|2|DescriptorPage|ObjectSelector32 then exact selected chunk bytes,32+n|
|3|Select|entry ordinalu32 then entry page numberu32,8|
|4|Open|Limits64 then ordinalu32 then entry page numberu32,72|
|5|Grant|phaseu16, statusu16, reservedu32, Limits64, ordinalu32, pageu32,80|
|6|RequestLeaves|empty,0|
|7|LeafData|payload lengthu32, proof lengthu16, reservedu16, payload, proof,8+p+q|
|8|Credit|empty,0|
|9|Cancel|reasonu16 then six zero bytes,8|
|10|Resume|leaf_countu16, six zero bytes, leaf indicesu64[count],8+8*count|
|11|Outcome|statusu16, stageu16, reservedu32, integrity_leaf_countu32, reservedu32,16|
|12|Close|reasonu16 then six zero bytes,8|

Grant phase1=Negotiated (ordinal/page0; metadata association),2=Opened (exact selected entry/ordinal/page); status0=Accepted only. It acknowledges protocol state/capacity, NOT an application permission. Describe creates a fresh metadata transfer ID; provider may return one small NegotiatedGrant and ONE credited DescriptorPage, each with distinct message IDs below. The response's agreed Limits are immutable for that open transfer. First client Credit may precede Describe using fixed v1 upper control bounds; it is sent only after actual metadata capacity admission. Smaller negotiated response is <=that existing credit; no permission from initial defaults.

Select checks actual SelectedPlainEntry/root/index mapping from completed InventoryVerifier+BoundClosure. Successful Select returns Outcome(stage1). Open validates that same association/current whole-source authority and negotiates actual limits; successful Open returns Grant phase2. Leaf request transfer IDs are fresh children of the opened association, with explicit actual local parent budgeting; they never mint a new root budget. RequestLeaves consumes one unspent serial/credit for exactly one nonempty leaf. Credit bytes ceiling must cover prefix/body/actual response and is ONE response only. Receiving Credit does not reserve resources locally or authorize disclosure; provider separately admits its own actual read/proof/send capacity/current grants.

LeafData payload1..16384, proof exactly existing encode_proof framing(count byte+48*siblings), q1..1537 and exact TreeSpec-derived sibling/final length. Maximum208+8+16384+1537=18137 bytes. Empty entries have canonical authenticated empty root and NO leaf request/LeafData; selection/open still meaningful. Plain representation epoch0 only; protected/ciphertext/tree epoch profiles refuse here. Current proof::decode_proof/verify_leaf own semantics; wire length is not proof validity.

Cancel/Close reasons0=Normal(Close only),1=User,2=Deadline,3=Policy,4=Protocol,5=Transport; wire reason never changes local policy or claims remote proof. Outcome statuses0=Ok,1=Refused,2=Cancelled,3=Expired,4=Unavailable,5=Unsupported. Stages0=None,1=Selected,2=Received,3=IntegrityStaged,4=Drained. integrity_leaf_count only0/1 and only stage3 may1. Stage3 means actual proof+live candidate staging, NOT durability/codec/consumer/economic commit. No wire Durable/Consumed/Payment status in v1. Public reasons are constrained; exact source errors stay internal, no arbitrary strings.

Resume leaf_count1..8, indices strictly increasing/unique, all bounds checked against actual selected TreeSpec. It is a NEW scheduling request listing missing leaves and is NOT a checkpoint/presence/permit import. Each resulting actual leaf request needs a fresh transfer ID, Credit and deadline/current authority; body list itself grants no sends. Transport reconnect uses fresh endpoint/keys/message counters and resumes only reverified current storage state. Cancel/Close are bounded terminal controls; no wait for peer acknowledgment is needed to stop local disclosure and start canonical drain.

## Common carrier record88 and maximums

Domain ASCII QBDL-SCT-V1 plus NUL is exactly12 bytes. Following offsets: transcript48@12, MESSAGE ID16@60, logical totalu32@76, offsetu32@80, chunku16@84, finalu16(0/1)@86. Message ID is NOT transfer ID: it is a checked monotonic nonzero128-bit per-direction counter, scoped to actual session transcript/role. Every Credit/Describe/Grant/Cancel/response consumes its own message ID; all chunks of a message share one. Logical total208..65776; chunk geometry exactly min(1024,total-offset), aligned offsets, exact final. Domain/transcript/header are PacketProtection AAD, not a new signature/hash. Private QBS10 RecordHeader remains QBDL-PCT-V1/8192 and is never loosened to this limit.

Canonical existing FrameType SessionDatagram64/NextProtocol QSession5, base80, no flags/extensions, default expected remaining fields (including hop16 because existing SessionDatagram.forwarded()==true). Common channel compares exact canonical header, actual RecvMeta/source/scope/route/MTU and AEAD/replay. Max frame80+88+8PN+1024+16tag=1216. No nested QFrame fragmentation, no ciphertext reseal on WouldBlock, no packet owner reconstruction or new crypto.

Data record maximum is DescriptorPage:208+32+65536=65776 (65chunks; final240 bytes). Leaf maximum18137 (18chunks; final729). Every control record <=304 and fits ONE1024-byte chunk; a separately charged1024-byte control assembly plus pending frame/control work reserve cannot be lent to bulk. Only data may use large assembly. Incoming total above exact negotiated/class cap refuses before accepting uncredited first fragment, allocation or payload read. An admitted caller buffer backs complete assembly; cap accounting includes entire buffer, pending exact sealed frame, proof/siblings/storage/consumer and retained metadata. No8MiB cold metadata budget is silently supplied by an8MiB transient transfer window.

One active data assembly+one control per direction. Control interrupts only BETWEEN completely drained underlying frames; an actual partial stream write remains the adapter's retained cursor or terminal failure. New message IDs increase; an exact already-active data message may continue after a newer control. Unknown older message IDs/replays, holes/out-of-order/overlap/conflicting totals/final/duplicate bytes refuse for this ReliableOrdered v1. Common packet replay does not duplicate application acceptance. Raw unreliable Ethernet requires actual existing QSession reliability composition/qualification, not an application retransmission loop.

## Encoding/decoding and tests before implementation

Pure codecs caller-output, checked bounds, nonallocating success/refusal. Parser consumes exact full record and returns borrowed untrusted views only; no CandidateIdentity/BoundClosure/SelectedPlainEntry/credit/grant/storage capability constructor from parsed headers. Source IDs and roots come only from existing authenticated/selected owners. All failed sends/receives preserve original actual I/O charge until canonical drain; erase unaccepted assembly on expiry/revoke/proof/framing/error/unwind, including outer-final expiry after inner check.

Proposed source owner net/peer/replication/qbdl/wire/{mod,prefix,limits,selector,variants,record,tests}.rs, fixed layouts split before500lines. Root approves magic/tags/classes/statuses/field rules and public schema/interface registration before codec source. Independent byte fixtures must assert all offsets/lengths and actual max proof4608, page65536, manifest2048. Negative cases every unknown/reserved field; all exact/max+1 lengths/overflow/trailing; initial/final index/artifact chunks; wrong totals/page/context/index mapping; empty/final leaf/proof; message versus transfer ID and cancellation fence; stale/double/overspent Credit; illegal state/role/direction; cancellation/control interleave without ciphertext reseal; reconnect cannot replay side effects. No actual native service transfer or security qualification is claimed by this freeze proposal.

## Root allocation decision — 2026-11-13

Root approved the namespaces, layouts and maxima above for implementation. Service prefix remains untrusted; this is an internal service allocation, not an external media/standards registration. Canonical terminal Close, like Cancel, carries cancellation fence1. Cancel/Outcome association count is only0(metadata/selection) or1(leaf), with leaf0 whenever count0. Describe/Page carry the one-response credit; Grant/Outcome/Cancel/Close do not. Pure codec validation never establishes current authorization, negotiated capacity, proof validity, or a completed state transition.
