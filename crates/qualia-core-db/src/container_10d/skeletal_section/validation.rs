use super::{Records,SkeletalSectionError};
use crate::render::skeletal::{Motor,valid_motor,MAX_JOINTS};

pub(super) fn validate(data:&impl Records)->Result<(),SkeletalSectionError> {
    let counts=data.counts();
    counts.encoded_len().ok_or(SkeletalSectionError::CountLimit)?;
    let bad=SkeletalSectionError::InvalidData;
    let mut globals=[Motor::identity();MAX_JOINTS];
    for i in 0..counts.joints {
        let joint=data.joint(i);let id=data.joint_id(i);
        if id==0 || (0..i).any(|j|data.joint_id(j)==id)
            || joint.parent.is_some_and(|p|p as usize>=i)
            || !valid_motor(&joint.rest_local)||!valid_motor(&joint.inverse_bind) {return Err(bad);}
        globals[i]=match joint.parent {Some(p)=>globals[p as usize].mul(&joint.rest_local),None=>joint.rest_local}.normalize();
        let (rotation,translation)=globals[i].mul(&joint.inverse_bind).to_rotation_translation();
        if (rotation[0].abs()-1.0).abs()>1e-5 || translation.iter().any(|v|v.abs()>1e-5) {return Err(bad);}
    }
    let mut next_track=0usize;let mut next_key=0usize;
    for c in 0..counts.clips {
        let clip=data.clip(c);
        let end=(clip.first_track as usize).checked_add(clip.track_count as usize).ok_or(bad)?;
        if clip.id==0||(0..c).any(|i|data.clip(i).id==clip.id)
            || !clip.duration.is_finite()||clip.duration<=0.0||clip.duration>86400.0
            || clip.first_track as usize!=next_track||clip.track_count==0||end>counts.tracks {return Err(bad);}
        let mut seen=[false;MAX_JOINTS];
        for t in next_track..end {
            let track=data.track(t);let joint=track.joint as usize;
            let key_end=(track.first_key as usize).checked_add(track.key_count as usize).ok_or(bad)?;
            if joint>=counts.joints||seen[joint]||track.key_count==0
                ||track.first_key as usize!=next_key||key_end>counts.keys {return Err(bad);}
            seen[joint]=true;
            let mut previous=-1.0;
            for k in next_key..key_end {
                let key=data.key(k);
                if !key.time.is_finite()||key.time<0.0||key.time>clip.duration
                    ||key.time<=previous||!valid_motor(&key.local) {return Err(bad);}
                previous=key.time;
            }
            if clip.looping {
                let first=data.key(next_key);let last=data.key(key_end-1);
                let (a,ta)=first.local.to_rotation_translation();let (b,tb)=last.local.to_rotation_translation();
                if first.time!=0.0||last.time!=clip.duration
                    || (a.iter().zip(b).map(|(x,y)|x*y).sum::<f64>().abs()-1.0).abs()>1e-5
                    ||ta.iter().zip(tb).any(|(x,y)|(x-y).abs()>1e-5) {return Err(bad);}
            }
            next_key=key_end;
        }
        next_track=end;
    }
    if next_track!=counts.tracks||next_key!=counts.keys {return Err(bad);}
    for i in 0..counts.vertices {
        let skin=data.skin(i);
        if skin.weights.iter().any(|w|!w.is_finite()||*w<0.0||*w>1.0)
            || (skin.weights.iter().sum::<f32>()-1.0).abs()>1e-5
            ||skin.joints.iter().zip(skin.weights).any(|(j,w)|w>0.0&&*j as usize>=counts.joints) {return Err(bad);}
    }
    Ok(())
}
