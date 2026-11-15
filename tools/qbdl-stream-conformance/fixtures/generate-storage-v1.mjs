import { createHash } from 'node:crypto';
import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

// Independent encoder for QBS-11 phase-A fixed-wire records. It deliberately
// makes no storage, authentication, freshness, provider, or durability claim.
const IDENTITY_BYTES = 256;
const CHECKPOINT_BYTES = 400;
const BITMAP_BYTES = 4096;
const BITMAP_BITS = BITMAP_BYTES * 8;
const IDENTITY_MAGIC = Buffer.from('QBSID01\0', 'ascii');
const CHECKPOINT_MAGIC = Buffer.from('QBSCP01\0', 'ascii');
const MAX_LENGTH = 1n << 40n;

function sha384(bytes) { return createHash('sha384').update(bytes).digest(); }
function hex(bytes) { return Buffer.from(bytes).toString('hex'); }
function be16(out, off, value) { out.writeUInt16BE(Number(value), off); }
function be32(out, off, value) { out.writeUInt32BE(Number(value), off); }
function be64(out, off, value) { out.writeBigUInt64BE(BigInt(value), off); }

function baseFields(overrides = {}) {
  return {
    descriptor: Buffer.alloc(48, 0x11),
    context: Buffer.alloc(48, 0x22),
    entry: 0x01020304,
    generation: 0x0102030405060708n,
    representation: 0,
    storageClass: 0,
    keyEpoch: 0,
    logicalLength: 1n,
    root: Buffer.alloc(48, 0x33),
    privacyPartition: Buffer.alloc(48, 0x44),
    policyGeneration: 0x1112131415161718n,
    storageGeneration: 0x2122232425262728n,
    ...overrides,
  };
}

export function encodeIdentity(fields = baseFields()) {
  const out = Buffer.alloc(IDENTITY_BYTES);
  IDENTITY_MAGIC.copy(out, 0);
  be16(out, 8, 1);
  be16(out, 10, 0);
  // bytes 12..16 reserved zero
  Buffer.from(fields.descriptor).copy(out, 16);
  Buffer.from(fields.context).copy(out, 64);
  be32(out, 112, fields.entry);
  be64(out, 116, fields.generation);
  out[124] = fields.representation;
  out[125] = fields.storageClass;
  // bytes 126..128 reserved zero
  be32(out, 128, fields.keyEpoch);
  be64(out, 132, fields.logicalLength);
  Buffer.from(fields.root).copy(out, 140);
  Buffer.from(fields.privacyPartition).copy(out, 188);
  be64(out, 236, fields.policyGeneration);
  be64(out, 244, fields.storageGeneration);
  // bytes 252..256 reserved zero
  return out;
}

export function encodeCheckpoint(identityBytes, pageNumber, bitmapBytes, sequence, overrides = {}) {
  const id = Buffer.from(identityBytes);
  const bitmap = Buffer.from(bitmapBytes);
  if (id.length !== IDENTITY_BYTES) throw new RangeError('identity must be exactly 256 bytes');
  if (bitmap.length !== BITMAP_BYTES) throw new RangeError('bitmap must be exactly 4096 bytes');
  const out = Buffer.alloc(CHECKPOINT_BYTES);
  CHECKPOINT_MAGIC.copy(out, 0);
  be16(out, 8, 1);
  be16(out, 10, overrides.flags ?? 0);
  // bytes 12..16 reserved zero
  id.copy(out, 16);
  be64(out, 272, pageNumber);
  be32(out, 280, overrides.bitmapLength ?? BITMAP_BYTES);
  // bytes 284..288 reserved zero
  sha384(bitmap).copy(out, 288);
  be64(out, 336, sequence);
  // bytes 344..352 reserved zero
  sha384(out.subarray(0, 352)).copy(out, 352);
  return out;
}

