use super::*;
use crate::sensing::acquisition as a;
use crate::q42::{field_envelope as env,field_semantics as sem};
pub const ACQUISITION_SCHEMA:u16=6;
pub const DERIVATION_SCHEMA:u16=7;
pub const ORIGINAL_MODE:u16=0x104;
pub const ORIGINAL_MODEL:&str="urn:qualia:acquisition:original:1";
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Error { Envelope(env::Error),Semantic(sem::Error),Acquisition(a::Error),Shape,Reference,Metadata,Limit,Unsupported,Scratch,Overflow }
impl From<env::Error> for Error {fn from(e:env::Error)->Self{Self::Envelope(e)}}
impl From<sem::Error> for Error {fn from(e:sem::Error)->Self{Self::Semantic(e)}}
impl From<a::Error> for Error {fn from(e:a::Error)->Self{Self::Acquisition(e)}}
/// Every event view has an explicit SignalAxes declaration, separate from EventView.
#[derive(Clone,Copy,Debug)]
pub enum View<'a> {Tensor(a::TensorView<'a>),Events{view:a::EventView<'a>,axes:a::SignalAxes},Correlation(a::CorrelationView<'a>)}
impl<'a> View<'a> {
 pub fn region(self)->a::Region<'a> {match self{Self::Tensor(v)=>v.region(),Self::Events{view,..}=>view.region(),Self::Correlation(v)=>v.tensor().region()}}
 pub fn context(self)->a::Context {match self{Self::Tensor(v)=>v.context(),Self::Events{view,..}=>view.context(),Self::Correlation(v)=>v.tensor().context()}}
 pub fn axes(self)->a::SignalAxes {match self{Self::Tensor(v)=>v.signal_axes(),Self::Events{axes,..}=>axes,Self::Correlation(v)=>v.tensor().signal_axes()}}
 pub fn clock(self)->Option<a::Clock> {match self{Self::Tensor(v)=>v.clock(),Self::Events{view,..}=>Some(view.clock()),Self::Correlation(v)=>Some(v.clock())}}
}
#[derive(Clone,Copy,Debug)]
pub struct Limits {pub entries:u32,pub source_bytes:u64,pub events:u64,pub links:usize,pub work:u64}
impl Default for Limits {fn default()->Self {Self{entries:65,source_bytes:256<<20,events:1_000_000,links:64,work:128_000_000}}}
pub struct Document<'a,'s> {
 pub view:View<'a>,
 pub lineage:Option<super::lineage::Lineage<'a>>,
 pub metadata_bytes:&'a [u8],
 pub uninterpreted_blocks:u32,
 pub uninterpreted_fields:u32,
 pub(super) originals:&'s [Option<a::Original<'a>>],
}
impl<'a> Document<'a,'_> {
 /// All originals were fully verified before this document was returned.
 pub fn original(&self,entry:u32)->Result<a::Original<'a>,Error> {
  self.originals.get(entry.checked_sub(1).ok_or(Error::Reference)? as usize).and_then(|x|*x).ok_or(Error::Reference)
 }
}
#[derive(Clone,Copy,Debug)]
pub struct Input<'a> {pub family:env::Family,pub view:View<'a>,pub derivation:Option<a::Derivation<'a>>}
pub(super) fn descriptor<'a>(entry:env::Entry<'a>,ctx:&sem::Context<'_,'a>)->Result<env::Block<'a>,Error> {
 let d=ctx.require_schema(entry.descriptor_id(),3)?;
 if entry.mode()!=ORIGINAL_MODE||entry.revision()!=1||entry.flags()!=0||entry.stride()!=1||entry.record_count()!=entry.stored_bytes()
  ||entry.frame_id()!=0||values::text(d,7)?!="original_bytes"||values::text(d,8)?!="1"
  ||values::text(d,10)?!="byte"||values::text(d,12)?!=ORIGINAL_MODEL||values::u32v(d,9)?!=1||values::u32v(d,11)?!=7
 {return Err(Error::Unsupported);}
 values::absent(d,&[1,2,4,5,6,13,14,15,16])?;
 Ok(d)
}
