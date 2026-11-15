import { createHash } from 'node:crypto';
import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

// Independent bounded CBOR encoders for the approved QBDL/QASD fixture profile.
// This module has no production imports and is safe to import without running main().
const PROFILE = 'qbdl-stream-pq-1';
const QBDL_HEADER_BYTES = 64;
const PAGE_BYTES = 64;
const LEAF_BYTES = 16384;
const ROWS_PER_PAGE = 128;
const MAX_LOGICAL_BYTES = 1n << 40n;
const TREE_LEAF = Buffer.from('qbdl-stream-pq-1/leaf/v1\0', 'ascii');
const TREE_NODE = Buffer.from('qbdl-stream-pq-1/node/v1\0', 'ascii');
const TREE_EMPTY = Buffer.from('qbdl-stream-pq-1/empty/v1\0', 'ascii');
const CONTEXT_DOMAIN = Buffer.from('QBDL-QASD-CONTEXT-V1\0', 'ascii');
const CONTEXT_PAGE_DOMAIN = Buffer.from('QBDL-QASD-CONTEXT-PAGE-V1\0', 'ascii');
const ENTRY_PAGE_DOMAIN = Buffer.from('QBDL-QASD-ENTRY-PAGE-V1\0', 'ascii');
const BODY_DOMAIN = Buffer.from('QBDL-QASD-BODY-V1\0', 'ascii');

function cborHead(major, value) {
  const n = BigInt(value);
  if (n < 0n) throw new RangeError('negative CBOR argument');
  if (n < 24n) return Buffer.from([(major << 5) | Number(n)]);
  if (n <= 0xffn) return Buffer.from([(major << 5) | 24, Number(n)]);
  if (n <= 0xffffn) { const b = Buffer.alloc(3); b[0] = (major << 5) | 25; b.writeUInt16BE(Number(n), 1); return b; }
  if (n <= 0xffffffffn) { const b = Buffer.alloc(5); b[0] = (major << 5) | 26; b.writeUInt32BE(Number(n), 1); return b; }
  if (n <= 0xffffffffffffffffn) { const b = Buffer.alloc(9); b[0] = (major << 5) | 27; b.writeBigUInt64BE(n, 1); return b; }
  throw new RangeError('outside CBOR uint64');
}
const uint = value => cborHead(0, value);
const array = values => Buffer.concat([cborHead(4, values.length), ...values]);
const map = pairs => Buffer.concat([cborHead(5, pairs.length), ...pairs.flatMap(([key, value]) => [key, value])]);
const bstr = value => { const b = Buffer.from(value); return Buffer.concat([cborHead(2, b.length), b]); };
const tstr = value => { const b = Buffer.from(value, 'utf8'); return Buffer.concat([cborHead(3, b.length), b]); };
const NULL = Buffer.from([0xf6]);
function mapIntRows(entries) { return map(entries.map(([k,v]) => [uint(k),v])); }
function sha384(bytes) { return createHash('sha384').update(bytes).digest(); }
function sha256(bytes) { return createHash('sha256').update(bytes).digest(); }
function domainDigest(domain, encoded) { return sha384(Buffer.concat([domain, encoded])); }
function putU32BE(n) { const b = Buffer.alloc(4); b.writeUInt32BE(Number(n)); return b; }
function putU64BE(n) { const b = Buffer.alloc(8); b.writeBigUInt64BE(BigInt(n)); return b; }
function align(n, a = PAGE_BYTES) { return Math.ceil(n / a) * a; }

