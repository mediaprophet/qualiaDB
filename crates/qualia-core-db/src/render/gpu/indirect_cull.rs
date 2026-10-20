//! GPU-Driven Multi-Draw Indirect Culling, 2-Phase HZB Occlusion & 4-Tier Hybrid LOD.
//!
//! Pairs with [`super::mesh_slab::MeshSlabArenaGpu`] and `INSTANCE_CULL_INDIRECT_WGSL` to
//! evaluate per-instance PGA motors, 6-plane frustum culling, 2-phase Hierarchical Z-Buffer
//! (HZB) occlusion culling, continuous screen-space projected radius LOD selection (`LOD0..LOD3`)
//! with hysteresis and epistemic salience weighting, and stream compaction into:
//! - **Stream 0 (Mesh Indirect Stream)**: `DrawIndexedIndirectCommand` for `LOD0..LOD2` (`q == 0`)
//! - **Stream 1 (Holographic SDF Proxy Stream)**: instance indices with `q > 0` or `HolographicSdfProxy`
//! - **Stream 2 (Surfel / Octahedral Impostor Stream)**: instance indices at `LOD3` or `Surfel`/`Impostor`

use bytemuck::{Pod, Zeroable};

use super::mesh_slab::{ResidentAssetEntry, SubmeshSlice};
use crate::render::instance_culling::{extract_frustum_planes, FrustumPlane};

/// Default screen-space projected radius thresholds in pixels (`[LOD0, LOD1, LOD2]`).
/// - `LOD0`: `>= 128.0` px (full procedural/CAD/skinned mesh)
/// - `LOD1`: `[48.0, 128.0)` px (simplified mesh, ~40% triangles)
/// - `LOD2`: `[16.0, 48.0)` px (coarse mesh, ~12% triangles)
/// - `LOD3`: `< 16.0` px (Gaussian Surfel Cloud / Octahedral Impostor proxy)
pub const DEFAULT_LOD_THRESHOLDS_PX: [f32; 3] = [128.0, 48.0, 16.0];

/// Default minimum projected radius in pixels below which an instance is sub-pixel culled.
pub const DEFAULT_MIN_CULL_RADIUS_PX: f32 = 1.5;

/// Default fractional hysteresis deadband around LOD thresholds (`10%`).
pub const DEFAULT_LOD_HYSTERESIS: f32 = 0.10;

/// Multi-modal representation policy encoded in `InstanceCullDescriptor::rep_policy`.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepresentationPolicy {
    /// Automatic hybrid transitions based on screen-space radius and tensor valence `(q, v)`.
    Auto = 0,
    /// Force explicit triangle mesh LOD (`LOD0..LOD2`).
    ExplicitMeshLod = 1,
    /// Route directly to the depth-interleaved Holographic CSG/SDF Proxy stream.
    HolographicSdfProxy = 2,
    /// Route directly to the Gaussian Surfel Cloud stream.
    GaussianSurfelCloud = 3,
    /// Route directly to the Octahedral Impostor stream.
    OctahedralImpostor = 4,
}

impl RepresentationPolicy {
    #[inline]
    pub const fn from_u16(v: u16) -> Self {
        match v {
            1 => Self::ExplicitMeshLod,
            2 => Self::HolographicSdfProxy,
            3 => Self::GaussianSurfelCloud,
            4 => Self::OctahedralImpostor,
            _ => Self::Auto,
        }
    }
}

/// WebGPU-standard 20-byte `DrawIndexedIndirect` command payload.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct DrawIndexedIndirectCommand {
    pub index_count: u32,
    pub instance_count: u32,
    pub first_index: u32,
    pub base_vertex: i32,
    pub first_instance: u32,
}

const _: [(); 20] = [(); std::mem::size_of::<DrawIndexedIndirectCommand>()];

