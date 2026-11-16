use super::*;
use crate::container_10d::correspondence_section::{
    self as codec, VertexTetBinding, CorrespondenceMiniHeader,
    BARYCENTRIC_UNORM16_SUM, FLAG_FEMC_CONFORMING_BARYCENTRIC, FLAG_FEMC_HAS_NORMAL_ROTATION,
};

#[derive(Debug,Clone,Copy)]
pub struct TetBindingInput {pub tet:u32,pub weights:[f32;4],pub normal:[f32;3]}

fn valid_binding(b:&VertexTetBinding,tets:usize)->Result<(),TransferError>{
    if b.tet_element_index as usize>=tets || b.barycentric_unorm16.iter().map(|w|*w as u32).sum::<u32>()!=BARYCENTRIC_UNORM16_SUM {
        Err(TransferError::InvalidInput("femc_binding"))
    }else{Ok(())}
}
fn flags(header:CorrespondenceMiniHeader)->Result<(),TransferError>{
    let known=FLAG_FEMC_CONFORMING_BARYCENTRIC|FLAG_FEMC_HAS_NORMAL_ROTATION;
    if header.flags!=known {return Err(TransferError::Unsupported("femc_flags"));}Ok(())
}

/// Strict preflight around the established codec constructor, which otherwise
/// clamps invalid input. Existing 16-byte FEMC records and magic are unchanged.
pub fn quantize_tet_bindings(input:&[TetBindingInput],tet_count:u32,output:&mut [VertexTetBinding],scratch:&mut [VertexTetBinding],
    budget:&mut ProductionBudget<'_>)->Result<usize,TransferError>{
    if input.is_empty() || tet_count==0 || codec::correspondence_encoded_len(input.len()).is_none(){return Err(TransferError::InvalidInput("femc_dimensions"));}
    capacity(input.len(),output.len())?;capacity(input.len(),scratch.len())?;charge(budget,&[input.len(),32])?;
    for (i,b) in input.iter().enumerate(){
        if b.tet>=tet_count || b.weights.iter().any(|w|!w.is_finite() || *w<0.0 || *w>1.0)
            || (b.weights.iter().map(|w|*w as f64).sum::<f64>()-1.0).abs()>1e-6
            || b.normal.iter().any(|n|!n.is_finite()) || b.normal.iter().map(|n|(*n as f64).powi(2)).sum::<f64>()<1e-20 {
            return Err(TransferError::InvalidInput("femc_constructor_input"));
        }
        scratch[i]=VertexTetBinding::from_barycentric(b.tet,b.weights,b.normal);
        valid_binding(&scratch[i],tet_count as usize)?;
    }
    budget.checkpoint(0)?;output[..input.len()].copy_from_slice(&scratch[..input.len()]);Ok(input.len())
}

pub fn encode_femc(bindings:&[VertexTetBinding],tet_count:u32,output:&mut [u8],budget:&mut ProductionBudget<'_>)->Result<usize,TransferError>{
    let size=codec::correspondence_encoded_len(bindings.len()).ok_or_else(capacity_overflow)?;capacity(size,output.len())?;
    charge(budget,&[bindings.len().checked_add(size).ok_or_else(capacity_overflow)?,3])?;
    for b in bindings {valid_binding(b,tet_count as usize)?;}
    let header=CorrespondenceMiniHeader {binding_count:bindings.len() as u32,tet_cell_count:tet_count,..Default::default()};
    budget.checkpoint(0)?;codec::encode_correspondence_section(&header,bindings,output).map_err(|_|TransferError::Codec)
}

/// Legacy decode writes prefixes before a late bad binding. Decode into
/// disposable scratch first and publish only after strict tetra bounds/flags.
pub fn decode_femc(bytes:&[u8],expected_tet_count:u32,output:&mut [VertexTetBinding],scratch:&mut [VertexTetBinding],
    budget:&mut ProductionBudget<'_>)->Result<usize,TransferError>{
    charge(budget,&[bytes.len(),3])?;
    let (header,n)=codec::decode_correspondence_section(bytes,scratch).map_err(|_|TransferError::Codec)?;
    flags(header)?;if header.tet_cell_count!=expected_tet_count {return Err(TransferError::IdentityMismatch);}
    capacity(n,output.len())?;for b in &scratch[..n] {valid_binding(b,expected_tet_count as usize)?;}
    budget.checkpoint(0)?;output[..n].copy_from_slice(&scratch[..n]);Ok(n)
}

/// Existing f32 deformation arithmetic with complete validation before
/// publication. Domain membership applies to the entire host cell; interfaces
/// requiring per-corner protected labels need explicit split cells first.
pub fn deform_femc(bindings:&[VertexTetBinding],tets:&[[u32;4]],vertices:&[[f32;3]],
    cell_domains:&[ProtectedDomain],surface_domains:&[ProtectedDomain],output:&mut [[f32;3]],scratch:&mut [[f32;3]],
    budget:&mut ProductionBudget<'_>)->Result<usize,TransferError>{
    if cell_domains.len()!=tets.len() || surface_domains.len()!=bindings.len(){return Err(TransferError::InvalidInput("femc_domains"));}
    capacity(bindings.len(),output.len())?;capacity(bindings.len(),scratch.len())?;
    charge(budget,&[bindings.len(),48])?;
    for (i,b) in bindings.iter().enumerate(){
        valid_binding(b,tets.len())?;let t=b.tet_element_index as usize;
        if cell_domains[t]!=surface_domains[i]{return Err(TransferError::DomainCrossing { row:i, col:t });}
        for &v in &tets[t]{if v as usize>=vertices.len() || vertices[v as usize].iter().any(|x|!x.is_finite()) {return Err(TransferError::InvalidInput("femc_vertex"));}}
    }
    codec::deform_surface_vertices_zero_heap(bindings,tets,vertices,scratch).map_err(|_|TransferError::Codec)?;
    if scratch[..bindings.len()].iter().flatten().any(|x|!x.is_finite()){return Err(TransferError::InvalidInput("femc_deformation_overflow"));}
    budget.checkpoint(0)?;output[..bindings.len()].copy_from_slice(&scratch[..bindings.len()]);Ok(bindings.len())
}
