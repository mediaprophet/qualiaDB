//! Staged collapse admission carrying an ACTUAL original-source interpolation
//! row per live original vertex slot. Lineage/compaction remains P06-owned.
//! UV corners retain their original corner identity or are explicitly deleted;
//! final corner compaction copies authored UVs, never averages across charts.
use super::*;
use super::super::ddg_operators::SparseEntry;
use super::super::primitives::Point3;
use super::super::production_contracts::{CollapseProposal,CollapseNeighborhood,TransferAdmission,HashNamespace};
use sha2::{Digest,Sha256};

#[derive(Debug,Default,Clone,Copy)]
pub struct TransferRow {start:usize,len:usize}
#[derive(Debug,Clone,Copy)]
pub struct PreparedCollapse {identity:ContentId,proposal:ContentId,count:usize,sequence:u64}

/// Caller-owned row segments have fixed capacity; failure cannot allocate or
/// grow them. Source positions/domains/stable IDs remain immutable for the run.
/// An adapter instance is single-driver state and is not concurrently writable.
pub struct CollapseTransfer<'a> {
    run:ContentId,source:ContentId,policy:ContentId,sequence:u64,
    original_positions:&'a [Point3],domains:&'a [ProtectedDomain],stable_ids:&'a [u64],
    rows:&'a mut [TransferRow],entries:&'a mut [SparseEntry],row_capacity:usize,
    original_faces:usize,position_tolerance:f64,
}
impl<'a> CollapseTransfer<'a> {
    pub fn identity(run:ContentId,source:ContentId,policy:ContentId,original_positions:&'a [Point3],
        domains:&'a [ProtectedDomain],stable_ids:&'a [u64],original_faces:usize,row_capacity:usize,
        rows:&'a mut [TransferRow],entries:&'a mut [SparseEntry],position_tolerance:f64,
        budget:&mut ProductionBudget<'_>)->Result<Self,TransferError>{
        let n=original_positions.len();let used=n.checked_mul(row_capacity).ok_or_else(capacity_overflow)?;
        if n==0 || n>50_000 || original_faces>100_000 || domains.len()!=n || stable_ids.len()!=n || row_capacity==0
            || !position_tolerance.is_finite() || position_tolerance<0.0 {return Err(TransferError::InvalidInput("collapse_transfer_dimensions"));}
        capacity(n,rows.len())?;capacity(used,entries.len())?;charge(budget,&[n,5])?;
        if original_positions.iter().any(|p|![p.x,p.y,p.z].iter().all(|x|x.is_finite())){return Err(TransferError::InvalidInput("collapse_source_position"));}
        // Stable-ID uniqueness is an upstream P06 input invariant, not inferred
        // from array order. Each actual proposal checks endpoint identities.
        budget.checkpoint(0)?;
        for i in 0..n {rows[i]=TransferRow{start:i*row_capacity,len:1};entries[i*row_capacity]=SparseEntry{row:i as u32,col:i as u32,value:1.0};}
        Ok(Self{run,source,policy,sequence:0,original_positions,domains,stable_ids,rows,entries,row_capacity,original_faces,position_tolerance})
    }
    pub fn committed_collapses(&self)->u64{self.sequence}
    pub fn run(&self)->ContentId{self.run}
    pub fn source(&self)->ContentId{self.source}
    pub fn policy(&self)->ContentId{self.policy}
    pub fn original_count(&self)->usize{self.original_positions.len()}
    pub fn stable_id(&self,vertex:usize)->Option<u64>{self.stable_ids.get(vertex).copied()}
    pub fn protected_domain(&self,vertex:usize)->Option<ProtectedDomain>{self.domains.get(vertex).copied()}
    pub fn original_support(&self,vertex:usize)->Option<&[SparseEntry]>{
        let r=*self.rows.get(vertex)?;Some(&self.entries[r.start..r.start+r.len])
    }
    /// Transfer-owned row/entry capacities added to the P06 plan. Original
    /// positions and stable IDs are already counted by the driver.
    fn storage_bytes(&self)->Result<usize,TransferError>{
        let mut n=0usize;
        for (count,size) in [(self.rows.len(),std::mem::size_of::<TransferRow>()),(self.entries.len(),std::mem::size_of::<SparseEntry>())]{
            n=n.checked_add(count.checked_mul(size).ok_or_else(capacity_overflow)?).ok_or_else(capacity_overflow)?;
        }Ok(n)
    }
    fn validate_proposal(&self,p:&CollapseProposal,n:CollapseNeighborhood<'_>)->Result<(),TransferError>{
        p.validate().map_err(|_|TransferError::InvalidInput("collapse_proposal"))?;
        if p.run!=self.run || p.source!=self.source || p.policy!=self.policy || p.sequence!=self.sequence {return Err(TransferError::IdentityMismatch);}
        if p.sequence==u64::MAX || p.topology_generation==u64::MAX{return Err(capacity_overflow());}
        let (a,b)=(p.removed_vertex as usize,p.kept_vertex as usize);
        if a>=self.original_positions.len() || b>=self.original_positions.len() || self.rows[a].len==0 || self.rows[b].len==0 {
            return Err(TransferError::InvalidInput("collapse_vertex"));
        }
        if p.stable_endpoint_ids!=[self.stable_ids[a],self.stable_ids[b]]{return Err(TransferError::IdentityMismatch);}
        if self.domains[a]!=self.domains[b]{return Err(TransferError::DomainCrossing { row:a, col:b });}
        if n.corners.len()!=n.faces.len().checked_mul(3).ok_or_else(capacity_overflow)?{return Err(TransferError::InvalidInput("collapse_corners"));}
        let mut previous=None;
        for (i,face) in n.faces.iter().enumerate(){
            if face.original_face as usize>=self.original_faces || previous.is_some_and(|f|f>=face.original_face)
                || !face.before.iter().any(|v|*v==p.removed_vertex || *v==p.kept_vertex)
                || face.before.iter().any(|v|*v as usize>=self.original_positions.len()){
                return Err(TransferError::InvalidInput("collapse_neighborhood"));
            }
            let after=face.before.map(|v|if v==p.removed_vertex{p.kept_vertex}else{v});
            let degenerate=after[0]==after[1] || after[1]==after[2] || after[0]==after[2];
            if face.after!=if degenerate{None}else{Some(after)}{return Err(TransferError::InvalidInput("collapse_face_delta"));}
            for k in 0..3 {
                let c=n.corners[i*3+k];
                if c.source.face!=face.original_face || c.source.slot!=k as u8 || c.target!=face.after.map(|_|c.source){
                    return Err(TransferError::Unsupported("collapse_corner_reassignment"));
                }
            }
            previous=Some(face.original_face);
        }Ok(())
    }
    fn merged(&self,p:&CollapseProposal,mut visit:impl FnMut(usize,SparseEntry)->Result<(),TransferError>)->Result<usize,TransferError>{
        let (a,b)=(p.removed_vertex as usize,p.kept_vertex as usize);
        let left=self.original_support(a).ok_or(TransferError::InvalidInput("collapse_row"))?;
        let right=self.original_support(b).ok_or(TransferError::InvalidInput("collapse_row"))?;
        let (mut i,mut j,mut count)=(0,0,0);let mut sum=0.0;let mut point=[0.0;3];
        while i<left.len() || j<right.len(){
            let col=if j==right.len() || (i<left.len() && left[i].col<right[j].col){left[i].col}else{right[j].col};
            let mut weight=0.0;
            if i<left.len() && left[i].col==col{weight+=(1.0-p.t_from_removed_to_kept)*left[i].value;i+=1;}
            if j<right.len() && right[j].col==col{weight+=p.t_from_removed_to_kept*right[j].value;j+=1;}
            if weight==0.0{continue;}
            if !weight.is_finite() || weight<0.0 {return Err(TransferError::InvalidInput("collapse_weight"));}
            if self.domains[col as usize]!=self.domains[b]{return Err(TransferError::DomainCrossing { row:b, col: col as usize });}
            if count==self.row_capacity{return Err(TransferError::Capacity { required: count.saturating_add(1), available: self.row_capacity });}
            let q=self.original_positions[col as usize];sum+=weight;
            for (k,value) in [q.x,q.y,q.z].iter().enumerate(){point[k]+=weight*value;}
            visit(count,SparseEntry{row:b as u32,col,value:weight})?;count+=1;
        }
        let q=[p.target.x,p.target.y,p.target.z];let error=(point[0]-q[0]).hypot(point[1]-q[1]).hypot(point[2]-q[2]);
        if count==0 || (sum-1.0).abs()>1e-10 || !error.is_finite() || error>self.position_tolerance{return Err(TransferError::Unsupported("collapse_affine_reproduction"));}
        Ok(count)
    }
    fn fingerprint(p:&CollapseProposal,n:CollapseNeighborhood<'_>,bytes:&[u8])->ContentId{
        let mut h=Sha256::new();h.update(b"qualia.collapse-transfer/v1");
        for id in [p.run,p.source,p.policy]{h.update(id.digest);}
        for v in [p.sequence,p.topology_generation,p.removed_vertex as u64,p.kept_vertex as u64,p.stable_endpoint_ids[0],p.stable_endpoint_ids[1],p.endpoint_generations[0],p.endpoint_generations[1]]{h.update(v.to_le_bytes());}
        for v in [p.target.x,p.target.y,p.target.z,p.t_from_removed_to_kept]{h.update(v.to_le_bytes());}
        h.update((n.faces.len() as u64).to_le_bytes());
        for f in n.faces{h.update(f.original_face.to_le_bytes());for v in f.before{h.update(v.to_le_bytes());}h.update([u8::from(f.after.is_some())]);if let Some(after)=f.after{for v in after{h.update(v.to_le_bytes());}}}
        h.update((n.corners.len() as u64).to_le_bytes());
        for c in n.corners{h.update(c.source.face.to_le_bytes());h.update([c.source.slot,u8::from(c.target.is_some())]);if let Some(t)=c.target{h.update(t.face.to_le_bytes());h.update([t.slot]);}}
        h.update(bytes);ContentId{namespace:HashNamespace::Sha256V1,digest:h.finalize().into()}
    }
    /// Export canonical sorted COO rows using the final P06 original-slot to
    /// compact target map. Dropped isolated live slots may be omitted. This is
    /// a numeric interpolation map, unlike vertex lineage or corner remaps.
    pub fn export_compacted(&self,compaction:&[u32],output:&mut [SparseEntry],budget:&mut ProductionBudget<'_>)->Result<usize,TransferError>{
        if compaction.len()!=self.original_positions.len(){return Err(TransferError::InvalidInput("transfer_compaction"));}
        charge(budget,&[self.rows.len().checked_add(self.entries.len()).ok_or_else(capacity_overflow)?,3])?;
        let mut count=0usize;let mut next=0u32;
        for (i,&target) in compaction.iter().enumerate(){if target==u32::MAX{continue;}
            if target!=next || self.rows[i].len==0{return Err(TransferError::InvalidInput("transfer_compaction_order"));}
            next=next.checked_add(1).ok_or_else(capacity_overflow)?;count=count.checked_add(self.rows[i].len).ok_or_else(capacity_overflow)?;
        }
        capacity(count,output.len())?;budget.checkpoint(0)?;
        let mut cursor=0;
        for (i,&target) in compaction.iter().enumerate(){if target==u32::MAX{continue;}
            for &e in self.original_support(i).expect("original slot"){output[cursor]=SparseEntry{row:target,..e};cursor+=1;}
        }Ok(count)
    }
}
fn put_entry(bytes:&mut[u8],i:usize,e:SparseEntry){let p=i*16;bytes[p..p+4].copy_from_slice(&e.row.to_le_bytes());bytes[p+4..p+8].copy_from_slice(&e.col.to_le_bytes());bytes[p+8..p+16].copy_from_slice(&e.value.to_le_bytes());}
fn get_entry(bytes:&[u8],i:usize)->SparseEntry{let p=i*16;SparseEntry{row:u32::from_le_bytes(bytes[p..p+4].try_into().expect("prepared range")),col:u32::from_le_bytes(bytes[p+4..p+8].try_into().expect("prepared range")),value:f64::from_le_bytes(bytes[p+8..p+16].try_into().expect("prepared range"))}}