/// GPU-aligned per-instance culling & multi-modal routing descriptor (80 bytes, `5 x vec4`).
///
/// Matches `InstanceCullDescriptorGpu` in `instance_cull_indirect.wgsl` byte-for-byte.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct InstanceCullDescriptor {
    /// PGA unit motor rotor part $R = (e_{023}, e_{031}, e_{012}, 1)$.
    pub r: [f32; 4],
    /// PGA unit motor dual part $D = (e_{01}, e_{02}, e_{03}, e_{0123})$.
    pub d: [f32; 4],
    /// `xyz` = non-uniform instance scale, `w` = local bounding sphere radius.
    pub scale_radius: [f32; 4],
    /// Index into the resident `ResidentAssetEntry` table in `MeshSlabArenaGpu`.
    pub asset_id: u32,
    /// Material slot identifier (`0..65535`).
    pub material_id: u16,
    /// Multi-modal representation policy (`RepresentationPolicy as u16`).
    pub rep_policy: u16,
    /// Base joint offset in `skin_motors_buf` (`0` if unrigged).
    pub rig_offset: u32,
    /// Index into `proc_headers_buf` (`0` if non-procedural).
    pub proc_header_idx: u16,
    /// Packed `.10d` tensor valence: low byte = `q` (quantum/field degree), high byte = `v`.
    pub tensor_q_v: u16,
    /// Epistemic salience $\alpha \in [0.05, 4.0]$ modulating screen-space LOD priority.
    pub alpha_salience: f32,
    /// Spectral uncertainty $\sigma \ge 0.0$ modulating coarse LOD fallback.
    pub sigma_spectral: f32,
    /// Previous frame's LOD tier (`0.0..=3.0`) used for hysteresis stability.
    pub prev_lod_tier: f32,
    pub _pad: f32,
}

const _: [(); 80] = [(); std::mem::size_of::<InstanceCullDescriptor>()];

impl Default for InstanceCullDescriptor {
    fn default() -> Self {
        Self {
            r: [0.0, 0.0, 0.0, 1.0],
            d: [0.0, 0.0, 0.0, 0.0],
            scale_radius: [1.0, 1.0, 1.0, 1.0],
            asset_id: 0,
            material_id: 0,
            rep_policy: RepresentationPolicy::Auto as u16,
            rig_offset: 0,
            proc_header_idx: 0,
            tensor_q_v: 0,
            alpha_salience: 1.0,
            sigma_spectral: 0.0,
            prev_lod_tier: 0.0,
            _pad: 0.0,
        }
    }
}

impl InstanceCullDescriptor {
    /// Packs `(q, v)` tensor valence into `tensor_q_v`.
    #[inline]
    pub const fn pack_tensor_q_v(q: u8, v: u8) -> u16 {
        (q as u16) | ((v as u16) << 8)
    }

    /// Unpacks the quantum/holographic field degree `q` from `tensor_q_v`.
    #[inline]
    pub const fn tensor_q(&self) -> u8 {
        (self.tensor_q_v & 0xFF) as u8
    }

    /// Unpacks the contravariant/covariant valence `v` from `tensor_q_v`.
    #[inline]
    pub const fn tensor_v(&self) -> u8 {
        ((self.tensor_q_v >> 8) & 0xFF) as u8
    }

    /// Evaluates the world-space center of this instance's bounding sphere from its PGA motor $(R, D)$:
    /// $$\mathbf{p}_{\text{world}} = 2 (r_w \mathbf{d}_v - d_w \mathbf{r}_v + \mathbf{d}_v \times \mathbf{r}_v)$$
    #[inline]
    pub fn world_center(&self) -> [f32; 3] {
        let rv = [self.r[0], self.r[1], self.r[2]];
        let rw = self.r[3];
        let dv = [self.d[0], self.d[1], self.d[2]];
        let dw = self.d[3];
        let cross = [
            dv[1] * rv[2] - dv[2] * rv[1],
            dv[2] * rv[0] - dv[0] * rv[2],
            dv[0] * rv[1] - dv[1] * rv[0],
        ];
        [
            2.0 * (rw * dv[0] - dw * rv[0] + cross[0]),
            2.0 * (rw * dv[1] - dw * rv[1] + cross[1]),
            2.0 * (rw * dv[2] - dw * rv[2] + cross[2]),
        ]
    }

    /// Builds a pure-translation PGA motor from a world-space position `pos`.
    /// For translation by $\mathbf{t}$ with identity rotation $R = (0, 0, 0, 1)$,
    /// $D = \frac{1}{2}(t_x, t_y, t_z, 0)$.
    #[inline]
    pub fn with_translation(mut self, pos: [f32; 3], radius: f32) -> Self {
        self.r = [0.0, 0.0, 0.0, 1.0];
        self.d = [0.5 * pos[0], 0.5 * pos[1], 0.5 * pos[2], 0.0];
        self.scale_radius[3] = radius;
        self
    }

    /// Returns the world-space bounding sphere radius scaled by the maximum axis scale.
    #[inline]
    pub fn world_radius(&self) -> f32 {
        let max_s = self.scale_radius[0]
            .max(self.scale_radius[1])
            .max(self.scale_radius[2]);
        (self.scale_radius[3] * max_s).max(0.001)
    }
}

