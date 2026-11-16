use super::*;
use super::super::ddg_operators::SparseEntry;
use super::super::production_contracts::EntityDomain;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarRule { Intensive, Extensive }

/// Positive, duplicate-free COO, lexicographically sorted (target, source).
/// Intensive rows sum to one; extensive columns sum to one. A categorical
/// operation additionally requires exact agreement of all source labels.
#[derive(Debug, Clone, Copy)]
pub struct SparseTransfer<'a> {
    pub(crate) identities: TransferIdentities,
    pub(crate) domain: EntityDomain,
    pub(crate) rule: ScalarRule,
    pub(crate) sum_tolerance: f64,
    pub(crate) entries: &'a [SparseEntry],
    pub(crate) source_domains: &'a [ProtectedDomain],
    pub(crate) target_domains: &'a [ProtectedDomain],
}
impl<'a> SparseTransfer<'a> {
    /// `column_sums` is disposable scratch. No allocation or implicit repair.
    /// Validation costs O(source+target+nnz); work is reserved before scans.
    pub fn new(identities: TransferIdentities, expected: TransferIdentities,
        domain: EntityDomain, rule: ScalarRule, entries: &'a [SparseEntry],
        source_domains: &'a [ProtectedDomain], target_domains: &'a [ProtectedDomain],
        column_sums: &mut [f64], sum_tolerance: f64, budget: &mut ProductionBudget<'_>) -> Result<Self, TransferError> {
        if identities != expected { return Err(TransferError::IdentityMismatch); }
        tolerance(sum_tolerance)?;
        if source_domains.is_empty() || target_domains.is_empty() || entries.is_empty()
            || source_domains.len() > u32::MAX as usize || target_domains.len() > u32::MAX as usize {
            return Err(TransferError::InvalidInput("sparse_dimensions"));
        }
        capacity(source_domains.len(), column_sums.len())?;
        charge(budget, &[source_domains.len().checked_add(target_domains.len())
            .and_then(|n| n.checked_add(entries.len())).ok_or(TransferError::InvalidInput("work_overflow"))?, 3])?;
        column_sums[..source_domains.len()].fill(0.0);
        let mut prev = None;
        let mut next_row = 0usize;
        let mut row_sum = 0.0;
        for e in entries {
            let (r,c) = (e.row as usize,e.col as usize);
            if r >= target_domains.len() || c >= source_domains.len() || !e.value.is_finite() || e.value <= 0.0 {
                return Err(TransferError::InvalidInput("sparse_entry"));
            }
            if prev.is_some_and(|p| p >= (e.row,e.col)) { return Err(TransferError::InvalidInput("sparse_order")); }
            if source_domains[c] != target_domains[r] { return Err(TransferError::DomainCrossing { row:r,col:c }); }
            if r != next_row {
                if rule == ScalarRule::Intensive && ((row_sum - 1.0f64).abs() > sum_tolerance || r != next_row+1) {
                    return Err(TransferError::InvalidInput("row_partition"));
                }
                next_row = r; row_sum = 0.0;
            }
            row_sum += e.value; column_sums[c] += e.value;
            if !row_sum.is_finite() || !column_sums[c].is_finite() { return Err(TransferError::InvalidInput("weight_overflow")); }
            prev = Some((e.row,e.col));
        }
        if rule == ScalarRule::Intensive {
            if entries[0].row != 0 || next_row+1 != target_domains.len() || (row_sum-1.0).abs() > sum_tolerance {
                return Err(TransferError::InvalidInput("row_partition"));
            }
        } else if column_sums[..source_domains.len()].iter().any(|s| (*s-1.0).abs() > sum_tolerance) {
            return Err(TransferError::InvalidInput("column_partition"));
        }
        Ok(Self { identities,domain,rule,sum_tolerance,entries,source_domains,target_domains })
    }
    pub fn identities(&self) -> TransferIdentities { self.identities }
    pub fn domain(&self) -> EntityDomain { self.domain }
    pub fn rule(&self) -> ScalarRule { self.rule }
    pub fn entries(&self) -> &[SparseEntry] { self.entries }
    pub fn source_count(&self) -> usize { self.source_domains.len() }
    pub fn target_count(&self) -> usize { self.target_domains.len() }