impl TransferAdmission for CollapseTransfer<'_>{
    type Prepared=PreparedCollapse;
    fn additional_storage_bytes(&self)->Result<usize,ContractError>{self.storage_bytes().map_err(Into::into)}
    fn scratch_bytes(&self)->Result<usize,ContractError>{self.row_capacity.checked_mul(16).ok_or(ContractError::new(ReasonCode::CapacityExhausted,"transfer_scratch"))}
    fn work_bound(&self,faces:usize,corners:usize)->Result<u64,ContractError>{
        self.row_capacity.checked_mul(300).and_then(|n|faces.checked_mul(100).and_then(|f|n.checked_add(f)))
            .and_then(|n|corners.checked_mul(30).and_then(|c|n.checked_add(c))).and_then(|n|n.checked_add(1024))
            .and_then(|n|u64::try_from(n).ok()).ok_or(ContractError::new(ReasonCode::CapacityExhausted,"transfer_work"))
    }
    fn prepare(&self,p:&CollapseProposal,n:CollapseNeighborhood<'_>,scratch:&mut[u8])->Result<PreparedCollapse,ContractError>{
        self.validate_proposal(p,n)?;
        let need=self.scratch_bytes()?;capacity(need,scratch.len())?;
        let count=self.merged(p,|i,e|{put_entry(scratch,i,e);Ok(())})?;
        let identity=Self::fingerprint(p,n,&scratch[..count*16]);
        Ok(PreparedCollapse{identity,proposal:Self::fingerprint(p,n,&[]),count,sequence:self.sequence})
    }
    fn revalidate(&self,p:&CollapseProposal,n:CollapseNeighborhood<'_>,prepared:&PreparedCollapse,scratch:&[u8])->Result<(),ContractError>{
        self.validate_proposal(p,n)?;
        if prepared.sequence!=self.sequence || prepared.count>self.row_capacity || scratch.len()<prepared.count*16
            || prepared.proposal!=Self::fingerprint(p,n,&[]) || prepared.identity!=Self::fingerprint(p,n,&scratch[..prepared.count*16]){
            return Err(ContractError::new(ReasonCode::StaleIdentity,"prepared_transfer"));
        }
        let count=self.merged(p,|i,e|{if i>=prepared.count || get_entry(scratch,i)!=e{Err(TransferError::IdentityMismatch)}else{Ok(())}})?;
        if count!=prepared.count{return Err(ContractError::new(ReasonCode::StaleIdentity,"prepared_transfer_count"));}Ok(())
    }
    fn prepared_identity(&self,p:&PreparedCollapse)->ContentId{p.identity}
    /// Driver precondition: successful revalidate with unchanged proposal,
    /// neighborhood, scratch and adapter; all mesh/journal capacities reserved.
    /// This commit has no failure/cancellation/allocation point.
    fn commit(&mut self,p:&CollapseProposal,prepared:PreparedCollapse,scratch:&mut[u8]){
        let kept=p.kept_vertex as usize;let start=self.rows[kept].start;
        for i in 0..prepared.count{self.entries[start+i]=get_entry(scratch,i);}
        self.rows[kept].len=prepared.count;self.rows[p.removed_vertex as usize].len=0;self.sequence+=1;
    }
}