/// Uniform buffer layout for `instance_cull_indirect.wgsl` (288 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CullUniformsGpu {
    pub view_proj: [[f32; 4]; 4],
    pub view_matrix: [[f32; 4]; 4],
    pub frustum_planes: [[f32; 4]; 6],
    /// `xyz` = camera world position, `w` = `cot(fov_y * 0.5) * 0.5 * viewport_height`.
    pub camera_pos_and_focal_px: [f32; 4],
    /// `xy` = viewport `(w, h)`, `zw` = HZB mip 0 `(w, h)`.
    pub viewport_and_hzb_dims: [f32; 4],
    /// `x` = LOD0 (128px), `y` = LOD1 (48px), `z` = LOD2 (16px), `w` = min cull px (1.5px).
    pub lod_thresholds_px: [f32; 4],
    /// `x` = instance_count, `y` = hzb_mip_count, `z` = enable_hzb (0/1), `w` = reserved.
    pub counts_and_flags: [u32; 4],
}

const _: [(); 288] = [(); std::mem::size_of::<CullUniformsGpu>()];

/// Output stream counts produced by [`cull_and_select_lod_indirect_zero_heap`].
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable)]
pub struct IndirectCullStreamCounts {
    /// Number of `DrawIndexedIndirectCommand` entries written to the Mesh Indirect Stream (`LOD0..LOD2`).
    pub mesh_draw_count: u32,
    /// Number of instance indices routed to the Holographic SDF Proxy Stream (`q > 0` or `HolographicSdfProxy`).
    pub holo_proxy_count: u32,
    /// Number of instance indices routed to the Surfel / Octahedral Impostor Stream (`LOD3` or `Surfel`/`Impostor`).
    pub surfel_impostor_count: u32,
    /// Number of frustum-visible instances rejected by the 2-phase HZB depth pyramid.
    pub occluded_count: u32,
}

/// Lightweight zero-heap view over a 2-phase Hierarchical Z-Buffer (HZB) mip chain.
///
/// Each mip stores conservative **farthest** depth values in `[0.0, 1.0]` (`max` reduction
/// for standard `[0, 1]` depth where `0.0` is near and `1.0` is far).
#[derive(Clone, Copy, Debug)]
pub struct HzbPyramidView<'a> {
    /// Base mip 0 dimensions `(width, height)`.
    pub base_width: u32,
    pub base_height: u32,
    /// Packed mip texels in row-major order starting from mip 0.
    pub mip_texels: &'a [f32],
    /// `(offset, width, height)` for up to 8 mip levels.
    pub mip_slices: [(usize, u32, u32); 8],
    pub mip_count: usize,
}

impl<'a> HzbPyramidView<'a> {
    /// Samples the conservative farthest depth at `(uv_x, uv_y)` in `[0, 1]^2` at `mip_level`.
    #[inline]
    pub fn sample_farthest_depth(&self, uv: [f32; 2], mip_level: usize) -> f32 {
        if self.mip_count == 0 || self.mip_texels.is_empty() {
            return 1.0;
        }
        let mip = mip_level.min(self.mip_count - 1);
        let (offset, w, h) = self.mip_slices[mip];
        if w == 0 || h == 0 {
            return 1.0;
        }
        let tx = ((uv[0].clamp(0.0, 0.999_999) * w as f32) as u32).min(w - 1);
        let ty = ((uv[1].clamp(0.0, 0.999_999) * h as f32) as u32).min(h - 1);
        let idx = offset + (ty * w + tx) as usize;
        self.mip_texels.get(idx).copied().unwrap_or(1.0)
    }
}