    /// Applies the declared scalar map. Scratch is invalid after any error;
    /// published output is unchanged, including cancellation/overflow errors.
    pub fn apply<const N: usize>(&self, values: &[[f64;N]], output: &mut [[f64;N]], scratch: &mut [[f64;N]],
        budget: &mut ProductionBudget<'_>) -> Result<usize, TransferError> {
        if N == 0 || values.len() != self.source_count() { return Err(TransferError::InvalidInput("field_dimensions")); }
        capacity(self.target_count(),output.len())?; capacity(self.target_count(),scratch.len())?;
        charge(budget,&[self.source_count()+self.target_count()+self.entries.len(),N,3])?;
        if values.iter().flatten().any(|v| !v.is_finite()) { return Err(TransferError::InvalidInput("field_nonfinite")); }
        scratch[..self.target_count()].fill([0.0;N]);
        for e in self.entries { for k in 0..N { scratch[e.row as usize][k] += e.value*values[e.col as usize][k]; } }
        if scratch[..self.target_count()].iter().flatten().any(|v| !v.is_finite()) { return Err(TransferError::InvalidInput("field_overflow")); }
        budget.checkpoint(0)?;
        output[..self.target_count()].copy_from_slice(&scratch[..self.target_count()]); Ok(self.target_count())
    }

    /// Adjoint force transfer f_source = P^T f_target for this SAME forward
    /// displacement map, giving <Pu,f> = <u,P^Tf> in serial f64 arithmetic.
    pub fn transpose_forces(&self, values: &[[f64;3]], output: &mut [[f64;3]], scratch: &mut [[f64;3]],
        budget: &mut ProductionBudget<'_>) -> Result<usize, TransferError> {
        if self.rule != ScalarRule::Intensive { return Err(TransferError::Unsupported("force_requires_displacement_map")); }
        if values.len() != self.target_count() { return Err(TransferError::InvalidInput("force_dimensions")); }
        capacity(self.source_count(),output.len())?; capacity(self.source_count(),scratch.len())?;
        charge(budget,&[self.source_count()+self.target_count()+self.entries.len(),9])?;
        if values.iter().flatten().any(|v| !v.is_finite()) { return Err(TransferError::InvalidInput("force_nonfinite")); }
        scratch[..self.source_count()].fill([0.0;3]);
        for e in self.entries { for k in 0..3 { scratch[e.col as usize][k] += e.value*values[e.row as usize][k]; } }
        if scratch[..self.source_count()].iter().flatten().any(|v| !v.is_finite()) { return Err(TransferError::InvalidInput("force_overflow")); }
        budget.checkpoint(0)?;
        output[..self.source_count()].copy_from_slice(&scratch[..self.source_count()]); Ok(self.source_count())
    }

    pub fn categorical(&self, labels: &[u64], output: &mut [u64], scratch: &mut [u64], budget: &mut ProductionBudget<'_>) -> Result<usize, TransferError> {
        if self.rule != ScalarRule::Intensive || labels.len() != self.source_count() { return Err(TransferError::InvalidInput("categorical_map")); }
        capacity(self.target_count(),output.len())?; capacity(self.target_count(),scratch.len())?;
        charge(budget,&[self.entries.len()+self.target_count(),3])?;
        let mut previous = None;
        for e in self.entries {
            let r=e.row as usize; let value=labels[e.col as usize];
            if previous == Some(r) && scratch[r] != value { return Err(TransferError::Unsupported("categorical_conflict")); }
            scratch[r]=value; previous=Some(r);
        }
        budget.checkpoint(0)?; output[..self.target_count()].copy_from_slice(&scratch[..self.target_count()]); Ok(self.target_count())
    }

    /// Affine/rigid reproduction is a measured property of coordinates, NOT
    /// inferred from lineage or the row-sum check alone.
    pub fn check_affine_positions(&self, source: &[[f64;3]], target: &[[f64;3]], absolute_tolerance:f64,
        scratch:&mut [[f64;3]], budget:&mut ProductionBudget<'_>) -> Result<f64,TransferError> {
        tolerance(absolute_tolerance)?;
        if self.rule != ScalarRule::Intensive || source.len()!=self.source_count() || target.len()!=self.target_count() {
            return Err(TransferError::InvalidInput("affine_dimensions"));
        }
        capacity(self.target_count(),scratch.len())?;
        charge(budget,&[self.source_count()+self.target_count()+self.entries.len(),12])?;
        if source.iter().chain(target).flatten().any(|x| !x.is_finite()) { return Err(TransferError::InvalidInput("position_nonfinite")); }
        scratch[..self.target_count()].fill([0.0;3]);
        for e in self.entries { for k in 0..3 { scratch[e.row as usize][k]+=e.value*source[e.col as usize][k]; } }
        let mut max_error=0.0f64;
        for (a,b) in scratch[..self.target_count()].iter().zip(target) {
            let error=(a[0]-b[0]).hypot(a[1]-b[1]).hypot(a[2]-b[2]);
            if !error.is_finite() || error>absolute_tolerance { return Err(TransferError::Unsupported("affine_reproduction")); }
            max_error=max_error.max(error);
        }
        Ok(max_error)
    }
}

