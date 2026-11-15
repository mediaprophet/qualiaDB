use crate::{allocation::measured,crypto_cose::*};
fn hex(s:&str)->Vec<u8> {(0..s.len()/2).map(|i|u8::from_str_radix(&s[2*i..2*i+2],16).unwrap()).collect()}
fn bytes(v:&serde_json::Value)->Vec<u8>{hex(v.as_str().unwrap())}
fn check_fixture(raw:&str,crypto:bool){
    let corpus:serde_json::Value=serde_json::from_str(raw).unwrap();
    let f=&corpus["valid_encoding_fixture"];
    let ml:[u8;1952]=bytes(&f["key_public_bytes"]["ml_dsa_65_hex"]).try_into().unwrap();
    let ed:[u8;32]=bytes(&f["key_public_bytes"]["ed25519_hex"]).try_into().unwrap();
    let public=PairedPublicKeys{ml_dsa_65:&ml,ed25519:&ed};
    let ids=public.key_ids();
    assert_eq!(ids.ml_dsa_65.as_slice(),bytes(&f["key_ids_sha384_hex"]["ml_dsa_65"]));
    assert_eq!(ids.ed25519.as_slice(),bytes(&f["key_ids_sha384_hex"]["ed25519"]));
    let descriptor:[u8;48]=bytes(&f["binding_fields"]["descriptor_digest_hex"]).try_into().unwrap();
    let purpose=bytes(&f["binding_fields"]["purpose_hex"]);let audience=bytes(&f["binding_fields"]["audience_hex"]);
    let binding=Binding{descriptor_digest:&descriptor,key_ids:ids,controller_epoch:f["binding_fields"]["controller_epoch"].as_str().unwrap().parse().unwrap(),purpose:&purpose,audience:&audience};
    let payload=bytes(&f["binding_hex"]);let envelope=bytes(&f["envelope_hex"]);
    let mut binding_out=[0u8;MAX_BINDING_BYTES];let n=encode_binding(&binding,&mut binding_out).unwrap();assert_eq!(&binding_out[..n],payload);
    let mut structure=[0u8;MAX_SIG_STRUCTURE_BYTES];
    for (alg,kid,name) in [(ML_DSA_65,ids.ml_dsa_65,"ml_dsa_65"),(ED25519,ids.ed25519,"ed25519")] {
        let n=sig_structure_into(alg,&kid,&payload,&mut structure).unwrap();assert_eq!(&structure[..n],bytes(&f["sig_structure_hex"][name]));
    }
    let signatures=PairSignatures{ml_dsa_65:bytes(&f["signature_bytes"]["ml_dsa_65_hex"]).try_into().unwrap(),ed25519:bytes(&f["signature_bytes"]["ed25519_hex"]).try_into().unwrap()};
    let mut out=[0u8;MAX_ENVELOPE_BYTES];let n=encode_pair(&payload,&signatures,&mut out).unwrap();assert_eq!(&out[..n],envelope);
    assert_eq!(decode_pair(&envelope).unwrap().binding(),&binding);
    for v in corpus["malformed_vectors"].as_array().unwrap(){let input=bytes(&v["envelope_hex"]);assert!(decode_pair(&input).is_err(),"{}",v["id"]);}
    let mut workspace=Workspace::new();
    if crypto {
        assert!(verify_pair(&envelope,&binding,&public,&mut workspace).is_ok());
        assert_eq!(measured(||{assert!(verify_pair(&envelope,&binding,&public,&mut workspace).is_ok());}),0);
        for v in corpus["crypto_negative_vectors"].as_array().unwrap(){
            let input=bytes(&v["envelope_hex"]); assert!(decode_pair(&input).is_ok());
            assert_eq!(measured(||{assert!(verify_pair(&input,&binding,&public,&mut workspace).is_err());}),0,"{}",v["id"]);
        }
    }else{assert!(verify_pair(&envelope,&binding,&public,&mut workspace).is_err());}
}
#[test]
fn independent_cbor_encoding_bytes_match(){check_fixture(include_str!("../fixtures/cose-encoding-v1.json"),false);}
#[test]
fn independent_openssl_signatures_verify_and_wrong_contexts_fail(){check_fixture(include_str!("../fixtures/cose-openssl-v1.json"),true);}
