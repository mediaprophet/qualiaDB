use super::{values::*,Error};
use crate::{q42::field_envelope::Block,sensing::{SemanticId,acquisition as a}};
pub(super) fn byte_order(b:Block<'_>)->Result<a::ByteOrder,Error>{match u32v(b,11)?{0=>Ok(a::ByteOrder::Little),1=>Ok(a::ByteOrder::Big),_=>Err(Error::Metadata)}}
pub(super) fn layout(b:Block<'_>)->Result<a::Layout,Error>{
 let code=u32v(b,10)?;
 let encoding=match code {
  1..=7=>a::Encoding::Shared(a::Scalar::from_code(code).map_err(|_|Error::Metadata)?),
  8=>a::Encoding::U16,9=>a::Encoding::U32,10=>a::Encoding::U64,11=>a::Encoding::ComplexI16,12=>a::Encoding::ComplexF64,
  255=>a::Encoding::Opaque{profile:SemanticId::new(u64v(b,18)?).map_err(|_|Error::Metadata)?,element_bytes:u32v(b,19)?},
  _=>return Err(Error::Unsupported)};
 if code!=255 {absent(b,&[18,19])?;}
 let rank=u32v(b,13)?;if !(1..=4).contains(&rank){return Err(Error::Shape);}
 let mut extents=[0;4];let mut strides=[0;4];let mut axes=[a::Axis::Sample;4];
 for key in [14,15,16,17] {if get(b,key)?.element_count()!=rank{return Err(Error::Shape);}}
 for i in 0..rank as usize {
  extents[i]=u32_at(get(b,14)?.value(),i);
  strides[i]=usize::try_from(u64_at(get(b,15)?.value(),i)).map_err(|_|Error::Overflow)?;
  let tag=u32_at(get(b,16)?.value(),i);let id=u64_at(get(b,17)?.value(),i);
  axes[i]=match tag {0=>a::Axis::Sample,1=>a::Axis::Channel,2=>a::Axis::Sensor,3=>a::Axis::Lag,4=>a::Axis::LeftChannel,5=>a::Axis::RightChannel,
  255=>a::Axis::Registered(SemanticId::new(id).map_err(|_|Error::Metadata)?),_=>return Err(Error::Unsupported)};
  if tag!=255&&id!=0{return Err(Error::Metadata);}
 }
 let components=match u32v(b,12)?{0=>a::ComponentOrder::Scalar,1=>a::ComponentOrder::RealImag,2=>a::ComponentOrder::ImagReal,_=>return Err(Error::Metadata)};
 let l=a::Layout{rank:rank as u8,extents,strides,axes,encoding,byte_order:byte_order(b)?,components};l.span()?;Ok(l)
}
pub(super) fn product(b:Block<'_>)->Result<a::ProductConvention,Error>{match u32v(b,45)?{0=>Ok(a::ProductConvention::LeftTimesConjugateRight),1=>Ok(a::ProductConvention::ConjugateLeftTimesRight),2=>Ok(a::ProductConvention::LeftTimesRight),_=>Err(Error::Metadata)}}
pub(super) fn normalization(b:Block<'_>)->Result<a::Normalization,Error>{
 match u32v(b,46)?{
  0=>{absent(b,&[47,48,49])?;Ok(a::Normalization::Sum)},
  1=>{absent(b,&[48,49])?;let n=u64v(b,47)?;if n==0{return Err(Error::Metadata);}Ok(a::Normalization::Mean{contributing_pairs:n})},
  2=>{absent(b,&[47])?;Ok(a::Normalization::Reported(super::context::link(b,48)?.ok_or(Error::Metadata)?))},
  _=>Err(Error::Metadata)}
}
