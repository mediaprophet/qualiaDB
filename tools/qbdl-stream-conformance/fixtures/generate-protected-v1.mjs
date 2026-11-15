import crypto from 'node:crypto';
import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const B = (n, v=0x01) => Buffer.alloc(n, v);
const U16 = n => { const b=Buffer.alloc(2); b.writeUInt16BE(n); return b; };
const U32 = n => { const b=Buffer.alloc(4); b.writeUInt32BE(n); return b; };
const U64 = n => { const b=Buffer.alloc(8); b.writeBigUInt64BE(BigInt(n)); return b; };
const cat = (...xs) => Buffer.concat(xs);
const sha = b => crypto.createHash('sha384').update(b).digest();
const hx = b => b.toString('hex');
const magic = s => Buffer.from(s + '\0', 'ascii');
const profile = 'qbdl-stream-protected-v1';
const suite = 1;
const leafBytes = 16384, segmentLeaves = 1048576, maxLength = 2 ** 40;
function emptyRoot(ctx, id, generation, length=0, role=1) {
  const common=cat(ctx,U32(id),U64(generation),Buffer.from([1]),U32(1),U64(length));
  return sha(cat(Buffer.from([2]),Buffer.from('qbdl-stream-pq-1/empty/v1\0','ascii'),common));
}
function geometryRows(count) { return Array.from({length:count},(_,i)=>({id:i===0?17:17+i*3,length:count===0?0:1})); }
function makePages(rows, contextId) {
  const G=[], P=[];
  for(let p=0;p<Math.ceil(rows.length/128);p++) {
    const subset=rows.slice(p*128,(p+1)*128);
    const g=cat(magic('QBDL-PGE-V1'),U16(suite),U16(p),U32(p*128),U16(subset.length),U16(0),...subset.map(r=>cat(U32(r.id),U64(r.length))));
    const roots=subset.map(r=>r.length===0?emptyRoot(contextId,r.id,1,0):sha(cat(Buffer.from('synthetic-ciphertext-root\0'),contextId,U32(r.id),U64(r.length))));
    const page=cat(magic('QBDL-PRT-V1'),U16(suite),contextId,U16(p),U32(p*128),U16(subset.length),U16(0),...subset.map((r,i)=>cat(U32(r.id),roots[i])));
    G.push({page:p,rows:subset,bytes:g,hash:sha(g)});
    P.push({page:p,rows:subset,bytes:page,hash:sha(page),roots});
  }
  return {G,P};
}
function makePublic(count) {
  const rows=geometryRows(count), nonce=B(32,0x31), scope=B(32,0x42), generation=1, epoch=7, predecessor=B(48,0x53);
  // Metadata length is for the fixed empty-index structural vector; rows are independent codec rows.
  const metadataLength=352+108*count;
  const pageCount=Math.ceil(rows.length/128);
  const geometryPagesPlaceholder=Array.from({length:pageCount},(_,i)=>({page:i,row_count:Math.min(128,rows.length-i*128)}));
  // Context includes geometry page hashes, so encode G once with final context-independent bytes.
  const roughPages=Array.from({length:pageCount},(_,i)=>rows.slice(i*128,(i+1)*128));
  const geometryBytes=roughPages.map((subset0,i)=>cat(magic('QBDL-PGE-V1'),U16(suite),U16(i),U32(i*128),U16(subset0.length),U16(0),...subset0.map(r=>cat(U32(r.id),U64(r.length)))));
  const C=cat(magic('QBDL-PCX-V1'),U16(suite),U16(0),nonce,scope,U64(generation),U32(epoch),predecessor,U32(rows.length),U64(metadataLength),U32(leafBytes),U32(segmentLeaves),U16(pageCount),U16(0),...geometryBytes.map((g,i)=>cat(U16(i),U16(Math.min(128,rows.length-i*128)),U32(g.length),sha(g))));
  const contextId=sha(C);
  const {G,P}=makePages(rows,contextId);
  if (G.some((p,i)=>!p.bytes.equals(geometryBytes[i]))) throw new Error("geometry commitment differs from emitted page");
  const D=cat(magic('QBDL-PCD-V1'),U16(suite),U16(0),U16(C.length),C,sha(Buffer.from('synthetic-metadata-ciphertext-root')),U16(P.length),...P.map(p=>cat(U16(p.page),U16(p.rows.length),U32(p.bytes.length),p.hash)));
  const descriptorId=sha(D);
  return {rows,C,contextId,G,P,D,descriptorId,metadataLength,nonce,scope,generation,epoch,predecessor,geometryPagesPlaceholder};
}
function makePrivateMetadata(pub) {
  const sourceHash=B(48,0x61), closure=B(48,0x62), policy=B(48,0x63), consumer=B(48,0x64);
  const originalHeader=B(64,0x65), index=Buffer.alloc(0), originalLength=64+pub.rows.length;
  const mappings=pub.rows.map((r,i)=>cat(U32(r.id),U64(64+i),U64(r.length),Buffer.from([1]),U32(i),Buffer.from([r.length>0?1:0]),U16(0),r.length>0?sha(Buffer.from([i&255])):sha(Buffer.alloc(0)),r.length>0?B(32,(i%251)+1):Buffer.alloc(32)));
  const M=cat(magic('QBDL-PMD-V1'),U16(suite),U16(0),pub.contextId,U16(1),U16(0),U64(originalLength),sourceHash,U64(originalLength),U32(64),U32(index.length),closure,policy,consumer,U32(pub.rows.length),...mappings,originalHeader,index);
  return {M,sourceHash,closure,policy,consumer,originalHeader,index};
}
function recipientAndWrap(pub, metadata) {
  const z48=B(48), z32=B(32), ml=B(1184,0x71), dh=B(32,0x72);
  const principal=B(48,0x73), publisher=B(48,0x74), service=B(48,0x75), purpose=B(48,0x76), instrument=B(48,0x77), scope=B(48,0x78), operation=B(16,0x79);
  const mlId=sha(cat(Buffer.from(profile+'/recipient/ml-kem-768\0','ascii'),U16(1184),ml));
  const dhId=sha(cat(Buffer.from(profile+'/recipient/x25519\0','ascii'),U16(32),dh));
  const A=cat(magic('QBDL-PRA-V1'),U16(suite),U16(0),pub.descriptorId,pub.contextId,principal,publisher,service,purpose,instrument,scope,operation,metadata.closure,metadata.policy,metadata.consumer,metadata.sourceHash,U64(64),U64(1),U64(1),U64(4102444800),ml,dh);
  const authId=sha(A), eph=B(32,0x81), kemCt=B(1088,0x82);
  const Wc=cat(magic('QBDL-PWC-V1'),U16(suite),authId,pub.contextId,pub.descriptorId,principal,U64(1),U64(pub.metadataLength),mlId,dhId,eph,kemCt);
  const kemSs=B(32,0x91), dhSs=B(32,0x92), master=B(32,0xa1);
  const wrapInfo=Buffer.from(profile+'/metadata-wrap-key\0','ascii');
  const kek=Buffer.from(crypto.hkdfSync('sha384',cat(kemSs,dhSs),sha(Wc),wrapInfo,32));
  const cipher=crypto.createCipheriv('chacha20-poly1305',kek,Buffer.alloc(12),{authTagLength:16});
  cipher.setAAD(Wc,{plaintextLength:32});
  const encrypted=cat(cipher.update(master),cipher.final()), tag=cipher.getAuthTag();
  const W=cat(magic('QBDL-PWR-V1'),U16(suite),U16(0),authId,mlId,dhId,eph,kemCt,encrypted,tag);
  const fakeProof=B(160,0xc1), req=B(16,0xc2);
  const Q=cat(magic('QBDL-PDL-V1'),pub.descriptorId,req,U16(A.length),A,U16(fakeProof.length),fakeProof,U16(W.length),W);
  return {A,authId,mlId,dhId,Wc,W,Q,kek,master,encrypted,tag,kemSs,dhSs,fakeProof};
}
function segmentInfo(ctx,role,id,generation,epoch,segment,length) {
  return cat(Buffer.from(profile+'/segment-key\0','ascii'),U16(suite),ctx,Buffer.from([role]),U32(id),U64(generation),U32(epoch),U32(segment),U64(length));
}
function leafAAD(ctx,role,id,generation,epoch,index,length,plainLength,final) {
  const segment=Math.floor(index/segmentLeaves);
  return cat(Buffer.from(profile+'/leaf-aad\0','ascii'),U16(suite),ctx,Buffer.from([role]),U32(id),U64(generation),U32(epoch),U32(segment),U64(index),U64(length),U32(plainLength),Buffer.from([final?1:0]));
}
function sealLeaf(ctx,master,role,id,generation,epoch,index,length,plaintext) {
  const segment=Math.floor(index/segmentLeaves), info=segmentInfo(ctx,role,id,generation,epoch,segment,length);
  const key=Buffer.from(crypto.hkdfSync('sha384',master,ctx,info,32));
  const nonce=cat(Buffer.alloc(4),U64(index)), aad=leafAAD(ctx,role,id,generation,epoch,index,length,plaintext.length,index+1===Math.ceil(length/leafBytes));
  const cipher=crypto.createCipheriv('chacha20-poly1305',key,nonce,{authTagLength:16});
  cipher.setAAD(aad,{plaintextLength:plaintext.length});
  const ciphertext=cat(cipher.update(plaintext),cipher.final()), tag=cipher.getAuthTag();
  return {info,key,nonce,aad,ciphertext,tag,wire:cat(ciphertext,tag)};
}
function attemptDecrypt(key,nonce,aad,ciphertext,tag) {
  try { const d=crypto.createDecipheriv('chacha20-poly1305',key,nonce,{authTagLength:16}); d.setAAD(aad,{plaintextLength:ciphertext.length});d.setAuthTag(tag);return {accepted:true,plaintext:hx(cat(d.update(ciphertext),d.final()))}; }
  catch { return {accepted:false}; }
}
function makeCryptoVectors(ctx) {
  const master=B(32,0x33), id=0x10203040, gen=9, epoch=7, index=0, length=16384, plaintext=B(16384,0x5a);
  const base=sealLeaf(ctx,master,1,id,gen,epoch,index,length,plaintext);
  const changed={
    role:sealLeaf(ctx,master,0,id,gen,epoch,index,length,plaintext),
    context:sealLeaf(B(48,0x44),master,1,id,gen,epoch,index,length,plaintext),
    epoch:sealLeaf(ctx,master,1,id,gen,epoch+1,index,length,plaintext),
    index:sealLeaf(ctx,master,1,id,gen,epoch,index+1,length+leafBytes,plaintext),
    length:sealLeaf(ctx,master,1,id,gen,epoch,index,length+1,plaintext),
  };
  const boundaries=[];
  for(const i of [1048575,1048576,67108863]) {
    const L=i===67108863?maxLength:(i+2)*leafBytes;
    const plain=B(leafBytes,i&255), sealed=sealLeaf(ctx,master,1,id,gen,epoch,i,L,plain);
    boundaries.push({index:i,segment:Math.floor(i/segmentLeaves),logical_length:L,segment_info_hex:hx(sealed.info),segment_info_length:sealed.info.length,key_hex:hx(sealed.key),nonce_hex:hx(sealed.nonce),aad_hex:hx(sealed.aad),aad_length:sealed.aad.length,ciphertext_hex:hx(sealed.ciphertext),tag_hex:hx(sealed.tag)});
  }
  const changedTests={};
  for(const [name,v] of Object.entries(changed)) {
    const nonce=name==='index'?v.nonce:base.nonce;
    changedTests[name]={derived_key_differs:!v.key.equals(base.key),nonce_differs:!v.nonce.equals(base.nonce),aad_differs:!v.aad.equals(base.aad),authentication:attemptDecrypt(base.key,nonce,v.aad,base.ciphertext,base.tag)};
  }
  const badTag=Buffer.from(base.tag);badTag[15]^=1;
  changedTests.tag={authentication:attemptDecrypt(base.key,base.nonce,base.aad,base.ciphertext,badTag)};
  return {base:{master_hex:hx(master),role:1,id,generation:gen,epoch,index,logical_length:length,plaintext_hex:hx(plaintext),segment_info_hex:hx(base.info),segment_info_length:base.info.length,key_hex:hx(base.key),nonce_hex:hx(base.nonce),aad_hex:hx(base.aad),aad_length:base.aad.length,ciphertext_hex:hx(base.ciphertext),tag_hex:hx(base.tag),ciphertext_and_tag_hex:hx(base.wire)},boundaries,changedTests,empty:{logical_length:0,leaf_count:0,master_present:0,master_hex:'00'.repeat(32),nonce:null,tag:null,aead_invocations:0,root_hex:hx(emptyRoot(ctx,id,gen,0,1))}};
}
function asCorpusCase(count) {
  const pub=makePublic(count), metadata=makePrivateMetadata(pub), recipient=recipientAndWrap(pub,metadata);
  return {payload_object_count:count,context:{length:pub.C.length,hex:hx(pub.C),id_hex:hx(pub.contextId),geometry_pages:pub.G.map(p=>({page:p.page,length:p.bytes.length,hex:hx(p.bytes),sha384_hex:hx(p.hash)}))},protected_descriptor:{length:pub.D.length,hex:hx(pub.D),descriptor_id_hex:hx(pub.descriptorId),root_pages:pub.P.map(p=>({page:p.page,length:p.bytes.length,hex:hx(p.bytes),sha384_hex:hx(p.hash),roots_are_synthetic_for_nonempty_rows:true,empty_roots:p.rows.map((r,i)=>r.length===0?hx(p.roots[i]):null)}))},private_metadata:{length:metadata.M.length,expected_length:288+108*count+64,hex:hx(metadata.M),mapping_count:count,synthetic_source_and_mapping_content:true},recipient_authorization:{length:recipient.A.length,auth_id_hex:hx(recipient.authId),hex:hx(recipient.A),ml_id_hex:hx(recipient.mlId),dh_id_hex:hx(recipient.dhId),synthetic_public_keys:true},wrap:{context_length:recipient.Wc.length,context_hex:hx(recipient.Wc),length:recipient.W.length,hex:hx(recipient.W),metadata_master_hex:hx(recipient.master),synthetic_kem_dh_shared_secrets:true,synthetic_kem_ciphertext_and_ephemeral_public:true},private_delivery:{length:recipient.Q.length,hex:hx(recipient.Q),proof_bytes_synthetic:true}};
}
export function generate() {
  const cases=[asCorpusCase(0),asCorpusCase(1),asCorpusCase(129)];
  const maxObjects=8193,maxPages=Math.ceil(maxObjects/128),maxContext=164+maxPages*56,maxDescriptor=18+maxContext+48+2+maxPages*56;
  for (const c of cases) {
    if (c.context.length !== 164 + c.context.geometry_pages.length*56 || c.protected_descriptor.length !== 18+c.context.length+48+2+c.protected_descriptor.root_pages.length*56) throw new Error('protected context/descriptor length arithmetic mismatch');
    if (c.private_metadata.length !== 288+108*c.payload_object_count+64 || c.recipient_authorization.length !== 1856 || c.wrap.context_length !== 1438 || c.wrap.length !== 1328 || c.private_delivery.length > 7874) throw new Error('protected private codec length arithmetic mismatch');
  }
  if (Buffer.byteLength(profile+'/segment-key\0','ascii')+79 !== 116 || Buffer.byteLength(profile+'/leaf-aad\0','ascii')+92 !== 126 || Buffer.byteLength(profile+'/metadata-wrap-key\0','ascii') !== 43) throw new Error('protected crypto domain length mismatch');
  const cryptoVectors=makeCryptoVectors(Buffer.from(cases[1].context.id_hex,'hex'));
  if (cryptoVectors.boundaries.map(x=>x.segment).join(',') !== '0,1,63' || Object.values(cryptoVectors.changedTests).some(x=>x.authentication.accepted)) throw new Error('protected crypto boundary/mutation vector mismatch');
  const lengths={geometry_header:24,geometry_row:12,context_fixed:164,context_ref:56,root_header:72,root_row:52,descriptor_fixed:18,descriptor_page_ref:56,metadata_fixed:288,metadata_mapping:108,authorization:1856,wrap_context:1438,wrap:1328,private_delivery_max:7874,segment_info:116,leaf_aad:126,metadata_wrap_info:43};
  return {format:'qbdl-stream-protected-v1',fixture_class:'independent canonical byte/primitive vectors; NOT production approval or crypto qualification',provenance:{source_contract:'evidence/qbs10-protection-contract.md (2026-11-12 frozen candidate)',tree_contract:'evidence/qbs05-proof-contract.md',crypto:'Node built-in SHA-384/HKDF-SHA384/ChaCha20-Poly1305; no production imports',synthetic_values:'All fixture IDs/keys/masters/nonces/source hashes/proofs/KEM ciphertext/X25519 ephemeral and shared secrets are deterministic placeholders. Wrap output is cryptographically valid only for those synthetic shared-secret inputs; it is not a real KEM/X25519 interoperability vector. No ML-KEM ciphertext or COSE proof is claimed valid.',limits:'0,1,129 corpus objects; 8193/65-page maximum is arithmetic only; no 8193-row expansion.'},cases,crypto_vectors:cryptoVectors,limits:{max_payload_objects:maxObjects,max_pages:maxPages,last_max_page_rows:1,max_context_bytes:maxContext,max_descriptor_bytes:maxDescriptor,computed_context_bytes:maxContext,computed_descriptor_bytes:maxDescriptor,checked_arithmetic:true},lengths,codec_notes:{public_caches_exclude_A_W_Q:true,empty_entry_has_no_master_nonce_tag_or_aead:true,all_multibyte_integers:'big-endian',all_magnitudes_and_hashes_full_width:true}};
}
export function makeCorpus(){return generate();}
if (typeof process !== 'undefined' && process.argv?.[1] && fileURLToPath(import.meta.url)===process.argv[1]) {
  const result=generate(); await writeFile(new URL('./protected-v1.json',import.meta.url),JSON.stringify(result,null,2)+'\n');
}
