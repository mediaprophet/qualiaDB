use crate::render::skeletal::Motor;
use crate::render::camera::{orbit_forward,CameraState};
#[derive(Debug,Clone,Copy,PartialEq)]
pub enum MountError {InvalidInput}
#[derive(Debug,Clone,Copy)]
pub struct MountedCamera {pub eye:[f32;3],pub yaw:f32,pub pitch:f32,pub zoom:f32,pub target:[f32;3]}
/// Local angles are relative to the body's forward (-Z). Scene center/scale
/// must be the same normalization used by geometry upload and HUD projection.
pub fn mounted_camera(body:Motor,eye:[f64;3],local_angles:[f32;2],center:[f64;3],scale:f64)->Result<MountedCamera,MountError> {
    if eye.iter().chain(center.iter()).any(|v|!v.is_finite())||local_angles.iter().any(|v|!v.is_finite())
        ||!scale.is_finite()||scale<=0.0 {return Err(MountError::InvalidInput);}
    let world_eye=body.transform_point(eye);
    let direction=orbit_forward(local_angles[0],local_angles[1]);
    let aim=body.transform_point([eye[0]+direction[0] as f64,eye[1]+direction[1] as f64,eye[2]+direction[2] as f64]);
    let forward=[aim[0]-world_eye[0],aim[1]-world_eye[1],aim[2]-world_eye[2]];
    let length=forward.iter().map(|n|n*n).sum::<f64>().sqrt();
    if !length.is_finite()||length<1e-12 {return Err(MountError::InvalidInput);}
    let yaw=(-forward[0]).atan2(-forward[2]) as f32;
    let pitch=(-(forward[1]/length).clamp(-1.0,1.0).asin()) as f32;
    let state=CameraState::new(yaw,pitch,0.35).clamped();
    let normalized=std::array::from_fn(|i|((world_eye[i]-center[i])*scale) as f32);
    if normalized.iter().any(|v:&f32|!v.is_finite()) {return Err(MountError::InvalidInput);}
    let direction=orbit_forward(state.yaw,state.pitch);
    let target=std::array::from_fn(|i|normalized[i]+state.zoom*direction[i]);
    Ok(MountedCamera{eye:normalized,yaw:state.yaw,pitch:state.pitch,zoom:state.zoom,target})
}
