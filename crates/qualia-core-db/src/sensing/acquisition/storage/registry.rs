use super::*;
use super::values::*;
use crate::{q42::{field_envelope::Block,field_semantics::{self as sem,DomainRegistry,FieldRule}},sensing::acquisition as a};
#[derive(Clone,Copy,Debug)]
pub struct AcquisitionRegistry {pub limits:Limits}
impl DomainRegistry for AcquisitionRegistry {
 fn rules(&self,s:u16,v:u16)->Option<&'static[FieldRule]>{if v!=1{return None;}match s{6=>Some(schema::VIEW),7=>Some(schema::LINEAGE),_=>None}}
 fn validate_block(&self,b:Block<'_>,ctx:&sem::Context<'_,'_>)->Result<(),sem::Error>{
  let result=(||->Result<(),Error>{
   ctx.charge_work((b.fields().count() as u64).checked_mul(128).ok_or(Error::Overflow)?)?;
   let rules=self.rules(b.schema_type(),1).ok_or(Error::Unsupported)?;
   for f in b.fields(){if f.flags()&1==0 && rules.iter().any(|r|r.key==f.key()){return Err(Error::Metadata);}}
   match b.schema_type(){6=>{declaration(b,ctx,self.limits)?;},7=>lineage::validate(b,ctx,self.limits)?,_=>return Err(Error::Unsupported)}
   Ok(())
  })();
  result.map_err(|e|match e{Error::Semantic(s)=>s,Error::Limit=>sem::Error::Limit,Error::Unsupported=>sem::Error::UnsupportedDomain,_=>sem::Error::Value})
 }
}
pub(super) fn attached<'a>(ctx:&sem::Context<'_,'a>,entry:u32,id:u32)->Result<Block<'a>,Error>{
 let d=descriptor(ctx.entry(entry)?,ctx)?;
 let mut count=0;
 for bytes in get(d,18)?.value().chunks_exact(4){ctx.charge_work(1)?;if u32::from_le_bytes(bytes.try_into().unwrap())==id{count+=1;}}
 if count!=1{return Err(Error::Reference);}Ok(d)
}
#[derive(Clone,Copy)]
pub(super) struct Declaration {
 pub kind:u32,pub entry:u32,pub offset:usize,pub bytes:usize,pub context:a::Context,pub axes:a::SignalAxes,
 pub clock:Option<a::Clock>,pub layout:Option<a::Layout>,pub events:usize,
}
pub(super) fn declaration<'a>(b:Block<'a>,ctx:&sem::Context<'_,'a>,limits:Limits)->Result<Declaration,Error>{
 let entry=u32v(b,2)?;let e=ctx.entry(entry)?;attached(ctx,entry,b.id())?;
 if digest(b,3)?.as_slice()!=e.digest(){return Err(Error::Reference);}
 let offset=usize::try_from(u64v(b,4)?).map_err(|_|Error::Overflow)?;
 let bytes=usize::try_from(u64v(b,5)?).map_err(|_|Error::Overflow)?;
 if offset.checked_add(bytes).ok_or(Error::Overflow)? as u64>e.decoded_bytes(){return Err(Error::Shape);}
 let context=context::context(b)?;let axes=context::axes(b)?;let clock=context::clock(b,ctx)?;
 let kind=u32v(b,1)?;let mut layout=None;let mut events=0usize;
 match kind{
  1|3=>{
   absent(b,&[40,41,42])?;
   let l=layout::layout(b)?;if l.span()?!=bytes{return Err(Error::Shape);}
   if !l.encoding.complex()&&(axes.complex_convention.is_some()||axes.phase_origin.is_some()){return Err(Error::Metadata);}
   if kind==1{
    absent(b,&[43,44,45,46,47,48,49])?;
    if let Some(c)=clock{
     let axis=l.axes[..usize::from(l.rank)].iter().position(|x|*x==a::Axis::Sample).ok_or(Error::Shape)?;
     c.time_at(u64::from(l.extents[axis]-1))?;
    }
   }else{
    clock.ok_or(Error::Metadata)?;
    if l.rank!=3||l.axes[..3]!=[a::Axis::Lag,a::Axis::LeftChannel,a::Axis::RightChannel]||!l.encoding.complex(){return Err(Error::Shape);}
    let step=i64v(b,44)?;if step==0{return Err(Error::Shape);}
    step.checked_mul(i64::from(l.extents[0]-1)).and_then(|x|i64v(b,43).ok().and_then(|first|first.checked_add(x))).ok_or(Error::Overflow)?;
    layout::product(b)?;layout::normalization(b)?;
   }
   layout=Some(l);
  },
  2=>{
   absent(b,&[10,12,13,14,15,16,17,18,19,37,38,39,43,44,45,46,47,48,49])?;
   layout::byte_order(b)?;clock.ok_or(Error::Metadata)?;
   let n=u64v(b,40)?;if n>limits.events{return Err(Error::Limit);}
   if n.checked_mul(20).ok_or(Error::Overflow)?!=bytes as u64||u32v(b,41)?==0||u32v(b,42)?>1{return Err(Error::Shape);}
   events=usize::try_from(n).map_err(|_|Error::Overflow)?;
  },
  _=>return Err(Error::Unsupported)
 }
 Ok(Declaration{kind,entry,offset,bytes,context,axes,clock,layout,events})
}
