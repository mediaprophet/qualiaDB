//! Zero-Heap Generational 10D Scene Graph Pool (`pool.rs`).
//!
//! Implements the Tier-1 (`HotZeroHeap`) generational node pool and intrusive sibling-tree
//! hierarchy for `QualiaDB` 10D entities:
//! - [`NodeHandle`]: 64-bit generational index (`index: u32`, `generation: u32`).
//! - [`SceneNodeRecord`]: `#[repr(C)]` 192-byte (3 cache-line) zero-heap scene record carrying:
//!   - 10D `Tensor10D` identity (`quin_token`, `tensor_q`, `tensor_v`, `tensor_w`, `deontic_lane`,
//!     `epistemic_q`, `alpha_salience`, `mu_carrier`, `sigma_spectral`),
//!   - Intrusive doubly-linked sibling tree (`parent`, `first_child`, `next_sibling`, `prev_sibling`),
//!   - PGA [`Motor`] local and world transforms (never $4\times 4$ matrices),
//!   - Change-detection ticks (`added_tick`, `changed_tick`),
//!   - Multi-representation `.10d` asset handles (`mesh_handle`, `proc_header_handle`, `eont_handle`,
//!     `material_id`, `rig_handle`, `collider_handle`, `light_handle`, `audio_emitter_handle`).
//! - [`NodePool`] and [`NodePoolSlice`]: $O(1)$ `spawn`, `despawn`, `get`, `get_mut`,
//!   `attach_child`, and `detach_from_parent` with zero heap allocations.

use bytemuck::{Pod, Zeroable};

use crate::render::pga::{
    blade4_to_quat, motor_mul, motor_reverse, motor_translate, quat_to_blade4,
    rotor_from_axis_angle, sandwich_point, Motor,
};

/// Sentinel generation value indicating an unallocated or invalid [`NodeHandle`].
pub const INVALID_GENERATION: u32 = 0;

/// Maximum supported node capacity for slice-backed caller arenas.
pub const MAX_SCENE_ARENA_NODES: usize = 16_384;

/// Generational handle referencing a [`SceneNodeRecord`] inside a [`NodePool`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Pod, Zeroable)]
pub struct NodeHandle {
    /// Zero-based slot index into the node array.
    pub index: u32,
    /// Monotonically increasing generation counter (`0 == INVALID_GENERATION`).
    pub generation: u32,
}

impl Default for NodeHandle {
    #[inline]
    fn default() -> Self {
        Self::NONE
    }
}

impl NodeHandle {
    /// Null / unassigned handle constant.
    pub const NONE: Self = Self {
        index: u32::MAX,
        generation: INVALID_GENERATION,
    };

    /// Creates a new handle from an index and non-zero generation.
    #[inline]
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }

    /// Returns `true` if this handle is `NONE` or has an invalid generation.
    #[inline]
    pub const fn is_none(&self) -> bool {
        self.generation == INVALID_GENERATION || self.index == u32::MAX
    }

    /// Returns `true` if this handle holds a valid generation.
    #[inline]
    pub const fn is_some(&self) -> bool {
        !self.is_none()
    }
}

/// Multi-modal representation kind of a scene node in the unified `.10d` / Holographic pipeline.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum NodeKind {
    /// Transform-only grouping node.
    #[default]
    Empty = 0,
    /// Standard boundary-representation (`.10d` B-Rep) triangle mesh.
    MeshBRep = 1,
    /// Analytical SDF proxy lens (`HolographicNodeGpu`) with fragment raymarching & N3 interference.
    HolographicSdf = 2,
    /// `.10d` `PROC1` mathematical procedural synthesis node.
    Procedural10d = 3,
    /// `.10d` `SURF` Gaussian covariance surfel cloud.
    SurfelCloud = 4,
    /// `.10d` `IMPO` hemi-octahedral depth-in-alpha impostor billboard.
    ImpostorOctahedral = 5,
    /// 4D/5D `EmfFieldCell` volumetric field slice.
    VolumetricEmf = 6,
    /// Punctual / analytical light source.
    Light = 7,
    /// Viewport / sensor camera node.
    Camera = 8,
    /// Holosonic continuous wavefield emitter node.
    AudioHolosonic = 9,
    /// Skeletal / kinematic joint node.
    Joint = 10,
}

