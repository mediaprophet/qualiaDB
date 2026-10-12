//! Cold resident allocation and allocation-free visual playback lifecycle.
use super::*;
use crate::container_10d::skeletal_section::{SkeletalSection,ClipRecord,clip_view};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PlayerError { InvalidInput, BudgetExceeded, UnknownClip, Kernel(SkeletalError), Tangents }
impl From<SkeletalError> for PlayerError {fn from(e:SkeletalError)->Self {Self::Kernel(e)}}

#[derive(Debug,Clone,Copy,PartialEq)]
pub struct PlaybackReceipt {pub clip_id:u64,pub time:f64,pub vertices:usize,pub joints:usize,pub reduced_motion:bool}

/// Buffers are allocated once on validated scene admission, never on advance.
/// Geometry stays in the authored bind frame; output can receive a uniform
/// orbit transform without altering the rig or losing attachment alignment.
pub struct SkeletalPlayer {
    joints:Vec<Joint>,joint_ids:Vec<u64>,skin:Vec<SkinWeight>,clips:Vec<ClipRecord>,tracks:Vec<Track>,keys:Vec<Key>,
    bind_positions:Vec<[f32;3]>,bind_normals:Vec<[f32;3]>,uv:Vec<[f32;2]>,indices:Vec<u32>,
    positions:Vec<[f32;3]>,normals:Vec<[f32;3]>,tangents:Vec<[f32;4]>,tangent_scratch:Vec<[f32;6]>,
    local:Vec<Motor>,previous:Vec<Motor>,globals:Vec<Motor>,palette:Vec<Motor>,
    joint_deltas:Vec<Motor>,
    controlled_local:Vec<Motor>,
    clip:usize,time:f64,transition:f64,transition_duration:f64,root:Motor,
    origin:[f32;3],scale:f32,allocated_bytes:usize,
    body_fit:Option<(crate::render::body_fit::AnatomyBodyFit,[f32;3],[f32;3])>,
    fit_bytes:usize,
}
impl SkeletalPlayer {
    pub fn from_section(section:&SkeletalSection<'_>,positions:&[[f32;3]],normals:&[[f32;3]],uv:&[[f32;2]],indices:&[u32])
        ->Result<Self,PlayerError> {
        Self::from_section_with_budget(section,positions,normals,uv,indices,42*1024*1024)
    }
    pub fn from_section_with_budget(section:&SkeletalSection<'_>,positions:&[[f32;3]],normals:&[[f32;3]],uv:&[[f32;2]],indices:&[u32],resident_budget:usize)
        ->Result<Self,PlayerError> {
        let c=section.counts();let n=c.vertices;
        if positions.len()!=n||normals.len()!=n||uv.len()!=n||indices.is_empty()||indices.len()%3!=0
            ||indices.len()>MAX_VERTICES*6||indices.iter().any(|i|*i as usize>=n)
            ||positions.iter().flatten().chain(normals.iter().flatten()).chain(uv.iter().flatten()).any(|v|!v.is_finite()||v.abs()>1e8)
            ||normals.iter().any(|v|v.iter().map(|x|x*x).sum::<f32>()<1e-12) {return Err(PlayerError::InvalidInput);}
        let allocated_bytes=std::mem::size_of::<Self>()+n*120+indices.len()*4+c.joints*(std::mem::size_of::<Joint>()+8+6*64)
            +c.clips*std::mem::size_of::<ClipRecord>()+c.tracks*std::mem::size_of::<Track>()+c.keys*std::mem::size_of::<Key>();
        // Include borrowed authoring inputs and the encoded section during the
        // admission pass; refuse before allocating beyond the execution budget.
        if allocated_bytes>resident_budget||allocated_bytes+n*32+indices.len()*4+c.encoded_len().unwrap_or(usize::MAX)>42*1024*1024 {
            return Err(PlayerError::BudgetExceeded);
        }
        let identity=Motor::identity();
        let mut result=Self {
            joints:vec![Joint{parent:None,rest_local:identity,inverse_bind:identity};c.joints],joint_ids:vec![0;c.joints],
            skin:vec![SkinWeight{joints:[0;4],weights:[1.0,0.0,0.0,0.0]};n],
            clips:vec![ClipRecord{id:0,duration:1.0,first_track:0,track_count:0,looping:false};c.clips],
            tracks:vec![Track{joint:0,first_key:0,key_count:0};c.tracks],keys:vec![Key{time:0.0,local:identity};c.keys],
            bind_positions:positions.to_vec(),bind_normals:normals.to_vec(),uv:uv.to_vec(),indices:indices.to_vec(),
            positions:vec![[0.0;3];n],normals:vec![[0.0;3];n],tangents:vec![[0.0;4];n],tangent_scratch:vec![[0.0;6];n],
            local:vec![identity;c.joints],previous:vec![identity;c.joints],globals:vec![identity;c.joints],palette:vec![identity;c.joints],
            joint_deltas:vec![identity;c.joints],
            controlled_local:vec![identity;c.joints],
            clip:0,time:0.0,transition:0.0,transition_duration:0.0,root:identity,origin:[0.0;3],scale:1.0,allocated_bytes,body_fit:None,fit_bytes:0,
        };
        section.decode_into(&mut result.joints,&mut result.joint_ids,&mut result.skin,&mut result.clips,&mut result.tracks,&mut result.keys)
            .map_err(|_|PlayerError::InvalidInput)?;
        result.advance(0.0,false)?;
        Ok(result)
    }
    pub fn allocated_bytes(&self)->usize {self.allocated_bytes}
    pub fn positions(&self)->&[[f32;3]] {&self.positions}
    pub fn normals(&self)->&[[f32;3]] {&self.normals}
    pub fn tangents(&self)->&[[f32;4]] {&self.tangents}
    pub fn has_clip(&self,id:u64)->bool {self.clips.iter().any(|c|c.id==id)}
    pub fn set_body_fit(&mut self,fit:&crate::render::body_fit::AnatomyBodyFit,min:[f32;3],max:[f32;3])->Result<(),PlayerError> {
        if min.iter().chain(max.iter()).any(|v|!v.is_finite())|| (0..3).any(|a|min[a]>max[a])
            ||fit.hidden_keys.len()>128||fit.hidden_keys.iter().map(|s|s.len()).sum::<usize>()>4096
            ||[fit.stature_scale,fit.torso_scale_y,fit.leg_scale_y,fit.arm_span_scale_x,fit.shoulder_scale_x,
                fit.chest_radial,fit.waist_radial,fit.hip_radial,fit.pregnancy_abdomen,fit.pelvis_y_norm,
                fit.waist_y_norm,fit.chest_y_norm,fit.shoulder_y_norm].iter().any(|v|!v.is_finite()||v.abs()>100.0) {
            return Err(PlayerError::InvalidInput);
        }
        let bytes=if fit.identity {0}else{fit.hidden_keys.len()*std::mem::size_of::<String>()+fit.hidden_keys.iter().map(|s|s.len()).sum::<usize>()};
        let allocated=self.allocated_bytes-self.fit_bytes+bytes;
        if allocated>42*1024*1024 {return Err(PlayerError::BudgetExceeded);}
        self.body_fit=if fit.identity {None}else{Some((fit.clone(),min,max))};
        self.allocated_bytes=allocated;self.fit_bytes=bytes;Ok(())
    }
    pub fn set_output_frame(&mut self,origin:[f32;3],scale:f32)->Result<(),PlayerError> {
        if origin.iter().any(|v|!v.is_finite()||v.abs()>1e8)||!scale.is_finite()||scale<=0.0||scale>1e8 {return Err(PlayerError::InvalidInput);}
        self.origin=origin;self.scale=scale;Ok(())
    }
    pub fn set_root(&mut self,root:Motor)->Result<(),PlayerError> {
        if !valid_motor(&root) {return Err(PlayerError::InvalidInput);}self.root=root;Ok(())
    }
    pub fn has_joint(&self,id:u64)->bool {self.joint_ids.contains(&id)}
    /// Persistent local control delta, composed after the sampled clip pose.
    /// Wheels, steering and authored constraints use the same skinning path.
    pub fn set_joint_delta(&mut self,id:u64,delta:Motor)->Result<(),PlayerError> {
        if !valid_motor(&delta) {return Err(PlayerError::InvalidInput);}
        let joint=self.joint_ids.iter().position(|key|*key==id).ok_or(PlayerError::InvalidInput)?;
        self.joint_deltas[joint]=delta;Ok(())
    }
    pub fn select_clip(&mut self,id:u64,fade_seconds:f64)->Result<(),PlayerError> {
        if self.clips[self.clip].id==id {
            if !fade_seconds.is_finite()||!(0.0..=10.0).contains(&fade_seconds) {return Err(PlayerError::InvalidInput);}
            return Ok(());
        }
        self.restart_clip(id,fade_seconds)
    }
    pub fn restart_clip(&mut self,id:u64,fade_seconds:f64)->Result<(),PlayerError> {
        if !fade_seconds.is_finite()||!(0.0..=10.0).contains(&fade_seconds) {return Err(PlayerError::InvalidInput);}
        let clip=self.clips.iter().position(|c|c.id==id).ok_or(PlayerError::UnknownClip)?;
        self.previous.copy_from_slice(&self.local);self.clip=clip;self.time=0.0;
        self.transition=0.0;self.transition_duration=fade_seconds;Ok(())
    }
    /// Deterministic fixture/replay positioning, without a wall-clock source.
    pub fn seek(&mut self,time:f64)->Result<(),PlayerError> {
        if !time.is_finite()||time.abs()>1e12 {return Err(PlayerError::InvalidInput);}
        self.time=time;self.transition_duration=0.0;Ok(())
    }
    /// Samples and deforms from immutable bind data, avoiding accumulated drift.
    /// Reduced motion selects the authored first pose and disables transitions.
    pub fn advance(&mut self,dt_seconds:f64,reduced_motion:bool)->Result<PlaybackReceipt,PlayerError> {
        if !dt_seconds.is_finite()||!(0.0..=1.0).contains(&dt_seconds) {return Err(PlayerError::InvalidInput);}
        self.time+=dt_seconds;self.transition+=dt_seconds;
        let clip=self.clips[self.clip];
        if clip.looping {self.time=self.time.rem_euclid(clip.duration);}else {self.time=self.time.clamp(0.0,clip.duration);}
        let blend=if reduced_motion||self.transition_duration==0.0 {1.0} else {(self.transition/self.transition_duration).clamp(0.0,1.0)};
        let previous=(blend<1.0).then_some(self.previous.as_slice());
        sample_pose_into(&self.joints,clip_view(clip,&self.tracks,&self.keys).ok_or(PlayerError::InvalidInput)?,
            if reduced_motion {0.0}else{self.time},previous,blend,&mut self.local)?;
        for ((controlled,local),delta) in self.controlled_local.iter_mut().zip(&self.local).zip(&self.joint_deltas) {*controlled=local.mul(delta);}
        compose_palette_into(&self.joints,&self.controlled_local,self.root,&mut self.globals,&mut self.palette)?;
        skin_vertices_into(&self.bind_positions,&self.bind_normals,&self.skin,&self.palette,&mut self.positions,&mut self.normals)?;
        if let Some((fit,min,max))=&self.body_fit {
            fit.apply_in_place(&mut self.positions,*min,*max);
            crate::render::mesh_surface_frames::vertex_normals_from_indices_into(&self.positions,&self.indices,&mut self.normals)
                .map_err(|_|PlayerError::InvalidInput)?;
        }
        for point in &mut self.positions {for (axis,v) in point.iter_mut().enumerate() {*v=(*v-self.origin[axis])*self.scale;}}
        crate::render::mesh_tangent_frames::tangents_from_uvs_into(&self.positions,&self.uv,&self.normals,&self.indices,
            &mut self.tangents,&mut self.tangent_scratch).map_err(|_|PlayerError::Tangents)?;
        Ok(PlaybackReceipt{clip_id:clip.id,time:self.time,vertices:self.positions.len(),joints:self.joints.len(),reduced_motion})
    }
    /// Attachment globals are in authored world space, as are rig roots.
    pub fn attachment(&self,joint_id:u64)->Option<Motor> {
        self.joint_ids.iter().position(|id|*id==joint_id).map(|i|self.globals[i])
    }
}