// Castagnoli CRC32C (reflected polynomial); QBDL stores this little-endian at 36.
function crc32c(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ ((crc & 1) ? 0x82f63b78 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function sourceIndexRow(entry) {
  // Field order follows BundleEntry's declared field order; absent optional meta is omitted.
  return map([
    [tstr('key'), tstr(entry.key)], [tstr('kind'), tstr(entry.kind)],
    [tstr('offset'), uint(entry.offset)], [tstr('length'), uint(entry.bytes.length)],
    [tstr('sha256'), bstr(entry.sha256)],
  ]);
}
function encodeSourceIndex(entries) { return array(entries.map(sourceIndexRow)); }
function encodeQbdl(entries) {
  let cursor = QBDL_HEADER_BYTES;
  const placed = entries.map((entry, ordinal) => {
    const offset = align(cursor);
    const out = { ...entry, ordinal, offset, sha256: sha256(entry.bytes) };
    cursor = offset + entry.bytes.length;
    return out;
  });
  const indexBytes = encodeSourceIndex(placed);
  const indexOffset = align(cursor);
  const totalLength = indexOffset + indexBytes.length;
  if (BigInt(totalLength) > MAX_LOGICAL_BYTES) throw new RangeError('source exceeds 1 TiB fixture cap');
  const source = Buffer.alloc(totalLength);
  for (const entry of placed) entry.bytes.copy(source, entry.offset);
  indexBytes.copy(source, indexOffset);
  source.write('QBDL', 0, 'ascii');
  source.writeUInt16LE(1, 4);
  source.writeUInt16LE(0, 6);
  source.writeUInt32LE(placed.length, 8);
  source.writeBigUInt64LE(BigInt(indexOffset), 12);
  source.writeBigUInt64LE(BigInt(indexBytes.length), 20);
  source.writeBigUInt64LE(BigInt(totalLength), 28);
  source.writeUInt32LE(0, 36);
  // The zero-initialized buffer supplies all-zero reserved bytes and zero padding.
  source.writeUInt32LE(crc32c(source), 36);
  const header = Buffer.from(source.subarray(0, QBDL_HEADER_BYTES));
  return { entries: placed, header, indexBytes, indexOffset, totalLength, source };
}

function treeCommon(contextId, entryId, generation, representation, keyEpoch, logicalLength) {
  return Buffer.concat([contextId, putU32BE(entryId), putU64BE(generation), Buffer.from([representation]), putU32BE(keyEpoch), putU64BE(logicalLength)]);
}
function entryTreeRoot({ contextId, entryId, generation, representation, keyEpoch, bytes }) {
  const common = treeCommon(contextId, entryId, generation, representation, keyEpoch, bytes.length);
  if (bytes.length === 0) return sha384(Buffer.concat([Buffer.from([2]), TREE_EMPTY, common]));
  if (bytes.length > LEAF_BYTES) throw new RangeError('fixture helper covers one-leaf entries only');
  // QBS-05 one-leaf tree: exact final logical/encoded length; no parent or padding node.
  return sha384(Buffer.concat([Buffer.from([0]), TREE_LEAF, common, putU64BE(0), putU32BE(bytes.length), putU32BE(bytes.length), bytes]));
}

function referenceDigests() {
  return {
    nonce: Buffer.alloc(32, 0x42),
    semanticContract: Buffer.alloc(48, 0xa1),
    policy: Buffer.alloc(48, 0xa2),
    consumerProfile: Buffer.alloc(48, 0xa3),
  };
}
function makePages(source, { generation, nonce, refs }) {
  const count = source.entries.length;
  const pageCount = Math.ceil(count / ROWS_PER_PAGE);
  const contextPages = [];
  for (let page = 0; page < pageCount; page++) {
    const first = page * ROWS_PER_PAGE;
    const slice = source.entries.slice(first, first + ROWS_PER_PAGE);
    const rows = slice.map((entry, i) => array([
      uint(entry.entryId), uint(entry.bytes.length), uint(0), uint(0),
    ]));
    const encoded = mapIntRows([[0,tstr(PROFILE)],[1,uint(1)],[2,uint(page)],[3,uint(first)],[4,array(rows)]]);
    contextPages.push({ page, first, rowCount: slice.length, bytes: encoded, digest: domainDigest(CONTEXT_PAGE_DOMAIN, encoded) });
  }
  const contextPageRefs = contextPages.map(p => array([uint(p.page),uint(p.first),uint(p.rowCount),uint(p.bytes.length),bstr(p.digest)]));
  const sourceCommit = mapIntRows([
    [0,uint(source.totalLength)],[1,uint(1)],[2,uint(0)],[3,bstr(sha384(source.header))],
    [4,uint(source.indexOffset)],[5,uint(source.indexBytes.length)],[6,bstr(sha384(source.indexBytes))],[7,bstr(sha384(source.source))],
  ]);
  const contextRoot = mapIntRows([
    [0,tstr(PROFILE)],[1,uint(1)],[2,bstr(nonce)],[3,uint(generation)],[4,NULL],[5,sourceCommit],
    [6,uint(LEAF_BYTES)],[7,uint(count)],[8,array(contextPageRefs)],[9,uint(0)],
    [10,bstr(refs.semanticContract)],[11,bstr(refs.policy)],[12,bstr(refs.consumerProfile)],
  ]);
  const contextId = domainDigest(CONTEXT_DOMAIN, contextRoot);
  return { sourceCommit, contextPages, contextRoot, contextId };
}
function makeEntryPages(source, contextId, generation) {
  const count = source.entries.length;
  const pages = [];
  const pageCount = Math.ceil(count / ROWS_PER_PAGE);
  for (let page = 0; page < pageCount; page++) {
    const first = page * ROWS_PER_PAGE;
    const slice = source.entries.slice(first, first + ROWS_PER_PAGE);
    const rows = slice.map(entry => {
      const treeRoot = entryTreeRoot({ contextId, entryId: entry.entryId, generation, representation: 0, keyEpoch: 0, bytes: entry.bytes });
      entry.treeRoot = treeRoot;
      return array([
        uint(entry.entryId), uint(entry.ordinal), tstr(entry.kind), uint(entry.innerVersion),
        uint(entry.offset), uint(entry.bytes.length), uint(0), uint(0), bstr(treeRoot),
      ]);
    });
    const encoded = mapIntRows([[0,tstr(PROFILE)],[1,uint(1)],[2,bstr(contextId)],[3,uint(page)],[4,uint(first)],[5,array(rows)]]);
    pages.push({ page, first, rowCount: slice.length, bytes: encoded, digest: domainDigest(ENTRY_PAGE_DOMAIN, encoded) });
  }
  return pages;
}
function makeOneFixture(name, entrySpecs, generation = 0n) {
  const refs = referenceDigests();
  const source = encodeQbdl(entrySpecs.map(e => ({ ...e, bytes: Buffer.from(e.bytes), entryId: e.entryId, innerVersion: e.innerVersion ?? 1 })));
  const context = makePages(source, { generation, nonce: refs.nonce, refs });
  const entryPages = makeEntryPages(source, context.contextId, generation);
  const entryPageRefs = entryPages.map(p => array([uint(p.page),uint(p.first),uint(p.rowCount),uint(p.bytes.length),bstr(p.digest)]));
  const body = mapIntRows([
    [0,tstr(PROFILE)],[1,uint(1)],[2,bstr(context.contextId)],[3,bstr(context.contextRoot)],
    [4,uint(generation)],[5,NULL],[6,uint(source.entries.length)],[7,array(entryPageRefs)],
  ]);
  const descriptorId = domainDigest(BODY_DOMAIN, body);
  return {
    name,
    provenance: 'Independent definite-CBOR/Node crypto fixture generated from the approved QBS-06 and QBS-05 candidate contracts. Candidate identity only; no publisher authority, disclosure permission, or consumer admission is implied.',
    placeholders: { nonce_hex: refs.nonce.toString('hex'), semantic_contract_ref_hex: refs.semanticContract.toString('hex'), policy_ref_hex: refs.policy.toString('hex'), consumer_profile_ref_hex: refs.consumerProfile.toString('hex'), note: 'Fixed nonzero fixture values only; they are not registered references and carry no rights or inner-format claim.' },
    qbdl_source: {
      version: 1, flags: 0, source_length: source.totalLength, entry_count: source.entries.length,
      header_hex: source.header.toString('hex'), index_offset: source.indexOffset, index_length: source.indexBytes.length,
      index_hex: source.indexBytes.toString('hex'), complete_source_hex: source.source.toString('hex'),
      header_sha384_hex: sha384(source.header).toString('hex'), index_sha384_hex: sha384(source.indexBytes).toString('hex'),
      complete_source_sha384_hex: sha384(source.source).toString('hex'),
      crc32c_le_hex: source.header.subarray(36,40).toString('hex'),
      entries: source.entries.map(e => ({ ordinal:e.ordinal, key:e.key, kind:e.kind, inner_version:e.innerVersion, offset:e.offset, length:e.bytes.length, opaque_entry_bytes_hex:e.bytes.toString('hex'), sha256_hex:e.sha256.toString('hex'), entry_id:e.entryId })),
    },
    source_commit_hex: context.sourceCommit.toString('hex'),
    context_pages: context.contextPages.map(p => ({ page_number:p.page, first_index_ordinal:p.first, row_count:p.rowCount, digest_hex:p.digest.toString('hex'), encoded_hex:p.bytes.toString('hex') })),
    context_root_hex: context.contextRoot.toString('hex'),
    context_id_sha384_hex: context.contextId.toString('hex'),
    entry_trees: source.entries.map(e => ({ entry_id:e.entryId, logical_length:e.bytes.length, representation:0, key_epoch:0, leaf_count:e.bytes.length === 0 ? 0 : 1, root_hex:e.treeRoot.toString('hex'), tree_kind:e.bytes.length === 0 ? 'qbs05-context-bound-empty-root' : 'qbs05-single-leaf-root' })),
    entry_pages: entryPages.map(p => ({ page_number:p.page, first_index_ordinal:p.first, row_count:p.rowCount, digest_hex:p.digest.toString('hex'), encoded_hex:p.bytes.toString('hex') })),
    descriptor_body_hex: body.toString('hex'),
    descriptor_id_sha384_hex: descriptorId.toString('hex'),
  };
}

export function makeCorpus() {
  const singleBytes = Buffer.from([0x51,0x00,0xff]);
  const emptySpec = [{ entryId:7001, key:'fixture-empty', kind:'opaque', innerVersion:1, bytes:Buffer.alloc(0) }];
  const oneSpec = [{ entryId:7002, key:'fixture-one', kind:'opaque', innerVersion:1, bytes:singleBytes }];
  const manySpecs = Array.from({length:129}, (_,i) => ({
    entryId:8000+i, key:`fixture-${String(i).padStart(3,'0')}`, kind:'opaque', innerVersion:1,
    bytes:Buffer.from([i & 0xff]),
  }));
  return {
    schema: 'qasd-plain-v1-independent-encoding-fixtures',
    generator: 'generate-qasd-plain-v1.mjs',
    construction: 'Hand-built shortest definite CBOR plus Node SHA-256/SHA-384 and local CRC32C. No production reader/writer, QASD implementation, or Rust code is imported.',
    contract_basis: ['evidence/qbs06-descriptor-contract.md','evidence/qbs05-proof-contract.md','schemas/qasd-plain-v1.cddl'],
    tree_scope: 'All non-empty fixture entries are <=3 bytes and therefore have exactly one QBS-05 plaintext leaf; RFC9162 has no parent for one leaf. Empty logical entry is included separately and uses QBS-05 context-bound empty root with zero leaves.',
    fixtures: [
      makeOneFixture('empty-bundle', []),
      makeOneFixture('one-opaque-entry', oneSpec),
      makeOneFixture('129-tiny-opaque-entries-two-pages', manySpecs),
      makeOneFixture('empty-opaque-entry-empty-tree-special', emptySpec),
    ],
  };
}
export function generate() { return makeCorpus(); }

async function main() {
  const here = dirname(fileURLToPath(import.meta.url));
  const output = join(here, 'qasd-plain-v1.json');
  await writeFile(output, JSON.stringify(makeCorpus(), null, 2) + '\n', 'utf8');
  if (typeof process !== 'undefined' && process.stdout) process.stdout.write(`wrote QASD plain fixtures to ${output}\n`);
}
if (typeof process !== 'undefined' && process.argv && process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
