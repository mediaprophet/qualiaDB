//! Independently written Node bytes compared with the actual Rust service codec.
use crate::qbdl_service_wire::{encode,Limits,Message,ObjectSelector,ServiceRecordHeader};
use serde_json::Value;
fn corpus()->Value {serde_json::from_str(include_str!("../fixtures/service-wire-v1.json")).unwrap()}
fn hex(v:&Value)->Vec<u8>{let s=v.as_str().unwrap();assert_eq!(s.len()%2,0);(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
fn n(v:&Value)->u64{v.as_u64().unwrap_or_else(||v.as_str().unwrap().parse().unwrap())}
#[test]
fn independent_service_offsets_and_every_variant_agree(){
    let c=corpus();let mut count=0;
    for v in c["positives"].as_array().unwrap().iter().chain(c["additionalPositives"].as_array().unwrap()) {
        let b=hex(&v["wireRecordHex"]);let e=&v["expected"];
        let parsed=Message::parse(&b).unwrap_or_else(|error|panic!("{}: {error:?}",v["id"]));let h=parsed.header;
        assert_eq!(h.kind as u64,n(&e["variant"]));
        assert_eq!(h.descriptor.0.as_slice(),hex(&e["descriptorDigestHex"]));
        assert_eq!(h.context.0.as_slice(),hex(&e["contextDigestHex"]));
        assert_eq!(h.transfer_id.as_slice(),hex(&e["transferIdHex"]));
        for (actual,key) in [(h.publication_generation,"generation"),(h.policy_generation,"policyGeneration"),
            (h.cancellation,"cancellationFence"),(h.entry as u64,"entryId"),(h.leaf,"leafIndex"),(h.count as u64,"count"),
            (h.credit_serial,"creditSerial"),(h.credit_bytes,"creditCeilingBytes"),(h.deadline,"deadlineUnixSeconds")] {
            assert_eq!(actual,n(&e[key]),"{} field{key}",v["id"]);
        }
        assert_eq!(parsed.body,hex(&v["bodyHex"]));assert_eq!(parsed.body.len() as u64,n(&e["bodyLength"]));
        let mut out=vec![0;b.len()];
        assert_eq!(crate::allocation::measured(||{
            let parsed=Message::parse(&b).unwrap();
            assert_eq!(encode(&parsed.header,parsed.body,&mut out).unwrap(),b.len());
        }),0);
        assert_eq!(out,b);
        let rb=hex(&v["carrierHeaderHex"]);let re=&e["carrier"];
        let r=ServiceRecordHeader::parse(&rb).unwrap();
        assert_eq!(r.transcript.0.as_slice(),hex(&re["transcriptHex"]));assert_eq!(r.message_id.as_slice(),hex(&re["messageIdHex"]));
        assert_eq!(r.total as u64,n(&re["logicalTotal"]));assert_eq!(r.offset as u64,n(&re["offset"]));
        assert_eq!(r.chunk as u64,n(&re["chunkLength"]));assert_eq!(r.final_chunk,n(&re["final"])==1);
        let mut out=[0;88];assert_eq!(r.encode(&mut out).unwrap(),88);assert_eq!(out.as_slice(),rb);
        count+=1;
    }
    assert_eq!(count,13);
}
#[test]
fn independent_limits_and_all_nine_object_classes_agree(){
    let c=corpus();let b=hex(&c["limits"]["bytesHex"]);let e=&c["limits"]["expected"];
    let l=Limits::parse(&b).unwrap();
    for (actual,key) in [(l.record as u64,"maxRecordBytes"),(l.page as u64,"maxPagePayload"),(l.cose as u64,"maxCoseBytes"),
        (l.index as u64,"maxIndexBytes"),(l.metadata as u64,"maxMetadataBytes"),(l.closure as u64,"maxClosureBytes"),
        (l.entries as u64,"maxEntries"),(l.inflight as u64,"maxInflight"),(l.window,"windowBytes"),
        (l.cold_rows as u64,"maxColdRows"),(l.extent,"maxLogicalExtent"),(l.lifetime_ms as u64,"requestLifetimeMs")] {
        assert_eq!(actual,n(&e[key]),"{key}");
    }
    let mut out=[0;64];l.encode(&mut out).unwrap();assert_eq!(out.as_slice(),b);
    let cases=c["selectors"].as_array().unwrap();assert_eq!(cases.len(),9);
    for case in cases {let b=hex(&case["bytesHex"]);let e=&case["expected"];let s=ObjectSelector::parse(&b,false).unwrap();
        assert_eq!(s.class as u64,n(&e["objectClass"]));assert_eq!(s.number as u64,n(&e["number"]));
        assert_eq!(s.total,n(&e["total"]));assert_eq!(s.offset,n(&e["offset"]));assert_eq!(s.chunk as u64,n(&e["chunkLength"]));
        let mut out=[0;32];s.encode(false,&mut out).unwrap();assert_eq!(out.as_slice(),b);
    }
}
#[test]
fn independently_malformed_records_refuse_with_zero_raw_allocations(){
    let c=corpus();let cases=c["negatives"].as_array().unwrap();assert_eq!(cases.len(),40);
    for case in cases {let b=hex(&case["bytesHex"]);
        let calls=crate::allocation::measured(||{match case["scope"].as_str().unwrap(){
            "wire-record"=>assert!(Message::parse(&b).is_err(),"{}",case["id"]),
            "carrier-header"=>assert!(ServiceRecordHeader::parse(&b).is_err(),"{}",case["id"]),
            _=>panic!("unknown fixture scope"),
        }});assert_eq!(calls,0,"{}",case["id"]);
    }
}
