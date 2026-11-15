use super::{write_metadata::{self as w,Fields},*};
use crate::{q42::field_envelope::{self as env,FieldValue as F},sensing::acquisition as a};
pub(super) fn encode(view:View<'_>,out:&mut[u8],cursor:&mut usize,limits:env::Limits)->Result<(),Error>{
 let region=view.region();let original=region.original();let context=view.context();let axes=view.axes();context.validate()?;axes.validate()?;
 let kind=match view{View::Tensor(_)=>1,View::Events{..}=>2,View::Correlation(_)=>3};
 let mut f=Fields::new();f.u32(1,kind)?;f.put(2,9,1,&1u32.to_le_bytes())?;f.put(3,7,32,&original.digest())?;
 f.u64(4,u64::try_from(region.offset()).map_err(|_|Error::Overflow)?)?;f.u64(5,region.bytes().len() as u64)?;
 if let Some(n)=context.quantity.kind.value(){f.u64(6,n)?;}if let Some(n)=context.quantity.unit.value(){f.u64(7,n)?;}
 f.u32(8,match context.stage{a::SampleStage::RawCodes=>0,a::SampleStage::ReportedPhysical=>1,a::SampleStage::DerivedPhysical=>2})?;
 if let Some(n)=context.sensor.value(){f.u64(9,n)?;}
 if let View::Events{view,..}=view{
  if axes.complex_convention.is_some()||axes.phase_origin.is_some(){return Err(Error::Unsupported);}
  f.u32(11,if view.byte_order()==a::ByteOrder::Little{0}else{1})?;
  f.u64(40,view.records() as u64)?;f.u32(41,view.channels())?;f.u32(42,if view.ordering()==a::EventOrdering::AcquisitionOrder{0}else{1})?;
 }else{
  let layout=match view{View::Tensor(v)=>v.layout(),View::Correlation(v)=>v.tensor().layout(),_=>unreachable!()};
  let code=match layout.encoding{a::Encoding::Shared(s)=>s as u32,a::Encoding::U16=>8,a::Encoding::U32=>9,a::Encoding::U64=>10,a::Encoding::ComplexI16=>11,a::Encoding::ComplexF64=>12,
   a::Encoding::Opaque{profile,element_bytes}=>{f.u64(18,profile.value().ok_or(Error::Metadata)?)?;f.u32(19,element_bytes)?;255}};
  f.u32(10,code)?;f.u32(11,if layout.byte_order==a::ByteOrder::Little{0}else{1})?;
  f.u32(12,match layout.components{a::ComponentOrder::Scalar=>0,a::ComponentOrder::RealImag=>1,a::ComponentOrder::ImagReal=>2})?;f.u32(13,u32::from(layout.rank))?;
  let mut extents=[0u8;16];let mut strides=[0u8;32];let mut kinds=[0u8;16];let mut ids=[0u8;32];
  for i in 0..usize::from(layout.rank){
   extents[i*4..i*4+4].copy_from_slice(&layout.extents[i].to_le_bytes());
   strides[i*8..i*8+8].copy_from_slice(&(layout.strides[i] as u64).to_le_bytes());
   let (kind,id)=match layout.axes[i]{a::Axis::Sample=>(0u32,0u64),a::Axis::Channel=>(1,0),a::Axis::Sensor=>(2,0),a::Axis::Lag=>(3,0),a::Axis::LeftChannel=>(4,0),a::Axis::RightChannel=>(5,0),
   a::Axis::Registered(id)=>(255,id.value().ok_or(Error::Metadata)?)};
   kinds[i*4..i*4+4].copy_from_slice(&kind.to_le_bytes());ids[i*8..i*8+8].copy_from_slice(&id.to_le_bytes());
  }
  let rank=u32::from(layout.rank);let n=usize::from(layout.rank);
  f.put(14,1,rank,&extents[..n*4])?;f.put(15,2,rank,&strides[..n*8])?;f.put(16,1,rank,&kinds[..n*4])?;f.put(17,2,rank,&ids[..n*8])?;
 }
 if let Some(c)=view.clock(){
  f.put(20,8,1,&4u32.to_le_bytes())?;f.u64(21,c.origin().value().ok_or(Error::Metadata)?)?;f.u64(22,c.restart())?;
  f.i64(23,c.timing().start_tick)?;f.u64(24,c.timing().samples.numerator)?;f.u64(25,c.timing().samples.denominator)?;
 }
 f.link(26,context.geometry)?;f.link(28,context.calibration)?;f.link(30,context.uncertainty.map(|u|u.artifact))?;
 match axes.carrier{a::Carrier::Unknown=>f.u32(32,0)?,a::Carrier::AperiodicOrStatic=>f.u32(32,1)?,a::Carrier::Hertz(hz)=>{f.u32(32,2)?;f.f64(33,hz)?;}}
 if let Some([lo,hi])=axes.baseband_hz{let mut b=[0;16];b[..8].copy_from_slice(&lo.to_le_bytes());b[8..].copy_from_slice(&hi.to_le_bytes());f.put(34,5,2,&b)?;}
 if let Some(rate)=axes.pulse_rate{f.u64(35,rate.numerator)?;f.u64(36,rate.denominator)?;}
 if let Some(c)=axes.complex_convention{f.u32(37,match c{a::ComplexConvention::RealPlusIImagPositiveExponent=>0,a::ComplexConvention::RealPlusIImagNegativeExponent=>1})?;}
 f.link(38,axes.phase_origin)?;
 if let View::Correlation(c)=view{
  f.i64(43,c.lag_ticks(0)?)?;f.i64(44,c.lag_step_ticks())?;
  f.u32(45,match c.product(){a::ProductConvention::LeftTimesConjugateRight=>0,a::ProductConvention::ConjugateLeftTimesRight=>1,a::ProductConvention::LeftTimesRight=>2})?;
  match c.normalization(){a::Normalization::Sum=>f.u32(46,0)?,a::Normalization::Mean{contributing_pairs:n}=>{f.u32(46,1)?;f.u64(47,n)?;},
   a::Normalization::Reported(r)=>{f.u32(46,2)?;f.link(48,Some(r))?;}}
 }
 f.encode(out,cursor,limits)?;
 if let Some(c)=view.clock(){
  let t=c.timing();let scale=[t.scale];let seconds=[t.epoch_seconds];let nanos=[t.epoch_nanoseconds];let numerator=[t.ticks.numerator];let denominator=[t.ticks.denominator];
  let fields=[w::field(1,F::U32(&scale)),w::field(2,F::I64(&seconds)),w::field(3,F::U32(&nanos)),
    w::field(4,F::U64(&numerator)),w::field(5,F::U64(&denominator))];
  w::block(out,cursor,4,1,&fields,limits)?;
 }
 Ok(())
}
