# Swarm S2 fixture notes

Support transcription only. Status: provisional. No cargo, no git, no `crates/**` edits, no algorithm decisions.

Working tree at transcription: `bundle/stream/policy/remote/` is absent; `bundle/stream/runtime/jobs/` and `storage/native/job.rs` are absent. Fixtures are placeholder JSON matching published field names.

Outputs:

- `tools/qbdl-stream-conformance/fixtures/remote-recognition-v1.json`
- `tools/qbdl-stream-conformance/fixtures/storage-job-v1.json`
- this file

## Field sources copied into `remote-recognition-v1.json`

### qbs07 R1 recognition row

From `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/qbdl-streaming/evidence/qbs07-remote-authority-integration.md`:

| Fixture name | Copied text | Path:line |
|---|---|---|
| principal | PinHandle/principal | qbs07-remote-authority-integration.md:34 |
| source SHA384 | exact source SHA384+length | qbs07-remote-authority-integration.md:34 |
| source_length | exact source SHA384+length | qbs07-remote-authority-integration.md:34 |
| policy | exact policy artifact digest | qbs07-remote-authority-integration.md:34 |
| publisher | allowed publisher principal | qbs07-remote-authority-integration.md:34 |
| operation / allowed operation set | allowed operation set | qbs07-remote-authority-integration.md:34 |
| expires / expiry | expiry | qbs07-remote-authority-integration.md:34 |
| max_decision_lifetime | maximum decision lifetime | qbs07-remote-authority-integration.md:34 |
| recognition_initial_capacity | Initial capacity eight, no implicit eviction | qbs07-remote-authority-integration.md:34 |
| default deny / no wildcard | Default deny; no source wildcard and no transitive delegation | qbs07-remote-authority-integration.md:34 |
| LocalBootstrap | unexported local-admin capability analogous to existing LocalBootstrap | qbs07-remote-authority-integration.md:34 |
| unknown principal reason | Reject unsupported delegation chains, unknown source controllers, unknown/mixed coverage, missing required co-consent, and unknown policy versions | qbs07-remote-authority-integration.md:11 |
| Stage R1 files | bundle/stream/policy/remote/{mod,recognition,policy,codec,decision,tests}.rs | qbs07-remote-authority-integration.md:74 |

### qbs07 decision ticket

| Fixture name | Copied text | Path:line |
|---|---|---|
| recipient_record_id | A.id() | qbs07-remote-authority-integration.md:46 |
| endpoint_transcript | exact accepted endpoint transcript | qbs07-remote-authority-integration.md:46 |
| request_id | request ID | qbs07-remote-authority-integration.md:46 |
| challenge | fresh verifier challenge[32] | qbs07-remote-authority-integration.md:46 |
| controller_generation | recognized controller generation | qbs07-remote-authority-integration.md:46 |
| policy | exact policy digest | qbs07-remote-authority-integration.md:46 |
| action | metadata release or payload use | qbs07-remote-authority-integration.md:46 |
| not_before / expires | not-before/expiry | qbs07-remote-authority-integration.md:46 |
| decision_id | monotonically consumed decision ID | qbs07-remote-authority-integration.md:46 |
| proposed_decision_purpose | qbdl-stream-protected-v1/disclosure-decision | qbs07-remote-authority-integration.md:46 |
| ticket_plus_paired_envelope_bound_bytes | Bound ticket+paired envelope to 8192 bytes | qbs07-remote-authority-integration.md:46 |
| 1024-byte chunking | existing canonical 1024-byte record chunking | qbs07-remote-authority-integration.md:46 |
| codec freeze | Canonical fixed-width codec and exact label/domain need a small subsequent freeze before code | qbs07-remote-authority-integration.md:46 |
| proposed_lease_ceiling_seconds | propose <=30 seconds, configurable only downward for this profile | qbs07-remote-authority-integration.md:46 |

### RecipientRecord / RecipientFields

From `crates/qualia-core-db/src/crypto/network/object_stream/types.rs`:

| Name | Path:line |
|---|---|
| PROFILE `qbdl-stream-protected-v1` | types.rs:3 |
| RECIPIENT_PURPOSE `qbdl-stream-protected-v1/recipient-authority` | types.rs:4 |
| RECIPIENT_BYTES = 1856 | types.rs:16 |
| RECORD_HEADER_BYTES = 88 | types.rs:21 |
| MAX_PRIVATE_PACKAGE = 8192 | types.rs:20 |
| MAX_OBJECT_BYTES | types.rs:10 |

From `crates/qualia-core-db/src/crypto/network/object_stream/recipient.rs`:

