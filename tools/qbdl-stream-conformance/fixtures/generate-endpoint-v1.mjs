import { createHash, createHmac } from 'node:crypto';
import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

// Independent endpoint transcript and key-schedule vectors. All share/secrets
// and proof envelopes here are synthetic byte inputs, not live KEM/DH vectors.
const PROFILE = Buffer.from('qpr-pq-1/endpoint-v1', 'ascii');
const TRANSCRIPT_DOMAIN = Buffer.from('QPR-TRANSCRIPT-PQ-1', 'ascii');
const MAGIC = Buffer.from('QSESSION-ENDPOINT-V1\0', 'ascii');
const T0_INFO_I2R = Buffer.from('qpr-pq-1/qlink/i2r/transcript', 'ascii');
const T0_INFO_R2I = Buffer.from('qpr-pq-1/qlink/r2i/transcript', 'ascii');
const FINISHED_INFO_I2R = Buffer.from('qpr-pq-1/endpoint-v1/finished/i2r', 'ascii');
const FINISHED_INFO_R2I = Buffer.from('qpr-pq-1/endpoint-v1/finished/r2i', 'ascii');
const TRAFFIC_INFO_I2R = Buffer.from('qpr-pq-1/endpoint-v1/traffic/i2r', 'ascii');
const TRAFFIC_INFO_R2I = Buffer.from('qpr-pq-1/endpoint-v1/traffic/r2i', 'ascii');
const ROLE_I2R = Buffer.from('qpr-pq-1/finished/i2r', 'ascii');
const ROLE_R2I = Buffer.from('qpr-pq-1/finished/r2i', 'ascii');
const TRANSCRIPT_LIMIT = 4096;
const MAX_MESSAGE = 8192;
const MAX_FRAGMENTS = 8;
const MAX_PROOF = 4608;

function sha384(bytes) { return createHash('sha384').update(bytes).digest(); }
function hmac384(key, bytes) { return createHmac('sha384', key).update(bytes).digest(); }
function hkdfExtractExpand(ikm, salt, info, length = 32) {
  const prk = hmac384(salt, ikm);
  const blocks = [];
  let previous = Buffer.alloc(0);
  for (let counter = 1; Buffer.concat(blocks).length < length; counter++) {
    previous = hmac384(prk, Buffer.concat([previous, info, Buffer.from([counter])]));
    blocks.push(previous);
  }
  return { prk, okm: Buffer.concat(blocks).subarray(0, length) };
}
function be16(n) { const b = Buffer.alloc(2); b.writeUInt16BE(n); return b; }
function be32(n) { const b = Buffer.alloc(4); b.writeUInt32BE(n); return b; }
function be64(n) { const b = Buffer.alloc(8); b.writeBigUInt64BE(BigInt(n)); return b; }
function repeated(n, b) { return Buffer.alloc(n, b); }
function pattern(n, start, step) { return Buffer.from(Array.from({length:n}, (_,i)=>(start+i*step)&0xff)); }
function frame(label, value) {
  if (label.length > 0xffff || value.length > 0xffffffff) throw new RangeError('Transcript item exceeds length field');
  return Buffer.concat([be16(label.length), label, be32(value.length), value]);
}
function transcript(items) {
  const encoded = Buffer.concat(items.map(([label, value]) => frame(Buffer.from(label, 'ascii'), value)));
  if (encoded.length > TRANSCRIPT_LIMIT) throw new RangeError('Transcript exceeds 4096 bytes');
  return { encoded, digest: sha384(Buffer.concat([TRANSCRIPT_DOMAIN, encoded])) };
}
function sequence(values, count) { return values.slice(0, count); }
function u64Bytes(v) { return be64(v); }
function mutationEffect(base, changed, field) {
  const compare = ['clientSignatureCommitment','serverSignatureCommitment','T0','T1','T2','responderFinished','initiatorFinished','baseI2R','baseR2I','trafficI2R','trafficR2I'];
  return { id:field, mutation:changed.mutation, changed: Object.fromEntries(compare.map(k => [k, hex(base.outputs[k]) !== hex(changed.outputs[k])])) };
}
function hex(b) { return Buffer.from(b).toString('hex'); }