impl NodeKind {
    /// Converts a raw `u8` discriminant into a [`NodeKind`].
    #[inline]
    pub const fn from_u8(raw: u8) -> Self {
        match raw {
            0 => Self::Empty,
            1 => Self::MeshBRep,
            2 => Self::HolographicSdf,
            3 => Self::Procedural10d,
            4 => Self::SurfelCloud,
            5 => Self::ImpostorOctahedral,
            6 => Self::VolumetricEmf,
            7 => Self::Light,
            8 => Self::Camera,
            9 => Self::AudioHolosonic,
            10 => Self::Joint,
            _ => Self::Empty,
        }
    }
}

/// Inherent PGA [`Motor`] ergonomic helpers for hierarchical scene graph and physics solvers.
impl Motor {
    /// Geometric product of two PGA motors: `self ⊗ rhs`.
    #[inline]
    pub fn motor_mul(&self, rhs: &Self) -> Self {
        motor_mul(*self, *rhs)
    }

    /// PGA reversion $\widetilde{M}$ (inverse for unit motors).
    #[inline]
    pub fn reverse(&self) -> Self {
        motor_reverse(*self)
    }

    /// Transforms a 3D Euclidean point via the PGA sandwich product $\mathbf{p}' = M \mathbf{p} \widetilde{M}$.
    #[inline]
    pub fn transform_point(&self, p: [f32; 3]) -> [f32; 3] {
        sandwich_point(*self, p)
    }

    /// Rotates a 3D direction vector using only the rotor part $R \mathbf{v} \widetilde{R}$.
    #[inline]
    pub fn transform_vector(&self, v: [f32; 3]) -> [f32; 3] {
        crate::render::pga::legacy_rotor_apply_vector(self.r, v)
    }

    /// Extracts the Euclidean world translation $[x, y, z]$ encoded by this motor.
    #[inline]
    pub fn translation(&self) -> [f32; 3] {
        sandwich_point(*self, [0.0, 0.0, 0.0])
    }

    /// Constructs a rigid motor from a translation vector and axis-angle rotation.
    #[inline]
    pub fn from_translation_axis_angle(translation: [f32; 3], axis: [f32; 3], angle_rad: f32) -> Self {
        let t = motor_translate(translation);
        let r = Self::from_rotor(rotor_from_axis_angle(axis, angle_rad));
        motor_mul(t, r)
    }

    /// Normalizes the motor so its rotor component has unit norm and its dual component is orthogonal.
    #[inline]
    pub fn normalize(&self) -> Self {
        let qr = blade4_to_quat(self.r);
        let qd = blade4_to_quat(self.d);
        let norm_sq = qr[0] * qr[0] + qr[1] * qr[1] + qr[2] * qr[2] + qr[3] * qr[3];
        if norm_sq < 1e-12 {
            return Self::identity();
        }
        let inv_norm = 1.0 / norm_sq.sqrt();
        let nr = [
            qr[0] * inv_norm,
            qr[1] * inv_norm,
            qr[2] * inv_norm,
            qr[3] * inv_norm,
        ];
        let dot = nr[0] * qd[0] + nr[1] * qd[1] + nr[2] * qd[2] + nr[3] * qd[3];
        let nd = [
            qd[0] * inv_norm - nr[0] * dot * inv_norm,
            qd[1] * inv_norm - nr[1] * dot * inv_norm,
            qd[2] * inv_norm - nr[2] * dot * inv_norm,
            qd[3] * inv_norm - nr[3] * dot * inv_norm,
        ];
        Self {
            r: quat_to_blade4(nr),
            d: quat_to_blade4(nd),
        }
    }

