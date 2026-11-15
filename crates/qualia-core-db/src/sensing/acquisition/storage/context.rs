use super::{values::*,Error};
use crate::{q42::{field_envelope::Block,field_semantics::Context as SemContext},sensing::{SemanticId,acquisition as a}};
fn id(v:u64)->Result<SemanticId,Error>{SemanticId::new(v).map_err(|_|Error::Metadata)}
fn optional_id(b:Block<'_>,k:u16)->Result<SemanticId,Error>{if field(b,k).is_some(){id(u64v(b,k)?)}else{Ok(SemanticId::Unknown)}}
pub(super) fn link(b:Block<'_>,k:u16)->Result<Option<a::ArtifactLink>,Error>{
 if !present(b,&[k,k+1])? {return Ok(None);}
 Ok(Some(a::ArtifactLink{digest:digest(b,k)?,identifier:id(u64v(b,k+1)?)?}))
}
pub(super) fn context(b:Block<'_>)->Result<a::Context,Error>{
 let quantity=a::Quantity{kind:optional_id(b,6)?,unit:optional_id(b,7)?};
 let result=a::Context{quantity,stage:match u32v(b,8)?{0=>a::SampleStage::RawCodes,1=>a::SampleStage::ReportedPhysical,2=>a::SampleStage::DerivedPhysical,_=>return Err(Error::Metadata)},
 sensor:optional_id(b,9)?,geometry:link(b,26)?,calibration:link(b,28)?,
 uncertainty:link(b,30)?.map(|artifact|a::UncertaintyLink{artifact,target:quantity})};
 result.validate()?;Ok(result)
}
pub(super) fn axes(b:Block<'_>)->Result<a::SignalAxes,Error>{
 let carrier=match u32v(b,32)?{
  0=>{absent(b,&[33])?;a::Carrier::Unknown},1=>{absent(b,&[33])?;a::Carrier::AperiodicOrStatic},
  2=>a::Carrier::Hertz(f64v(b,33)?),_=>return Err(Error::Metadata)};
 let bounds=if let Some(f)=field(b,34){Some([f64::from_bits(u64_at(f.value(),0)),f64::from_bits(u64_at(f.value(),1))])}else{None};
 let pulse_rate=if present(b,&[35,36])?{Some(a::Rate::new(u64v(b,35)?,u64v(b,36)?).map_err(|_|Error::Metadata)?)}else{None};
 let complex_convention=if field(b,37).is_some(){Some(match u32v(b,37)?{0=>a::ComplexConvention::RealPlusIImagPositiveExponent,1=>a::ComplexConvention::RealPlusIImagNegativeExponent,_=>return Err(Error::Metadata)})}else{None};
 let axes=a::SignalAxes{carrier,baseband_hz:bounds,pulse_rate,complex_convention,phase_origin:link(b,38)?};
 axes.validate()?;Ok(axes)
}
pub(super) fn clock(b:Block<'_>,ctx:&SemContext<'_,'_>)->Result<Option<a::Clock>,Error>{
 if !present(b,&[20,21,22,23,24,25])?{return Ok(None);}
 let c=ctx.require_schema(u32v(b,20)?,1)?;
 let timing=a::Timing{scale:u32v(c,1)?,epoch_seconds:i64v(c,2)?,epoch_nanoseconds:u32v(c,3)?,
 ticks:a::Rate::new(u64v(c,4)?,u64v(c,5)?).map_err(|_|Error::Metadata)?,
 samples:a::Rate::new(u64v(b,24)?,u64v(b,25)?).map_err(|_|Error::Metadata)?,start_tick:i64v(b,23)?};
 Ok(Some(a::Clock::new(id(u64v(b,21)?)?,u64v(b,22)?,timing)?))
}
