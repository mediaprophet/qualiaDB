# QDNF Cryptographic Profile

**Status:** Classical compatibility design 0.1; see the QPR replacement profile below
**Profile identifier:** `qdnf-crypto-1`

## 1. Scope

This profile defines cryptographic algorithms, key purposes, transcript construction, key
derivation, signatures, record encryption, nonces, rotation, and validation order for QLink,
QRoute, QResolve, QPolicy, and QSession.

It does not define a universal trust root. Trust in a public key comes from the relevant DID method,
realm constitution, relationship agreement, content digest, or locally configured authority.

The new QPR implementation targets [post-quantum security](./post-quantum-security.md) by default.
That profile defines hybrid X25519/ML-KEM-768, dual ML-DSA-65/Ed25519 authority proofs, SHA-384
commitments, staged handshake keys and bounded PQ bootstrap. Sections below specify the original
classical suite only; its X25519 key schedule, Ed25519-only proofs and classical HPKE envelopes do
not satisfy a PQ claim. New suite/record/transcript versions are mandatory, with no silent fallback.

## 2. Design principles

1. Keys are purpose-separated. A signing key is not silently converted into a key-agreement key.
2. Algorithm identifiers are signed and negotiated; no algorithm is inferred from key length alone.
3. Protocol transcripts are domain-separated and length-delimited.
4. Compact Q42 hashes are indexes, not cryptographic hashes.
5. Stored/forwarded assertions remain signed end-to-end even on authenticated links.
6. Online traffic uses forward-secret ephemeral key agreement.
7. Private discovery and store-and-forward use separate secrets from session traffic.
8. Expiry, sequence, audience, context, and purpose are inside the authenticated content.
9. Verification failures are indistinguishable at unauthenticated public interfaces where detailed
   errors would enable an oracle.
10. Cryptographic work is bounded and follows cheap structural/rate checks.

## 3. Initial suite

| Purpose | Algorithm |
|---|---|
| Strong digest | SHA-256 |
| HMAC | HMAC-SHA-256 |
| Key derivation | HKDF-SHA-256 |
| Ephemeral key agreement | X25519 |
| Controller/record signature | Ed25519 |
| Link/session AEAD | ChaCha20-Poly1305 |
| Private asynchronous envelope | HPKE base/auth/PSK mode as specified per use |
| Stored signed CBOR object | COSE_Sign1 profile with deterministic CBOR payload |
| Stored encrypted CBOR object | COSE_Encrypt0 or HPKE envelope profile |