| Name | Path:line |
|---|---|
| RecipientFields.descriptor | recipient.rs:9 |
| RecipientFields.context | recipient.rs:10 |
| RecipientFields.recipient | recipient.rs:11 |
| RecipientFields.publisher | recipient.rs:12 |
| RecipientFields.service | recipient.rs:13 |
| RecipientFields.purpose | recipient.rs:14 |
| RecipientFields.instrument | recipient.rs:15 |
| RecipientFields.scope | recipient.rs:16 |
| RecipientFields.operation `[u8; 16]` | recipient.rs:17 |
| RecipientFields.closure | recipient.rs:18 |
| RecipientFields.policy | recipient.rs:19 |
| RecipientFields.consumer | recipient.rs:20 |
| RecipientFields.source | recipient.rs:21 |
| RecipientFields.source_length | recipient.rs:22 |
| RecipientFields.key_epoch | recipient.rs:23 |
| RecipientFields.not_before | recipient.rs:24 |
| RecipientFields.expires | recipient.rs:25 |
| RecipientFields.ml_public `&[u8; 1184]` | recipient.rs:26 |
| RecipientFields.dh_public `&[u8; 32]` | recipient.rs:27 |
| parse length check `bytes.len() != RECIPIENT_BYTES` → `Error::Range` | recipient.rs:67-68 |
| magic `QBDL-PRA-V1\0` | recipient.rs:70 |
| parse field order (descriptor … dh_public) | recipient.rs:73-92 |
| RecipientRecord::id = sha384(self.bytes) | recipient.rs:104-106 |
| valid_at Unauthorized | recipient.rs:110-115 |
| encode_recipient magic | recipient.rs:133 |
| source_length < 64 \|\| > MAX_OBJECT_BYTES Malformed | recipient.rs:50-55 |

From `crates/qualia-core-db/src/crypto/network/object_stream/codec.rs`:

| Name | Path:line |
|---|---|
| magic `&[u8; 12]` | codec.rs:8 |
| suite must be 1 | codec.rs:36-41 |
| flags/zero must be 0 | codec.rs:43-48 |

Layout confirmation (copied, not invented) also in `evidence/qbs10-protection-contract.md:110-120` (`magic = "QBDL-PRA-V1\0"`, 1856-byte A).

### Pin / authority names used as recognition identifiers

| Name | Path:line |
|---|---|
| MAX_PINS = 8 | `crates/qualia-core-db/src/crypto/network/authority/mod.rs:15` |
| PinHandle.instance `[u8; 16]` | `authority/types.rs:16` |
| PinHandle.slot `u8` | `authority/types.rs:17` |
| PinHandle.generation | `authority/types.rs:18` |
| PinConfig.principal | `authority/types.rs:28` |
| PinConfig.expires_unix | `authority/types.rs:34` |
| LocalBootstrap | `authority/types.rs:9-11` |
| StrongDigest `[u8; 48]` | `net/qdnf/types.rs:8` |
| Generation | `net/qdnf/types.rs:138` |
| OperationId `[u8; 16]` | `net/qdnf/types.rs:156` |
| AuthorityBinding.principal | `net/qdnf/authority/digest_bind.rs:10` |
| AuthorityBinding.recipient | `digest_bind.rs:11` |
| AuthorityBinding.instrument | `digest_bind.rs:12` |
| AuthorityBinding.scope | `digest_bind.rs:13` |
| AuthorityBinding.purpose | `digest_bind.rs:14` |
| AuthorityBinding.operation | `digest_bind.rs:15` |
| MAX_RECORDS = 8 | `net/qdnf/authority/owner.rs:15` |
| QdnfError::Capacity | `net/qdnf/errors.rs:13` |
| QdnfError::Range | `net/qdnf/errors.rs:40` |
| QdnfError::Denied | `net/qdnf/errors.rs:17` |

Delivery `request_id:[16]` width copied from `evidence/qbs10-protection-contract.md:157`.

## Field sources copied into `storage-job-v1.json`

From `crates/qualia-core-db/src/bundle/stream/runtime/types.rs`:

| Name | Path:line |
|---|---|
| RuntimeProfile::Constrained / Desktop | types.rs:11-13 |
| inflight_limit 8 / 30 | types.rs:16-20 |
| window_bytes 8MiB / 32MiB | types.rs:22-26 |
| CancelReason::Cancelled | types.rs:31 |
| CancelReason::Expired | types.rs:32 |
| CancelReason::Revoked | types.rs:33 |
| TransferState::Vacant | types.rs:47 |
| TransferState::Queued | types.rs:48 |
| TransferState::Ready | types.rs:49 |
| TransferState::InFlight | types.rs:50 |
| TransferState::Cancelling | types.rs:51 |
| FinishOutcome::Released | types.rs:55 |
| FinishOutcome::Cancelled | types.rs:56 |
| FinishOutcome::Expired | types.rs:57 |
| FinishOutcome::Revoked | types.rs:58 |
| CompletionKey.owner | types.rs:63 |
| CompletionKey.slot | types.rs:64 |
| CompletionKey.generation | types.rs:65 |
| CompletionKey.operation | types.rs:66 |
| TransferTicket.key | types.rs:75 |
| TransferTicket.spent | types.rs:76 |
| RequestCredit.key | types.rs:86 |
| RequestCredit.bytes | types.rs:87 |
| RequestCredit.cancel_epoch | types.rs:88 |
| RuntimePoll::Ready | types.rs:106 |
| RuntimePoll::Cancelled | types.rs:107 |
| RuntimePoll::External | types.rs:108 |

