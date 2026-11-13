# QBS-11 phase-A storage candidate bytes

Version1 working local format. This phase has no backend, durable receipt, serving capability, authenticated checkpoint, key/provider call, quota owner or new runtime reservation. All integers below are unsigned big endian. Exact input lengths, version1, zero flags/reserved bytes and supported class/representation pairing are mandatory. SHA384 is delegated to the existing crypto-library digest owner and proves record integrity only, never local authenticity/freshness.

## Identity: exactly256 bytes

| Offset | Length | Field |
|---:|---:|---|
|0|8|ASCII QBSID01 then NUL (51 42 53 49 44 30 31 00)|
|8|2|version=1|
|10|2|flags=0|
|12|4|reserved=0|
|16|48|descriptor ID|
|64|48|context ID|
|112|4|entry ID|
|116|8|content generation|
|124|1|representation:0 plaintext,1 ciphertext (existing QBS05 enum)|
|125|1|storage class:0 public-plaintext candidate,1 shared-ciphertext candidate|
|126|2|reserved=0|
|128|4|key epoch|
|132|8|logical entry length|
|140|48|entry tree root|
|188|48|privacy partition|
|236|8|policy generation|
|244|8|storage generation|
|252|4|reserved=0|

Class0 requires representation0/epoch0. Class1 requires representation1; epoch is the existing QBS05 u32 without implying any QBS10 per-key byte allowance. Class2/private plaintext and all unknown classes/representations refuse Unsupported. TreeSpec::new enforces its1TiB logical ceiling and plaintext epoch rule. Full48-byte identities are retained and compared; compact hashes never authorize anything. Zero-valued opaque fields are not authority and are not implicitly upgraded.

## Checkpoint candidate: exactly400 bytes

| Offset | Length | Field |
|---:|---:|---|
|0|8|ASCII QBSCP01 then NUL (51 42 53 43 50 30 31 00)|
|8|2|version=1|
|10|2|flags=0|
|12|4|reserved=0|
|16|256|exact identity record above, including its header/reserved bytes|
|272|8|bitmap page number|
|280|4|bitmap payload length=4096|
|284|4|reserved=0|
|288|48|SHA384 of exact separate4096-byte bitmap payload|
|336|8|writer/checkpoint sequence (untrusted replay hint)|
|344|8|reserved=0|
|352|48|SHA384 of exact bytes0..352|

All checkpoint fields, including the bitmap commitment, are covered. The final digest cannot authenticate who wrote the record and can be recomputed by an attacker. Decoder returns UntrustedCheckpoint, never a durable/verified-present receipt. Its identity/page/payload bind operation checks exact expected identity, exact expected page, bitmap digest, page bounds and zero unused tail bits, and returns only ResumeHint.

## Bitmap and leaf views

Each caller-owned payload is exactly4096 bytes=32768 bits/leaves. Page p maps entry-relative leaves [p*32768,(p+1)*32768), bounded by TreeSpec.leaf_count. Bit k uses byte k/8 and mask1<<(k%8), least-significant bit first. Unused final-page bits must be zero. Empty entries have zero bitmap pages; no implicit empty data leaf.

CandidateBitmapPage::new clears its caller buffer. Its only bit-marking method accepts a private-constructor VerifiedLeafView with the same full identity and matching page. verify_leaf_view calls actual QBS05 verify_leaf over the exact supplied bytes/path/root/spec; final encoded length follows QBS05, including a16-byte ciphertext tag. Ciphertext inclusion is not AEAD decryption/authenticated plaintext.

Bits record only that a caller-buffer leaf passed mathematical integrity checking. They do not prove that bytes were persisted or remain present. ResumeHint claims are untrusted and cannot construct VerifiedLeafView or CandidateBitmapPage with inherited marks. File extent/hole/zero-filled reads are absent from this phase and never imply verified zeroes. Actual all-zero bytes may pass a supplied root only through explicit QBS05 verification, not through a bitmap or length shortcut. A later backend must reverify/rewrite/flush and enforce policy/storage generations before durable publication.

All operations are bounded and zero-heap by construction; actual allocator-positive-control evidence belongs to root. No StorageDriver/backend/quota/runtime owner changes in phase A.
