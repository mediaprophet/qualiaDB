//! Cold resident allocation and allocation-free visual playback lifecycle (`etalon_render::skeletal::player`).

use super::{Joint, Key, SkinWeight, Track};
use crate::container_10d::skeletal_section::SkeletalSection;
use crate::render::body_fit::AnatomyBodyFit;

pub use etalon_render::skeletal::player::{
    clip_view, BodyFitSource, BodyFitSpec, ClipRecord, PlaybackReceipt, PlayerError,
    SkeletalCounts, SkeletalPlayer, SkeletalRigDescriptor, SkeletalSectionSource,
};

impl SkeletalSectionSource for SkeletalSection<'_> {
    fn section_counts(&self) -> SkeletalCounts {
        let c = self.counts();
        SkeletalCounts {
            joints: c.joints,
            vertices: c.vertices,
            clips: c.clips,
            tracks: c.tracks,
            keys: c.keys,
            encoded_bytes: c.encoded_len(),
        }
    }

    fn decode_section_into(
        &self,
        joints: &mut [Joint],
        joint_ids: &mut [u64],
        skin: &mut [SkinWeight],
        clips: &mut [ClipRecord],
        tracks: &mut [Track],
        keys: &mut [Key],
    ) -> Result<(), PlayerError> {
        let c = self.counts();
        if joints.len() < c.joints
            || joint_ids.len() < c.joints
            || skin.len() < c.vertices
            || clips.len() < c.clips
            || tracks.len() < c.tracks
            || keys.len() < c.keys
        {
            return Err(PlayerError::InvalidInput);
        }
        for i in 0..c.joints {
            joints[i] = self.joint(i).ok_or(PlayerError::InvalidInput)?;
            joint_ids[i] = self.joint_id(i).ok_or(PlayerError::InvalidInput)?;
        }
        for (i, slot) in skin[..c.vertices].iter_mut().enumerate() {
            *slot = self.skin(i).ok_or(PlayerError::InvalidInput)?;
        }
        for (i, slot) in clips[..c.clips].iter_mut().enumerate() {
            let clip = self.clip(i).ok_or(PlayerError::InvalidInput)?;
            *slot = ClipRecord {
                id: clip.id,
                duration: clip.duration,
                first_track: clip.first_track,
                track_count: clip.track_count,
                looping: clip.looping,
            };
        }
        for (i, slot) in tracks[..c.tracks].iter_mut().enumerate() {
            *slot = self.track(i).ok_or(PlayerError::InvalidInput)?;
        }
        for (i, slot) in keys[..c.keys].iter_mut().enumerate() {
            *slot = self.key(i).ok_or(PlayerError::InvalidInput)?;
        }
        Ok(())
    }
}

impl BodyFitSource for AnatomyBodyFit {
    fn to_body_fit_spec(&self) -> BodyFitSpec {
        BodyFitSpec {
            stature_scale: self.stature_scale,
            torso_scale_y: self.torso_scale_y,
            leg_scale_y: self.leg_scale_y,
            arm_span_scale_x: self.arm_span_scale_x,
            shoulder_scale_x: self.shoulder_scale_x,
            chest_radial: self.chest_radial,
            waist_radial: self.waist_radial,
            hip_radial: self.hip_radial,
            pregnancy_abdomen: self.pregnancy_abdomen,
            pelvis_y_norm: self.pelvis_y_norm,
            waist_y_norm: self.waist_y_norm,
            chest_y_norm: self.chest_y_norm,
            shoulder_y_norm: self.shoulder_y_norm,
            hidden_keys_count: self.hidden_keys.len(),
            hidden_keys_bytes: self.hidden_keys.iter().map(|s| s.len()).sum(),
            identity: self.identity,
        }
    }
}
