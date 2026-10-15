use super::*;
use crate::render::skeletal::Motor;

fn u16_at(b: &[u8], p: usize) -> u16 {
    u16::from_le_bytes(b[p..p + 2].try_into().unwrap())
}
fn u32_at(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}
fn u64_at(b: &[u8], p: usize) -> u64 {
    u64::from_le_bytes(b[p..p + 8].try_into().unwrap())
}
fn f64_at(b: &[u8], p: usize) -> f64 {
    f64::from_bits(u64_at(b, p))
}
fn motor_at(b: &[u8], p: usize) -> Motor {
    let a: [f64; 8] = std::array::from_fn(|i| f64_at(b, p + i * 8));
    Motor {
        r_w: a[0],
        r_x: a[1],
        r_y: a[2],
        r_z: a[3],
        d_w: a[4],
        d_x: a[5],
        d_y: a[6],
        d_z: a[7],
    }
}
fn write_motor(b: &mut [u8], p: usize, m: Motor) {
    for (i, v) in [m.r_w, m.r_x, m.r_y, m.r_z, m.d_w, m.d_x, m.d_y, m.d_z]
        .iter()
        .enumerate()
    {
        b[p + i * 8..p + i * 8 + 8].copy_from_slice(&v.to_le_bytes());
    }
}

#[derive(Clone, Copy)]
struct Offsets {
    clips: usize,
    tracks: usize,
    keys: usize,
    skin: usize,
}
impl Offsets {
    fn new(c: Counts) -> Self {
        let clips = HEADER_BYTES + c.joints * JOINT_BYTES;
        let tracks = clips + c.clips * CLIP_BYTES;
        let keys = tracks + c.tracks * TRACK_BYTES;
        Self {
            clips,
            tracks,
            keys,
            skin: keys + c.keys * KEY_BYTES,
        }
    }
}

/// Zero-copy validated record view. Private fields prevent unchecked construction.
pub struct SkeletalSection<'a> {
    bytes: &'a [u8],
    counts: Counts,
    offsets: Offsets,
}
impl<'a> SkeletalSection<'a> {
    pub fn parse(bytes: &'a [u8], expected_vertices: usize) -> Result<Self, SkeletalSectionError> {
        if bytes.len() < HEADER_BYTES
            || bytes[..4] != *b"SKL1"
            || u16_at(bytes, 4) != 1
            || u16_at(bytes, 6) != 0
            || u32_at(bytes, 28) != 0
        {
            return Err(SkeletalSectionError::Header);
        }
        let c = Counts {
            joints: u32_at(bytes, 8) as usize,
            vertices: u32_at(bytes, 12) as usize,
            clips: u32_at(bytes, 16) as usize,
            tracks: u32_at(bytes, 20) as usize,
            keys: u32_at(bytes, 24) as usize,
        };
        let len = c.encoded_len().ok_or(SkeletalSectionError::CountLimit)?;
        if bytes.len() != len || c.vertices != expected_vertices {
            return Err(SkeletalSectionError::Length);
        }
        let result = Self {
            bytes,
            counts: c,
            offsets: Offsets::new(c),
        };
        for i in 0..c.joints {
            let p = HEADER_BYTES + i * JOINT_BYTES;
            if bytes[p + 2..p + 8].iter().any(|b| *b != 0) {
                return Err(SkeletalSectionError::Header);
            }
        }
        for i in 0..c.clips {
            let p = result.offsets.clips + i * CLIP_BYTES;
            if u32_at(bytes, p + 24) > 1 || u32_at(bytes, p + 28) != 0 {
                return Err(SkeletalSectionError::Header);
            }
        }
        validation::validate(&result)?;
        Ok(result)
    }
    pub fn counts(&self) -> Counts {
        self.counts
    }
    pub fn joint(&self, i: usize) -> Option<Joint> {
        (i < self.counts.joints).then(|| Records::joint(self, i))
    }
    pub fn joint_id(&self, i: usize) -> Option<u64> {
        (i < self.counts.joints).then(|| Records::joint_id(self, i))
    }
    pub fn clip(&self, i: usize) -> Option<ClipRecord> {
        (i < self.counts.clips).then(|| Records::clip(self, i))
    }
    pub fn track(&self, i: usize) -> Option<Track> {
        (i < self.counts.tracks).then(|| Records::track(self, i))
    }
    pub fn key(&self, i: usize) -> Option<Key> {
        (i < self.counts.keys).then(|| Records::key(self, i))
    }
    pub fn skin(&self, i: usize) -> Option<SkinWeight> {
        (i < self.counts.vertices).then(|| Records::skin(self, i))
    }