function commonIntent() {
  const fields = {
    initiator: repeated(48,0x11), responder: repeated(48,0x22), service: repeated(48,0x33),
    purpose: repeated(48,0x44), instrument: repeated(48,0x55), scope: repeated(48,0x66),
    operation: repeated(16,0x77), initiatorMlKeyId: repeated(48,0x88), initiatorEdKeyId: repeated(48,0x99),
    responderMlKeyId: repeated(48,0xaa), responderEdKeyId: repeated(48,0xbb),
    initiatorControllerEpoch: 0x0102030405060708n, responderControllerEpoch: 0x1112131415161718n,
    expiresUnix: 0x2122232425262728n,
  };
  const bytes = Buffer.concat([
    fields.initiator,fields.responder,fields.service,fields.purpose,fields.instrument,fields.scope,
    fields.operation,fields.initiatorMlKeyId,fields.initiatorEdKeyId,fields.responderMlKeyId,fields.responderEdKeyId,
    u64Bytes(fields.initiatorControllerEpoch),u64Bytes(fields.responderControllerEpoch),u64Bytes(fields.expiresUnix),
  ]);
  if (bytes.length !== 520) throw new Error(`common intent must be 520 bytes, got ${bytes.length}`);
  return { fields, bytes };
}

function buildVector(overrides = {}) {
  const { fields, bytes:baseIntent } = commonIntent();
  const intent = Buffer.from(overrides.intent ?? baseIntent);
  const clientNonce = Buffer.from(overrides.clientNonce ?? pattern(32,0x31,7));
  const responderNonce = Buffer.from(overrides.responderNonce ?? pattern(32,0x51,9));
  const clientOffer = Buffer.from(overrides.clientOffer ?? Buffer.concat([be16(1),be16(1400),be16(8192),Buffer.from([8,0])]));
  const serverSelection = Buffer.from(overrides.serverSelection ?? Buffer.concat([be16(1),be16(1500),be16(1400),be16(8192),Buffer.from([8,0])]));
  const initiatorShare = Buffer.from(overrides.initiatorShare ?? pattern(1216,0x61,13));
  const responderShare = Buffer.from(overrides.responderShare ?? pattern(1120,0x71,17));
  const clientProofEnvelope = Buffer.from(overrides.clientProofEnvelope ?? pattern(160,0x81,19));
  const serverProofEnvelope = Buffer.from(overrides.serverProofEnvelope ?? pattern(192,0x91,23));
  const kemSecretInput = Buffer.from(overrides.kemSecretInput ?? repeated(32,0xa5));
  const dhSecretInput = Buffer.from(overrides.dhSecretInput ?? repeated(32,0x5a));
  const responderFinishedRole = Buffer.from(overrides.responderFinishedRole ?? ROLE_R2I);
  const initiatorFinishedRole = Buffer.from(overrides.initiatorFinishedRole ?? ROLE_I2R);
  if (intent.length !== 520 || clientNonce.length !== 32 || responderNonce.length !== 32 || clientOffer.length !== 8 || serverSelection.length !== 10 || initiatorShare.length !== 1216 || responderShare.length !== 1120 || kemSecretInput.length !== 32 || dhSecretInput.length !== 32) throw new RangeError('fixed endpoint field length mismatch');

  const clientHello = Buffer.concat([MAGIC,Buffer.from([1]),intent,clientNonce,clientOffer,initiatorShare,be16(clientProofEnvelope.length),clientProofEnvelope]);
  const items = [];
  const add = (label, value) => items.push([label,Buffer.from(value)]);
  add('endpoint-profile',PROFILE);
  add('intent',intent);
  add('client-nonce',clientNonce);
  add('client-offer',clientOffer);
  add('initiator-share',initiatorShare);
  const clientCommit = transcript(items);
  const clientProofDigest = sha384(clientProofEnvelope);
  add('client-proof',clientProofDigest);
  add('responder-nonce',responderNonce);
  add('server-selection',serverSelection);
  add('responder-share',responderShare);
  const serverCommit = transcript(items);
  const serverProofDigest = sha384(serverProofEnvelope);
  add('server-proof',serverProofDigest);
  const beforeResponderFinished = transcript(items);
  const t0 = beforeResponderFinished.digest;

  const ikm = Buffer.concat([kemSecretInput,dhSecretInput]);
  const baseI2R = hkdfExtractExpand(ikm,t0,T0_INFO_I2R).okm;
  const baseR2I = hkdfExtractExpand(ikm,t0,T0_INFO_R2I).okm;
  const finishedI2R = hkdfExtractExpand(baseI2R,t0,FINISHED_INFO_I2R).okm;
  const finishedR2I = hkdfExtractExpand(baseR2I,t0,FINISHED_INFO_R2I).okm;
  const responderFinishedInput = Buffer.concat([t0,responderFinishedRole]);
  const responderFinished = hmac384(finishedR2I,responderFinishedInput);
  const serverHello = Buffer.concat([MAGIC,Buffer.from([2]),sha384(clientHello),clientNonce,responderNonce,serverSelection,responderShare,be16(serverProofEnvelope.length),serverProofEnvelope,responderFinished]);
  add('responder-finished',responderFinished);
  const afterResponderFinished = transcript(items);
  const t1 = afterResponderFinished.digest;
  const initiatorFinishedInput = Buffer.concat([t1,initiatorFinishedRole]);
  const initiatorFinished = hmac384(finishedI2R,initiatorFinishedInput);
  const initiatorFinishedWire = Buffer.concat([MAGIC,Buffer.from([3]),clientNonce,responderNonce,t1,initiatorFinished]);
  add('initiator-finished',initiatorFinished);
  const afterInitiatorFinished = transcript(items);
  const t2 = afterInitiatorFinished.digest;
  const trafficI2R = hkdfExtractExpand(baseI2R,t2,TRAFFIC_INFO_I2R).okm;
  const trafficR2I = hkdfExtractExpand(baseR2I,t2,TRAFFIC_INFO_R2I).okm;

  const maxSizes = {
    clientHello: MAGIC.length+1+520+32+8+1216+2+MAX_PROOF,
    serverHello: MAGIC.length+1+48+32+32+10+1120+2+MAX_PROOF+48,
    initiatorFinished: MAGIC.length+1+32+32+48+48,
  };
  if (clientHello.length > MAX_MESSAGE || serverHello.length > MAX_MESSAGE || initiatorFinishedWire.length > MAX_MESSAGE) throw new Error('actual synthetic message exceeds max message size');
  const outputs = {
    clientSignatureCommitment:clientCommit.digest, serverSignatureCommitment:serverCommit.digest,
    T0:t0,T1:t1,T2:t2,responderFinished,initiatorFinished,baseI2R,baseR2I,trafficI2R,trafficR2I,
  };
  return {
    mutation: overrides.mutation ?? null,
    commonIntent: intent,
    clientNonce,responderNonce,clientOffer,serverSelection,initiatorShare,responderShare,
    clientProofEnvelope,serverProofEnvelope,clientHello,serverHello,initiatorFinishedWire,
    items,
    clientCommit,serverCommit,beforeResponderFinished,afterResponderFinished,afterInitiatorFinished,
    kemSecretInput,dhSecretInput,ikm,baseI2R,baseR2I,finishedI2R,finishedR2I,
    responderFinishedRole,initiatorFinishedRole,responderFinishedInput,initiatorFinishedInput,
    responderFinished,initiatorFinished,t0,t1,t2,trafficI2R,trafficR2I,outputs,maxSizes,
  };
}

