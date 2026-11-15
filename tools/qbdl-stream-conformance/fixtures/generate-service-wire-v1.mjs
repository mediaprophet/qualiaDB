import { writeFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { dirname, join } from 'node:path';

// Independent byte fixture generator for the frozen QBS-09 service-wire
// proposal. This encodes bytes only; it does not validate roles, grants,
// signatures, proofs, resource reservations, or real transfer behavior.
const PREFIX_BYTES = 208;
const LIMITS_BYTES = 64;
const SELECTOR_BYTES = 32;
const RECORD_HEADER_BYTES = 88;
const RECORD_DOMAIN = Buffer.from('QBDL-SCT-V1\0', 'ascii');
const TRANSCRIPT = Buffer.alloc(48, 0x33);

function be16(n) { const b = Buffer.alloc(2); b.writeUInt16BE(n); return b; }
function be32(n) { const b = Buffer.alloc(4); b.writeUInt32BE(n); return b; }
function be64(n) { const b = Buffer.alloc(8); b.writeBigUInt64BE(BigInt(n)); return b; }
function pattern(length, start, step = 1) {
  return Buffer.from(Array.from({ length }, (_, i) => (start + i * step) & 0xff));
}
function hex(bytes) { return Buffer.from(bytes).toString('hex'); }

export function encodeLimits(values = {}) {
  const v = {
    maxRecordBytes: 65776,
    maxPagePayload: 65536,
    maxCoseBytes: 4608,
    maxIndexBytes: 8388608,
    maxMetadataBytes: 8388608,
    maxClosureBytes: 1048576,
    maxEntries: 4096,
    maxInflight: 2,
    reassemblyLanes: 2,
    windowBytes: 8388608n,
    maxColdRows: 64,
    carrierMode: 1,
    flags: 0,
    maxLogicalExtent: 1099511627776n,
    requestLifetimeMs: 30000,
    reserved: 0,
    ...values,
  };
  const out = Buffer.alloc(LIMITS_BYTES);
  out.writeUInt32BE(v.maxRecordBytes, 0);
  out.writeUInt32BE(v.maxPagePayload, 4);
  out.writeUInt32BE(v.maxCoseBytes, 8);
  out.writeUInt32BE(v.maxIndexBytes, 12);
  out.writeUInt32BE(v.maxMetadataBytes, 16);
  out.writeUInt32BE(v.maxClosureBytes, 20);
  out.writeUInt32BE(v.maxEntries, 24);
  out.writeUInt16BE(v.maxInflight, 28);
  out.writeUInt16BE(v.reassemblyLanes, 30);
  out.writeBigUInt64BE(BigInt(v.windowBytes), 32);
  out.writeUInt32BE(v.maxColdRows, 40);
  out.writeUInt16BE(v.carrierMode, 44);
  out.writeUInt16BE(v.flags, 46);
  out.writeBigUInt64BE(BigInt(v.maxLogicalExtent), 48);
  out.writeUInt32BE(v.requestLifetimeMs, 56);
  out.writeUInt32BE(v.reserved, 60);
  return { bytes: out, values: v };
}

export function encodeSelector({ objectClass, number = 0, total, offset = 0n, chunkLength }) {
  const out = Buffer.alloc(SELECTOR_BYTES);
  out[0] = objectClass;
  out.writeUInt32BE(number, 4);
  out.writeBigUInt64BE(BigInt(total), 8);
  out.writeBigUInt64BE(BigInt(offset), 16);
  out.writeUInt32BE(chunkLength, 24);
  return out;
}

function makePrefix(variant, body, fields = {}) {
  const v = {
    descriptor: Buffer.alloc(48, 0x11),
    context: Buffer.alloc(48, 0x22),
    generation: 0n,
    policyGeneration: 1n,
    cancellationFence: 0n,
    transferId: pattern(16, (0x41 + variant * 7) & 0xff, 3),
    entryId: 0,
    representation: 0,
    representationEpoch: 0n,
    leafIndex: 0n,
    count: 0,
    creditSerial: 0n,
    creditCeilingBytes: 0n,
    deadlineUnixSeconds: 1800000000n,
    ...fields,
  };
  const out = Buffer.alloc(PREFIX_BYTES);
  Buffer.from('QBS1', 'ascii').copy(out, 0);
  out.writeUInt16BE(1, 4);
  out.writeUInt16BE(variant, 6);
  out.writeUInt16BE(1, 8);
  Buffer.from(v.descriptor).copy(out, 16);
  Buffer.from(v.context).copy(out, 64);
  out.writeBigUInt64BE(BigInt(v.generation), 112);
  out.writeBigUInt64BE(BigInt(v.policyGeneration), 120);
  out.writeBigUInt64BE(BigInt(v.cancellationFence), 128);
  Buffer.from(v.transferId).copy(out, 136);
  out.writeUInt32BE(v.entryId, 152);
  out[156] = v.representation;
  out.writeBigUInt64BE(BigInt(v.representationEpoch), 160);
  out.writeBigUInt64BE(BigInt(v.leafIndex), 168);
  out.writeUInt32BE(v.count, 176);
  out.writeUInt32BE(body.length, 180);
  out.writeBigUInt64BE(BigInt(v.creditSerial), 184);
  out.writeBigUInt64BE(BigInt(v.creditCeilingBytes), 192);
  out.writeBigUInt64BE(BigInt(v.deadlineUnixSeconds), 200);
  return { bytes: out, fields: v };
}

function carrierHeader(logical, messageId) {
  const out = Buffer.alloc(RECORD_HEADER_BYTES);
  RECORD_DOMAIN.copy(out, 0);
  TRANSCRIPT.copy(out, 12);
  Buffer.from(messageId).copy(out, 60);
  out.writeUInt32BE(logical.length, 76);
  out.writeUInt32BE(0, 80);
  const count = Math.min(1024, logical.length);
  out.writeUInt16BE(count, 84);
  out.writeUInt16BE(count === logical.length ? 1 : 0, 86);
  return out;
}

function expectedPrefix(variant, body, fields, carrier) {
  return {
    offsets: {
      magic: 0, schema: 4, variant: 6, profile: 8, flags: 10, reserved0: 12,
      descriptor: 16, context: 64, generation: 112, policyGeneration: 120,
      cancellationFence: 128, transferId: 136, entryId: 152, representation: 156,
      reserved1: 157, representationEpoch: 160, leafIndex: 168, count: 176,
      bodyLength: 180, creditSerial: 184, creditCeilingBytes: 192, deadlineUnixSeconds: 200,
    },
    magic: 'QBS1', schema: 1, variant, profile: 1, flags: 0,
    descriptorDigestHex: hex(fields.descriptor), contextDigestHex: hex(fields.context),
    generation: String(fields.generation), policyGeneration: String(fields.policyGeneration),
    cancellationFence: String(fields.cancellationFence), transferIdHex: hex(fields.transferId),
    entryId: fields.entryId, representation: fields.representation,
    representationEpoch: String(fields.representationEpoch), leafIndex: String(fields.leafIndex),
    count: fields.count, bodyLength: body.length, creditSerial: String(fields.creditSerial),
    creditCeilingBytes: String(fields.creditCeilingBytes), deadlineUnixSeconds: String(fields.deadlineUnixSeconds),
    carrier: {
      offsets: { domain: 0, transcript: 12, messageId: 60, logicalTotal: 76, offset: 80, chunkLength: 84, final: 86 },
      domainDescription: 'ASCII QBDL-SCT-V1 followed by one NUL byte', transcriptHex: hex(TRANSCRIPT),
      messageIdHex: hex(carrier.messageId), logicalTotal: carrier.logical.length,
      offset: 0, chunkLength: Math.min(1024, carrier.logical.length),
      final: carrier.logical.length <= 1024 ? 1 : 0,
    },
  };
}

function makePositive(variant, body, fields, messageNumber, note) {
  const { bytes: prefix, fields: headerFields } = makePrefix(variant, body, fields);
  const logical = Buffer.concat([prefix, body]);
  const messageId = Buffer.alloc(16);
  messageId.writeBigUInt64BE(0x0102030405060708n, 0);
  messageId.writeBigUInt64BE(BigInt(messageNumber), 8);
  const ch = carrierHeader(logical, messageId);
  return {
    id: `variant-${String(variant).padStart(2, '0')}`,
    variant,
    note,
    encoding: 'big-endian fixed fields; opaque deterministic bytes only',
    prefixHex: hex(prefix),
    bodyHex: hex(body),
    wireRecordHex: hex(logical),
    carrierHeaderHex: hex(ch),
    firstCarrierFragmentHex: hex(Buffer.concat([ch, logical.subarray(0, Math.min(1024, logical.length))])),
    expected: expectedPrefix(variant, body, headerFields, { logical, messageId }),
  };
}

function buildPositives() {
  const limits = encodeLimits();
  const selector = encodeSelector({ objectClass: 1, total: 8, chunkLength: 8 });
  const dummyPayload = pattern(8, 0xa1, 7);
  const selectedEntry = { entryId: 0, leafIndex: 5n, count: 1 };
  const leaves = Buffer.concat([be16(2), Buffer.alloc(6), be64(6), be64(8)]);
  const proof = Buffer.concat([Buffer.from([1]), pattern(48, 0xc1, 5)]);
  const leafPayload = pattern(3, 0x91, 11);
  const leafBody = Buffer.concat([be32(leafPayload.length), be16(proof.length), Buffer.alloc(2), leafPayload, proof]);
  const outcomeBody = Buffer.concat([be16(0), be16(3), Buffer.alloc(4), be32(1), Buffer.alloc(4)]);
  return [
    makePositive(1, Buffer.concat([limits.bytes, selector]), {
      creditSerial: 1n, creditCeilingBytes: 1024n,
    }, 1, 'Describe request bound to an earlier admitted one-response metadata Credit.'),
    makePositive(2, Buffer.concat([selector, dummyPayload]), {
      creditSerial: 1n, creditCeilingBytes: 1024n,
    }, 2, 'DescriptorPage with opaque bytes; no descriptor signature or validity claim.'),
    makePositive(3, Buffer.concat([be32(7), be32(0)]), { entryId: 0 }, 3, 'Select ordinal and page association; prefix leaf/count remain zero.'),
    makePositive(4, Buffer.concat([limits.bytes, be32(7), be32(0)]), { entryId: 0 }, 4, 'Open with exact Limits64 and selection association; prefix leaf/count remain zero.'),
    makePositive(5, Buffer.concat([be16(2), be16(0), Buffer.alloc(4), limits.bytes, be32(7), be32(0)]), { entryId: 0 }, 5, 'Opened Grant; status Accepted is only encoded, not an authority claim.'),
    makePositive(6, Buffer.alloc(0), {
      ...selectedEntry, creditSerial: 2n, creditCeilingBytes: 20000n,
    }, 6, 'RequestLeaves structural request association; no proof is supplied.'),
    makePositive(7, leafBody, {
      ...selectedEntry, creditSerial: 2n, creditCeilingBytes: 20000n,
    }, 7, 'LeafData byte framing with a syntactically framed dummy sibling; Merkle validity is not claimed.'),
    makePositive(8, Buffer.alloc(0), {
      creditSerial: 1n, creditCeilingBytes: 304n,
    }, 8, 'Initial metadata Credit body is empty; it is only a byte fixture.'),
    makePositive(9, Buffer.concat([be16(1), Buffer.alloc(6)]), {
      ...selectedEntry, cancellationFence: 1n,
    }, 9, 'Terminal Cancel with User reason and fence one.'),
    makePositive(10, leaves, { entryId: 0 }, 10, 'Resume with strictly increasing leaf numbers; not a checkpoint or presence proof.'),
    makePositive(11, outcomeBody, { ...selectedEntry }, 11, 'Outcome structurally encodes Ok, IntegrityStaged, one leaf; no staged proof claim.'),
    makePositive(12, Buffer.concat([be16(0), Buffer.alloc(6)]), {
      cancellationFence: 1n,
    }, 12, 'Terminal Close with Normal reason and fence one.'),
  ];
}

function buildAdditionalPositives() {
  return [makePositive(8, Buffer.alloc(0), {
    entryId: 0, leafIndex: 5n, count: 1, creditSerial: 2n, creditCeilingBytes: 20000n,
    transferId: pattern(16, 0xe1, 3),
  }, 13, 'Leaf Credit association; no local reservation or disclosure permission is implied.')];
}

function makeSelectorVectors() {
  const definitions = [
    { objectClass: 1, number: 0, total: 65536n, offset: 0n, chunkLength: 65536, purpose: 'QASD body maximum page' },
    { objectClass: 2, number: 0, total: 4608n, offset: 0n, chunkLength: 4608, purpose: 'paired COSE envelope maximum; no signature claim' },
    { objectClass: 3, number: 0, total: 2048n, offset: 0n, chunkLength: 2048, purpose: 'signed closure manifest byte ceiling; no signature claim' },
    { objectClass: 4, number: 15, total: 1048576n, offset: 0n, chunkLength: 65536, purpose: 'highest valid canonical artifact ordinal and bounded chunk' },
    { objectClass: 5, number: 0, total: 4608n, offset: 0n, chunkLength: 4608, purpose: 'closure paired COSE envelope maximum; no signature claim' },
    { objectClass: 6, number: 0, total: 64n, offset: 0n, chunkLength: 64, purpose: 'exact original header size' },
    { objectClass: 7, number: 0, total: 8388608n, offset: 0n, chunkLength: 65536, purpose: 'maximum admitted index ceiling; first bounded chunk' },
    { objectClass: 8, number: 31, total: 65536n, offset: 0n, chunkLength: 65536, purpose: 'highest context page number, maximum page' },
    { objectClass: 9, number: 31, total: 65536n, offset: 0n, chunkLength: 65536, purpose: 'highest entry page number, maximum page' },
  ];
  return definitions.map((d) => ({
    id: `selector-class-${d.objectClass}`,
    purpose: d.purpose,
    bytesHex: hex(encodeSelector(d)),
    expected: {
      offsets: { objectClass: 0, reserved: 1, number: 4, total: 8, offset: 16, chunkLength: 24, reservedTail: 28 },
      objectClass: d.objectClass, number: d.number, total: String(d.total), offset: String(d.offset), chunkLength: d.chunkLength,
      reservedBytesZero: true,
    },
  }));
}

function makeNegativeVectors(positives) {
  const negatives = [];
  const addWire = (id, variant, mutate) => {
    const out = Buffer.from(positives[variant - 1].wireRecordHex, 'hex');
    mutate(out);
    negatives.push({ id, name: id, scope: 'wire-record', variant, bytesHex: hex(out), expected: 'structural refusal' });
  };
  negatives.push({ id: 'wire-short-prefix', name: 'wire-short-prefix', scope: 'wire-record', variant: 1,
    bytesHex: hex(Buffer.from(positives[0].wireRecordHex, 'hex').subarray(0, PREFIX_BYTES - 1)), expected: 'structural refusal' });
  negatives.push({ id: 'wire-trailing-byte', name: 'wire-trailing-byte', scope: 'wire-record', variant: 1,
    bytesHex: hex(Buffer.concat([Buffer.from(positives[0].wireRecordHex, 'hex'), Buffer.from([0])])), expected: 'structural refusal' });

  addWire('wire-unknown-schema', 1, (b) => b.writeUInt16BE(2, 4));
  addWire('wire-unknown-variant', 1, (b) => b.writeUInt16BE(13, 6));
  addWire('wire-unknown-profile', 1, (b) => b.writeUInt16BE(2, 8));
  addWire('wire-nonzero-prefix-flags', 1, (b) => b.writeUInt16BE(1, 10));
  addWire('wire-nonzero-prefix-reserved', 1, (b) => { b[12] = 1; });
  addWire('wire-zero-descriptor-digest', 1, (b) => b.fill(0, 16, 64));
  addWire('wire-zero-context-digest', 1, (b) => b.fill(0, 64, 112));
  addWire('wire-invalid-cancellation-fence', 9, (b) => b.writeBigUInt64BE(2n, 128));
  addWire('wire-zero-transfer-id', 1, (b) => b.fill(0, 136, 152));
  addWire('wire-protected-representation', 1, (b) => { b[156] = 1; });
  addWire('wire-nonzero-representation-reserved', 1, (b) => { b[157] = 1; });
  addWire('wire-nonzero-plain-epoch', 1, (b) => b.writeBigUInt64BE(1n, 160));
  addWire('wire-body-length-mismatch', 1, (b) => b.writeUInt32BE(b.readUInt32BE(180) + 1, 180));
  addWire('limits-unknown-carrier-mode', 1, (b) => b.writeUInt16BE(2, PREFIX_BYTES + 44));
  addWire('limits-nonzero-flags', 1, (b) => b.writeUInt16BE(1, PREFIX_BYTES + 46));
  addWire('limits-nonzero-reserved-tail', 1, (b) => b.writeUInt32BE(1, PREFIX_BYTES + 60));
  addWire('limits-one-lane', 1, (b) => b.writeUInt16BE(1, PREFIX_BYTES + 30));
  addWire('limits-max-record-overflow', 1, (b) => b.writeUInt32BE(65777, PREFIX_BYTES));
  addWire('selector-unknown-class', 2, (b) => { b[PREFIX_BYTES] = 10; });
  addWire('selector-nonzero-reserved', 2, (b) => { b[PREFIX_BYTES + 1] = 1; });
  addWire('selector-zero-chunk', 2, (b) => b.writeUInt32BE(0, PREFIX_BYTES + 24));
  addWire('selector-offset-length-overflow', 2, (b) => {
    b.writeBigUInt64BE(8n, PREFIX_BYTES + 8);
    b.writeBigUInt64BE(4n, PREFIX_BYTES + 16);
    b.writeUInt32BE(8, PREFIX_BYTES + 24);
  });
  addWire('leaf-data-payload-length-mismatch', 7, (b) => b.writeUInt32BE(4, PREFIX_BYTES));
  addWire('leaf-data-proof-length-mismatch', 7, (b) => b.writeUInt16BE(48, PREFIX_BYTES + 4));
  addWire('leaf-data-proof-sibling-count-mismatch', 7, (b) => { b[PREFIX_BYTES + 8 + 3] ^= 1; });
  addWire('cancel-nonzero-reserved', 9, (b) => { b[PREFIX_BYTES + 2] = 1; });
  addWire('resume-zero-count', 10, (b) => b.writeUInt16BE(0, PREFIX_BYTES));
  addWire('resume-duplicate-leaves', 10, (b) => b.writeBigUInt64BE(6n, PREFIX_BYTES + 16));
  addWire('resume-nonzero-reserved', 10, (b) => { b[PREFIX_BYTES + 2] = 1; });
  addWire('outcome-unknown-status', 11, (b) => b.writeUInt16BE(6, PREFIX_BYTES));
  addWire('outcome-unknown-stage', 11, (b) => b.writeUInt16BE(5, PREFIX_BYTES + 2));
  addWire('outcome-integrity-count-too-large', 11, (b) => b.writeUInt32BE(2, PREFIX_BYTES + 8));
  addWire('close-nonzero-reserved', 12, (b) => { b[PREFIX_BYTES + 2] = 1; });

  const validCarrier = Buffer.from(positives[0].carrierHeaderHex, 'hex');
  for (const [id, mutate] of [
    ['carrier-domain-mismatch', (b) => { b[0] ^= 1; }],
    ['carrier-total-mismatch', (b) => b.writeUInt32BE(b.readUInt32BE(76) + 1, 76)],
    ['carrier-nonzero-offset', (b) => b.writeUInt32BE(1, 80)],
    ['carrier-zero-chunk', (b) => b.writeUInt16BE(0, 84)],
    ['carrier-final-flag-invalid', (b) => b.writeUInt16BE(2, 86)],
  ]) {
    const b = Buffer.from(validCarrier);
    mutate(b);
    negatives.push({ id, name: id, scope: 'carrier-header', bytesHex: hex(b), expected: 'structural refusal' });
  }
  return negatives;
}

export function makeCorpus() {
  const positives = buildPositives();
  return {
    format: 'qbdl-service-wire-v1-independent-byte-fixtures',
    provenance: {
      contract: 'evidence/qbs09-wire-contract.md',
      generator: 'generate-service-wire-v1.mjs using only Node Buffer byte writes',
      claim: 'structural encoding fixtures only; no signatures, authorization, live Merkle proofs, resource reservation, or transfer claims',
      opaqueFixtureBytes: 'all digests, transcript, message IDs, signatures, payloads, and proof sibling bytes are deterministic placeholders',
      merkleProofLengthRule: 'encoded sibling framing is count-u8 plus 48 bytes per sibling; largest current bound is 1537 bytes (32 siblings)',
      pairedCoseLimit: 4608,
    },
    layout: {
      servicePrefixBytes: PREFIX_BYTES,
      limitsBytes: LIMITS_BYTES,
      objectSelectorBytes: SELECTOR_BYTES,
      commonCarrierHeaderBytes: RECORD_HEADER_BYTES,
      recordDomainHex: hex(RECORD_DOMAIN),
      allNumbersBigEndian: true,
      offsets: {
        prefix: { magic: 0, schema: 4, variant: 6, profile: 8, flags: 10, reserved0: 12, descriptor: 16, context: 64, generation: 112, policyGeneration: 120, cancellationFence: 128, transferId: 136, entryId: 152, representation: 156, reserved1: 157, representationEpoch: 160, leafIndex: 168, count: 176, bodyLength: 180, creditSerial: 184, creditCeiling: 192, deadline: 200 },
        limits: { maxRecordBytes: 0, maxPagePayload: 4, maxCoseBytes: 8, maxIndexBytes: 12, maxMetadataBytes: 16, maxClosureBytes: 20, maxEntries: 24, maxInflight: 28, lanes: 30, windowBytes: 32, maxColdRows: 40, carrierMode: 44, flags: 46, maxLogicalExtent: 48, requestLifetimeMs: 56, reserved: 60 },
        selector: { objectClass: 0, reserved: 1, number: 4, total: 8, offset: 16, chunkLength: 24, reservedTail: 28 },
        carrier: { domain: 0, transcript: 12, messageId: 60, logicalTotal: 76, offset: 80, chunkLength: 84, final: 86 },
      },
    },
    limits: (() => { const encoded = encodeLimits(); return { bytesHex: hex(encoded.bytes), expected: Object.fromEntries(Object.entries(encoded.values).map(([k, v]) => [k, typeof v === 'bigint' ? String(v) : v])) }; })(),
    positives,
    additionalPositives: buildAdditionalPositives(),
    selectors: makeSelectorVectors(),
    boundaries: {
      maxServiceRecordBytes: 65776,
      maxPagePayloadBytes: 65536,
      maxPairedCoseBytes: 4608,
      maxIndexBytes: 8388608,
      maxMetadataBytes: 8388608,
      maxClosureBytes: 1048576,
      maxManifestObjectBytes: 2048,
      maxMerkleProofBytes: 1537,
      maxLeafPayloadBytes: 16384,
      maxLeafDataBodyBytes: 17929,
      maxLeafDataRecordBytes: 18137,
      maxResumeLeafCount: 8,
      carrierChunkBytes: 1024,
      maxCarrierFrameBytes: 1216,
      maxDescriptorPageChunks: 65,
      descriptorPageFinalChunkBytes: 240,
    },
    negatives: makeNegativeVectors(positives),
  };
}

export function generate() {
  return `${JSON.stringify(makeCorpus(), null, 2)}\n`;
}

if (typeof process !== 'undefined' && process.argv?.[1]
    && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const output = join(dirname(fileURLToPath(import.meta.url)), 'service-wire-v1.json');
  await writeFile(output, generate(), 'utf8');
}
