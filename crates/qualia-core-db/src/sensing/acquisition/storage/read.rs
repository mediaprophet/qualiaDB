use super::{values::*,*};
use crate::{q42::{field_envelope as env,field_semantics as sem},sensing::acquisition as a};
/// Scratch may change on failure. Every returned Original has been fully verified.
/// This acquisition-only document reader rejects other profiles rather than guessing.
pub fn admit<'a,'s>(
 view:&env::StructuralView<'a>,selected:u32,originals:&'s mut[Option<a::Original<'a>>],
 scratch:&mut sem::Scratch<'_>,mut shared:sem::Limits,limits:Limits,
)->Result<Document<'a,'s>,Error>{
 if view.entry_count()>limits.entries||view.entry_count() as usize>originals.len(){return Err(Error::Scratch);}
 if selected==0||selected>view.entry_count(){return Err(Error::Reference);}
 let mut work=limits.work;let mut bytes=0u64;
 // Reject reference/validity paths that might hash before our aggregate admission.
 for i in 1..=view.entry_count(){
  let e=view.entry(i)?;
  bytes=bytes.checked_add(e.stored_bytes()).ok_or(Error::Overflow)?;
  if e.mode()!=ORIGINAL_MODE||e.revision()!=1||e.flags()!=0||e.stride()!=1||e.frame_id()!=0{return Err(Error::Unsupported);}
  let mut found=None;
  for b in view.blocks(){work=work.checked_sub(1).ok_or(Error::Limit)?;if b.id()==e.descriptor_id(){found=Some(b);break;}}
  let d=found.ok_or(Error::Reference)?;
  if d.schema_type()!=3||d.revision()!=1{return Err(Error::Metadata);}
  absent(d,&[1,2,4,5,6,13,14,15,16])?;
 }
 if bytes>limits.source_bytes{return Err(Error::Limit);}
 // verify_chunk + Original::admit both scan each complete source.
 work=work.checked_sub(bytes.checked_mul(2).ok_or(Error::Overflow)?).ok_or(Error::Limit)?;
 for b in view.blocks(){
  work=work.checked_sub(1).ok_or(Error::Limit)?;
  if b.schema_type()==6{
   if field(b,40).is_some(){
    let n=u64v(b,40)?;if n>limits.events{return Err(Error::Limit);}
    work=work.checked_sub(n.checked_mul(20).ok_or(Error::Overflow)?).ok_or(Error::Limit)?;
   }
  }
 }
 shared.work=shared.work.min(work);
 let admitted=sem::admit_shared(view,&AcquisitionRegistry{limits},scratch,shared)?;
 let ctx=&admitted.context;
 let mut uninterpreted_fields=0u32;
 for b in view.blocks(){let known=match b.schema_type(){6=>Some(schema::VIEW),7=>Some(schema::LINEAGE),_=>None};
  if let Some(rules)=known{for f in b.fields(){if !rules.iter().any(|r|r.key==f.key()){uninterpreted_fields=uninterpreted_fields.checked_add(1).ok_or(Error::Overflow)?;}}}
 }
 for i in 1..=view.entry_count(){
  let e=ctx.entry(i)?;let d=descriptor(e,ctx)?;
  let mut views=0;let mut lines=0;
  for raw in get(d,18)?.value().chunks_exact(4){
   let b=ctx.block(u32::from_le_bytes(raw.try_into().unwrap()))?;
   match b.schema_type(){6=>{views+=1;if u32v(b,2)?!=i{return Err(Error::Reference);}},7=>{lines+=1;if u32v(b,1)?!=i{return Err(Error::Reference);}},_=>return Err(Error::Unsupported)}
  }
  if views>1||lines>1{return Err(Error::Metadata);}
  // No interpreted numeric payload is scanned until the complete index/metadata passed.
 }
 let e=ctx.entry(selected)?;let d=descriptor(e,ctx)?;
 let mut declaration=None;let mut line=None;
 for raw in get(d,18)?.value().chunks_exact(4){
  let b=ctx.block(u32::from_le_bytes(raw.try_into().unwrap()))?;
  match b.schema_type(){6=>declaration=Some((b,registry::declaration(b,ctx,limits)?)),7=>line=Some(lineage::Lineage{block:b}),_=>return Err(Error::Unsupported)}
 }
 let (block,decl)=declaration.ok_or(Error::Unsupported)?;
 // Verify every original, including lineage inputs, before any view is released.
 for i in 1..=view.entry_count(){
  let payload=view.verify_chunk(i)?;
  originals[i as usize-1]=Some(a::Original::admit(payload,usize::try_from(limits.source_bytes).map_err(|_|Error::Overflow)?)?);
 }
 let original=originals[decl.entry as usize-1].ok_or(Error::Reference)?;
 let region=original.region(decl.offset,decl.bytes)?;
 let output=match decl.kind {
  1=>View::Tensor(a::TensorView::admit(region,decl.layout.ok_or(Error::Metadata)?,decl.context,decl.clock,decl.axes)?),
  2=>View::Events{view:a::EventView::admit(region,u32v(block,41)?,layout::byte_order(block)?,
    if u32v(block,42)?==0{a::EventOrdering::AcquisitionOrder}else{a::EventOrdering::NondecreasingTick},
    decl.clock.ok_or(Error::Metadata)?,decl.context,decl.events)?,axes:decl.axes},
  3=>{
   let tensor=a::TensorView::admit(region,decl.layout.ok_or(Error::Metadata)?,decl.context,None,decl.axes)?;
   View::Correlation(a::CorrelationView::admit(tensor,decl.clock.ok_or(Error::Metadata)?,i64v(block,43)?,i64v(block,44)?,layout::product(block)?,layout::normalization(block)?)?)
  },_=>return Err(Error::Unsupported)
 };
 Ok(Document{view:output,lineage:line,metadata_bytes:view.metadata_bytes(),uninterpreted_blocks:admitted.preserved_unknown_blocks,uninterpreted_fields,originals:&originals[..view.entry_count() as usize]})
}
