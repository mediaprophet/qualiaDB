use crate::{allocation::measured, proof::*, qdnf_types::StrongDigest};
fn digest(hex: &str) -> StrongDigest {
    assert_eq!(hex.len(),96);
    let mut b=[0;48]; for i in 0..48 { b[i]=u8::from_str_radix(&hex[i*2..i*2+2],16).unwrap(); } StrongDigest(b)
}
fn spec(length:u64) -> TreeSpec { TreeSpec::new(StrongDigest([3;48]),1,2,Representation::Plaintext,0,length).unwrap() }
#[test]
fn independent_recursive_reference_vectors_and_mutations() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../fixtures/tree-v1.json")).unwrap();
    let context=digest(fixture["context_hex"].as_str().unwrap());
    for case in fixture["cases"].as_array().unwrap() {
        let count=case["count"].as_u64().unwrap();
        let rep=if case["representation"]==0 {Representation::Plaintext}else{Representation::Ciphertext};
        let epoch=case["epoch"].as_u64().unwrap() as u32;
        let length=case["logical_length"].as_u64().unwrap();
        let s=TreeSpec::new(context,19,29,rep,epoch,length).unwrap();
        let expected=digest(case["root"].as_str().unwrap());
        let mut builder=TreeBuilder::new(s);
        let mut observed=Vec::new();
        for i in 0..count {
            let mut bytes:Vec<u8>=(0..s.encoded_leaf_length(i).unwrap()).map(|j|(i as usize*31+j*7) as u8).collect();
            let proof:Vec<_>=case["proofs"][i as usize].as_array().unwrap().iter().map(|v|digest(v.as_str().unwrap())).collect();
            assert_eq!(verify_leaf(&s,expected,i,&bytes,&proof),Ok(()));
            assert_eq!(measured(|| { assert_eq!(verify_leaf(&s,expected,i,&bytes,&proof),Ok(())); }),0);
            builder.append(&bytes,|n| { observed.push(n); Ok(()) }).unwrap();
            for sibling in 0..proof.len() {
                let mut mutated=proof.clone(); mutated[sibling].0[47]^=1;
                assert_eq!(verify_leaf(&s,expected,i,&bytes,&mutated),Err(ProofError::RootMismatch));
            }
            if !proof.is_empty() { assert_eq!(verify_leaf(&s,expected,i,&bytes,&proof[..proof.len()-1]),Err(ProofError::ProofLength)); }
            let mut extra=proof.clone(); extra.push(StrongDigest::ZERO);
            assert_eq!(verify_leaf(&s,expected,i,&bytes,&extra),Err(ProofError::ProofLength));
            bytes[0]^=1;
            assert_eq!(measured(|| { assert_eq!(verify_leaf(&s,expected,i,&bytes,&proof),Err(ProofError::RootMismatch)); }),0);
            bytes[0]^=1;
            let mutations=[
                TreeSpec::new(StrongDigest([4;48]),19,29,rep,epoch,length).unwrap(),
                TreeSpec::new(context,20,29,rep,epoch,length).unwrap(),
                TreeSpec::new(context,19,30,rep,epoch,length).unwrap(),
                TreeSpec::new(context,19,29,rep,epoch,length+1).unwrap(),
            ];
            for changed in mutations { assert!(verify_leaf(&changed,expected,i,&bytes,&proof).is_err()); }
            if rep==Representation::Ciphertext {
                let changed=TreeSpec::new(context,19,29,rep,epoch+1,length).unwrap();
                assert!(verify_leaf(&changed,expected,i,&bytes,&proof).is_err());
                let changed=TreeSpec::new(context,19,29,Representation::Plaintext,0,length).unwrap();
                assert!(verify_leaf(&changed,expected,i,&bytes,&proof).is_err());
            }
            assert!(verify_leaf(&s,expected,count,&bytes,&proof).is_err());
            assert!(verify_leaf(&s,expected,i,&bytes[..bytes.len()-1],&proof).is_err());
        }
        assert_eq!(builder.finish(|n| { observed.push(n); Ok(()) }),Ok(expected));
        assert_eq!(builder.append(&[], |_|Ok(())),Err(ProofError::Complete));
        for n in observed { assert!(n.leaf_count>0 && n.start_leaf+n.leaf_count<=count); }
        if count==0 { assert_eq!(empty_root(&s),Ok(expected)); assert_eq!(leaf_hash(&s,0,&[]),Err(ProofError::Index)); }
    }
}
#[test]
fn builder_state_and_limits_do_not_allocate() {
    let s=spec(16385); let mut b=TreeBuilder::new(s); let full=[1u8;16384];
    assert_eq!(measured(|| {
        assert_eq!(b.finish(|_|Ok(())),Err(ProofError::Incomplete));
        assert_eq!(b.append(&full[..2],|_|Ok(())),Err(ProofError::Length));
        assert_eq!(b.appended(),0);
        b.append(&full,|_|Ok(())).unwrap(); b.append(&[9],|_|Ok(())).unwrap();
        b.finish(|_|Ok(())).unwrap();
        assert_eq!(b.append(&[9],|_|Ok(())),Err(ProofError::Complete));
    }),0);
    assert_eq!(TreeSpec::new(StrongDigest::ZERO,0,0,Representation::Plaintext,0,MAX_LOGICAL_BYTES+1),Err(ProofError::Limit));
    assert_eq!(TreeSpec::new(StrongDigest::ZERO,0,0,Representation::Plaintext,1,0),Err(ProofError::InvalidSpec));
    assert_eq!(spec(MAX_LOGICAL_BYTES).leaf_count(),1<<26);
    assert_eq!(spec(MAX_LOGICAL_BYTES).logical_leaf_length((1<<26)-1),Ok(16384));
}
#[test]
fn proof_codec_exact_framing_and_capacity() {
    let siblings=[StrongDigest([8;48]);32]; let mut bytes=[0u8;1538]; let mut decoded=[StrongDigest::ZERO;32];
    assert_eq!(measured(|| {
        let n=encode_proof(&siblings,&mut bytes).unwrap();
        assert_eq!(decode_proof(&bytes[..n],&mut decoded).unwrap(),siblings);
        assert_eq!(decode_proof(&bytes[..n-1],&mut decoded),Err(ProofError::ProofLength));
        assert_eq!(decode_proof(&bytes[..n+1],&mut decoded),Err(ProofError::ProofLength));
        assert_eq!(decode_proof(&bytes[..n],&mut decoded[..31]),Err(ProofError::Capacity));
        assert_eq!(encode_proof(&siblings,&mut bytes[..1]),Err(ProofError::Capacity));
        assert_eq!(encode_proof(&[StrongDigest::ZERO;33],&mut bytes),Err(ProofError::ProofLength));
        assert_eq!(decode_proof(&[],&mut decoded),Err(ProofError::ProofLength));
        assert_eq!(decode_proof(&[0],&mut decoded).unwrap(),&[]);
    }),0);
}

