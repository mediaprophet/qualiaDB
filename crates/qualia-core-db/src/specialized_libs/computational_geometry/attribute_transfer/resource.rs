//! Canonical EXTERNAL typed sparse-map resource. Not a `.10d` section.
//! QATR0001: 160-byte LE header (counts, domain/rule, zero reserved, tolerance,
//! four SHA256 identities), then 40-byte source/target protected-domain rows,
//! then existing SparseEntry's explicit LE (u32,u32,f64) records. Rust layout
//! is never serialized. ContentId and ExternalTyped schema hash cover all bytes.
use super::*;
use super::super::ddg_operators::SparseEntry;
use super::super::production_contracts::{EntityDomain,ResourceRef,ResourceKind,ResourceEncoding};

const HEADER:usize=160;
const SCHEMA:&[u8]=b"qualia.attribute-transfer/QATR0001;le;header160;magic8=QATR0001;u32-source,target,nnz;u8-domain(vertex0,face1,corner2,cell3,dof4),rule(intensive0,extensive1);reserved2=0;f64-partition-tolerance;sha256-source,target,source-frame,target-frame;domain40=u64-material,subpart,provenance,chart,protected;entry16=u32-target,source,f64-positive-weight;strict-order;no-trailing-bytes";
pub fn sparse_resource_schema()->ContentId {ContentId::sha256(SCHEMA)}
pub fn sparse_resource_len(sources:usize,targets:usize,entries:usize)->Result<usize,TransferError>{
    if [sources,targets,entries].iter().any(|n|*n>u32::MAX as usize){
        return Err(TransferError::Capacity { required: sources.max(targets).max(entries), available: u32::MAX as usize });
    }
    sources.checked_add(targets).and_then(|n|n.checked_mul(40)).and_then(|n|entries.checked_mul(16).and_then(|e|n.checked_add(e)))
        .and_then(|n|n.checked_add(HEADER)).ok_or_else(capacity_overflow)
}
fn domain_code(d:EntityDomain)->u8{match d{EntityDomain::Vertex=>0,EntityDomain::Face=>1,EntityDomain::Corner=>2,EntityDomain::Cell=>3,EntityDomain::DegreeOfFreedom=>4}}
fn read_domain(n:u8)->Result<EntityDomain,TransferError>{match n{0=>Ok(EntityDomain::Vertex),1=>Ok(EntityDomain::Face),2=>Ok(EntityDomain::Corner),3=>Ok(EntityDomain::Cell),4=>Ok(EntityDomain::DegreeOfFreedom),_=>Err(TransferError::Unsupported("sparse_domain"))}}
fn u32_at(bytes:&[u8],p:usize)->u32{u32::from_le_bytes(bytes[p..p+4].try_into().expect("admitted range"))}
fn u64_at(bytes:&[u8],p:usize)->u64{u64::from_le_bytes(bytes[p..p+8].try_into().expect("admitted range"))}
fn id_at(bytes:&[u8],p:usize)->ContentId{ContentId{namespace:super::super::production_contracts::HashNamespace::Sha256V1,digest:bytes[p..p+32].try_into().expect("admitted header")}}