From `crates/qualia-core-db/src/bundle/stream/runtime/transfer.rs`:

| Name | Path:line |
|---|---|
| admit → TransferTicket | transfer.rs:11-17 |
| admit state Queued | transfer.rs:103 |
| TransferTicket { key, spent: false } | transfer.rs:116-119 |
| complete_io(ticket, CompletionKey, now) → FinishOutcome | transfer.rs:123-128 |
| complete_io WouldBlock if queued | transfer.rs:136-137 |
| complete_io requires InFlight \| Cancelling | transfer.rs:140-145 |
| ticket.spent = true | transfer.rs:162 |
| Cancelling(Cancelled) → FinishOutcome::Cancelled | transfer.rs:164 |
| Cancelling(Expired) → FinishOutcome::Expired | transfer.rs:165 |
| Cancelling(Revoked) → FinishOutcome::Revoked | transfer.rs:166 |
| else → FinishOutcome::Released | transfer.rs:167 |

From `crates/qualia-core-db/src/bundle/stream/runtime/dispatch.rs`:

| Name | Path:line |
|---|---|
| poll RuntimePoll::Cancelled | dispatch.rs:28-29 |
| poll sets Ready | dispatch.rs:31-32 |
| issue_credit sets InFlight | dispatch.rs:57 |
| cancel → CancelReason::Cancelled | dispatch.rs:70-73 |
| expire live InFlight/Ready/Queued | dispatch.rs:78-84 |
| cancel_slot sets Cancelling(reason) | dispatch.rs:96 |

From `crates/qualia-core-db/src/bundle/stream/runtime/storage_stage.rs`:

| Name | Path:line |
|---|---|
| storage_stage | storage_stage.rs:26-34 |
| InFlight required | storage_stage.rs:65 |
| Cancelling → reason.error() | storage_stage.rs:66 |
| StorageStage::recheck | storage_stage.rs:73 |
| StorageStage::cancel | storage_stage.rs:78 |
| StorageStage::buffers | storage_stage.rs:83 |

From `crates/qualia-core-db/src/bundle/stream/runtime/owner.rs`:

| Name | Path:line |
|---|---|
| CompletionKey construction | owner.rs:100-108 |
| spent → DoubleRelease | owner.rs:112-114 |

Transcript sequences follow `runtime/tests/lifecycle.rs` (Released 24, Cancelled/Expired 61-67, WouldBlock 99, Revoked 229) and `runtime/tests/storage_stage.rs` (Cancelled 44).

QdnfError names used in transcripts: WouldBlock `errors.rs:14`, Expired `:24`, Revoked `:25`, Cancelled `:35`, DoubleRelease `:27`, Capacity `:13`.

## UNVERIFIED items

No Merkle or COSE bytes were generated. Every guessed placeholder is the literal string `UNVERIFIED`.

In `remote-recognition-v1.json`:

- `proposed_decision_purpose` freeze (`qbdl-stream-protected-v1/disclosure-decision`) — design text; codec/label not frozen
- `proposed_lease_ceiling_seconds` — design text only
- `max_decision_lifetime` Rust field name — remote/recognition.rs not present
- `skeleton_recognition_row.principal_hex`
- `skeleton_recognition_row.pin_handle.instance_hex`
- `skeleton_recognition_row.pin_handle.slot`
- `skeleton_recognition_row.pin_handle.generation`
- `skeleton_recognition_row.source_hex`
- `skeleton_recognition_row.source_length`
- `skeleton_recognition_row.policy_hex`
- `skeleton_recognition_row.publisher_hex`
- `skeleton_recognition_row.operation_hex`
- `skeleton_recognition_row.expires`
- `skeleton_recognition_row.max_decision_lifetime`
- `skeleton_recognition_row.recipient_record_hex`
- `skeleton_recognition_row.decision_ticket_hex`
- `skeleton_recognition_row.cose_hex`
- `skeleton_recognition_row.merkle_hex`
- cases `wildcard-source`, `unknown-principal`: all `*_hex` / numeric row values
- case `truncated-ticket.bytes_hex`
- truncated decision-ticket encoding (no frozen width besides RecipientRecord 1856 and 8192 envelope bound)

In `storage-job-v1.json`:

- `completion_key_placeholder.owner`
- `completion_key_placeholder.slot`
- `completion_key_placeholder.generation`
- `completion_key_placeholder.operation`

`Generation: 1` / `Generation: 2` / `now` / `deadline: 10` in transcripts are copied from `runtime/tests/lifecycle.rs` and `runtime/tests/storage_stage.rs` argument literals, not live process keys.

## Not done

- no `crates/**` edits
- no Cargo.toml
- no generators that contact the network
- no cargo
- no git commit/push
- Synth MCP not killed
