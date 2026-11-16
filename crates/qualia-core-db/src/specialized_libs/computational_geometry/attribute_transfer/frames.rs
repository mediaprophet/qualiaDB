use super::*;

/// Proper orthonormal local-to-world frame. Translation is irrelevant for
/// displacements and forces; positions use `check_affine_positions` separately.
#[derive(Debug, Clone, Copy)]
pub struct RigidFrame { identity:ContentId, axes:[[f64;3];3] }
impl RigidFrame {
    pub fn new(identity:ContentId, axes:[[f64;3];3]) -> Result<Self,TransferError> {
        if axes.iter().flatten().any(|x| !x.is_finite()) { return Err(TransferError::InvalidInput("frame")); }
        for i in 0..3 { for j in 0..3 {
            if (dot(axes[i],axes[j])-if i==j {1.0} else {0.0}).abs()>1e-10 { return Err(TransferError::Unsupported("nonrigid_frame")); }
        } }
        if (dot(cross(axes[0],axes[1]),axes[2])-1.0).abs()>1e-10 { return Err(TransferError::Unsupported("improper_frame")); }
        Ok(Self {identity,axes})
    }
    fn world(self, v:[f64;3])->[f64;3] { std::array::from_fn(|k| self.axes[0][k]*v[0]+self.axes[1][k]*v[1]+self.axes[2][k]*v[2]) }
    fn local(self, v:[f64;3])->[f64;3] { self.axes.map(|a|dot(a,v)) }
}

impl SparseTransfer<'_> {
    /// `P_vec = R_target^T (P ⊗ I) R_source`; the exact same basis transport
    /// is transposed for forces. All DOFs share the named rigid frames.
    pub fn vectors_in_frames(&self, source:RigidFrame,target:RigidFrame, transpose:bool,
        input:&[[f64;3]], output:&mut [[f64;3]], scratch:&mut [[f64;3]], budget:&mut ProductionBudget<'_>) -> Result<usize,TransferError> {
        if self.rule()!=ScalarRule::Intensive { return Err(TransferError::Unsupported("vector_rule")); }
        if source.identity!=self.identities.source_frame || target.identity!=self.identities.target_frame { return Err(TransferError::IdentityMismatch); }
        let (n_in,n_out)=if transpose {(self.target_count(),self.source_count())}else{(self.source_count(),self.target_count())};
        if input.len()!=n_in {return Err(TransferError::InvalidInput("vector_dimensions"));}
        capacity(n_out,output.len())?; capacity(n_out,scratch.len())?;
        charge(budget,&[n_in.checked_add(n_out).and_then(|n|n.checked_add(self.entries.len())).ok_or(TransferError::InvalidInput("work_overflow"))?,40])?;
        if input.iter().flatten().any(|x|!x.is_finite()) {return Err(TransferError::InvalidInput("vector_nonfinite"));}
        scratch[..n_out].fill([0.0;3]);
        for e in self.entries {
            let (i,o,frame)=if transpose {(e.row as usize,e.col as usize,target)}else{(e.col as usize,e.row as usize,source)};
            let v=frame.world(input[i]); for k in 0..3 {scratch[o][k]+=e.value*v[k];}
        }
        let frame=if transpose {source}else{target};
        for v in &mut scratch[..n_out] { *v=frame.local(*v); }
        if scratch[..n_out].iter().flatten().any(|x|!x.is_finite()) {return Err(TransferError::InvalidInput("vector_overflow"));}
        budget.checkpoint(0)?; output[..n_out].copy_from_slice(&scratch[..n_out]); Ok(n_out)
    }
}

pub(super) fn dot(a:[f64;3],b:[f64;3])->f64 {a[0]*b[0]+a[1]*b[1]+a[2]*b[2]}
pub(super) fn cross(a:[f64;3],b:[f64;3])->[f64;3] {[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]]}
pub(super) fn sub(a:[f64;3],b:[f64;3])->[f64;3] {std::array::from_fn(|i|a[i]-b[i])}
pub(super) fn unit(v:[f64;3])->Result<[f64;3],TransferError> {
    let n=v[0].hypot(v[1]).hypot(v[2]);
    if !n.is_finite() || n<=0.0 {return Err(TransferError::InvalidInput("degenerate_frame"));}
    Ok(v.map(|x|x/n))
}
