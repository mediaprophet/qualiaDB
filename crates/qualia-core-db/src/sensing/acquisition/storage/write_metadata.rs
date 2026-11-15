use super::*;
use crate::q42::field_envelope::{self as env,BlockInput,FieldInput,FieldValue as F,MetadataInput};
pub(super) fn field<'a>(key:u16,value:F<'a>)->FieldInput<'a>{FieldInput{key,flags:1,value}}
pub(super) fn block(out:&mut[u8],cursor:&mut usize,id:u32,schema:u16,fields:&[FieldInput<'_>],limits:env::Limits)->Result<(),Error>{
 let end=MetadataInput{blocks:&[BlockInput{id,schema_type:schema,revision:1,flags:1,fields}]}.encode_into(out.get_mut(*cursor..).ok_or(Error::Scratch)?,limits)?.len();
 *cursor=cursor.checked_add(end).ok_or(Error::Overflow)?;Ok(())
}
pub(super) struct Fields {data:[[u8;32];49],len:[u8;49],tags:[u8;49],counts:[u32;49]}
impl Fields {
 pub fn new()->Self{Self{data:[[0;32];49],len:[0;49],tags:[0;49],counts:[0;49]}}
 pub fn put(&mut self,k:u16,tag:u8,count:u32,b:&[u8])->Result<(),Error>{
  let i=usize::from(k.checked_sub(1).ok_or(Error::Metadata)?);
  if i>=49||b.len()>32{return Err(Error::Metadata);}
  self.data[i][..b.len()].copy_from_slice(b);self.len[i]=b.len() as u8;self.tags[i]=tag;self.counts[i]=count;Ok(())
 }
 pub fn u32(&mut self,k:u16,n:u32)->Result<(),Error>{self.put(k,1,1,&n.to_le_bytes())}
 pub fn u64(&mut self,k:u16,n:u64)->Result<(),Error>{self.put(k,2,1,&n.to_le_bytes())}
 pub fn i64(&mut self,k:u16,n:i64)->Result<(),Error>{self.put(k,3,1,&n.to_le_bytes())}
 pub fn f64(&mut self,k:u16,n:f64)->Result<(),Error>{self.put(k,5,1,&n.to_le_bytes())}
 pub fn link(&mut self,k:u16,r:Option<crate::sensing::acquisition::ArtifactLink>)->Result<(),Error>{
  if let Some(r)=r{r.validate()?;self.put(k,7,32,&r.digest)?;self.u64(k+1,r.identifier.value().ok_or(Error::Metadata)?)?;}Ok(())
 }
 pub fn encode(&self,out:&mut[u8],cursor:&mut usize,limits:env::Limits)->Result<(),Error>{
  let empty=field(1,F::Bytes(&[]));let mut fields=[empty;49];let mut n=0;
  for i in 0..49 {if self.tags[i]!=0{fields[n]=field(i as u16+1,F::Raw{type_tag:self.tags[i],element_count:self.counts[i],bytes:&self.data[i][..self.len[i] as usize]});n+=1;}}
  block(out,cursor,3,6,&fields[..n],limits)
 }
}
pub(super) fn original_blocks(out:&mut[u8],cursor:&mut usize,index:usize,inputs:&[u32],digests:&[u8],limits:env::Limits)->Result<(),Error>{
 let id=u32::try_from(index).map_err(|_|Error::Overflow)?.checked_mul(10).and_then(|n|n.checked_add(1)).ok_or(Error::Overflow)?;
 let role=if index==0&&!inputs.is_empty(){2}else{4};
 let role_array=[role];let p=[field(1,F::U32(&role_array)),field(2,F::Bytes(digests))];let plen=if digests.is_empty(){1}else{2};
 block(out,cursor,id,4,&p[..plen],limits)?;
 let channels=[1];let scalar=[7];let prov=[id];let extension=[3u32,5];
 let ext=if index==0{if inputs.is_empty(){&extension[..1]}else{&extension[..2]}}else{&extension[..0]};
 let mut f=[field(3,F::MetadataRefs(&prov)),field(7,F::Utf8("original_bytes")),field(8,F::Utf8("1")),field(9,F::U32(&channels)),
  field(10,F::Utf8("byte")),field(11,F::U32(&scalar)),field(12,F::Utf8(ORIGINAL_MODEL)),field(17,F::EntryRefs(inputs)),field(18,F::MetadataRefs(ext))];
 if inputs.is_empty(){f[7]=f[8];block(out,cursor,id+1,3,&f[..8],limits)?;}else{block(out,cursor,id+1,3,&f,limits)?;}
 Ok(())
}