function identityRecord(id, fields, expectation = 'decode-accept', note = '') {
  const bytes = encodeIdentity(fields);
  return {
    id,
    identity_hex: hex(bytes),
    expected: expectation,
    note,
    logical_length: BigInt(fields.logicalLength).toString(),
    representation: fields.representation,
    storage_class: fields.storageClass,
    key_epoch: fields.keyEpoch,
  };
}
function checkpointRecord(id, identityBytes, pageNumber, bitmapBytes, sequence, leafCount, note) {
  const checkpoint = encodeCheckpoint(identityBytes, pageNumber, bitmapBytes, sequence);
  return {
    id,
    identity_hex: hex(identityBytes),
    checkpoint_hex: hex(checkpoint),
    bitmap_hex: hex(bitmapBytes),
    expected: 'decode-accept-and-bind-resume-hint',
    expected_identity_hex: hex(identityBytes),
    expected_page_number: pageNumber,
    entry_leaf_count: String(leafCount),
    bitmap_sha384_hex: hex(sha384(bitmapBytes)),
    checkpoint_prefix_sha384_hex: hex(sha384(checkpoint.subarray(0, 352))),
    note,
    trust: 'UntrustedCheckpoint/ResumeHint only; bytes do not establish authenticity, freshness, presence, durability, or authority.',
  };
}
function refusal(id, stage, checkpointBytes, identityBytes, bitmapBytes, expectedIdentity, expectedPage, leafCount, note) {
  return {
    id, stage, expected: 'refuse', note,
    ...(checkpointBytes ? { checkpoint_hex: hex(checkpointBytes) } : {}),
    ...(identityBytes ? { identity_hex: hex(identityBytes) } : {}),
    ...(bitmapBytes ? { bitmap_hex: hex(bitmapBytes) } : {}),
    ...(expectedIdentity ? { expected_identity_hex: hex(expectedIdentity) } : {}),
    ...(expectedPage === undefined ? {} : { expected_page_number: expectedPage }),
    ...(leafCount === undefined ? {} : { entry_leaf_count: String(leafCount) }),
  };
}