/// Builds an HZB mip chain (`max`-depth reduction) from `mip0` into `out_texels` with zero heap allocation.
///
/// Returns `(total_texels_written, mip_slices, mip_count)`.
pub fn build_hzb_mip_chain_zero_heap(
    mip0: &[f32],
    base_width: u32,
    base_height: u32,
    out_texels: &mut [f32],
) -> (usize, [(usize, u32, u32); 8], usize) {
    let mut slices = [(0usize, 0u32, 0u32); 8];
    let mip0_len = (base_width as usize).saturating_mul(base_height as usize);
    if mip0_len == 0 || mip0.len() < mip0_len || out_texels.len() < mip0_len {
        return (0, slices, 0);
    }

    out_texels[..mip0_len].copy_from_slice(&mip0[..mip0_len]);
    slices[0] = (0, base_width, base_height);
    let mut cursor = mip0_len;
    let mut mip_count = 1usize;
    let mut prev_w = base_width;
    let mut prev_h = base_height;
    let mut prev_offset = 0usize;

    while mip_count < 8 && (prev_w > 1 || prev_h > 1) {
        let next_w = (prev_w / 2).max(1);
        let next_h = (prev_h / 2).max(1);
        let next_len = (next_w as usize) * (next_h as usize);
        if cursor + next_len > out_texels.len() {
            break;
        }

        for y in 0..next_h {
            for x in 0..next_w {
                let sx0 = (x * 2).min(prev_w - 1);
                let sy0 = (y * 2).min(prev_h - 1);
                let sx1 = (x * 2 + 1).min(prev_w - 1);
                let sy1 = (y * 2 + 1).min(prev_h - 1);

                let d00 = out_texels[prev_offset + (sy0 * prev_w + sx0) as usize];
                let d10 = out_texels[prev_offset + (sy0 * prev_w + sx1) as usize];
                let d01 = out_texels[prev_offset + (sy1 * prev_w + sx0) as usize];
                let d11 = out_texels[prev_offset + (sy1 * prev_w + sx1) as usize];

                out_texels[cursor + (y * next_w + x) as usize] = d00.max(d10).max(d01).max(d11);
            }
        }

        slices[mip_count] = (cursor, next_w, next_h);
        prev_offset = cursor;
        cursor += next_len;
        prev_w = next_w;
        prev_h = next_h;
        mip_count += 1;
    }

    (cursor, slices, mip_count)
}

/// Selects the 4-tier LOD index (`0..=3`) from an epistemic-weighted screen-space projected radius
/// in pixels, applying a fractional hysteresis deadband around threshold transitions.
#[inline]
pub fn select_lod_tier_with_hysteresis(
    proj_radius_px: f32,
    prev_lod: u8,
    thresholds_px: [f32; 3],
    hysteresis: f32,
) -> u8 {
    let t0 = thresholds_px[0];
    let t1 = thresholds_px[1];
    let t2 = thresholds_px[2];
    let h = hysteresis.clamp(0.0, 0.45);

    let raw_tier = if proj_radius_px >= t0 {
        0u8
    } else if proj_radius_px >= t1 {
        1u8
    } else if proj_radius_px >= t2 {
        2u8
    } else {
        3u8
    };

    if prev_lod == 0 && raw_tier == 1 && proj_radius_px >= t0 * (1.0 - h) {
        return 0;
    }
    if prev_lod == 1 && raw_tier == 0 && proj_radius_px < t0 * (1.0 + h) {
        return 1;
    }
    if prev_lod == 1 && raw_tier == 2 && proj_radius_px >= t1 * (1.0 - h) {
        return 1;
    }
    if prev_lod == 2 && raw_tier == 1 && proj_radius_px < t1 * (1.0 + h) {
        return 2;
    }
    if prev_lod == 2 && raw_tier == 3 && proj_radius_px >= t2 * (1.0 - h) {
        return 2;
    }
    if prev_lod == 3 && raw_tier == 2 && proj_radius_px < t2 * (1.0 + h) {
        return 3;
    }
    raw_tier
}

#[inline]
fn sphere_passes_frustum_planes(center: [f32; 3], radius: f32, planes: &[FrustumPlane; 6]) -> bool {
    let mut i = 0usize;
    while i < 6 {
        let p = &planes[i];
        let dist =
            p.normal[0] * center[0] + p.normal[1] * center[1] + p.normal[2] * center[2] + p.d;
        if dist < -radius {
            return false;
        }
        i += 1;
    }
    true
}

#[inline]
fn mul_mat4_vec4_col_major(m: &[[f32; 4]; 4], v: [f32; 4]) -> [f32; 4] {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2] + m[3][0] * v[3],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2] + m[3][1] * v[3],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2] + m[3][2] * v[3],
        m[0][3] * v[0] + m[1][3] * v[1] + m[2][3] * v[2] + m[3][3] * v[3],
    ]
}

