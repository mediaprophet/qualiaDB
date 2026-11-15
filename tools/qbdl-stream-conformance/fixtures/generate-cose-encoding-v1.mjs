import { createHash } from 'node:crypto';
import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

// Standalone RFC 8949 definite-length encoder for this fixed QBS-03 profile.
// It does not import production code or implement either signature primitive.
const PROFILE = 'qbdl-stream-pq-1';
const AAD = Buffer.from('qbdl-stream-pq-1/paired-sign1/v1', 'ascii');
const ML_ALG = -49;
const ED_ALG = -19;
const ML_PUBLIC_BYTES = 1952;
const ED_PUBLIC_BYTES = 32;
const ML_SIGNATURE_BYTES = 3309;
const ED_SIGNATURE_BYTES = 64;
const KEY_ID_DOMAIN = Buffer.from('QBDL-COSE-KEY-ID-V1\0', 'ascii');
const ML_KEY_PREFIX = Buffer.from('a30107033830205907a0', 'hex');
const ED_KEY_PREFIX = Buffer.from('a4010103322006215820', 'hex');

function head(major, value) {
  const n = BigInt(value);
  if (n < 0n) throw new RangeError('negative CBOR argument');
  if (n < 24n) return Buffer.from([(major << 5) | Number(n)]);
  if (n <= 0xffn) return Buffer.from([(major << 5) | 24, Number(n)]);
  if (n <= 0xffffn) { const b = Buffer.alloc(3); b[0] = (major << 5) | 25; b.writeUInt16BE(Number(n), 1); return b; }
  if (n <= 0xffffffffn) { const b = Buffer.alloc(5); b[0] = (major << 5) | 26; b.writeUInt32BE(Number(n), 1); return b; }
  if (n <= 0xffffffffffffffffn) { const b = Buffer.alloc(9); b[0] = (major << 5) | 27; b.writeBigUInt64BE(n, 1); return b; }
  throw new RangeError('outside CBOR uint64');
}
const uint = n => head(0, n);
const bytes = b => Buffer.concat([head(2, b.length), Buffer.from(b)]);
const text = s => { const b = Buffer.from(s, 'utf8'); return Buffer.concat([head(3, b.length), b]); };
const array = xs => Buffer.concat([head(4, xs.length), ...xs]);
const map = pairs => Buffer.concat([head(5, pairs.length), ...pairs.flatMap(([k, v]) => [k, v])]);
function integer(n) { return n >= 0 ? uint(n) : head(1, BigInt(-1 - n)); }
function keyId(prefix, rawKey) {
  return createHash('sha384').update(KEY_ID_DOMAIN).update(prefix).update(rawKey).digest();
}
function syntheticBytes(n, multiplier, addend) {
  return Buffer.from(Array.from({ length: n }, (_, i) => (i * multiplier + addend) & 0xff));
}

export function defaultTestKeys() {
  return {
    mlPublic: syntheticBytes(ML_PUBLIC_BYTES, 29, 7),
    edPublic: syntheticBytes(ED_PUBLIC_BYTES, 11, 3),
  };
}

export function encodeBinding({ descriptorDigest, mlKeyId, edKeyId, controllerEpoch, purpose, audience }) {
  for (const [name, value, size] of [['descriptorDigest', descriptorDigest, 48], ['mlKeyId', mlKeyId, 48], ['edKeyId', edKeyId, 48]]) {
    if (!Buffer.isBuffer(value) || value.length !== size) throw new RangeError(`${name} must be ${size} bytes`);
  }
  if (!Buffer.isBuffer(purpose) || purpose.length < 1 || purpose.length > 64) throw new RangeError('purpose must be 1..64 opaque bytes');
  if (!Buffer.isBuffer(audience) || audience.length < 1 || audience.length > 128) throw new RangeError('audience must be 1..128 opaque bytes');
  const epoch = BigInt(controllerEpoch);
  if (epoch < 0n || epoch > 0xffffffffffffffffn) throw new RangeError('controllerEpoch must be uint64');
  return map([
    [uint(0), text(PROFILE)], [uint(1), uint(1)], [uint(2), bytes(descriptorDigest)],
    [uint(3), array([array([integer(ML_ALG), bytes(mlKeyId)]), array([integer(ED_ALG), bytes(edKeyId)])])],
    [uint(4), uint(epoch)], [uint(5), bytes(purpose)], [uint(6), bytes(audience)],
  ]);
}

export function encodeProtected(alg, kid) {
  if (alg !== ML_ALG && alg !== ED_ALG) throw new RangeError('unsupported algorithm');
  if (!Buffer.isBuffer(kid) || kid.length !== 48) throw new RangeError('kid must be 48 bytes');
  return map([[uint(1), integer(alg)], [uint(4), bytes(kid)]]);
}

export function sigStructure(protectedBytes, bindingBytes) {
  return array([text('Signature1'), bytes(protectedBytes), bytes(AAD), bytes(bindingBytes)]);
}