function publicVector(v) {
  const stages = [
    ['client_signature_commitment',v.clientCommit],
    ['server_signature_commitment',v.serverCommit],
    ['T0_before_responder_finished',v.beforeResponderFinished],
    ['T1_after_responder_finished',v.afterResponderFinished],
    ['T2_after_initiator_finished',v.afterInitiatorFinished],
  ];
  return {
    mutation: v.mutation,
    common_intent_hex: hex(v.commonIntent),
    intent_fields: {
      initiator_hex:hex(v.commonIntent.subarray(0,48)), responder_hex:hex(v.commonIntent.subarray(48,96)),
      service_hex:hex(v.commonIntent.subarray(96,144)), purpose_hex:hex(v.commonIntent.subarray(144,192)),
      instrument_hex:hex(v.commonIntent.subarray(192,240)), scope_hex:hex(v.commonIntent.subarray(240,288)),
      operation_hex:hex(v.commonIntent.subarray(288,304)), initiator_ml_key_id_hex:hex(v.commonIntent.subarray(304,352)),
      initiator_ed_key_id_hex:hex(v.commonIntent.subarray(352,400)), responder_ml_key_id_hex:hex(v.commonIntent.subarray(400,448)),
      responder_ed_key_id_hex:hex(v.commonIntent.subarray(448,496)), initiator_controller_epoch_hex:hex(v.commonIntent.subarray(496,504)),
      responder_controller_epoch_hex:hex(v.commonIntent.subarray(504,512)), expires_unix_hex:hex(v.commonIntent.subarray(512,520)),
    },
    nonces_hex: { client:hex(v.clientNonce), responder:hex(v.responderNonce) },
    offer_hex:hex(v.clientOffer), selection_hex:hex(v.serverSelection),
    synthetic_share_inputs_hex: { initiator:hex(v.initiatorShare), responder:hex(v.responderShare) },
    synthetic_proof_envelopes_hex: { client:hex(v.clientProofEnvelope), server:hex(v.serverProofEnvelope), cryptographically_valid:false },
    client_hello_wire_hex:hex(v.clientHello), client_hello_sha384_hex:hex(sha384(v.clientHello)),
    server_hello_wire_hex:hex(v.serverHello), initiator_finished_wire_hex:hex(v.initiatorFinishedWire),
    transcript: stages.map(([name,stage]) => ({name,encoded_length:stage.encoded.length,encoded_items_hex:hex(stage.encoded),digest_hex:hex(stage.digest)})),
    schedule_inputs: {
      ml_kem_shared_secret_synthetic_hex:hex(v.kemSecretInput), x25519_shared_secret_synthetic_hex:hex(v.dhSecretInput),
      input_key_material_order:'ml_kem_shared_secret || x25519_shared_secret', ikm_hex:hex(v.ikm),
    },
    schedule_outputs: {
      T0_hex:hex(v.t0), base_i2r_hkdf_sha384_hex:hex(v.baseI2R), base_r2i_hkdf_sha384_hex:hex(v.baseR2I),
      finished_key_i2r_hkdf_sha384_hex:hex(v.finishedI2R), finished_key_r2i_hkdf_sha384_hex:hex(v.finishedR2I),
      responder_finished_r2i_hmac_sha384_hex:hex(v.responderFinished), T1_hex:hex(v.t1),
      initiator_finished_i2r_hmac_sha384_hex:hex(v.initiatorFinished), T2_hex:hex(v.t2),
      responder_finished_hmac_input_hex:hex(v.responderFinishedInput), initiator_finished_hmac_input_hex:hex(v.initiatorFinishedInput),
      traffic_key_i2r_hkdf_sha384_hex:hex(v.trafficI2R), traffic_key_r2i_hkdf_sha384_hex:hex(v.trafficR2I),
      info_ascii: {base_i2r:T0_INFO_I2R.toString('ascii'),base_r2i:T0_INFO_R2I.toString('ascii'),finished_i2r:FINISHED_INFO_I2R.toString('ascii'),finished_r2i:FINISHED_INFO_R2I.toString('ascii'),traffic_i2r:TRAFFIC_INFO_I2R.toString('ascii'),traffic_r2i:TRAFFIC_INFO_R2I.toString('ascii')},
      hmac_role_ascii: {i2r:ROLE_I2R.toString('ascii'),r2i:ROLE_R2I.toString('ascii')},
    },
    role_effects: {
      i2r_and_r2i_base_keys_differ:!v.baseI2R.equals(v.baseR2I),
      i2r_and_r2i_finished_keys_differ:!v.finishedI2R.equals(v.finishedR2I),
      correct_responder_tag_hex:hex(v.responderFinished),
      wrong_role_same_r2i_finished_key_hex:hex(hmac384(v.finishedR2I,Buffer.concat([v.t0,ROLE_I2R]))),
      wrong_role_tag_differs:!hmac384(v.finishedR2I,Buffer.concat([v.t0,ROLE_I2R])).equals(v.responderFinished),
      direction_mapping:{initiator_send:'i2r',initiator_receive:'r2i',responder_send:'r2i',responder_receive:'i2r'},
    },
    actual_lengths: {client_hello:v.clientHello.length,server_hello:v.serverHello.length,initiator_finished:v.initiatorFinishedWire.length,final_transcript:v.afterInitiatorFinished.encoded.length},
    maximum_lengths: v.maxSizes,
  };
}