#[test]
fn outboard_failure_poisoning_and_repeat_finish() {
    let full=[1u8;16384];
    let mut b=TreeBuilder::new(spec(32769));
    b.append(&full,|_|Ok(())).unwrap();
    let mut calls=0;
    assert_eq!(b.append(&full,|_|{ calls+=1; if calls==2 {Err(ProofError::Sink)}else{Ok(())} }),Err(ProofError::Sink));
    assert!(b.is_poisoned());
    assert_eq!(b.append(&full,|_|Ok(())),Err(ProofError::Poisoned));
    assert_eq!(b.finish(|_|Ok(())),Err(ProofError::Poisoned));
    let mut b=TreeBuilder::new(spec(32769));
    b.append(&full,|_|Ok(())).unwrap(); b.append(&full,|_|Ok(())).unwrap(); b.append(&[1],|_|Ok(())).unwrap();
    assert_eq!(b.finish(|_|Err(ProofError::Sink)),Err(ProofError::Sink));
    assert_eq!(b.finish(|_|Ok(())),Err(ProofError::Poisoned));
    let mut b=TreeBuilder::new(spec(1)); b.append(&[1],|_|Ok(())).unwrap();
    let root=b.finish(|_|Ok(())).unwrap();
    assert_eq!(b.finish(|_|panic!("repeat finish must not write")),Ok(root));
    let mut b=TreeBuilder::new(spec(1));
    let panic=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||{let _=b.append(&[1],|_|panic!("sink"));}));
    assert!(panic.is_err()); assert!(b.is_poisoned()); assert_eq!(b.finish(|_|Ok(())),Err(ProofError::Poisoned));
}