pub fn encode_sparse_resource(map:&SparseTransfer<'_>,output:&mut[u8],budget:&mut ProductionBudget<'_>)->Result<ResourceRef,TransferError>{
    let n=sparse_resource_len(map.source_count(),map.target_count(),map.entries.len())?;capacity(n,output.len())?;charge(budget,&[n,3])?;
    budget.checkpoint(0)?;
    output[..n].fill(0);output[..8].copy_from_slice(b"QATR0001");
    for (p,x) in [(8,map.source_count()),(12,map.target_count()),(16,map.entries.len())]{output[p..p+4].copy_from_slice(&(x as u32).to_le_bytes());}
    output[20]=domain_code(map.domain);output[21]=if map.rule==ScalarRule::Intensive{0}else{1};output[24..32].copy_from_slice(&map.sum_tolerance.to_le_bytes());
    for (i,id) in [map.identities.source,map.identities.target,map.identities.source_frame,map.identities.target_frame].iter().enumerate(){output[32+i*32..64+i*32].copy_from_slice(&id.digest);}
    let mut p=HEADER;
    for d in map.source_domains.iter().chain(map.target_domains){
        for v in [d.material,d.subpart,d.provenance,d.chart,d.protected]{output[p..p+8].copy_from_slice(&v.to_le_bytes());p+=8;}
    }
    for e in map.entries{output[p..p+4].copy_from_slice(&e.row.to_le_bytes());output[p+4..p+8].copy_from_slice(&e.col.to_le_bytes());output[p+8..p+16].copy_from_slice(&e.value.to_le_bytes());p+=16;}
    Ok(ResourceRef{id:ContentId::sha256(&output[..n]),kind:ResourceKind::SparseMap,encoding:ResourceEncoding::ExternalTyped{schema:sparse_resource_schema()},byte_len:n as u64,element_count:Some(map.entries.len() as u64)})
}

/// Entire output arguments are disposable decode SCRATCH, never a published
/// map on error. The immutable returned view is the publication token.
pub fn decode_sparse_resource<'a>(resource:&ResourceRef,bytes:&[u8],expected:TransferIdentities,
    source_domains:&'a mut[ProtectedDomain],target_domains:&'a mut[ProtectedDomain],entries:&'a mut[SparseEntry],column_sums:&mut[f64],
    budget:&mut ProductionBudget<'_>)->Result<SparseTransfer<'a>,TransferError>{
    charge(budget,&[bytes.len(),3])?;
    if resource.kind!=ResourceKind::SparseMap || resource.encoding!=(ResourceEncoding::ExternalTyped{schema:sparse_resource_schema()}) {return Err(TransferError::Unsupported("sparse_resource_encoding"));}
    resource.verify(bytes).map_err(|_|TransferError::IdentityMismatch)?;
    if bytes.len()<HEADER || &bytes[..8]!=b"QATR0001" || bytes[22..24]!=[0,0]{return Err(TransferError::InvalidInput("sparse_header"));}
    let (s,t,e)=(u32_at(bytes,8) as usize,u32_at(bytes,12) as usize,u32_at(bytes,16) as usize);
    if sparse_resource_len(s,t,e)?!=bytes.len() || resource.element_count!=Some(e as u64){return Err(TransferError::InvalidInput("sparse_length"));}
    capacity(s,source_domains.len())?;capacity(t,target_domains.len())?;capacity(e,entries.len())?;
    let ids=TransferIdentities{source:id_at(bytes,32),target:id_at(bytes,64),source_frame:id_at(bytes,96),target_frame:id_at(bytes,128)};
    if ids!=expected{return Err(TransferError::IdentityMismatch);}
    let domain=read_domain(bytes[20])?;
    let rule=match bytes[21]{0=>ScalarRule::Intensive,1=>ScalarRule::Extensive,_=>return Err(TransferError::Unsupported("sparse_rule"))};
    let tol=f64::from_bits(u64_at(bytes,24));let mut p=HEADER;
    for d in source_domains[..s].iter_mut().chain(target_domains[..t].iter_mut()){
        *d=ProtectedDomain{material:u64_at(bytes,p),subpart:u64_at(bytes,p+8),provenance:u64_at(bytes,p+16),chart:u64_at(bytes,p+24),protected:u64_at(bytes,p+32)};p+=40;
    }
    for slot in &mut entries[..e]{*slot=SparseEntry{row:u32_at(bytes,p),col:u32_at(bytes,p+4),value:f64::from_bits(u64_at(bytes,p+8))};p+=16;}
    SparseTransfer::new(ids,expected,domain,rule,&entries[..e],&source_domains[..s],&target_domains[..t],column_sums,tol,budget)
}
