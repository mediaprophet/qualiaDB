use super::{Motor, SkeletalError, MAX_JOINTS, MAX_VERTICES, valid_motor};

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SkinWeight { pub joints: [u16;4], pub weights: [f32;4] }

/// Four-influence linear blend skinning of immutable bind geometry.
/// Positions and normals share the same palette. UVs, tangents and material
/// topology are unchanged; tangent reconstruction remains the renderer's job.
/// No partial writes on malformed influences or inadequate output buffers.
pub fn skin_vertices_into(positions: &[[f32;3]], normals: &[[f32;3]], skin: &[SkinWeight],
    palette: &[Motor], out_positions: &mut [[f32;3]], out_normals: &mut [[f32;3]])
    -> Result<usize,SkeletalError> {
    let count=positions.len();
    if count>MAX_VERTICES || palette.len()>MAX_JOINTS {return Err(SkeletalError::BudgetExceeded);}
    if count==0 || normals.len()!=count || skin.len()!=count || palette.is_empty()
        || palette.iter().any(|m|!valid_motor(m))
        || positions.iter().flatten().chain(normals.iter().flatten()).any(|v|!v.is_finite()||v.abs()>1e8)
        || normals.iter().any(|n|n.iter().map(|v|v*v).sum::<f32>()<1e-12)
        || skin.iter().any(|s|s.weights.iter().any(|w|!w.is_finite()||*w<0.0||*w>1.0)
            || (s.weights.iter().sum::<f32>()-1.0).abs()>1e-5
            || s.joints.iter().zip(s.weights).any(|(j,w)|w>0.0&&*j as usize>=palette.len())) {
        return Err(SkeletalError::InvalidInput);
    }
    if out_positions.len()<count || out_normals.len()<count {return Err(SkeletalError::OutputTooSmall);}
    // Fixed scratch: translation for each joint removes translation from the
    // normal transform without allocating or rebuilding matrices per vertex.
    let mut origins=[[0.0f64;3];MAX_JOINTS];
    for (i,motor) in palette.iter().enumerate() {origins[i]=motor.transform_point([0.0;3]);}
    for i in 0..count {
        let mut p=[0.0f64;3];let mut n=[0.0f64;3];
        for slot in 0..4 {
            let weight=skin[i].weights[slot] as f64;
            if weight==0.0 {continue;}
            let joint=skin[i].joints[slot] as usize;
            let vertex=palette[joint].transform_point(positions[i].map(f64::from));
            let normal=palette[joint].transform_point(normals[i].map(f64::from));
            for axis in 0..3 {p[axis]+=weight*vertex[axis];n[axis]+=weight*(normal[axis]-origins[joint][axis]);}
        }
        let length=n.iter().map(|v|v*v).sum::<f64>().sqrt();
        out_positions[i]=p.map(|v|v as f32);
        // Opposing joint rotations can cancel a linear normal; retain a valid
        // bind normal rather than emitting NaNs into GPU shading.
        out_normals[i]=if length>1e-12 {n.map(|v|(v/length) as f32)} else {
            let bind=normals[i];let length=bind.iter().map(|v|v*v).sum::<f32>().sqrt();
            bind.map(|v|v/length)
        };
    }
    Ok(count)
}