function sign1(protectedBytes, payload, signature) {
  return Buffer.concat([Buffer.from([0xd2]), array([bytes(protectedBytes), map([]), bytes(payload), bytes(signature)])]);
}
export function encodePair(bindingBytes, mlSignature, edSignature, { mlProtected, edProtected } = {}) {
  if (!mlProtected || !edProtected) throw new Error('explicit independently encoded protected headers required');
  return array([sign1(mlProtected, bindingBytes, mlSignature), sign1(edProtected, bindingBytes, edSignature)]);
}

export function buildFixture({ mlPublic, edPublic, mlSignature, edSignature, descriptorDigest, controllerEpoch = 9n, purpose = Buffer.from('qbdl:publish', 'ascii'), audience = Buffer.from('model.bundle.v1', 'ascii') } = {}) {
  const defaults = defaultTestKeys();
  mlPublic = Buffer.from(mlPublic ?? defaults.mlPublic);
  edPublic = Buffer.from(edPublic ?? defaults.edPublic);
  if (mlPublic.length !== ML_PUBLIC_BYTES || edPublic.length !== ED_PUBLIC_BYTES) throw new RangeError('wrong raw public-key size');
  const mlKid = keyId(ML_KEY_PREFIX, mlPublic);
  const edKid = keyId(ED_KEY_PREFIX, edPublic);
  descriptorDigest = Buffer.from(descriptorDigest ?? Buffer.from(Array.from({ length: 48 }, (_, i) => i)));
  if (descriptorDigest.length !== 48) throw new RangeError('descriptor digest must be 48 bytes');
  const bindingBytes = encodeBinding({ descriptorDigest, mlKeyId: mlKid, edKeyId: edKid, controllerEpoch, purpose: Buffer.from(purpose), audience: Buffer.from(audience) });
  const mlProtected = encodeProtected(ML_ALG, mlKid);
  const edProtected = encodeProtected(ED_ALG, edKid);
  const mlSigStructure = sigStructure(mlProtected, bindingBytes);
  const edSigStructure = sigStructure(edProtected, bindingBytes);
  const dummy = mlSignature === undefined || edSignature === undefined;
  mlSignature = Buffer.from(mlSignature ?? Buffer.alloc(ML_SIGNATURE_BYTES));
  edSignature = Buffer.from(edSignature ?? Buffer.alloc(ED_SIGNATURE_BYTES));
  if (mlSignature.length !== ML_SIGNATURE_BYTES || edSignature.length !== ED_SIGNATURE_BYTES) throw new RangeError('wrong raw signature length');
  const envelopeBytes = encodePair(bindingBytes, mlSignature, edSignature, { mlProtected, edProtected });
  return { mlPublic, edPublic, mlKid, edKid, descriptorDigest, purpose: Buffer.from(purpose), audience: Buffer.from(audience), controllerEpoch: BigInt(controllerEpoch), bindingBytes, mlProtected, edProtected, mlSigStructure, edSigStructure, mlSignature, edSignature, envelopeBytes, dummySignatures: dummy };
}

function malformedVectors(f) {
  const base = f.envelopeBytes;
  const bindingAlt = encodeBinding({ descriptorDigest: Buffer.alloc(48, 0xa5), mlKeyId: f.mlKid, edKeyId: f.edKid, controllerEpoch: f.controllerEpoch, purpose: f.purpose, audience: f.audience });
  const malformedProtectedOrder = map([[uint(4), bytes(f.mlKid)], [uint(1), integer(ML_ALG)]]);
  const malformedProtectedExtra = map([[uint(1), integer(ML_ALG)], [uint(4), bytes(f.mlKid)], [uint(2), uint(1)]]);
  const wrongKid = map([[uint(1), integer(ML_ALG)], [uint(4), bytes(f.mlKid.subarray(0, 47))]]);
  const unknownAlg = map([[uint(1), integer(-8)], [uint(4), bytes(f.mlKid)]]);
  const badFirst = (protectedBytes, payload = f.bindingBytes, signature = f.mlSignature) => Buffer.concat([Buffer.from([0xd2]), array([bytes(protectedBytes), map([]), bytes(payload), bytes(signature)])]);
  const second = (protectedBytes = f.edProtected, payload = f.bindingBytes, signature = f.edSignature) => Buffer.concat([Buffer.from([0xd2]), array([bytes(protectedBytes), map([]), bytes(payload), bytes(signature)])]);
  const validMembers = [badFirst(f.mlProtected), second()];
  const entries = [
    ['truncated-final-byte', base.subarray(0, base.length - 1), 'Final CBOR byte omitted; outer/item is incomplete.'],
    ['trailing-byte', Buffer.concat([base, Buffer.from([0])]), 'Trailing bytes follow the exact two-member envelope.'],
    ['outer-array-count-one', Buffer.concat([Buffer.from([0x81]), base.subarray(1)]), 'Outer array length is not exactly two.'],
    ['missing-first-tag', Buffer.concat([base.subarray(0, 1), base.subarray(2)]), 'First Sign1 tag 18 is absent.'],
    ['swapped-member-roles', array([second(), badFirst(f.mlProtected)]), 'Ed25519 member appears in the required ML-DSA-first position.'],
    ['protected-map-out-of-order', array([badFirst(malformedProtectedOrder), second()]), 'Protected map keys are not in exact profile order.'],
    ['protected-map-extra-field', array([badFirst(malformedProtectedExtra), second()]), 'Protected map contains unreviewed alg/key-id extension.'],
    ['protected-kid-short', array([badFirst(wrongKid), second()]), 'Protected key id is not exactly 48 bytes.'],
    ['unsupported-polymorphic-eddsa-alg', array([badFirst(unknownAlg), second()]), 'Algorithm -8 is outside the profile; expected -49 first.'],
    ['wrong-signature-length', array([badFirst(f.mlProtected), second(f.edProtected, f.bindingBytes, f.edSignature.subarray(0, 63))]), 'Ed25519 signature is 63 bytes rather than 64.'],
    ['mismatched-payloads', array([badFirst(f.mlProtected), second(f.edProtected, bindingAlt)]), 'Two validly encoded binding payloads differ byte-for-byte.'],
    ['nonminimal-outer-array', Buffer.concat([Buffer.from([0x98, 0x02]), base.subarray(1)]), 'Outer array length uses nonminimal additional-information encoding.'],
    ['extra-protected-byte', array([badFirst(Buffer.concat([f.mlProtected, Buffer.from([0])])), second()]), 'Protected bstr contains trailing data after its exact map.'],
    ['unprotected-header-nonempty', array([Buffer.concat([Buffer.from([0xd2, 0x84]), bytes(f.mlProtected), Buffer.from([0xa1, 0x01, 0x01]), bytes(f.bindingBytes), bytes(f.mlSignature)]), second()]), 'Unprotected map is nonempty.'],
    ['extra-tag-nesting', array([Buffer.concat([Buffer.from([0xd2]), badFirst(f.mlProtected)]), second()]), 'Unexpected nested tag precedes the Sign1 array.'],
  ];
  return entries.map(([id, bytesValue, description]) => ({ id, envelope_hex: bytesValue.toString('hex'), expected: 'refuse', description }));
}

