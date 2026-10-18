//! SKL1: versioned rigid rigs, four-weight vertex skins and named motor clips.
//! Stable joint/clip IDs use the engine's q_hash identity domain. All records
//! are little endian; decoding validates the entire payload before writes.

mod binary;
mod validation;
use crate::render::skeletal::{Clip, Joint, Key, SkinWeight, Track};
pub use binary::{encode_skeletal_section_into, SkeletalSection};

pub const MAX_CLIPS: usize = 32;
pub const MAX_TRACKS: usize = 4096;
pub const HEADER_BYTES: usize = 32;
pub const JOINT_BYTES: usize = 144;
pub const CLIP_BYTES: usize = 32;
pub const TRACK_BYTES: usize = 8;
pub const KEY_BYTES: usize = 72;
pub const SKIN_BYTES: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletalSectionError {
    Header,
    CountLimit,
    Length,
    InvalidData,
    OutputTooSmall,
}

#[derive(Debug, Clone, Copy)]
pub struct ClipRecord {
    pub id: u64,
    pub duration: f64,
    pub first_track: u32,
    pub track_count: u32,
    pub looping: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub joints: usize,
    pub vertices: usize,
    pub clips: usize,
    pub tracks: usize,
    pub keys: usize,
}

impl Counts {
    /// `encoded` length.
    pub fn encoded_len(self) -> Option<usize> {
        use crate::render::skeletal::{MAX_JOINTS, MAX_KEYS, MAX_VERTICES};
        if self.joints == 0
            || self.joints > MAX_JOINTS
            || self.vertices == 0
            || self.vertices > MAX_VERTICES
            || self.clips == 0
            || self.clips > MAX_CLIPS
            || self.tracks == 0
            || self.tracks > MAX_TRACKS
            || self.keys == 0
            || self.keys > MAX_KEYS
        {
            return None;
        }
        Some(
            HEADER_BYTES
                + self.joints * JOINT_BYTES
                + self.clips * CLIP_BYTES
                + self.tracks * TRACK_BYTES
                + self.keys * KEY_BYTES
                + self.vertices * SKIN_BYTES,
        )
    }
}

pub struct SkeletalData<'a> {
    pub joints: &'a [Joint],
    pub joint_ids: &'a [u64],
    pub skin: &'a [SkinWeight],
    pub clips: &'a [ClipRecord],
    pub tracks: &'a [Track],
    pub keys: &'a [Key],
}

pub(crate) trait Records {
    fn counts(&self) -> Counts;
    fn joint(&self, i: usize) -> Joint;
    fn joint_id(&self, i: usize) -> u64;
    fn clip(&self, i: usize) -> ClipRecord;
    fn track(&self, i: usize) -> Track;
    fn key(&self, i: usize) -> Key;
    fn skin(&self, i: usize) -> SkinWeight;
}
impl Records for SkeletalData<'_> {
    fn counts(&self) -> Counts {
        Counts {
            joints: self.joints.len(),
            vertices: self.skin.len(),
            clips: self.clips.len(),
            tracks: self.tracks.len(),
            keys: self.keys.len(),
        }
    }
    fn joint(&self, i: usize) -> Joint {
        self.joints[i]
    }
    fn joint_id(&self, i: usize) -> u64 {
        self.joint_ids[i]
    }
    fn clip(&self, i: usize) -> ClipRecord {
        self.clips[i]
    }
    fn track(&self, i: usize) -> Track {
        self.tracks[i]
    }
    fn key(&self, i: usize) -> Key {
        self.keys[i]
    }
    fn skin(&self, i: usize) -> SkinWeight {
        self.skin[i]
    }
}

/// Build a runtime view after loading the section into caller-owned arrays.
pub fn clip_view<'a>(record: ClipRecord, tracks: &'a [Track], keys: &'a [Key]) -> Option<Clip<'a>> {
    let end = (record.first_track as usize).checked_add(record.track_count as usize)?;
    Some(Clip {
        duration: record.duration,
        looping: record.looping,
        tracks: tracks.get(record.first_track as usize..end)?,
        keys,
    })
}

#[cfg(test)]
mod tests;
