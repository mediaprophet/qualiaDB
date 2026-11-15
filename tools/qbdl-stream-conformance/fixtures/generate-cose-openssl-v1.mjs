// Independent OpenSSL signatures + standalone reference encoder; no Rust imported.
import { generateKeyPairSync, sign, verify } from 'node:crypto';
import { buildFixture, makeCorpus, encodePair } from './generate-cose-encoding-v1.mjs';
import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
export function generate() {
  const ml=generateKeyPairSync('ml-dsa-65');
  const ed=generateKeyPairSync('ed25519');
  const mlSpki=ml.publicKey.export({type:'spki',format:'der'});
  const edSpki=ed.publicKey.export({type:'spki',format:'der'});
  const mlPublic=mlSpki.subarray(mlSpki.length-1952);
  const edPublic=edSpki.subarray(edSpki.length-32);
  const f=buildFixture({mlPublic,edPublic});
  const mlSignature=sign(null,f.mlSigStructure,{key:ml.privateKey,context:Buffer.alloc(0)});
  const edSignature=sign(null,f.edSigStructure,ed.privateKey);
  if(!verify(null,f.mlSigStructure,{key:ml.publicKey,context:Buffer.alloc(0)},mlSignature) || !verify(null,f.edSigStructure,ed.publicKey,edSignature)) throw Error('provider self-check failed');
  const out=makeCorpus({mlPublic,edPublic,mlSignature,edSignature});
  out.schema='qbdl-cose-openssl-v1';
  out.provenance={encoder:'generate-cose-encoding-v1.mjs (independent definite CBOR)',generator:'generate-cose-openssl-v1.mjs',provider:(typeof process === 'undefined' ? 'Node crypto / OpenSSL (restricted REPL; versions unreported)' : 'Node '+process.versions.node+' / OpenSSL '+process.versions.openssl),public_keys:'Fresh test-only provider-generated public keys. Private keys never serialized or retained.',signatures:'Pure ML-DSA-65 with explicit empty context and pure Ed25519 over exact independent COSE Sig_structure bytes.',provider_self_check:true,documentation:'https://nodejs.org/api/crypto.html#cryptosignalgorithm-data-key-callback',scope:'Interoperability fixture; not publisher authority or external composition certification.'};
  out.valid_encoding_fixture.signature_bytes.cryptographically_valid=true;
  out.valid_encoding_fixture.expected='verify-accept under exactly supplied test public pair and binding; no authority';
  const wrongContext=sign(null,f.mlSigStructure,{key:ml.privateKey,context:Buffer.from('wrong')});
  if(verify(null,f.mlSigStructure,{key:ml.publicKey,context:Buffer.alloc(0)},wrongContext))throw Error('context negative self-check failed');
  const rawMl=sign(null,f.bindingBytes,{key:ml.privateKey,context:Buffer.alloc(0)});
  const rawEd=sign(null,f.bindingBytes,ed.privateKey);
  const headers={mlProtected:f.mlProtected,edProtected:f.edProtected};
  out.crypto_negative_vectors=[
    {id:'nonempty-ml-context',envelope_hex:encodePair(f.bindingBytes,wrongContext,edSignature,headers).toString('hex')},
    {id:'ml-over-binding-not-sig-structure',envelope_hex:encodePair(f.bindingBytes,rawMl,edSignature,headers).toString('hex')},
    {id:'ed-over-binding-not-sig-structure',envelope_hex:encodePair(f.bindingBytes,mlSignature,rawEd,headers).toString('hex')},
    {id:'stripped-ml-signature',envelope_hex:encodePair(f.bindingBytes,Buffer.alloc(3309),edSignature,headers).toString('hex')},
    {id:'stripped-ed-signature',envelope_hex:encodePair(f.bindingBytes,mlSignature,Buffer.alloc(64),headers).toString('hex')}
  ];
  return out;
}
if(typeof process !== 'undefined' && process.argv[1] && fileURLToPath(import.meta.url)===process.argv[1])await writeFile(new URL('./cose-openssl-v1.json',import.meta.url),JSON.stringify(generate(),null,2)+'\n');