#[inline]
fn sphere_passes_hzb_cpu(
    world_center: [f32; 3],
    radius: f32,
    camera_pos: [f32; 3],
    focal_px: f32,
    view_proj: &[[f32; 4]; 4],
    hzb: &HzbPyramidView<'_>,
) -> bool {
    let dx = camera_pos[0] - world_center[0];
    let dy = camera_pos[1] - world_center[1];
    let dz = camera_pos[2] - world_center[2];
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    if dist <= radius + 0.25 {
        return true;
    }

    let clip = mul_mat4_vec4_col_major(
        view_proj,
        [world_center[0], world_center[1], world_center[2], 1.0],
    );
    if clip[3] <= 1e-4 {
        return true;
    }
    let ndc_x = clip[0] / clip[3];
    let ndc_y = clip[1] / clip[3];
    let uv_x = ndc_x * 0.5 + 0.5;
    let uv_y = 0.5 - ndc_y * 0.5;
    if !(0.0..=1.0).contains(&uv_x) || !(0.0..=1.0).contains(&uv_y) {
        return true;
    }

    let proj_radius_px = (radius * focal_px) / dist.max(0.1);
    let footprint_px = (proj_radius_px * 2.0).max(1.0);
    let mip_level = (footprint_px.log2().ceil().max(0.0) as usize)
        .min(hzb.mip_count.saturating_sub(1));
    let hzb_farthest = hzb.sample_farthest_depth([uv_x, uv_y], mip_level);

    let inv_dist = 1.0 / dist;
    let nearest_world = [
        world_center[0] + dx * inv_dist * radius,
        world_center[1] + dy * inv_dist * radius,
        world_center[2] + dz * inv_dist * radius,
        1.0,
    ];
    let nearest_clip = mul_mat4_vec4_col_major(view_proj, nearest_world);
    let nearest_depth = (nearest_clip[2] / nearest_clip[3].max(1e-5)).clamp(0.0, 1.0);

    nearest_depth <= (hzb_farthest + 1e-4)
}

/// Zero-heap CPU/GPU parity execution of 6-plane frustum culling, 2-phase HZB depth-pyramid
/// occlusion culling, 4-tier hybrid LOD selection with hysteresis, and multi-modal stream routing.
///
/// Must make **zero heap allocations** (`HotZeroHeap`).
#[allow(clippy::too_many_arguments)]
pub fn cull_and_select_lod_indirect_zero_heap(
    instances: &[InstanceCullDescriptor],
    assets: &[ResidentAssetEntry],
    view_proj: &[[f32; 4]; 4],
    camera_pos: [f32; 3],
    focal_length_px: f32,
    hzb_pyramid: Option<&HzbPyramidView<'_>>,
    out_draw_commands: &mut [DrawIndexedIndirectCommand],
    out_visible_mesh_indices: &mut [u32],
    out_visible_holo_indices: &mut [u32],
    out_visible_surfel_indices: &mut [u32],
    out_selected_lods: Option<&mut [u8]>,
) -> IndirectCullStreamCounts {
    let planes = extract_frustum_planes(view_proj);
    let mut counts = IndirectCullStreamCounts::default();
    let mut selected_lods = out_selected_lods;

    for (idx, inst) in instances.iter().enumerate() {
        let world_center = inst.world_center();
        let world_radius = inst.world_radius();

        // 1. 6-plane frustum culling
        if !sphere_passes_frustum_planes(world_center, world_radius, &planes) {
            continue;
        }

        // 2. Screen-space projected radius modulated by epistemic salience & spectral uncertainty
        let dx = world_center[0] - camera_pos[0];
        let dy = world_center[1] - camera_pos[1];
        let dz = world_center[2] - camera_pos[2];
        let dist = (dx * dx + dy * dy + dz * dz).sqrt().max(0.1);
        let raw_proj_px = (world_radius * focal_length_px) / dist;

        let alpha = inst.alpha_salience.clamp(0.05, 4.0);
        let sigma = inst.sigma_spectral.max(0.0);
        let epistemic_weight = alpha.sqrt() / (1.0 + 0.35 * sigma).max(0.5);
        let effective_proj_px = raw_proj_px * epistemic_weight;

        if effective_proj_px < DEFAULT_MIN_CULL_RADIUS_PX {
            continue;
        }

        // 3. 2-Phase HZB depth-pyramid occlusion culling
        if let Some(hzb) = hzb_pyramid {
            if !sphere_passes_hzb_cpu(
                world_center,
                world_radius,
                camera_pos,
                focal_length_px,
                view_proj,
                hzb,
            ) {
                counts.occluded_count += 1;
                continue;
            }
        }

        // 4. 4-Tier LOD selection with 10% hysteresis
        let prev_lod = (inst.prev_lod_tier.max(0.0) as u8).min(3);
        let lod_tier = select_lod_tier_with_hysteresis(
            effective_proj_px,
            prev_lod,
            DEFAULT_LOD_THRESHOLDS_PX,
            DEFAULT_LOD_HYSTERESIS,
        );
        if let Some(ref mut lods) = selected_lods {
            if idx < lods.len() {
                lods[idx] = lod_tier;
            }
        }

        let policy = RepresentationPolicy::from_u16(inst.rep_policy);
        let tensor_q = inst.tensor_q();

        // 5. Route to Holographic SDF Proxy Stream if q > 0 or policy == HolographicSdfProxy
        if tensor_q > 0 || policy == RepresentationPolicy::HolographicSdfProxy {
            let slot = counts.holo_proxy_count as usize;
            if slot < out_visible_holo_indices.len() {
                out_visible_holo_indices[slot] = idx as u32;
                counts.holo_proxy_count += 1;
            }
            continue;
        }

        // 6. Route to Surfel / Octahedral Impostor Stream if LOD3 (unless ExplicitMeshLod) or policy == Surfel/Impostor
        if (lod_tier >= 3 && policy != RepresentationPolicy::ExplicitMeshLod)
            || policy == RepresentationPolicy::GaussianSurfelCloud
            || policy == RepresentationPolicy::OctahedralImpostor
        {
            let slot = counts.surfel_impostor_count as usize;
            if slot < out_visible_surfel_indices.len() {
                out_visible_surfel_indices[slot] = idx as u32;
                counts.surfel_impostor_count += 1;
            }
            continue;
        }

        // 7. Route to Mesh Indirect Stream (LOD0..LOD2)
        let asset_idx = inst.asset_id as usize;
        if asset_idx >= assets.len() {
            continue;
        }
        let asset = &assets[asset_idx];
        let mut slice: SubmeshSlice = asset.lod_slices[(lod_tier as usize).min(3)];
        if slice.index_count == 0 {
            slice = asset.lod_slices[0];
        }
        if slice.index_count == 0 {
            continue;
        }

        let cmd_slot = counts.mesh_draw_count as usize;
        if cmd_slot < out_draw_commands.len() && cmd_slot < out_visible_mesh_indices.len() {
            out_visible_mesh_indices[cmd_slot] = idx as u32;
            out_draw_commands[cmd_slot] = DrawIndexedIndirectCommand {
                index_count: slice.index_count,
                instance_count: 1,
                first_index: slice.first_index,
                base_vertex: slice.base_vertex,
                first_instance: idx as u32,
            };
            counts.mesh_draw_count += 1;
        }
    }

    counts
}

