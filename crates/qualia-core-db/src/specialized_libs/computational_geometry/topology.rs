use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

pub const INVALID_INDEX: u32 = u32::MAX;

/// One directed edge in a triangle half-edge graph.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
pub struct HalfEdge {
    pub origin: u32,
    pub twin: u32,
    pub next: u32,
    pub face: u32,
}

impl Default for HalfEdge {
    fn default() -> Self {
        Self {
            origin: INVALID_INDEX,
            twin: INVALID_INDEX,
            next: INVALID_INDEX,
            face: INVALID_INDEX,
        }
    }
}

/// Caller-owned open-addressing slot used while constructing twins.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
pub struct EdgeSlot {
    pub key: u64,
    pub half_edge: u32,
    pub _reserved: u32,
}

impl Default for EdgeSlot {
    fn default() -> Self {
        Self {
            key: 0,
            half_edge: INVALID_INDEX,
            _reserved: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySummary {
    pub vertex_count: u32,
    pub face_count: u32,
    pub half_edge_count: u32,
    pub boundary_half_edges: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyError {
    TooManyFaces,
    HalfEdgeBufferTooSmall { required: usize },
    SlotBufferTooSmall { required: usize },
    VertexOutOfRange { face: usize, vertex: u32 },
    DegenerateFace { face: usize },
    DuplicateDirectedEdge { from: u32, to: u32 },
    NonManifoldEdge { from: u32, to: u32 },
}

#[inline]
pub fn required_edge_slots(triangle_count: usize) -> usize {
    triangle_count
        .saturating_mul(6)
        .max(1)
        .checked_next_power_of_two()
        .unwrap_or(usize::MAX)
}

#[inline]
fn edge_key(from: u32, to: u32) -> u64 {
    ((from as u64) << 32) | to as u64
}

#[inline]
fn hash_key(key: u64) -> usize {
    // SplitMix64 finalizer: fast, deterministic, and strong on sequential IDs.
    let mut x = key;
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
    (x ^ (x >> 31)) as usize
}

fn find_slot(slots: &[EdgeSlot], key: u64) -> Option<u32> {
    let mask = slots.len() - 1;
    let mut at = hash_key(key) & mask;
    for _ in 0..slots.len() {
        let slot = slots[at];
        if slot.half_edge == INVALID_INDEX {
            return None;
        }
        if slot.key == key {
            return Some(slot.half_edge);
        }
        at = (at + 1) & mask;
    }
    None
}

fn insert_slot(
    slots: &mut [EdgeSlot],
    key: u64,
    half_edge: u32,
    from: u32,
    to: u32,
) -> Result<(), TopologyError> {
    let mask = slots.len() - 1;
    let mut at = hash_key(key) & mask;
    for _ in 0..slots.len() {
        if slots[at].half_edge == INVALID_INDEX {
            slots[at] = EdgeSlot {
                key,
                half_edge,
                _reserved: 0,
            };
            return Ok(());
        }
        if slots[at].key == key {
            return Err(TopologyError::DuplicateDirectedEdge { from, to });
        }
        at = (at + 1) & mask;
    }
    Err(TopologyError::SlotBufferTooSmall {
        required: slots.len().saturating_mul(2),
    })
}

/// Build a half-edge graph from triangle indices without heap allocation.
///
/// The output requires `3 * triangles.len()` entries. `slots` requires
/// [`required_edge_slots(triangles.len())`] entries and is cleared by this
/// function. Oppositely directed edges become twins; unmatched edges are the
/// boundary. Duplicate directions and edges with more than two incident faces
/// fail closed as non-manifold input.
pub fn build_triangle_half_edges(
    vertex_count: u32,
    triangles: &[[u32; 3]],
    out: &mut [HalfEdge],
    slots: &mut [EdgeSlot],
) -> Result<TopologySummary, TopologyError> {
    if triangles.len() > (u32::MAX as usize) / 3 {
        return Err(TopologyError::TooManyFaces);
    }
    let edge_count = triangles.len() * 3;
    if out.len() < edge_count {
        return Err(TopologyError::HalfEdgeBufferTooSmall {
            required: edge_count,
        });
    }
    let required_slots = required_edge_slots(triangles.len());
    if slots.len() < required_slots || !slots.len().is_power_of_two() {
        return Err(TopologyError::SlotBufferTooSmall {
            required: required_slots,
        });
    }
    for slot in slots.iter_mut() {
        *slot = EdgeSlot::default();
    }

    for (face, triangle) in triangles.iter().copied().enumerate() {
        if triangle[0] == triangle[1] || triangle[1] == triangle[2] || triangle[2] == triangle[0] {
            return Err(TopologyError::DegenerateFace { face });
        }
        for &vertex in &triangle {
            if vertex >= vertex_count {
                return Err(TopologyError::VertexOutOfRange { face, vertex });
            }
        }

        let base = face * 3;
        for local in 0..3 {
            let from = triangle[local];
            let to = triangle[(local + 1) % 3];
            let current = (base + local) as u32;
            out[base + local] = HalfEdge {
                origin: from,
                twin: INVALID_INDEX,
                next: (base + (local + 1) % 3) as u32,
                face: face as u32,
            };

            if let Some(reverse) = find_slot(slots, edge_key(to, from)) {
                if out[reverse as usize].twin != INVALID_INDEX {
                    return Err(TopologyError::NonManifoldEdge { from, to });
                }
                out[base + local].twin = reverse;
                out[reverse as usize].twin = current;
            }
            insert_slot(slots, edge_key(from, to), current, from, to)?;
        }
    }

    let boundary_half_edges = out[..edge_count]
        .iter()
        .filter(|edge| edge.twin == INVALID_INDEX)
        .count() as u32;
    Ok(TopologySummary {
        vertex_count,
        face_count: triangles.len() as u32,
        half_edge_count: edge_count as u32,
        boundary_half_edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_triangles_share_twinned_edge() {
        let triangles = [[0, 1, 2], [2, 1, 3]];
        let mut edges = [HalfEdge::default(); 6];
        let mut slots = [EdgeSlot::default(); 16];
        let summary = build_triangle_half_edges(4, &triangles, &mut edges, &mut slots).unwrap();
        assert_eq!(summary.boundary_half_edges, 4);
        assert_eq!(edges[1].twin, 3);
        assert_eq!(edges[3].twin, 1);
        assert_eq!(edges[0].next, 1);
        assert_eq!(edges[2].next, 0);
    }

    #[test]
    fn rejects_duplicate_oriented_edge() {
        let triangles = [[0, 1, 2], [0, 1, 3]];
        let mut edges = [HalfEdge::default(); 6];
        let mut slots = [EdgeSlot::default(); 16];
        assert_eq!(
            build_triangle_half_edges(4, &triangles, &mut edges, &mut slots),
            Err(TopologyError::DuplicateDirectedEdge { from: 0, to: 1 })
        );
    }

    #[test]
    fn reports_fixed_buffer_requirements() {
        let triangles = [[0, 1, 2]];
        let mut edges = [HalfEdge::default(); 2];
        let mut slots = [EdgeSlot::default(); 8];
        assert_eq!(
            build_triangle_half_edges(3, &triangles, &mut edges, &mut slots),
            Err(TopologyError::HalfEdgeBufferTooSmall { required: 3 })
        );
    }
}