    /// Normalized linear interpolation (NLERP / chordal SCLERP approximation) between two motors
    /// with shortest-path hemisphere alignment.
    #[inline]
    pub fn sclerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let dot = self.r[0] * other.r[0]
            + self.r[1] * other.r[1]
            + self.r[2] * other.r[2]
            + self.r[3] * other.r[3];
        let sign = if dot < 0.0 { -1.0 } else { 1.0 };
        let one_minus_t = 1.0 - t;
        let blended = Self {
            r: [
                one_minus_t * self.r[0] + t * sign * other.r[0],
                one_minus_t * self.r[1] + t * sign * other.r[1],
                one_minus_t * self.r[2] + t * sign * other.r[2],
                one_minus_t * self.r[3] + t * sign * other.r[3],
            ],
            d: [
                one_minus_t * self.d[0] + t * sign * other.d[0],
                one_minus_t * self.d[1] + t * sign * other.d[1],
                one_minus_t * self.d[2] + t * sign * other.d[2],
                one_minus_t * self.d[3] + t * sign * other.d[3],
            ],
        };
        blended.normalize()
    }
}

/// Visibility bitflags for [`SceneNodeRecord::visibility_flags`].
pub const VISIBILITY_VISIBLE: u16 = 1 << 0;
pub const VISIBILITY_CAST_SHADOWS: u16 = 1 << 1;
pub const VISIBILITY_RECEIVE_SHADOWS: u16 = 1 << 2;
pub const VISIBILITY_RAYCAST_PICKABLE: u16 = 1 << 3;
pub const VISIBILITY_HOLOGRAPHIC_INTERFERENCE: u16 = 1 << 4;

/// 192-byte (3 cache-line) zero-heap scene graph node record.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneNodeRecord {
    // ── 10D Semantic & Deontic Identity (32 bytes) ───────────────────────────
    /// 60-bit `NQuin` entity token or bit-63 `did:q42` topological pointer.
    pub quin_token: u64,
    /// Epistemic `q` lane selector (`0 = Collapsed Ground Truth`, `>0 = Superposition Escrow`).
    pub tensor_q: u8,
    /// Topological `v`-band selector (`0 = Euclidean`, `1 = Cyclic`, `2 = Hyperbolic`, `3+ = SovereignClique`).
    pub tensor_v: u8,
    /// Domain bifurcation `w` lane selector.
    pub tensor_w: u8,
    /// Deontic routing lane (`0 = Passthrough`, `1 = Commons`, `2 = Bilateral`).
    pub deontic_lane: u8,
    /// Continuous epistemic certainty $q \in [0.0, 1.0]$.
    pub epistemic_q: f32,
    /// Continuous perceptual salience weight $\alpha \in [0.0, 1.0]$.
    pub alpha_salience: f32,
    /// In-band consent / DID carrier word $\mu$.
    pub mu_carrier: u32,
    /// Spectral signature coordinate $\sigma$ (normalized wavelength / cluster slot).
    pub sigma_spectral: f32,
    /// Node representation archetype (`MeshBRep`, `HolographicSdf`, `Procedural10d`, etc.).
    pub node_kind: NodeKind,
    /// Alignment padding bytes.
    pub _pad_kind: [u8; 3],

    // ── Intrusive Sibling-Tree Links (36 bytes) ──────────────────────────────
    /// Parent node handle (`NodeHandle::NONE` for root nodes).
    pub parent: NodeHandle,
    /// First child node handle (`NodeHandle::NONE` if leaf).
    pub first_child: NodeHandle,
    /// Next sibling node handle (`NodeHandle::NONE` if last child).
    pub next_sibling: NodeHandle,
    /// Previous sibling node handle (`NodeHandle::NONE` if first child).
    pub prev_sibling: NodeHandle,
    /// Topological depth from root (`0` for root nodes).
    pub hierarchy_depth: u16,
    /// Visibility and rendering participation flags (`VISIBILITY_*`).
    pub visibility_flags: u16,

    // ── PGA Motor Transforms & Non-Uniform Scale (88 bytes) ──────────────────
    /// Local transform relative to parent as a 3D PGA [`Motor`].
    pub local_motor: Motor,
    /// Local non-uniform scale `[sx, sy, sz]` (driven by `EONT1` ontogenetic allometry).
    pub local_scale: [f32; 3],
    /// World-space composed PGA [`Motor`] (updated by `propagate_transforms_zero_heap`).
    pub world_motor: Motor,
    /// World-space composed non-uniform scale `[wx, wy, wz]`.
    pub world_scale: [f32; 3],

    // ── Change-Detection Ticks & Typed Asset Handles (36 bytes) ──────────────
    /// Simulation/frame tick when this node was spawned.
    pub added_tick: u32,
    /// Simulation/frame tick when this node's local state or transform last changed.
    pub changed_tick: u32,
    /// Handle into `MeshSlabArenaGpu` (`u32::MAX` = none).
    pub mesh_handle: u32,
    /// Handle into `.10d` `PROC1` `ProceduralSynthesisHeader` table (`u16::MAX` = none).
    pub proc_header_handle: u16,
    /// Handle into `.10d` `EONT1` `EntityOntologyMiniHeader` table (`u16::MAX` = none).
    pub eont_handle: u16,
    /// Material slot index.
    pub material_id: u16,
    /// Skeletal rig palette handle (`u16::MAX` = none).
    pub rig_handle: u16,
    /// 3D physics collider handle (`u32::MAX` = none).
    pub collider_handle: u32,
    /// Punctual light source handle (`u16::MAX` = none).
    pub light_handle: u16,
    /// Holosonic audio emitter handle (`u16::MAX` = none).
    pub audio_emitter_handle: u16,
    /// 64-bit prefab property override bitmask (`PrefabDeltaMask`).
    pub modified_mask: u64,
}

