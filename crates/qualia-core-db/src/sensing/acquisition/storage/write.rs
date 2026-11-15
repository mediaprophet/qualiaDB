use super::*;
use crate::{q42::{field_envelope as env,field_semantics as sem}};
pub const MAX_WRITER_LINKS:usize=64;
/// Build into caller-owned metadata/staging, re-admit with production reader, then
/// copy to final output. Every error leaves final output unchanged. Scratch may change.
pub fn encode(input:Input<'_>,metadata:&mut[u8],staging:&mut[u8],
 scratch:&mut sem::Scratch<'_>,output:&mut[u8],envelope:env::Limits,shared:sem::Limits,limits:Limits)->Result<usize,Error>{
 let original=input.view.region().original();
 let inputs=input.derivation.map(|d|d.inputs()).unwrap_or(&[]);
 if inputs.len()>MAX_WRITER_LINKS||inputs.len()>limits.links||inputs.len()+1>limits.entries as usize{return Err(Error::Limit);}
 if let Some(d)=input.derivation{
  if d.losses().len()>MAX_WRITER_LINKS||d.losses().len()>limits.links{return Err(Error::Limit);}
  if d.output().digest()!=original.digest()||d.output().bytes()!=original.bytes(){return Err(Error::Reference);}
 }
 let mut total=original.bytes().len() as u64;
 if original.bytes().is_empty(){return Err(Error::Unsupported);}
 for src in inputs{if src.bytes().is_empty(){return Err(Error::Unsupported);}total=total.checked_add(src.bytes().len() as u64).ok_or(Error::Overflow)?;}
 if total>limits.source_bytes{return Err(Error::Limit);}
 // Admit a lower bound before metadata/staging or payload hashing.
 if total.checked_mul(6).ok_or(Error::Overflow)?>limits.work{return Err(Error::Limit);}
 let mut source_ids=[0u32;MAX_WRITER_LINKS];let mut source_digests=[[0u8;32];MAX_WRITER_LINKS];
 for (i,src) in inputs.iter().enumerate(){source_ids[i]=i as u32+2;source_digests[i]=src.digest();}
 let ids=&source_ids[..inputs.len()];let hashes=source_digests[..inputs.len()].as_flattened();
 let mut cursor=0usize;
 write_metadata::original_blocks(metadata,&mut cursor,0,ids,hashes,envelope)?;
 write_view::encode(input.view,metadata,&mut cursor,envelope)?;
 if let Some(d)=input.derivation{lineage::encode(d,ids,hashes,metadata,&mut cursor,envelope)?;}
 for i in 0..inputs.len(){write_metadata::original_blocks(metadata,&mut cursor,i+1,&[],&[],envelope)?;}
 let blank=env::EntryInput{mode:ORIGINAL_MODE,revision:1,flags:0,stream_id:0,frame_id:0,start_tick:0,time_count:0,spatial_key:0,record_count:1,stride:1,codec:0,descriptor_id:1,payload:&[]};
 let mut entries=[blank;MAX_WRITER_LINKS+1];
 for (i,entry) in entries[..inputs.len()+1].iter_mut().enumerate(){
  let bytes=if i==0{original.bytes()}else{inputs[i-1].bytes()};
  *entry=env::EntryInput{stream_id:i as u32+1,descriptor_id:i as u32*10+2,record_count:bytes.len() as u64,payload:bytes,..blank};
 }
 let data=env::EnvelopeInput{family:input.family,flags:0,metadata:&metadata[..cursor],entries:&entries[..inputs.len()+1]};
 let len=data.encoded_len(envelope)?;
 if len>output.len()||len>staging.len(){return Err(Error::Scratch);}
 let reserve=(len as u64).checked_mul(4).ok_or(Error::Overflow)?;
 let work=limits.work.checked_sub(reserve).ok_or(Error::Limit)?;
 let staged=data.encode_into(staging,envelope)?;
 let parsed=env::StructuralView::open(staged,input.family,envelope)?;
 let mut originals=[None;MAX_WRITER_LINKS+1];
 let result=admit(&parsed,1,&mut originals,scratch,shared,Limits{work,..limits})?;
 // Production read admission establishes all required source/shape/graph conditions.
 drop(result);
 output[..len].copy_from_slice(staged);Ok(len)
}
