use super::{values::*,*};
use crate::{q42::{field_envelope::Block,field_semantics::Context},sensing::acquisition as a};
/// Immutable declared lineage after graph, full source and field admission.
#[derive(Clone,Copy,Debug)]
pub struct Lineage<'a>{pub(super) block:Block<'a>}
impl Lineage<'_> {
 pub fn output_entry(self)->u32{u32v(self.block,1).unwrap()}
 pub fn output_digest(self)->[u8;32]{digest(self.block,2).unwrap()}
 pub fn algorithm(self)->a::ArtifactLink{a::ArtifactLink{digest:digest(self.block,3).unwrap(),identifier:crate::sensing::SemanticId::new(u64v(self.block,4).unwrap()).unwrap()}}
 pub fn parameters(self)->a::ArtifactLink{a::ArtifactLink{digest:digest(self.block,5).unwrap(),identifier:crate::sensing::SemanticId::new(u64v(self.block,6).unwrap()).unwrap()}}
 pub fn inputs(self)->usize{get(self.block,7).unwrap().element_count() as usize}
 pub fn input_entry(self,index:usize)->Result<u32,Error>{if index>=self.inputs(){return Err(Error::Reference);}Ok(u32_at(get(self.block,7)?.value(),index))}
 pub fn input_digest(self,index:usize)->Result<[u8;32],Error>{if index>=self.inputs(){return Err(Error::Reference);}Ok(get(self.block,8)?.value()[index*32..index*32+32].try_into().unwrap())}
 pub fn losses(self)->usize{get(self.block,9).unwrap().element_count() as usize}
 pub fn loss(self,index:usize)->Result<a::Loss,Error>{
  if index>=self.losses(){return Err(Error::Reference);}
  Ok(match u32_at(get(self.block,9)?.value(),index){
   0=>a::Loss::NoneDeclared,1=>a::Loss::Quantization,2=>a::Loss::Resampling,3=>a::Loss::PhaseDiscarded,4=>a::Loss::ChannelReduction,
   5=>a::Loss::EventAggregation,6=>a::Loss::Clipping,7=>a::Loss::Windowing,8=>a::Loss::BandSelection,
   255=>{
    let positions=get(self.block,10)?.value();let n=positions.chunks_exact(4).position(|x|u32::from_le_bytes(x.try_into().unwrap()) as usize==index).ok_or(Error::Reference)?;
    a::Loss::Registered(a::ArtifactLink{digest:get(self.block,12)?.value()[n*32..n*32+32].try_into().unwrap(),
       identifier:crate::sensing::SemanticId::new(u64_at(get(self.block,11)?.value(),n)).map_err(|_|Error::Metadata)?})
   },_=>return Err(Error::Unsupported)})
 }
}
pub(super) fn validate(b:Block<'_>,ctx:&Context<'_,'_>,limits:Limits)->Result<(),Error>{
 let output=u32v(b,1)?;let d=registry::attached(ctx,output,b.id())?;
 if digest(b,2)?.as_slice()!=ctx.entry(output)?.digest(){return Err(Error::Reference);}
 context::link(b,3)?.ok_or(Error::Metadata)?.validate()?;
 context::link(b,5)?.ok_or(Error::Metadata)?.validate()?;
 let inputs=get(b,7)?;let n=inputs.element_count() as usize;
 let losses=get(b,9)?.element_count() as usize;
 if n==0||losses==0||n>limits.links||losses>limits.links{return Err(Error::Limit);}
 ctx.charge_work((n as u64).checked_mul(64).and_then(|x|x.checked_add(losses as u64*8)).ok_or(Error::Overflow)?)?;
 if get(b,8)?.value().len()!=n.checked_mul(32).ok_or(Error::Overflow)?||get(d,17)?.value()!=inputs.value(){return Err(Error::Reference);}
 let p=ctx.require_schema(u32v(d,3)?,4)?;
 if u32v(p,1)?!=2||get(p,2)?.value()!=get(b,8)?.value(){return Err(Error::Reference);}
 for (i,raw) in inputs.value().chunks_exact(4).enumerate(){
  let id=u32::from_le_bytes(raw.try_into().unwrap());descriptor(ctx.entry(id)?,ctx)?;
  if id==output||ctx.entry(id)?.digest()!=&get(b,8)?.value()[i*32..i*32+32]{return Err(Error::Reference);}
 }
 let line=Lineage{block:b};let mut registered=0;
 for i in 0..losses{
  match u32_at(get(b,9)?.value(),i){0 if losses==1=>{},1..=8=>{},255=>registered+=1,_=>return Err(Error::Metadata)}
 }
 if registered==0{absent(b,&[10,11,12])?;}else{
  if !present(b,&[10,11,12])?||get(b,10)?.element_count() as usize!=registered
   ||get(b,11)?.element_count() as usize!=registered||get(b,12)?.value().len()!=registered.checked_mul(32).ok_or(Error::Overflow)?{return Err(Error::Metadata);}
  let mut prior=None;for raw in get(b,10)?.value().chunks_exact(4){
   let i=u32::from_le_bytes(raw.try_into().unwrap()) as usize;
   if i>=losses||prior.is_some_and(|p|p>=i)||u32_at(get(b,9)?.value(),i)!=255{return Err(Error::Metadata);}prior=Some(i);
   line.loss(i)?;
  }
 }
 Ok(())
}

/// Canonical schema7 encoding, bounded by writer MAX_WRITER_LINKS.
pub(super) fn encode(d:a::Derivation<'_>,inputs:&[u32],hashes:&[u8],out:&mut[u8],cursor:&mut usize,limits:crate::q42::field_envelope::Limits)->Result<(),Error>{
 use crate::q42::field_envelope::FieldValue as F;
 use super::write_metadata::{field as f,block};
 let output=[1u32];let od=d.output().digest();let algorithm=d.algorithm();let parameters=d.parameters();
 let aid=[algorithm.identifier.value().ok_or(Error::Metadata)?];let pid=[parameters.identifier.value().ok_or(Error::Metadata)?];
 let mut codes=[0u32;super::write::MAX_WRITER_LINKS];let mut positions=[0u32;super::write::MAX_WRITER_LINKS];
 let mut ids=[0u64;super::write::MAX_WRITER_LINKS];let mut digests=[[0u8;32];super::write::MAX_WRITER_LINKS];let mut n=0;
 for (i,loss) in d.losses().iter().enumerate(){
  codes[i]=match loss{a::Loss::NoneDeclared=>0,a::Loss::Quantization=>1,a::Loss::Resampling=>2,a::Loss::PhaseDiscarded=>3,a::Loss::ChannelReduction=>4,
   a::Loss::EventAggregation=>5,a::Loss::Clipping=>6,a::Loss::Windowing=>7,a::Loss::BandSelection=>8,
   a::Loss::Registered(r)=>{positions[n]=i as u32;ids[n]=r.identifier.value().ok_or(Error::Metadata)?;digests[n]=r.digest;n+=1;255}};
 }
 let fields=[f(1,F::EntryRefs(&output)),f(2,F::Bytes(&od)),f(3,F::Bytes(&algorithm.digest)),f(4,F::U64(&aid)),
  f(5,F::Bytes(&parameters.digest)),f(6,F::U64(&pid)),f(7,F::EntryRefs(inputs)),f(8,F::Bytes(hashes)),
  f(9,F::U32(&codes[..d.losses().len()])),f(10,F::U32(&positions[..n])),f(11,F::U64(&ids[..n])),f(12,F::Bytes(digests[..n].as_flattened()))];
 block(out,cursor,5,7,&fields[..if n==0{9}else{12}],limits)
}