const _: [(); 192] = [(); core::mem::size_of::<SceneNodeRecord>()];

impl Default for SceneNodeRecord {
    #[inline]
    fn default() -> Self {
        Self::new(0, NodeKind::Empty)
    }
}

impl SceneNodeRecord {
    /// Creates a new scene node record with identity PGA [`Motor`]s and unit scale.
    #[inline]
    pub const fn new(quin_token: u64, node_kind: NodeKind) -> Self {
        Self {
            quin_token,
            tensor_q: 0,
            tensor_v: 0,
            tensor_w: 0,
            deontic_lane: 0,
            epistemic_q: 1.0,
            alpha_salience: 1.0,
            mu_carrier: 0,
            sigma_spectral: 0.5,
            node_kind,
            _pad_kind: [0; 3],
            parent: NodeHandle::NONE,
            first_child: NodeHandle::NONE,
            next_sibling: NodeHandle::NONE,
            prev_sibling: NodeHandle::NONE,
            hierarchy_depth: 0,
            visibility_flags: VISIBILITY_VISIBLE
                | VISIBILITY_CAST_SHADOWS
                | VISIBILITY_RECEIVE_SHADOWS
                | VISIBILITY_RAYCAST_PICKABLE,
            local_motor: Motor::identity(),
            local_scale: [1.0, 1.0, 1.0],
            world_motor: Motor::identity(),
            world_scale: [1.0, 1.0, 1.0],
            added_tick: 0,
            changed_tick: 0,
            mesh_handle: u32::MAX,
            proc_header_handle: u16::MAX,
            eont_handle: u16::MAX,
            material_id: 0,
            rig_handle: u16::MAX,
            collider_handle: u32::MAX,
            light_handle: u16::MAX,
            audio_emitter_handle: u16::MAX,
            modified_mask: 0,
        }
    }

    /// Builder: set local PGA [`Motor`].
    #[inline]
    pub const fn with_local_motor(mut self, motor: Motor) -> Self {
        self.local_motor = motor;
        self
    }

