use super::*;
use super::frames::{cross,dot,sub,unit};
use super::super::production_contracts::EntityDomain;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerFrame {pub normal:[f32;3],pub tangent:[f32;4]}

impl SparseTransfer<'_> {
    /// Corner support MUST use explicit chart+material/provenance equivalence;
    /// geometric vertex identity is never a substitute for a corner index.
    pub fn corner_uvs(&self,uvs:&[[f64;2]],out:&mut [[f64;2]],scratch:&mut [[f64;2]],budget:&mut ProductionBudget<'_>)->Result<usize,TransferError>{
        if self.domain()!=EntityDomain::Corner || self.rule()!=ScalarRule::Intensive {return Err(TransferError::Unsupported("uv_requires_corner_map"));}
        self.apply(uvs,out,scratch,budget)
    }
}

/// Recompute FLAT triangle corner frames after topology changes, with one UV
/// per face corner. This explicitly qualified split-normal mode never smooths
/// across a UV/material seam. Smooth/MikkTSpace mode requires its existing
/// authored surface provider and is not silently substituted here.
/// Degenerate geometry/UVs fail closed; no invented tangent basis.
pub fn recompute_flat_corner_frames(vertices:&[[f64;3]],faces:&[[u32;3]],uvs:&[[f64;2]],
    out:&mut [CornerFrame],scratch:&mut [CornerFrame],budget:&mut ProductionBudget<'_>)->Result<usize,TransferError>{
    let n=faces.len().checked_mul(3).ok_or_else(capacity_overflow)?;
    if faces.is_empty() || uvs.len()!=n {return Err(TransferError::InvalidInput("corner_dimensions"));}
    capacity(n,out.len())?;capacity(n,scratch.len())?;
    charge(budget,&[vertices.len().checked_add(n).ok_or_else(capacity_overflow)?,100])?;
    if vertices.iter().flatten().chain(uvs.iter().flatten()).any(|x|!x.is_finite()){return Err(TransferError::InvalidInput("corner_nonfinite"));}
    for (i,face) in faces.iter().enumerate(){
        if face.iter().any(|v|*v as usize>=vertices.len()){return Err(TransferError::InvalidInput("face_index"));}
        let [a,b,c]=face.map(|v|vertices[v as usize]);
        let e1=sub(b,a);let e2=sub(c,a);let normal=unit(cross(e1,e2))?;
        let [u0,u1,u2]=[uvs[i*3],uvs[i*3+1],uvs[i*3+2]];
        let d1=[u1[0]-u0[0],u1[1]-u0[1]];let d2=[u2[0]-u0[0],u2[1]-u0[1]];
        let det=d1[0]*d2[1]-d1[1]*d2[0];
        if !det.is_finite() || det==0.0{return Err(TransferError::Unsupported("degenerate_uv"));}
        let raw:[f64;3]=std::array::from_fn(|k|(e1[k]*d2[1]-e2[k]*d1[1])/det);
        let tangent=unit(std::array::from_fn(|k|raw[k]-normal[k]*dot(normal,raw)))?;
        let bitangent:[f64;3]=std::array::from_fn(|k|(e2[k]*d1[0]-e1[k]*d2[0])/det);
        let orientation=dot(cross(normal,tangent),bitangent);
        if !orientation.is_finite() || orientation==0.0{return Err(TransferError::InvalidInput("tangent_orientation"));}
        let frame=CornerFrame {normal:normal.map(|x|x as f32),tangent:[tangent[0] as f32,tangent[1] as f32,tangent[2] as f32,if orientation<0.0{-1.0}else{1.0}]};
        scratch[i*3..i*3+3].fill(frame);
    }
    budget.checkpoint(0)?;out[..n].copy_from_slice(&scratch[..n]);Ok(n)
}