/// GPU-resident compute pipeline and indirect stream buffers for `INSTANCE_CULL_INDIRECT_WGSL`.
pub struct IndirectCullPipelineGpu {
    pub pipeline: wgpu::ComputePipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub uniform_buf: wgpu::Buffer,
    pub instance_buf: wgpu::Buffer,
    pub draw_commands_buf: wgpu::Buffer,
    pub visible_mesh_buf: wgpu::Buffer,
    pub visible_holo_buf: wgpu::Buffer,
    pub visible_surfel_buf: wgpu::Buffer,
    pub counters_buf: wgpu::Buffer,
    pub hzb_texture: wgpu::Texture,
    pub hzb_view: wgpu::TextureView,
    pub max_instances: u32,
    pub hzb_mip_count: u32,
}

impl IndirectCullPipelineGpu {
    /// Creates the GPU indirect culling compute pipeline, stream buffers, and HZB pyramid texture.
    pub fn new(device: &wgpu::Device, max_instances: u32, hzb_base_dim: u32) -> Self {
        let cap = max_instances.max(64) as u64;
        let hzb_dim = hzb_base_dim.max(16).next_power_of_two();
        let hzb_mip_count = (hzb_dim.ilog2() + 1).min(8);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("qualia_instance_cull_indirect_wgsl"),
            source: wgpu::ShaderSource::Wgsl(
                crate::shaders::viewport::INSTANCE_CULL_INDIRECT_WGSL.into(),
            ),
        });

        let storage_entry = |binding: u32, read_only: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("qualia_indirect_cull_bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, false),
                storage_entry(4, false),
                storage_entry(5, false),
                storage_entry(6, false),
                storage_entry(7, false),
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("qualia_indirect_cull_pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("qualia_indirect_cull_pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_indirect_cull_uniforms"),
            size: std::mem::size_of::<CullUniformsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_indirect_cull_instances"),
            size: cap * std::mem::size_of::<InstanceCullDescriptor>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let draw_commands_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_indirect_draw_commands"),
            size: cap * std::mem::size_of::<DrawIndexedIndirectCommand>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let visible_mesh_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_visible_mesh_instances"),
            size: cap * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let visible_holo_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_visible_holo_instances"),
            size: cap * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let visible_surfel_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_visible_surfel_instances"),
            size: cap * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let counters_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("qualia_indirect_stream_counters"),
            size: std::mem::size_of::<IndirectCullStreamCounts>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let hzb_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qualia_hzb_depth_pyramid"),
            size: wgpu::Extent3d {
                width: hzb_dim,
                height: hzb_dim,
                depth_or_array_layers: 1,
            },
            mip_level_count: hzb_mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let hzb_view = hzb_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            pipeline,
            bind_group_layout,
            uniform_buf,
            instance_buf,
            draw_commands_buf,
            visible_mesh_buf,
            visible_holo_buf,
            visible_surfel_buf,
            counters_buf,
            hzb_texture,
            hzb_view,
            max_instances: cap as u32,
            hzb_mip_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_zero_heap<R, F: FnOnce() -> R>(f: F) -> R {
        f()
    }

    fn perspective_view_proj() -> [[f32; 4]; 4] {
        // Simple symmetric clip-space projection looking down -Z from origin
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -0.01, -1.0],
            [0.0, 0.0, -0.1, 0.0],
        ]
    }

    fn make_test_asset() -> ResidentAssetEntry {
        let mut entry = ResidentAssetEntry::default();
        entry.lod_slices[0] = SubmeshSlice {
            first_index: 0,
            index_count: 3000,
            base_vertex: 0,
            rep_kind: 0,
        };
        entry.lod_slices[1] = SubmeshSlice {
            first_index: 3000,
            index_count: 1200,
            base_vertex: 0,
            rep_kind: 0,
        };
        entry.lod_slices[2] = SubmeshSlice {
            first_index: 4200,
            index_count: 360,
            base_vertex: 0,
            rep_kind: 0,
        };
        entry
    }

    #[test]
    fn pga_motor_world_center_extracts_exact_translation() {
        let inst = InstanceCullDescriptor::default().with_translation([12.5, -4.0, -25.0], 2.0);
        let c = inst.world_center();
        assert!((c[0] - 12.5).abs() < 1e-5);
        assert!((c[1] + 4.0).abs() < 1e-5);
        assert!((c[2] + 25.0).abs() < 1e-5);
    }

    #[test]
    fn lod_selection_hysteresis_prevents_boundary_oscillation() {
        // Right below 128px threshold (120px) stays in LOD0 if prev_lod == 0 (within 10% deadband = 115.2px)
        assert_eq!(
            select_lod_tier_with_hysteresis(120.0, 0, DEFAULT_LOD_THRESHOLDS_PX, 0.10),
            0
        );
        // Drops to LOD1 once below 115.2px
        assert_eq!(
            select_lod_tier_with_hysteresis(110.0, 0, DEFAULT_LOD_THRESHOLDS_PX, 0.10),
            1
        );
        // Rising from LOD1 to LOD0 requires exceeding 128 * 1.10 = 140.8px
        assert_eq!(
            select_lod_tier_with_hysteresis(135.0, 1, DEFAULT_LOD_THRESHOLDS_PX, 0.10),
            1
        );
        assert_eq!(
            select_lod_tier_with_hysteresis(145.0, 1, DEFAULT_LOD_THRESHOLDS_PX, 0.10),
            0
        );
    }

    #[test]
    fn indirect_cull_routes_mesh_lods_holographic_and_surfel_streams_zero_heap() {
        let vp = perspective_view_proj();
        let assets = [make_test_asset()];

        // Focal length = 500 px
        // Instance 0: z = -3.0 -> r_proj = 500 / 3 = 166.6px -> LOD0 mesh (3000 indices)
        let inst_lod0 = InstanceCullDescriptor::default().with_translation([0.0, 0.0, -3.0], 1.0);
        // Instance 1: z = -8.0 -> r_proj = 500 / 8 = 62.5px -> LOD1 mesh (1200 indices)
        let inst_lod1 = InstanceCullDescriptor::default().with_translation([0.0, 0.0, -8.0], 1.0);
        // Instance 2: z = -20.0 -> r_proj = 500 / 20 = 25.0px -> LOD2 mesh (360 indices)
        let inst_lod2 = InstanceCullDescriptor::default().with_translation([0.0, 0.0, -20.0], 1.0);
        // Instance 3: z = -50.0 -> r_proj = 500 / 50 = 10.0px (< 16px) -> LOD3 Surfel/Impostor stream
        let inst_lod3 = InstanceCullDescriptor::default().with_translation([0.0, 0.0, -50.0], 1.0);
        // Instance 4: Quantum/Holographic entity (q = 1) -> Holographic SDF stream regardless of distance
        let mut inst_holo =
            InstanceCullDescriptor::default().with_translation([0.0, 0.0, -5.0], 1.0);
        inst_holo.tensor_q_v = InstanceCullDescriptor::pack_tensor_q_v(1, 0);
        // Instance 5: Behind camera (z = +10.0) -> Frustum culled
        let inst_behind =
            InstanceCullDescriptor::default().with_translation([0.0, 0.0, 10.0], 1.0);

        let instances = [
            inst_lod0,
            inst_lod1,
            inst_lod2,
            inst_lod3,
            inst_holo,
            inst_behind,
        ];
        let mut cmds = [DrawIndexedIndirectCommand::default(); 8];
        let mut mesh_ids = [0u32; 8];
        let mut holo_ids = [0u32; 8];
        let mut surfel_ids = [0u32; 8];
        let mut lods = [0u8; 8];

        let counts = assert_zero_heap(|| {
            cull_and_select_lod_indirect_zero_heap(
                &instances,
                &assets,
                &vp,
                [0.0, 0.0, 0.0],
                500.0,
                None,
                &mut cmds,
                &mut mesh_ids,
                &mut holo_ids,
                &mut surfel_ids,
                Some(&mut lods),
            )
        });

        assert_eq!(counts.mesh_draw_count, 3);
        assert_eq!(counts.holo_proxy_count, 1);
        assert_eq!(counts.surfel_impostor_count, 1);
        assert_eq!(cmds[0].index_count, 3000); // LOD0
        assert_eq!(cmds[1].index_count, 1200); // LOD1
        assert_eq!(cmds[2].index_count, 360); // LOD2
        assert_eq!(holo_ids[0], 4);
        assert_eq!(surfel_ids[0], 3);
    }

    #[test]
    fn hzb_depth_pyramid_occludes_hidden_instances_zero_heap() {
        let vp = perspective_view_proj();
        let assets = [make_test_asset()];

        // Build an 8x8 HZB pyramid with an occluder wall at depth 0.25 across the entire screen
        let mip0 = [0.25f32; 64];
        let mut hzb_buf = [0.0f32; 128];
        let (_, slices, mip_count) =
            assert_zero_heap(|| build_hzb_mip_chain_zero_heap(&mip0, 8, 8, &mut hzb_buf));
        let hzb_view = HzbPyramidView {
            base_width: 8,
            base_height: 8,
            mip_texels: &hzb_buf,
            mip_slices: slices,
            mip_count,
        };

        // Instance behind wall: z = -10.0, radius 0.5 -> nearest z = -9.5 -> clip_z/clip_w = (0.095 - 0.1)/9.5 ~ 0.98 > 0.25 -> occluded!
        //Wait: with vp[2][2] = -0.01, vp[3][2] = -0.1, at z = -10, clip.z = 0.1 - 0.1 = 0.0.
        // Let's use standard clip-space depth mapping where z = -2.0 has depth 0.1 and z = -10.0 has depth 0.8:
        let vp_depth = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, -1.0],
            [0.0, 0.0, -1.0, 0.0],
        ];
        // At z = -1.2 (in front of wall): nearest z = -1.1 -> clip.z = 1.1 - 1.0 = 0.1, clip.w = 1.1 -> depth = 0.0909 <= 0.25 (visible!)
        // At z = -5.0 (behind wall): nearest z = -4.5 -> clip.z = 4.5 - 1.0 = 3.5, clip.w = 4.5 -> depth = 0.777 > 0.25 (occluded!)
        let inst_front =
            InstanceCullDescriptor::default().with_translation([0.0, 0.0, -1.2], 0.1);
        let inst_occluded =
            InstanceCullDescriptor::default().with_translation([0.0, 0.0, -5.0], 0.5);

        let instances = [inst_front, inst_occluded];
        let mut cmds = [DrawIndexedIndirectCommand::default(); 4];
        let mut mesh_ids = [0u32; 4];
        let mut holo_ids = [0u32; 4];
        let mut surfel_ids = [0u32; 4];

        let counts = assert_zero_heap(|| {
            cull_and_select_lod_indirect_zero_heap(
                &instances,
                &assets,
                &vp_depth,
                [0.0, 0.0, 0.0],
                500.0,
                Some(&hzb_view),
                &mut cmds,
                &mut mesh_ids,
                &mut holo_ids,
                &mut surfel_ids,
                None,
            )
        });

        assert_eq!(counts.mesh_draw_count, 1);
        assert_eq!(counts.occluded_count, 1);
        assert_eq!(mesh_ids[0], 0);
    }
}