    /// Builder: set local non-uniform scale.
    #[inline]
    pub const fn with_local_scale(mut self, scale: [f32; 3]) -> Self {
        self.local_scale = scale;
        self
    }

    /// Builder: set 10D topological band (`v`), domain (`w`), and deontic lane.
    #[inline]
    pub const fn with_tensor_lanes(
        mut self,
        tensor_q: u8,
        tensor_v: u8,
        tensor_w: u8,
        deontic_lane: u8,
    ) -> Self {
        self.tensor_q = tensor_q;
        self.tensor_v = tensor_v;
        self.tensor_w = tensor_w;
        self.deontic_lane = deontic_lane;
        self
    }
}

/// Fixed-capacity, zero-heap generational scene node pool.
///
/// Supports $O(1)$ `spawn`, `despawn`, `get`, `get_mut`, `attach_child`, and `detach_from_parent`.
#[derive(Clone, Debug)]
pub struct NodePool<const CAP: usize = 1024> {
    /// Contiguous node storage array.
    pub records: [SceneNodeRecord; CAP],
    /// Per-slot generation counters (`0` before first allocation; incremented on spawn/despawn).
    pub generations: [u32; CAP],
    /// Per-slot occupancy mask.
    pub active: [bool; CAP],
    /// LIFO stack of free slot indices.
    pub free_indices: [u32; CAP],
    /// Number of valid entries in `free_indices`.
    pub free_len: usize,
    /// Number of currently active nodes.
    pub active_count: usize,
    /// Current monotonically advancing change-detection tick.
    pub current_tick: u32,
}

impl<const CAP: usize> Default for NodePool<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> NodePool<CAP> {
    /// Creates an empty generational node pool with all `CAP` slots on the free list.
    pub fn new() -> Self {
        let mut free_indices = [0u32; CAP];
        let mut i = 0;
        while i < CAP {
            // Reverse order so slot 0 is popped first.
            free_indices[i] = (CAP - 1 - i) as u32;
            i += 1;
        }
        Self {
            records: [SceneNodeRecord::default(); CAP],
            generations: [INVALID_GENERATION; CAP],
            active: [false; CAP],
            free_indices,
            free_len: CAP,
            active_count: 0,
            current_tick: 1,
        }
    }

    /// Borrows this pool as a slice-backed [`NodePoolSlice`] view.
    #[inline]
    pub fn as_slice_view(&mut self) -> NodePoolSlice<'_> {
        NodePoolSlice {
            records: &mut self.records,
            generations: &mut self.generations,
            active: &mut self.active,
            free_indices: &mut self.free_indices,
            free_len: &mut self.free_len,
            active_count: &mut self.active_count,
            current_tick: &mut self.current_tick,
        }
    }

    /// Advances the pool's change-detection tick counter.
    #[inline]
    pub fn advance_tick(&mut self) -> u32 {
        self.current_tick = self.current_tick.wrapping_add(1).max(1);
        self.current_tick
    }

    /// Number of currently active scene nodes.
    #[inline]
    pub const fn len(&self) -> usize {
        self.active_count
    }

    /// Returns `true` if no nodes are active.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.active_count == 0
    }

    /// Maximum node capacity of this pool.
    #[inline]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Returns `true` if `handle` refers to a currently live node with matching generation.
    #[inline]
    pub fn is_valid(&self, handle: NodeHandle) -> bool {
        let idx = handle.index as usize;
        handle.is_some()
            && idx < CAP
            && self.active[idx]
            && self.generations[idx] == handle.generation
    }

    /// Spawns a new [`SceneNodeRecord`] in $O(1)$ time without heap allocation.
    #[inline]
    pub fn spawn(&mut self, record: SceneNodeRecord) -> Option<NodeHandle> {
        self.as_slice_view().spawn(record)
    }

    /// Despawns `handle` in $O(1)$ time, unlinking it from its parent and detaching direct children.
    #[inline]
    pub fn despawn(&mut self, handle: NodeHandle) -> bool {
        self.as_slice_view().despawn(handle)
    }

    /// Shared $O(1)$ lookup by [`NodeHandle`].
    #[inline]
    pub fn get(&self, handle: NodeHandle) -> Option<&SceneNodeRecord> {
        if !self.is_valid(handle) {
            return None;
        }
        Some(&self.records[handle.index as usize])
    }

    /// Mutable $O(1)$ lookup by [`NodeHandle`]; automatically stamps `changed_tick = current_tick`.
    #[inline]
    pub fn get_mut(&mut self, handle: NodeHandle) -> Option<&mut SceneNodeRecord> {
        if !self.is_valid(handle) {
            return None;
        }
        let idx = handle.index as usize;
        self.records[idx].changed_tick = self.current_tick;
        Some(&mut self.records[idx])
    }

    /// Attaches `child` to `parent` in $O(1)$ using the intrusive sibling doubly-linked list.
    #[inline]
    pub fn attach_child(&mut self, parent: NodeHandle, child: NodeHandle) -> bool {
        self.as_slice_view().attach_child(parent, child)
    }

    /// Detaches `child` from its current parent in $O(1)$ time.
    #[inline]
    pub fn detach_from_parent(&mut self, child: NodeHandle) -> bool {
        self.as_slice_view().detach_from_parent(child)
    }
}

