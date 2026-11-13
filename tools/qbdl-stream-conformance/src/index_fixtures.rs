use crate::{allocation::measured,bundle_index::*};
fn hex(s:&str)->Vec<u8> { assert_eq!(s.len()%2,0); (0..s.len()/2).map(|i|u8::from_str_radix(&s[i*2..i*2+2],16).unwrap()).collect() }
#[test]
fn independent_literal_fixtures_and_zero_allocation() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../fixtures/index-v1.json")).unwrap();
    let mut accepts=0; let mut refuses=0;
    for v in fixture["vectors"].as_array().unwrap() {
        let header=hex(v["header_hex"].as_str().unwrap()); let index=hex(v["index_hex"].as_str().unwrap());
        let extent=v["expected_extent"].as_u64().unwrap();
        let limits=IndexReadLimits::default();
        let cap=ExtentLimit{max_logical_bytes:v["max_logical_bytes"].as_u64().unwrap()};
        let mut scratch=[EntryScratch::EMPTY;8];
        let expected=v["expected"]=="accept";
        let mut accepted=false;
        let allocations=measured(|| {
            let result=admit_header(&header,extent,cap,limits).and_then(|h| {
                let _ = measure_index(&h,&index,limits)?;
                admit_index(&h,&index,limits,&mut scratch)
            });
            accepted=result.is_ok();
        });
        assert_eq!(allocations,0,"{} allocated",v["id"]);
        assert_eq!(accepted,expected,"fixture {}",v["id"]);
        if accepted {
            accepts+=1;
            let h=admit_header(&header,extent,cap,limits).unwrap();
            let admitted=admit_index(&h,&index,limits,&mut scratch).unwrap();
            let owned=materialize_entries(&admitted,limits).unwrap();
            assert_eq!(owned.len(),h.entry_count() as usize);
            assert_eq!(admitted.encoded_bytes(),index);
        } else { refuses+=1; }
    }
    assert!(accepts>=6 && refuses>=15);
}