Relevant standards are [RFC 5869](https://www.rfc-editor.org/rfc/rfc5869.html),
[RFC 7748](https://www.rfc-editor.org/rfc/rfc7748.html),
[RFC 8032](https://www.rfc-editor.org/rfc/rfc8032.html),
[RFC 8439](https://www.rfc-editor.org/rfc/rfc8439.html),
[RFC 9052](https://www.rfc-editor.org/rfc/rfc9052.html),
[RFC 9053](https://www.rfc-editor.org/rfc/rfc9053.html), and
[RFC 9180](https://www.rfc-editor.org/rfc/rfc9180.html).

The exact HPKE suite is `DHKEM(X25519, HKDF-SHA256) / HKDF-SHA256 /
ChaCha20Poly1305`. Low-entropy passwords/PINs are never HPKE PSKs. Qualia's separately hardened
passphrase derivation remains a storage-vault concern.

## 4. Key roles

| Key role | Lifetime | Published | Authorized by |
|---|---|---|---|
| DID controller signing | Long-lived, rotatable | DID document/identifier method | DID method |
| Route-update signing | Medium-lived | Verification method reference | DID `capabilityInvocation` or QDNF route-update relationship |
| Realm constitution signing | Long-lived or threshold | Realm constitution | Genesis/constitutional rule |
| Router signing | Short/medium-lived | Realm membership credential | Realm authority/delegation |
| QLink ephemeral DH | One interface epoch/adjacency | Discovery/handshake only | Possession plus rendezvous proof; not persistent identity |
| QLink frame AEAD | One adjacency epoch | Never | Derived from QLink transcript |
| QSession ephemeral DH | One session/handshake | Handshake only | Bound by controller/session proof |
| QSession traffic AEAD | One key phase | Never | Derived from session transcript |
| Pairwise discovery | Relationship epoch | Never public | Relationship agreement/secret exchange |
| Group discovery | Group epoch | Never outside group | Group key governance |
| Envelope recipient | Medium-lived X25519 | DID key-agreement or relationship record | Target controller |
| Recovery share | Long-lived but periodically refreshed | Never | Human-governed recovery policy |

Private key bytes must be stored in the Qualia key vault or hardware-backed provider where
available. They never appear in Quins, logs, invitations, DHT records, route advertisements, crash
reports, or telemetry.

## 5. Canonical transcript encoding

Every transcript item is encoded as:

```text
u16_be(label_length) || label_utf8 || u32_be(value_length) || value
```

The transcript digest is:

```text
SHA-256("QDNF-TRANSCRIPT-V1" || ordered_items)
```

Labels are protocol-defined ASCII. Values are raw canonical bytes, not rendered JSON, host-order
integers, or locale-dependent strings. Unknown critical transcript fields fail negotiation. Both
sides compare the final transcript digest before accepting keys.

Required QLink transcript fields:

- version and suite;
- bearer profile and scope digest;
- discovery mode and rendezvous-tag digest;
- initiator/responder link IDs and X25519 public keys;
- adapter-observed initiator/responder locator digests;
- both 256-bit nonces;
- negotiated MTU and feature bitmap; and
- previous key-phase digest during rekey.

Required QSession transcript fields:

- QLink/QRoute path transcript digest;
- initiator/responder ephemeral session keys and nonces;
- persistent target identifier and strong digest;
- selected RAR and DNI digests;
- requester identifier or anonymous/pseudonymous mode;
- offered/selected service and protocol versions;
- capability request digest;
- sensitivity ceiling; and
- downgrade/legacy-gateway state.

## 6. QLink key schedule

For X25519 ephemeral keys `e_i` and `e_r`:

```text
dh          = X25519(e_i.private, e_r.public)
psk_input   = relationship_or_group_secret, or empty in public/manual mode
salt        = SHA-256("QDNF-QLINK-SALT-V1" || transcript_digest)
ikm         = dh || u16_be(len(psk_input)) || psk_input
prk         = HKDF-Extract(salt, ikm)
```

Directional keys are HKDF-Expand outputs with these exact ASCII labels:

```text
"qdnf qlink initiator frame key v1"    32 bytes
"qdnf qlink responder frame key v1"    32 bytes
"qdnf qlink initiator nonce base v1"   12 bytes
"qdnf qlink responder nonce base v1"   12 bytes
"qdnf qlink rekey secret v1"            32 bytes
"qdnf qlink exporter v1"                32 bytes
```

An all-zero X25519 shared secret is rejected. A private mode without the expected PSK/rendezvous
proof fails without revealing whether the tag exists.

QLink public discovery provides encryption against passive observers after the handshake but does
not authenticate a persistent target. QSession/controller proof supplies end-to-end identity.

## 7. QSession key schedule

QSession performs fresh X25519 even when QLink or a transition carrier is already encrypted. For
ephemeral session keys `s_i` and `s_r`:

```text
dh      = X25519(s_i.private, s_r.public)
salt    = SHA-256("QDNF-QSESSION-SALT-V1" || final_transcript_digest)
prk     = HKDF-Extract(salt, dh)
```

Outputs:

```text
"qdnf qsession initiator handshake key v1"  32 bytes
"qdnf qsession responder handshake key v1"  32 bytes
"qdnf qsession initiator traffic key v1"    32 bytes
"qdnf qsession responder traffic key v1"    32 bytes
"qdnf qsession initiator nonce base v1"     12 bytes
"qdnf qsession responder nonce base v1"     12 bytes
"qdnf qsession rekey secret v1"              32 bytes
"qdnf qsession exporter v1"                  32 bytes
```

The responder signs:

```text
SHA-256("QDNF-SESSION-RESPONDER-PROOF-V1" || final_transcript_digest)
```

The requester signs the analogous `...REQUESTER-PROOF-V1` value when mutual controller
authentication is required. Anonymous/pseudonymous requesters still prove capability possession;
the capability presentation is bound to the transcript and must support that disclosure mode.

## 8. Nonces and packet numbers

Each direction maintains a 64-bit packet number per encryption level/key phase. Packet numbers never
repeat under one key. The 96-bit AEAD nonce is:

```text
nonce = nonce_base XOR (0x00000000 || packet_number_u64_be)
```

The authenticated associated data is the canonical QFrame and QSession header excluding mutable
bearer-only fields. Packet-number exhaustion forces rekey or close before `2^64 - 1`.

Implementations also impose conservative suite limits on encrypted packets/failed decryptions and
rotate long before cryptographic bounds. Default operational limits:

- rekey after 2^30 packets or 24 hours, whichever occurs first;
- rekey after path migration if the exporter binding changes;
- close after 32 consecutive invalid tags from one adjacency within a minute;
- never send detailed tag-failure diagnostics to an unauthenticated source.

## 9. Stored signatures and COSE

RAR, withdrawal, LSA, realm path, SDR, realm constitution, replica grant, Alias Assertion, capability,
and durable receipt objects use a QDNF profile of COSE_Sign1.

Protected headers include:

- algorithm;
- content type/profile identifier;
- key identifier equal to the verification-method DID URL digest;
- QDNF object type and version; and
- critical-header list where applicable.

The deterministic CBOR payload remains available as the signed content. External associated data is
the exact ASCII domain separator for the record type. Unprotected headers are hints only and never
influence authorization.

Verifiers check COSE key type, declared algorithm, DID-method key material, verification purpose,
controller, validity/revocation, and object semantics. A valid signature is not a complete valid
record.

## 10. Private records and HPKE

Private RARs, invitations, capability offers, and relay envelopes use HPKE.

| Scenario | HPKE mode |
|---|---|
| Unknown recipient with authenticated target key | Base, followed by signed content verification |
| Known authenticated sender and recipient | Auth |
| Pairwise relationship with high-entropy PSK | AuthPSK or PSK per relationship policy |
| Closed group | Group envelope profile; individual HPKE wrapping of a group content key |

HPKE `info` binds protocol, record type, target digest, relationship context digest, and epoch.
Associated data binds routing metadata that must remain visible. Recipient identifiers are opaque
mailbox/relationship tokens where possible.

Forward secrecy for store-and-forward is limited by recipient static key compromise. Short-lived
envelope keys and erasure after receipt reduce exposure; they do not make historical ciphertext
magically unrecoverable if keys were copied.

## 11. DID verification

For a DID-authorized proof, the verifier:

1. parses the DID/DID URL and selects a supported method driver;
2. resolves method state with explicit source/evidence;
3. canonicalizes the verification-method identifier;
4. verifies the method belongs to or is authorized by the controller;
5. checks the exact verification relationship required by the operation;
6. checks method-specific version time, revocation/deactivation, and key validity;
7. validates key type and algorithm compatibility; and
8. verifies the signature over the domain-separated bytes.

Native Independent nodes must support at least one offline/self-certifying or realm-resolvable method.
A method such as `did:web` that requires DNS/HTTPS is available only through the explicit LIG and is
marked `legacy-dependent` in evidence.

## 12. Rotation and recovery

### 12.1 QLink

Link IDs and ephemeral keys rotate on attachment change, privacy epoch, manual request, suspected
exposure, or key lifetime. Rekey uses old and new transcript digests and requires bidirectional key
confirmation before deleting the old receive key. A short overlap accepts reordered packets only
within the replay window.

### 12.2 QSession

Either endpoint may request key update. New traffic secrets derive from:

```text
next_secret = HKDF-Expand(current_rekey_secret,
                          "qdnf qsession next key phase v1" || transcript_digest,
                          32)
```

Only one unconfirmed update is outstanding. Simultaneous updates resolve by transcript order and
endpoint role.

### 12.3 Controller and realm keys

Controller and router keys rotate through signed method/constitution transitions with sequence and
effective time. A route signed after a key's revocation is invalid. Historical verification uses the
method state valid at issuance when the method provides trustworthy version history.

Recovery shares are combined only in a user-governed recovery ceremony. Recovered secret material is
not reused directly as a network key; it authorizes rotation to fresh purpose-separated keys.

## 13. Algorithm agility

Negotiation offers ordered supported suites. Selection is included in the signed transcript. A peer
must not select a suite absent from the offer or downgrade below local policy. Cached downgrade
evidence is context-specific and expires.

New suites require:

- public specification and test vectors;
- security analysis for key/nonce limits and cross-protocol separation;
- assigned registry value;
- bounded implementation compatible with Sentinel limits;
- explicit hybrid composition rules where combining classical and post-quantum mechanisms; and
- a migration story for stored records and offline peers.

Hybrid secrets are combined with a labelled KDF, not concatenated ad hoc into application keys.

## 14. Randomness

Long-term and ephemeral keys, nonces, salts requiring entropy, link boot IDs, session IDs, and
invitation replay nonces use the operating system CSPRNG or an audited hardware source. Failure to
obtain sufficient randomness fails closed.

Deterministic tests inject an explicit test RNG. Production APIs do not accept caller-provided low-
entropy strings as cryptographic nonces or keys. Display codes and PINs are authentication aids,
not key material unless processed by a separately specified password-authenticated protocol.

## 15. Zeroization and memory

- Secret types should implement zeroization on drop and avoid implicit clone/debug/serialization.
- Hot-path AEAD uses reusable caller buffers and never allocates per packet.
- Signature verification batches are capped before allocation or expensive curve work.
- Failed-handshake state has strict byte/time quotas.
- Crash dumps and diagnostic formatting redact secret and full private-routing material.
- Hardware-backed keys expose signing/agreement operations rather than raw secret bytes where possible.

## 16. Required negative tests

- all-zero X25519 result;
- repeated nonce/packet number;
- wrong direction/key phase;
- altered suite/version/MTU/locator/transcript;
- wrong domain separator;
- Ed25519 key presented as X25519 or vice versa;
- valid signature from a key with the wrong DID verification relationship;
- revoked/deactivated controller key;
- expired RAR with otherwise valid proof;
- HPKE ciphertext replayed into a different relationship/context;
- unknown critical COSE header;
- non-deterministic or duplicate-key CBOR;
- simultaneous rekey and path migration; and
- key/packet lifetime exhaustion.

## 17. Security claim boundary

This profile authenticates cryptographic keys and signed statements within defined contexts. It does
not prove a credential's real-world truth, the moral legitimacy of a realm authority, the safety of
remote content, a natural person's identity as a whole, or anonymity against global traffic analysis.