export function makeCorpus() {
  const base = buildVector();
  const mutate = (id, field, fn) => {
    const next = buildVector({ ...fn(base), mutation:{id,field} });
    return { id, changed_field:field, outputs: Object.fromEntries(Object.entries(next.outputs).map(([k,val])=>[k,hex(val)])), effect:mutationEffect(base,next,field) };
  };
  const mutations = [
    mutate('changed-intent','common intent principal byte',b=>{const x=Buffer.from(b.commonIntent);x[0]^=1;return {intent:x};}),
    mutate('changed-initiator-share','initiator share byte',b=>{const x=Buffer.from(b.initiatorShare);x[700]^=1;return {initiatorShare:x};}),
    mutate('changed-client-proof','client paired-envelope byte',b=>{const x=Buffer.from(b.clientProofEnvelope);x[17]^=1;return {clientProofEnvelope:x};}),
    mutate('changed-responder-share','responder share byte',b=>{const x=Buffer.from(b.responderShare);x[900]^=1;return {responderShare:x};}),
    mutate('changed-server-proof','server paired-envelope byte',b=>{const x=Buffer.from(b.serverProofEnvelope);x[31]^=1;return {serverProofEnvelope:x};}),
    mutate('changed-ml-kem-secret-input','synthetic ML-KEM shared-secret input byte',b=>{const x=Buffer.from(b.kemSecretInput);x[0]^=1;return {kemSecretInput:x};}),
    mutate('changed-x25519-secret-input','synthetic X25519 shared-secret input byte',b=>{const x=Buffer.from(b.dhSecretInput);x[0]^=1;return {dhSecretInput:x};}),
    mutate('wrong-responder-finished-role','responder Finished role label changed from r2i to i2r',()=>({responderFinishedRole:ROLE_I2R})),
  ];
  const finalBytes = base.afterInitiatorFinished.encoded.length;
  const arithmetic = {
    transcript_item_lengths: base.items.map(([label,value])=>({label:label.toString('ascii'),label_bytes:Buffer.byteLength(label),value_bytes:value.length,framed_bytes:2+Buffer.byteLength(label)+4+value.length})),
    final_transcript_bytes:finalBytes,
    transcript_limit_bytes:TRANSCRIPT_LIMIT,
    client_hello_max_bytes:base.maxSizes.clientHello,
    server_hello_max_bytes:base.maxSizes.serverHello,
    initiator_finished_max_bytes:base.maxSizes.initiatorFinished,
    mtu_negotiation:{client_advertised:base.clientOffer.readUInt16BE(2),responder_advertised:base.serverSelection.readUInt16BE(2),selected:base.serverSelection.readUInt16BE(4),expected_minimum:Math.min(base.clientOffer.readUInt16BE(2),base.serverSelection.readUInt16BE(2))},
    message_limit_bytes:MAX_MESSAGE,
    proof_cap_bytes:MAX_PROOF,
    max_fragments:MAX_FRAGMENTS,
  };
  if (finalBytes !== 3389) throw new Error(`mechanical transcript length ${finalBytes}, contract expects 3389`);
  if (base.maxSizes.clientHello !== 6408 || base.maxSizes.serverHello !== 5922 || base.maxSizes.initiatorFinished !== 182) throw new Error('contract message arithmetic mismatch');
  if (base.serverSelection.readUInt16BE(4) !== Math.min(base.clientOffer.readUInt16BE(2),base.serverSelection.readUInt16BE(2))) throw new Error('selection does not match min-MTU negotiation');
  return {
    schema:'qpr-pq-1-endpoint-v1-schedule-vectors',
    provenance:{
      contract:'evidence/qbs04-endpoint-contract.md; exact ordered transcript labels, fixed share sizes, schedule infos and Finished roles.',
      implementation:'Standalone Node crypto SHA-384/HMAC-SHA-384 and independent HKDF-SHA-384 extract/expand; no production imports.',
      secrets:'ML-KEM and X25519 shared-secret inputs are fixed synthetic byte patterns for schedule arithmetic only; they are NOT real KEM/DH outputs or cryptographic agreement vectors.',
      proofs:'Client/server proof envelopes are fixed synthetic bytes. Their SHA-384 commitments are transcript test inputs; envelopes are NOT valid COSE proofs and no signatures are verified.',
      authority:'No peer pin, authority, consent, permission, endpoint state, installed key, traffic capability, or real transport is exercised.',
      digest_rule:'Transcript digest = SHA384(ASCII QPR-TRANSCRIPT-PQ-1 || exact length-delimited ordered items); each item is u16be(label_len)||label||u32be(value_len)||value.',
      hkdf_rule:'HKDF-SHA384: Extract(salt, IKM), then RFC 5869 Expand(PRK, exact ASCII info, 32 bytes). Finished MACs are HMAC-SHA384(finished_key, transcript_digest || existing role label).',
    },
    arithmetic,
    base:publicVector(base),
    mutations,
  };
}
export function generate() { return makeCorpus(); }

async function main() {
  const here = dirname(fileURLToPath(import.meta.url));
  const output = join(here,'endpoint-v1.json');
  await writeFile(output,JSON.stringify(makeCorpus(),null,2)+'\n','utf8');
  if (typeof process !== 'undefined' && process.stdout) process.stdout.write(`wrote endpoint vectors to ${output}\n`);
}
if (typeof process !== 'undefined' && process.argv && process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) await main();
