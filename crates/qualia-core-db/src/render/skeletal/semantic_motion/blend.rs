//! Bind a Q42 blend-space identity to the clip table of one resident SKL1 rig.

use crate::{container_10d::skeletal_container::read_skeletal_section, q_hash};
use crate::etalon_render::skeletal::{BlendSample2D, BlendSpace2D};

use super::{MacroChannel, MacroValue, MotionGraph, SemanticMotionError as Error, graph::index, rest_poses_agree};

const MAX_POINTS: usize = 16;
const MAX_COORDINATE: f64 = 1e6;

macro_rules! k {
    ($name:literal) => { q_hash(concat!("https://qualiadb.org/ns/kinematics#", $name)) };
}

const POINTS: [u64; MAX_POINTS] = [
    k!("blendPoint0"), k!("blendPoint1"), k!("blendPoint2"), k!("blendPoint3"),
    k!("blendPoint4"), k!("blendPoint5"), k!("blendPoint6"), k!("blendPoint7"),
    k!("blendPoint8"), k!("blendPoint9"), k!("blendPoint10"), k!("blendPoint11"),
    k!("blendPoint12"), k!("blendPoint13"), k!("blendPoint14"), k!("blendPoint15"),
];

pub struct PreparedClipBlend {
    semantic_id: u64,
    space: BlendSpace2D<MAX_POINTS, 32>,
}

impl PreparedClipBlend {
    /// `blendSpaceGroup` links the space to its actor group. The space names
    /// `blendPointCount` and the corresponding `blendPoint0` through
    /// `blendPoint15`. Each point has `blendPointX`, `blendPointY` and an
    /// `animationClip` IRI whose hash is already the resident SKL1 clip ID.
    pub fn bind(
        asset: &[u8], graph: &mut MotionGraph<'_, '_, '_>, group: u64, semantic_id: u64, rotations: &[u64],
    ) -> Result<Self, Error> {
        graph.check_budget(std::mem::size_of::<Self>())?;
        graph.iri(semantic_id)?;
        if graph.one(semantic_id, k!("blendSpaceGroup"))? != group {
            return Err(Error::UnboundTarget);
        }
        let count = index(graph.one(semantic_id, k!("blendPointCount"))?)?;
        if !(3..=MAX_POINTS).contains(&count) { return Err(Error::InvalidFact); }
        let rig = read_skeletal_section(asset).map_err(|_| Error::UnboundTarget)?;
        let mut space = BlendSpace2D::new();
        for &predicate in &POINTS[..count] {
            let point = graph.one(semantic_id, predicate)?;
            graph.iri(point)?;
            let [x, y] = graph.numeric_lanes(point, [k!("blendPointX"), k!("blendPointY")])?;
            let position = [coordinate(x)?, coordinate(y)?];
            if space.points[..space.point_count].iter().any(|other| other.pos == position) {
                return Err(Error::AmbiguousTarget);
            }
            let clip_hash = graph.one(point, k!("animationClip"))?;
            graph.iri(clip_hash)?;
            let (clip_index, clip) = (0..rig.counts().clips).filter_map(|i| rig.clip(i).map(|row| (i, row)))
                .find(|(_, row)| row.id == clip_hash).ok_or(Error::UnboundTarget)?;
            for t in clip.first_track..clip.first_track + clip.track_count {
                let track = rig.track(t as usize).ok_or(Error::InvalidFact)?;
                let joint = usize::from(track.joint);
                let joint_hash = rig.joint_id(joint).ok_or(Error::InvalidFact)?;
                if rotations.contains(&joint_hash) {
                    let rest = rig.joint(joint).ok_or(Error::InvalidFact)?.rest_local;
                    for k in track.first_key as usize..track.first_key as usize + track.key_count as usize {
                        let key = rig.key(k).ok_or(Error::InvalidFact)?;
                        if !rest_poses_agree(rest, key.local) { return Err(Error::InvalidFit); }
                    }
                }
            }
            if !space.add_point(position[0], position[1], u16::try_from(clip_index).map_err(|_| Error::InvalidFact)?) {
                return Err(Error::InvalidFact);
            }
        }
        let triangles = space.build_triangulation();
        let mut used = [false; MAX_POINTS];
        for triangle in &space.triangles[..triangles] {
            for i in triangle.indices { used[usize::from(i)] = true; }
        }
        if used[..count].iter().any(|&used| !used) { return Err(Error::InvalidFact); }
        Ok(Self { semantic_id, space })
    }

    pub fn sample(&self, values: &[Option<MacroValue>]) -> Result<BlendSample2D, Error> {
        let mut coordinates = [None; 2];
        for value in values.iter().flatten() {
            let axis = match value.channel {
                MacroChannel::BlendX => 0,
                MacroChannel::BlendY => 1,
                _ => continue,
            };
            if value.semantic_id != self.semantic_id || coordinates[axis].is_some() {
                return Err(Error::AmbiguousTarget);
            }
            coordinates[axis] = Some(coordinate(value.value)?);
        }
        let [Some(x), Some(y)] = coordinates else { return Err(Error::MissingFact); };
        Ok(self.space.sample_weights([x, y]))
    }
}

fn coordinate(value: f64) -> Result<f32, Error> {
    if !(-MAX_COORDINATE..=MAX_COORDINATE).contains(&value) { return Err(Error::InvalidFact); }
    Ok(value as f32)
}