export function makeCorpus() {
  const plainFields = baseFields();
  const plainIdentity = encodeIdentity(plainFields);
  const cipherFields = baseFields({ representation: 1, storageClass: 1, keyEpoch: 0 });
  const cipherIdentity = encodeIdentity(cipherFields);
  const cipherEpochFields = baseFields({ representation: 1, storageClass: 1, keyEpoch: 9 });
  const cipherEpochIdentity = encodeIdentity(cipherEpochFields);
  const emptyFields = baseFields({ entry: 0x01020305, logicalLength: 0n });
  const emptyIdentity = encodeIdentity(emptyFields);

  const emptyBitmap = Buffer.alloc(BITMAP_BYTES, 0);
  const plainCheckpoint = encodeCheckpoint(plainIdentity, 0, emptyBitmap, 0x0102030405060708n);
  const onePage = checkpointRecord('plaintext-empty-bitmap-page', plainIdentity, 0, emptyBitmap, 0x0102030405060708n, 1n,
    'One-leaf plaintext identity, page 0 all-zero bitmap. This is only an encoded candidate record; it does not claim the leaf is durably present.');

  // 32769 leaves require two bitmap pages; page 1 has one meaningful low-order bit.
  const lastLength = BigInt(BITMAP_BITS) * 16384n + 1n;
  const lastFields = baseFields({ entry: 0x01020306, logicalLength: lastLength });
  const lastIdentity = encodeIdentity(lastFields);
  const lastBitmap = Buffer.alloc(BITMAP_BYTES, 0);
  lastBitmap[0] = 0x01;
  const lastPage = checkpointRecord('plaintext-last-partial-page', lastIdentity, 1, lastBitmap, 7n, BigInt(BITMAP_BITS) + 1n,
    'Page 1 of 32769 leaves; bit 0 set, all other bits zero. The final page has one in-range leaf and 32767 unused tail bits.');

  const malformedIdentities = [];
  const identityMutations = [
    ['identity-truncated', 0, null, '255-byte identity is not exact length.'],
    ['identity-extra-byte', 1, null, '257-byte identity is not exact length.'],
    ['identity-bad-magic', null, [0, 0x01], 'Magic differs from QBSID01 NUL.'],
    ['identity-version-two', null, [8, 0x02], 'Unsupported version.'],
    ['identity-nonzero-flags', null, [11, 0x01], 'Flags must be zero.'],
    ['identity-reserved-byte', null, [12, 0x01], 'Reserved bytes must be zero.'],
    ['identity-reserved-tail', null, [255, 0x01], 'Final reserved bytes must be zero.'],
  ];
  for (const [id, lengthDelta, mutation, note] of identityMutations) {
    let b = Buffer.from(plainIdentity);
    if (lengthDelta === 0) b = b.subarray(0, 255);
    else if (lengthDelta === 1) b = Buffer.concat([b, Buffer.from([0])]);
    else if (mutation) b[mutation[0]] ^= mutation[1];
    malformedIdentities.push({ id, identity_hex: hex(b), expected: 'decode-refuse', note });
  }
  const semanticIdentityMutations = [
    ['identity-plaintext-class-mismatch', { representation: 0, storageClass: 1 }, 'Plaintext representation cannot use class 1.'],
    ['identity-ciphertext-class-mismatch', { representation: 1, storageClass: 0 }, 'Ciphertext representation cannot use class 0.'],
    ['identity-private-plaintext-unsupported', { representation: 0, storageClass: 2 }, 'Private plaintext class is unsupported in phase A.'],
    ['identity-unknown-class', { representation: 0, storageClass: 9 }, 'Unknown storage class is unsupported.'],
    ['identity-unknown-representation', { representation: 2, storageClass: 0 }, 'Unknown representation is unsupported.'],
    ['identity-plaintext-nonzero-epoch', { representation: 0, storageClass: 0, keyEpoch: 1 }, 'Plaintext requires key epoch zero.'],
    ['identity-length-over-one-tib', { logicalLength: MAX_LENGTH + 1n }, 'Logical length exceeds the QBS-05 1 TiB ceiling.'],
  ];
  for (const [id, changes, note] of semanticIdentityMutations) {
    malformedIdentities.push({ id, identity_hex: hex(encodeIdentity(baseFields(changes))), expected: 'decode-refuse', note });
  }

  const malformedCheckpoints = [];
  const addCheckpointMutation = (id, offset, value, note) => {
    const b = Buffer.from(plainCheckpoint); b[offset] ^= value;
    malformedCheckpoints.push({ id, checkpoint_hex: hex(b), expected: 'decode-refuse', note });
  };
  malformedCheckpoints.push({ id: 'checkpoint-truncated', checkpoint_hex: hex(plainCheckpoint.subarray(0, 399)), expected: 'decode-refuse', note: '399 bytes; checkpoint must be exactly 400 bytes.' });
  malformedCheckpoints.push({ id: 'checkpoint-extra-byte', checkpoint_hex: hex(Buffer.concat([plainCheckpoint, Buffer.from([0])])), expected: 'decode-refuse', note: '401 bytes; checkpoint must be exactly 400 bytes.' });
  addCheckpointMutation('checkpoint-bad-magic', 0, 1, 'Magic differs from QBSCP01 NUL.');
  addCheckpointMutation('checkpoint-version-mutated', 9, 1, 'Unsupported version value.');
  addCheckpointMutation('checkpoint-nonzero-flags', 11, 1, 'Flags must be zero.');
  addCheckpointMutation('checkpoint-reserved-header', 12, 1, 'Reserved header bytes must be zero.');
  addCheckpointMutation('checkpoint-reserved-page', 284, 1, 'Reserved page bytes must be zero.');
  addCheckpointMutation('checkpoint-reserved-sequence-tail', 344, 1, 'Reserved sequence-tail bytes must be zero.');
  addCheckpointMutation('checkpoint-prefix-digest-mutated', 352, 1, 'SHA-384 over bytes 0..352 does not match.');
  const wrongBitmapLength = encodeCheckpoint(plainIdentity, 0, emptyBitmap, 1n, { bitmapLength: BITMAP_BYTES - 1 });
  malformedCheckpoints.push({ id: 'checkpoint-wrong-bitmap-length', checkpoint_hex: hex(wrongBitmapLength), expected: 'decode-refuse', note: 'Declared payload length is not 4096; prefix digest was recomputed.' });

  const refusals = [];
  const staleFields = baseFields({ storageGeneration: BigInt(plainFields.storageGeneration) + 1n });
  const staleExpected = encodeIdentity(staleFields);
  refusals.push(refusal('bind-stale-full-identity', 'bind-payload', plainCheckpoint, plainIdentity, emptyBitmap, staleExpected, 0, 1n,
    'Checkpoint identity differs from the caller-expected full 256-byte identity by storage_generation.'));
  refusals.push(refusal('bind-wrong-expected-page', 'bind-payload', plainCheckpoint, plainIdentity, emptyBitmap, plainIdentity, 1, 2n,
    'Checkpoint page 0 does not equal caller-expected page 1.'));

  const emptyCheckpoint = encodeCheckpoint(emptyIdentity, 0, emptyBitmap, 2n);
  refusals.push(refusal('bind-empty-entry-has-no-page-zero', 'checkpoint-decode', emptyCheckpoint, emptyIdentity, emptyBitmap, emptyIdentity, 0, 0n,
    'Empty entries have zero bitmap pages; the checkpoint decoder rejects page 0 during geometry admission.'));
  const pageOutOfRange = encodeCheckpoint(plainIdentity, 1, emptyBitmap, 3n);
  refusals.push(refusal('bind-page-out-of-range', 'checkpoint-decode', pageOutOfRange, plainIdentity, emptyBitmap, plainIdentity, 1, 1n,
    'A one-leaf entry has only page 0; the checkpoint decoder rejects page 1 during geometry admission.'));

  const mutatedBitmap = Buffer.from(emptyBitmap); mutatedBitmap[0] = 1;
  refusals.push(refusal('bind-bitmap-digest-mismatch', 'bind-payload', plainCheckpoint, plainIdentity, mutatedBitmap, plainIdentity, 0, 1n,
    'Supplied exact 4096-byte bitmap differs from checkpoint commitment.'));
  refusals.push(refusal('bind-bitmap-truncated', 'bind-payload', plainCheckpoint, plainIdentity, emptyBitmap.subarray(0, 4095), plainIdentity, 0, 1n,
    'Bitmap payload must be exactly 4096 bytes.'));

  const tailBad = Buffer.from(lastBitmap); tailBad[0] |= 0x02;
  const tailBadCheckpoint = encodeCheckpoint(lastIdentity, 1, tailBad, 8n);
  refusals.push(refusal('bind-final-page-nonzero-unused-tail', 'bind-payload', tailBadCheckpoint, lastIdentity, tailBad, lastIdentity, 1, BigInt(BITMAP_BITS) + 1n,
    'With one in-range bit, any other bit in this final page is unused and must be zero; checkpoint digest is valid for these bytes.'));

  return {
    schema: 'qbs11-storage-fixed-wire-v1-independent-fixtures',
    provenance: {
      format: 'bundle/stream/storage/FORMAT.md, phase-A candidate bytes, version 1',
      encoder: 'generate-storage-v1.mjs; independent fixed-offset big-endian encoder and Node SHA-384; no production imports',
      scope: 'Structural encoding and caller-bind expectation vectors only. SHA-384 records integrity of encoded bytes, not authenticity, freshness, verified leaf presence, persistence, durability, serving authority, or authorization.',
      bitmap: '4096 bytes = 32768 LSB-first bits; raw bitmaps below are candidate payload bytes. A set bit is not itself evidence that a leaf was verified or persisted.',
    },
    identity_vectors: [
      identityRecord('plaintext-class-zero', plainFields, 'decode-accept', 'Class 0 pairs only with representation 0 and key epoch 0.'),
      identityRecord('ciphertext-class-one-epoch-zero', cipherFields, 'decode-accept', 'Class 1 pairs with representation 1; epoch zero is valid and does not imply decryption.'),
      identityRecord('ciphertext-class-one-epoch-nine', cipherEpochFields, 'decode-accept', 'Ciphertext class preserves the QBS-05 u32 key epoch.'),
      identityRecord('empty-entry-no-bitmap-pages', emptyFields, 'decode-accept-no-pages', 'Logical length zero has zero bitmap pages; this is fixed-wire identity encoding only.'),
      identityRecord('last-partial-page-two-page-identity', lastFields, 'decode-accept', '32769 leaves require pages 0 and 1; final page contains one in-range bit.'),
    ],
    valid_checkpoints: [onePage, lastPage],
    malformed_identity_vectors: malformedIdentities,
    malformed_checkpoint_vectors: malformedCheckpoints,
    bind_refusal_vectors: refusals,
  };
}
export function generate() { return makeCorpus(); }

async function main() {
  const here = dirname(fileURLToPath(import.meta.url));
  const out = join(here, 'storage-v1.json');
  await writeFile(out, JSON.stringify(makeCorpus(), null, 2) + '\n', 'utf8');
  if (typeof process !== 'undefined' && process.stdout) process.stdout.write(`wrote storage fixtures to ${out}\n`);
}
if (typeof process !== 'undefined' && process.argv && process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
