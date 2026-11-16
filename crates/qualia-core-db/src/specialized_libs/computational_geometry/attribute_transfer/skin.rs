use super::*;
use crate::render::skeletal::SkinWeight;

impl SparseTransfer<'_> {
    /// Exact union of joint influences, normalized after interpolation. Existing
    /// SKL1/SKL2 four-weight representation is retained. A union requiring five
    /// influences fails, never truncates an animation influence silently.
    pub fn skin_weights(&self,input:&[SkinWeight],joint_count:usize,output:&mut [SkinWeight],scratch:&mut [SkinWeight],
        budget:&mut ProductionBudget<'_>)->Result<usize,TransferError>{
        if self.rule()!=ScalarRule::Intensive || input.len()!=self.source_count() || joint_count==0 || joint_count>u16::MAX as usize+1 {
            return Err(TransferError::InvalidInput("skin_dimensions"));
        }
        capacity(self.target_count(),output.len())?;capacity(self.target_count(),scratch.len())?;
        charge(budget,&[input.len().checked_add(self.entries.len()).and_then(|n|n.checked_add(self.target_count())).ok_or_else(capacity_overflow)?,50])?;
        for skin in input {
            if skin.weights.iter().any(|x|!x.is_finite() || *x<0.0 || *x>1.0) || (skin.weights.iter().sum::<f32>()-1.0).abs()>1e-5
                || skin.joints.iter().any(|j|*j as usize>=joint_count) {return Err(TransferError::InvalidInput("skin_influence"));}
            for i in 0..4 {for j in 0..i {if skin.weights[i]>0.0 && skin.weights[j]>0.0 && skin.joints[i]==skin.joints[j] {
                return Err(TransferError::InvalidInput("duplicate_skin_joint"));
            }}}
        }
        let mut cursor=0;
        for (row,slot) in scratch[..self.target_count()].iter_mut().enumerate(){
            let mut joints=[0u16;4];let mut weights=[0.0f64;4];let mut used=0;
            while cursor<self.entries.len() && self.entries[cursor].row as usize==row {
                let e=self.entries[cursor];let skin=input[e.col as usize];
                for i in 0..4 {
                    if skin.weights[i]==0.0 {continue;}
                    let joint=skin.joints[i];
                    let k=match joints[..used].iter().position(|j|*j==joint){Some(k)=>k,None=>{
                        if used==4{return Err(TransferError::Unsupported("more_than_four_skin_influences"));}
                        joints[used]=joint;used+=1;used-1
                    }};
                    weights[k]+=e.value*skin.weights[i] as f64;
                }
                cursor+=1;
            }
            let sum=weights.iter().sum::<f64>();
            if !sum.is_finite() || sum<=0.0 {return Err(TransferError::InvalidInput("skin_partition"));}
            for i in 1..used {let mut j=i;while j>0 && joints[j]<joints[j-1] {joints.swap(j,j-1);weights.swap(j,j-1);j-=1;}}
            *slot=SkinWeight {joints,weights:weights.map(|w|(w/sum) as f32)};
        }
        budget.checkpoint(0)?;output[..self.target_count()].copy_from_slice(&scratch[..self.target_count()]);Ok(self.target_count())
    }
}