    pub fn decode_into(
        &self,
        joints: &mut [Joint],
        ids: &mut [u64],
        skin: &mut [SkinWeight],
        clips: &mut [ClipRecord],
        tracks: &mut [Track],
        keys: &mut [Key],
    ) -> Result<Counts, SkeletalSectionError> {
        let c = self.counts;
        if joints.len() < c.joints
            || ids.len() < c.joints
            || skin.len() < c.vertices
            || clips.len() < c.clips
            || tracks.len() < c.tracks
            || keys.len() < c.keys
        {
            return Err(SkeletalSectionError::OutputTooSmall);
        }
        for i in 0..c.joints {
            joints[i] = Records::joint(self, i);
            ids[i] = Records::joint_id(self, i);
        }
        for (i, s) in skin[..c.vertices].iter_mut().enumerate() {
            *s = Records::skin(self, i);
        }
        for (i, s) in clips[..c.clips].iter_mut().enumerate() {
            *s = Records::clip(self, i);
        }
        for (i, s) in tracks[..c.tracks].iter_mut().enumerate() {
            *s = Records::track(self, i);
        }
        for (i, s) in keys[..c.keys].iter_mut().enumerate() {
            *s = Records::key(self, i);
        }
        Ok(c)
    }
}
impl Records for SkeletalSection<'_> {
    fn counts(&self) -> Counts {
        self.counts
    }
    fn joint(&self, i: usize) -> Joint {
        let p = HEADER_BYTES + i * JOINT_BYTES;
        let parent = u16_at(self.bytes, p);
        Joint {
            parent: (parent != u16::MAX).then_some(parent),
            rest_local: motor_at(self.bytes, p + 16),
            inverse_bind: motor_at(self.bytes, p + 80),
        }
    }
    fn joint_id(&self, i: usize) -> u64 {
        u64_at(self.bytes, HEADER_BYTES + i * JOINT_BYTES + 8)
    }
    fn clip(&self, i: usize) -> ClipRecord {
        let p = self.offsets.clips + i * CLIP_BYTES;
        let b = self.bytes;
        ClipRecord {
            id: u64_at(b, p),
            duration: f64_at(b, p + 8),
            first_track: u32_at(b, p + 16),
            track_count: u32_at(b, p + 20),
            looping: u32_at(b, p + 24) == 1,
        }
    }
    fn track(&self, i: usize) -> Track {
        let p = self.offsets.tracks + i * TRACK_BYTES;
        Track {
            joint: u16_at(self.bytes, p),
            key_count: u16_at(self.bytes, p + 2),
            first_key: u32_at(self.bytes, p + 4),
        }
    }
    fn key(&self, i: usize) -> Key {
        let p = self.offsets.keys + i * KEY_BYTES;
        Key {
            time: f64_at(self.bytes, p),
            local: motor_at(self.bytes, p + 8),
        }
    }
    fn skin(&self, i: usize) -> SkinWeight {
        let p = self.offsets.skin + i * SKIN_BYTES;
        SkinWeight {
            joints: std::array::from_fn(|j| u16_at(self.bytes, p + j * 2)),
            weights: std::array::from_fn(|j| f32::from_bits(u32_at(self.bytes, p + 8 + j * 4))),
        }
    }
}

pub fn encode_skeletal_section_into(
    data: &SkeletalData<'_>,
    out: &mut [u8],
) -> Result<usize, SkeletalSectionError> {
    let c = data.counts();
    let len = c.encoded_len().ok_or(SkeletalSectionError::CountLimit)?;
    if data.joint_ids.len() != c.joints {
        return Err(SkeletalSectionError::InvalidData);
    }
    validation::validate(data)?;
    if out.len() < len {
        return Err(SkeletalSectionError::OutputTooSmall);
    }
    let o = Offsets::new(c);
    out[..len].fill(0);
    out[..4].copy_from_slice(b"SKL1");
    out[4..6].copy_from_slice(&1u16.to_le_bytes());
    for (i, count) in [c.joints, c.vertices, c.clips, c.tracks, c.keys]
        .iter()
        .enumerate()
    {
        out[8 + i * 4..12 + i * 4].copy_from_slice(&(*count as u32).to_le_bytes());
    }
    for (i, j) in data.joints.iter().enumerate() {
        let p = HEADER_BYTES + i * JOINT_BYTES;
        out[p..p + 2].copy_from_slice(&j.parent.unwrap_or(u16::MAX).to_le_bytes());
        out[p + 8..p + 16].copy_from_slice(&data.joint_ids[i].to_le_bytes());
        write_motor(out, p + 16, j.rest_local);
        write_motor(out, p + 80, j.inverse_bind);
    }
    for (i, clip) in data.clips.iter().enumerate() {
        let p = o.clips + i * CLIP_BYTES;
        out[p..p + 8].copy_from_slice(&clip.id.to_le_bytes());
        out[p + 8..p + 16].copy_from_slice(&clip.duration.to_le_bytes());
        out[p + 16..p + 20].copy_from_slice(&clip.first_track.to_le_bytes());
        out[p + 20..p + 24].copy_from_slice(&clip.track_count.to_le_bytes());
        out[p + 24..p + 28].copy_from_slice(&(clip.looping as u32).to_le_bytes());
    }
    for (i, t) in data.tracks.iter().enumerate() {
        let p = o.tracks + i * TRACK_BYTES;
        out[p..p + 2].copy_from_slice(&t.joint.to_le_bytes());
        out[p + 2..p + 4].copy_from_slice(&t.key_count.to_le_bytes());
        out[p + 4..p + 8].copy_from_slice(&t.first_key.to_le_bytes());
    }
    for (i, k) in data.keys.iter().enumerate() {
        let p = o.keys + i * KEY_BYTES;
        out[p..p + 8].copy_from_slice(&k.time.to_le_bytes());
        write_motor(out, p + 8, k.local);
    }
    for (i, s) in data.skin.iter().enumerate() {
        let p = o.skin + i * SKIN_BYTES;
        for j in 0..4 {
            out[p + j * 2..p + j * 2 + 2].copy_from_slice(&s.joints[j].to_le_bytes());
            out[p + 8 + j * 4..p + 12 + j * 4].copy_from_slice(&s.weights[j].to_le_bytes());
        }
    }
    Ok(len)
}