export function makeCorpus(options = {}) {
  const f = buildFixture(options);
  const malformed = malformedVectors(f);
  return {
    schema: 'qbdl-cose-encoding-v1',
    provenance: {
      encoder: 'generate-cose-encoding-v1.mjs; standalone definite-CBOR encoder and node:crypto SHA-384; no production implementation imported',
      contract: 'QBS-03 candidate encoding contract dated 2026-11-12; independent encoding fixture only',
      public_keys: 'Synthetic deterministic fixed-length byte patterns for encoding coverage; these are NOT provider-generated keys and are NOT asserted cryptographically valid.',
      signatures: f.dummySignatures ? 'All-zero fixed-size placeholders; envelope shape and sizes are encoded, signatures are NOT cryptographically valid.' : 'Caller-supplied fixed-size signatures; this generator does not assert or verify their validity.',
      cryptographic_scope: 'Key identifiers are SHA-384 of the contract-defined domain-separated deterministic COSE_Key encodings. No signature primitive, authority, trust, permission or policy result is asserted.'
    },
    valid_encoding_fixture: {
      algorithm_order: [ML_ALG, ED_ALG],
      key_public_bytes: { ml_dsa_65_hex: f.mlPublic.toString('hex'), ed25519_hex: f.edPublic.toString('hex') },
      key_ids_sha384_hex: { ml_dsa_65: f.mlKid.toString('hex'), ed25519: f.edKid.toString('hex') },
      binding_fields: { profile: PROFILE, schema_version: 1, descriptor_digest_hex: f.descriptorDigest.toString('hex'), controller_epoch: f.controllerEpoch.toString(), purpose_hex: f.purpose.toString('hex'), audience_hex: f.audience.toString('hex') },
      binding_hex: f.bindingBytes.toString('hex'),
      external_aad_ascii: AAD.toString('ascii'),
      protected_headers_hex: { ml_dsa_65: f.mlProtected.toString('hex'), ed25519: f.edProtected.toString('hex') },
      sig_structure_hex: { ml_dsa_65: f.mlSigStructure.toString('hex'), ed25519: f.edSigStructure.toString('hex') },
      signature_bytes: { ml_dsa_65_hex: f.mlSignature.toString('hex'), ed25519_hex: f.edSignature.toString('hex'), cryptographically_valid: false },
      envelope_hex: f.envelopeBytes.toString('hex'),
      expected: 'encoding-accept-only; signature verification deliberately not claimed'
    },
    malformed_vectors: malformed
  };
}

async function main() {
  const here = dirname(fileURLToPath(import.meta.url));
  const output = join(here, 'cose-encoding-v1.json');
  await writeFile(output, JSON.stringify(makeCorpus(), null, 2) + '\n', 'utf8');
  process.stdout.write(`wrote ${malformedCount()} malformed vectors to ${output}\n`);
}
function malformedCount() { return malformedVectors(buildFixture()).length; }
if (typeof process !== 'undefined' && process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
