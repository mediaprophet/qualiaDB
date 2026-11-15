use super::*;
use crate::{q42::{field_envelope as env,field_semantics as sem},sensing::{SemanticId,acquisition as a}};
pub(super) fn id(n:u64)->SemanticId{SemanticId::new(n).unwrap()}
pub(super) fn context()->a::Context{a::Context{quantity:a::Quantity{kind:id(11),unit:id(12)},stage:a::SampleStage::RawCodes,sensor:id(13),geometry:None,calibration:None,uncertainty:None}}
pub(super) fn axes()->a::SignalAxes{a::SignalAxes{carrier:a::Carrier::Unknown,baseband_hz:None,pulse_rate:None,complex_convention:None,phase_origin:None}}
pub(super) fn layout(enc:a::Encoding,n:u32)->a::Layout{a::Layout{rank:1,extents:[n,0,0,0],strides:[enc.bytes(),0,0,0],axes:[a::Axis::Sample;4],encoding:enc,byte_order:a::ByteOrder::Little,components:if enc.complex(){a::ComponentOrder::RealImag}else{a::ComponentOrder::Scalar}}}
pub(super) fn encode_test(input:Input<'_>)->Vec<u8>{
 let mut meta=[0u8;16_384];let mut staging=vec![0u8;65_536];let mut out=vec![0xcc;65_536];
 let mut m=[sem::NodeState::default();256];let mut e=[sem::NodeState::default();128];
 let n=encode(input,&mut meta,&mut staging,&mut sem::Scratch{metadata:&mut m,entries:&mut e},&mut out,Default::default(),Default::default(),Default::default()).unwrap();
 out.truncate(n);out
}
pub(super) fn inspect(bytes:&[u8],family:Family,check:impl FnOnce(&Document<'_,'_>)){
 let structural=env::StructuralView::open(bytes,family,Default::default()).unwrap();
 let mut originals=[None;65];let mut m=[sem::NodeState::default();256];let mut e=[sem::NodeState::default();128];
 let result=admit(&structural,1,&mut originals,&mut sem::Scratch{metadata:&mut m,entries:&mut e},Default::default(),Default::default()).unwrap();
 check(&result);
}
#[test]
fn both_current_families_preserve_f64_original_and_subrange_bits(){
 // Independent literal source: metadata prefix, -0f64, signed quiet NaN payload, suffix.
 let bytes=[0x51,0x44,0x01,0,0,0,0,0,0,0,0x80,0x34,0x12,0,0,0,0,0xf8,0xff,0xee,0xaa];
 let original=a::Original::admit(&bytes,bytes.len()).unwrap();
 let v=a::TensorView::admit(original.region(3,16).unwrap(),layout(a::Encoding::Shared(a::Scalar::F64),2),context(),None,axes()).unwrap();
 for family in [Family::QacV1Candidate,Family::EmfV4Candidate]{
  let encoded=encode_test(Input{family,view:View::Tensor(v),derivation:None});
  inspect(&encoded,family,|d|{
   assert_eq!(d.view.region().original().bytes(),bytes);
   assert_eq!(d.view.region().original().digest(),original.digest());
   let View::Tensor(t)=d.view else{panic!("tensor expected")};
   assert_eq!(t.element([0;4]).unwrap(),&bytes[3..11]);
   assert_eq!(t.element([1,0,0,0]).unwrap(),&bytes[11..19]);
   assert_eq!(t.numeric([1,0,0,0]),Err(a::Error::Nonfinite));
  });
 }
}
