use super::{Error};
use crate::q42::{field_envelope::{Block,Field},field_semantics as sem};
pub(super) fn field<'a>(b:Block<'a>,k:u16)->Option<Field<'a>> {sem::field(b,k)}
pub(super) fn get<'a>(b:Block<'a>,k:u16)->Result<Field<'a>,Error>{field(b,k).ok_or(Error::Metadata)}
pub(super) fn u32v(b:Block<'_>,k:u16)->Result<u32,Error>{Ok(u32::from_le_bytes(get(b,k)?.value().try_into().map_err(|_|Error::Metadata)?))}
pub(super) fn u64v(b:Block<'_>,k:u16)->Result<u64,Error>{Ok(u64::from_le_bytes(get(b,k)?.value().try_into().map_err(|_|Error::Metadata)?))}
pub(super) fn i64v(b:Block<'_>,k:u16)->Result<i64,Error>{Ok(u64v(b,k)? as i64)}
pub(super) fn f64v(b:Block<'_>,k:u16)->Result<f64,Error>{Ok(f64::from_bits(u64v(b,k)?))}
pub(super) fn text(b:Block<'_>,k:u16)->Result<&str,Error>{std::str::from_utf8(get(b,k)?.value()).map_err(|_|Error::Metadata)}
pub(super) fn digest(b:Block<'_>,k:u16)->Result<[u8;32],Error>{get(b,k)?.value().try_into().map_err(|_|Error::Metadata)}
pub(super) fn present(b:Block<'_>,keys:&[u16])->Result<bool,Error>{
 let n=keys.iter().filter(|&&k|field(b,k).is_some()).count();
 if n!=0&&n!=keys.len(){return Err(Error::Metadata);}Ok(n!=0)
}
pub(super) fn absent(b:Block<'_>,keys:&[u16])->Result<(),Error>{
 if keys.iter().any(|&k|field(b,k).is_some()){return Err(Error::Metadata);}Ok(())
}
pub(super) fn u32_at(b:&[u8],i:usize)->u32{u32::from_le_bytes(b[i*4..i*4+4].try_into().unwrap())}
pub(super) fn u64_at(b:&[u8],i:usize)->u64{u64::from_le_bytes(b[i*8..i*8+8].try_into().unwrap())}