/// Slice-backed view over caller-provided arena slices (supports up to [`MAX_SCENE_ARENA_NODES`] = 16,384+ nodes).
pub struct NodePoolSlice<'a> {
    pub records: &'a mut [SceneNodeRecord],
    pub generations: &'a mut [u32],
    pub active: &'a mut [bool],
    pub free_indices: &'a mut [u32],
    pub free_len: &'a mut usize,
    pub active_count: &'a mut usize,
    pub current_tick: &'a mut u32,
}

impl<'a> NodePoolSlice<'a> {
    /// Initializes caller-supplied slices as an empty generational node pool.
    pub fn init_arena(
        records: &'a mut [SceneNodeRecord],
        generations: &'a mut [u32],
        active: &'a mut [bool],
        free_indices: &'a mut [u32],
        free_len: &'a mut usize,
        active_count: &'a mut usize,
        current_tick: &'a mut u32,
    ) -> Self {
        let cap = records
            .len()
            .min(generations.len())
            .min(active.len())
            .min(free_indices.len());
        let mut i = 0;
        while i < cap {
            records[i] = SceneNodeRecord::default();
            generations[i] = INVALID_GENERATION;
            active[i] = false;
            free_indices[i] = (cap - 1 - i) as u32;
            i += 1;
        }
        *free_len = cap;
        *active_count = 0;
        if *current_tick == 0 {
            *current_tick = 1;
        }
        Self {
            records,
            generations,
            active,
            free_indices,
            free_len,
            active_count,
            current_tick,
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.records
            .len()
            .min(self.generations.len())
            .min(self.active.len())
            .min(self.free_indices.len())
    }

    #[inline]
    pub fn is_valid(&self, handle: NodeHandle) -> bool {
        let idx = handle.index as usize;
        handle.is_some()
            && idx < self.capacity()
            && self.active[idx]
            && self.generations[idx] == handle.generation
    }

    /// $O(1)$ node allocation from the free stack.
    pub fn spawn(&mut self, mut record: SceneNodeRecord) -> Option<NodeHandle> {
        if *self.free_len == 0 {
            return None;
        }
        *self.free_len -= 1;
        let slot = self.free_indices[*self.free_len] as usize;
        let next_gen = self.generations[slot].wrapping_add(1).max(1);
        self.generations[slot] = next_gen;
        self.active[slot] = true;
        *self.active_count += 1;

        record.parent = NodeHandle::NONE;
        record.first_child = NodeHandle::NONE;
        record.next_sibling = NodeHandle::NONE;
        record.prev_sibling = NodeHandle::NONE;
        record.hierarchy_depth = 0;
        record.added_tick = *self.current_tick;
        record.changed_tick = *self.current_tick;
        record.world_motor = record.local_motor;
        record.world_scale = record.local_scale;

        self.records[slot] = record;
        Some(NodeHandle::new(slot as u32, next_gen))
    }

    /// $O(1)$ node deallocation; detaches from parent and orphans direct children cleanly.
    pub fn despawn(&mut self, handle: NodeHandle) -> bool {
        if !self.is_valid(handle) {
            return false;
        }
        let _ = self.detach_from_parent(handle);

        let slot = handle.index as usize;
        let mut child = self.records[slot].first_child;
        let cap = self.capacity();
        let mut guard = 0usize;
        while child.is_some() && guard < cap {
            guard += 1;
            let c_idx = child.index as usize;
            if c_idx >= cap || !self.active[c_idx] {
                break;
            }
            let next = self.records[c_idx].next_sibling;
            self.records[c_idx].parent = NodeHandle::NONE;
            self.records[c_idx].prev_sibling = NodeHandle::NONE;
            self.records[c_idx].next_sibling = NodeHandle::NONE;
            self.records[c_idx].hierarchy_depth = 0;
            self.records[c_idx].changed_tick = *self.current_tick;
            child = next;
        }

        self.records[slot].first_child = NodeHandle::NONE;
        self.active[slot] = false;
        self.generations[slot] = self.generations[slot].wrapping_add(1).max(1);
        self.free_indices[*self.free_len] = slot as u32;
        *self.free_len += 1;
        *self.active_count = self.active_count.saturating_sub(1);
        true
    }

    #[inline]
    pub fn get(&self, handle: NodeHandle) -> Option<&SceneNodeRecord> {
        if !self.is_valid(handle) {
            return None;
        }
        Some(&self.records[handle.index as usize])
    }

    #[inline]
    pub fn get_mut(&mut self, handle: NodeHandle) -> Option<&mut SceneNodeRecord> {
        if !self.is_valid(handle) {
            return None;
        }
        let idx = handle.index as usize;
        self.records[idx].changed_tick = *self.current_tick;
        Some(&mut self.records[idx])
    }

    /// Detaches `child` from its current parent in $O(1)$ time.
    pub fn detach_from_parent(&mut self, child: NodeHandle) -> bool {
        if !self.is_valid(child) {
            return false;
        }
        let c_idx = child.index as usize;
        let parent = self.records[c_idx].parent;
        if parent.is_none() {
            return false;
        }
        let prev = self.records[c_idx].prev_sibling;
        let next = self.records[c_idx].next_sibling;

        if self.is_valid(parent) {
            let p_idx = parent.index as usize;
            if self.records[p_idx].first_child == child {
                self.records[p_idx].first_child = next;
            }
            self.records[p_idx].changed_tick = *self.current_tick;
        }

        if self.is_valid(prev) {
            self.records[prev.index as usize].next_sibling = next;
        }
        if self.is_valid(next) {
            self.records[next.index as usize].prev_sibling = prev;
        }

        self.records[c_idx].parent = NodeHandle::NONE;
        self.records[c_idx].prev_sibling = NodeHandle::NONE;
        self.records[c_idx].next_sibling = NodeHandle::NONE;
        self.records[c_idx].hierarchy_depth = 0;
        self.records[c_idx].changed_tick = *self.current_tick;
        true
    }

    /// Attaches `child` as the first child of `parent` in $O(1)$ time.
    pub fn attach_child(&mut self, parent: NodeHandle, child: NodeHandle) -> bool {
        if parent == child || !self.is_valid(parent) || !self.is_valid(child) {
            return false;
        }

        // Prevent cycles: walk up `parent`'s ancestors to verify `child` is not an ancestor.
        let cap = self.capacity();
        let mut cursor = parent;
        let mut steps = 0usize;
        while cursor.is_some() && steps < cap {
            if cursor == child {
                return false;
            }
            let idx = cursor.index as usize;
            if idx >= cap || !self.active[idx] {
                break;
            }
            cursor = self.records[idx].parent;
            steps += 1;
        }

        // Detach from any previous parent first.
        let _ = self.detach_from_parent(child);

        let p_idx = parent.index as usize;
        let c_idx = child.index as usize;
        let old_first = self.records[p_idx].first_child;

        if self.is_valid(old_first) {
            self.records[old_first.index as usize].prev_sibling = child;
        }

        self.records[c_idx].parent = parent;
        self.records[c_idx].prev_sibling = NodeHandle::NONE;
        self.records[c_idx].next_sibling = old_first;
        self.records[c_idx].hierarchy_depth = self.records[p_idx].hierarchy_depth.saturating_add(1);
        self.records[c_idx].changed_tick = *self.current_tick;

        self.records[p_idx].first_child = child;
        self.records[p_idx].changed_tick = *self.current_tick;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_pool_spawn_despawn_and_generational_safety() {
        let mut pool = NodePool::<16>::new();
        let h1 = pool
            .spawn(SceneNodeRecord::new(0x1111, NodeKind::MeshBRep))
            .unwrap();
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(h1).unwrap().quin_token, 0x1111);

        assert!(pool.despawn(h1));
        assert_eq!(pool.len(), 0);
        assert!(pool.get(h1).is_none());

        // Re-spawning reuses slot 0 with a strictly newer generation.
        let h2 = pool
            .spawn(SceneNodeRecord::new(0x2222, NodeKind::HolographicSdf))
            .unwrap();
        assert_eq!(h2.index, h1.index);
        assert!(h2.generation > h1.generation);
        assert!(pool.get(h1).is_none());
        assert_eq!(pool.get(h2).unwrap().quin_token, 0x2222);
    }

    #[test]
    fn intrusive_hierarchy_attach_and_detach_o1() {
        let mut pool = NodePool::<16>::new();
        let root = pool
            .spawn(SceneNodeRecord::new(1, NodeKind::Empty))
            .unwrap();
        let c1 = pool
            .spawn(SceneNodeRecord::new(2, NodeKind::MeshBRep))
            .unwrap();
        let c2 = pool
            .spawn(SceneNodeRecord::new(3, NodeKind::HolographicSdf))
            .unwrap();

        assert!(pool.attach_child(root, c1));
        assert!(pool.attach_child(root, c2));

        // c2 was attached second, so it is at the head of `first_child`.
        assert_eq!(pool.get(root).unwrap().first_child, c2);
        assert_eq!(pool.get(c2).unwrap().next_sibling, c1);
        assert_eq!(pool.get(c1).unwrap().prev_sibling, c2);
        assert_eq!(pool.get(c1).unwrap().hierarchy_depth, 1);

        // Prevent self/cycle attachment.
        assert!(!pool.attach_child(c1, root));

        // Detach c2 and verify c1 becomes `first_child`.
        assert!(pool.detach_from_parent(c2));
        assert_eq!(pool.get(root).unwrap().first_child, c1);
        assert_eq!(pool.get(c1).unwrap().prev_sibling, NodeHandle::NONE);
        assert_eq!(pool.get(c2).unwrap().parent, NodeHandle::NONE);
    }

    #[test]
    fn get_mut_stamps_changed_tick() {
        let mut pool = NodePool::<8>::new();
        let h = pool
            .spawn(SceneNodeRecord::new(42, NodeKind::Procedural10d))
            .unwrap();
        let initial_tick = pool.get(h).unwrap().changed_tick;

        pool.advance_tick();
        pool.get_mut(h).unwrap().local_scale = [2.0, 3.0, 2.0];
        assert!(pool.get(h).unwrap().changed_tick > initial_tick);
    }
}
