// Copyright (c) 2026 QualiaDB Contributors
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Deterministic Topological Terrain DAG Workspace (P10.5 / ColdBounded).

pub const MAX_DAG_NODES: usize = 64;

/// Errors produced during Terrain DAG operations and resource enforcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainDagError {
    CycleDetected,
    NodeLimitExceeded,
    InvalidNodeId(usize),
    DimensionMismatch,
    ByteBudgetExceeded { requested: usize, budget: usize },
}

impl core::fmt::Display for TerrainDagError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CycleDetected => write!(f, "Terrain DAG contains a dependency cycle"),
            Self::NodeLimitExceeded => write!(f, "Terrain DAG exceeded limit of {MAX_DAG_NODES}"),
            Self::InvalidNodeId(id) => write!(f, "Invalid node ID: {id}"),
            Self::DimensionMismatch => write!(f, "Terrain tile dimension mismatch"),
            Self::ByteBudgetExceeded { requested, budget } => {
                write!(f, "Byte budget exceeded: requested {requested}, budget is {budget}")
            }
        }
    }
}

impl std::error::Error for TerrainDagError {}

/// Primitive procedural kernel operations evaluated per cell on heightfield tiles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DagNodeKernel {
    Constant(f32),
    GradientX { min: f32, max: f32 },
    GradientY { min: f32, max: f32 },
    RadialPeak { cx: f32, cy: f32, radius: f32, height: f32 },
    Add { weight_a: f32, weight_b: f32 },
    Multiply,
    Min,
    Max,
    BlendMask,
    Terrace { steps: u32, sharpness: f32 },
    BoxBlur3x3,
}

/// Specification of a single node within the Terrain DAG.
#[derive(Debug, Clone, PartialEq)]
pub struct DagNodeSpec {
    pub id: usize,
    pub kernel: DagNodeKernel,
    pub inputs: [Option<usize>; 3],
    pub dirty: bool,
    pub param_hash: u64,
}

/// Bounded workspace managing terrain node DAG evaluation, caching, and halo apron geometry.
pub struct TerrainDagWorkspace {
    pub nodes: Vec<DagNodeSpec>,
    pub cached_outputs: Vec<Vec<f32>>,
    pub tile_width: usize,
    pub tile_height: usize,
    pub halo_px: usize,
    pub byte_budget: usize,
    pub bytes_used: usize,
}

impl TerrainDagWorkspace {
    /// Create a new terrain workspace with explicit tile dimensions and memory budget.
    pub fn new(tile_width: usize, tile_height: usize, halo_px: usize, byte_budget: usize) -> Result<Self, TerrainDagError> {
        if tile_width == 0 || tile_height == 0 {
            return Err(TerrainDagError::DimensionMismatch);
        }
        Ok(Self { nodes: Vec::new(), cached_outputs: Vec::new(), tile_width, tile_height, halo_px, byte_budget, bytes_used: 0 })
    }

    /// Add a procedural node to the DAG, allocating its tile cache buffer within the byte budget.
    pub fn add_node(&mut self, kernel: DagNodeKernel, inputs: [Option<usize>; 3], param_hash: u64) -> Result<usize, TerrainDagError> {
        if self.nodes.len() >= MAX_DAG_NODES {
            return Err(TerrainDagError::NodeLimitExceeded);
        }
        for inp in inputs.iter().copied().flatten() {
            if inp >= self.nodes.len() {
                return Err(TerrainDagError::InvalidNodeId(inp));
            }
        }

        let full_w = self.tile_width + 2 * self.halo_px;
        let full_h = self.tile_height + 2 * self.halo_px;
        let buffer_len = full_w.checked_mul(full_h).ok_or(TerrainDagError::DimensionMismatch)?;
        let bytes_needed = buffer_len.checked_mul(core::mem::size_of::<f32>()).ok_or(TerrainDagError::DimensionMismatch)?;
        let requested = self.bytes_used.checked_add(bytes_needed).ok_or(TerrainDagError::ByteBudgetExceeded {
            requested: usize::MAX,
            budget: self.byte_budget,
        })?;

        if requested > self.byte_budget {
            return Err(TerrainDagError::ByteBudgetExceeded { requested, budget: self.byte_budget });
        }

        let id = self.nodes.len();
        self.cached_outputs.push(vec![0.0f32; buffer_len]);
        self.bytes_used = requested;
        self.nodes.push(DagNodeSpec { id, kernel, inputs, dirty: true, param_hash });
        Ok(id)
    }

    /// Kahn's in-degree topological sort evaluated on stack arrays.
    pub fn topological_order(&self, out_order: &mut [usize; MAX_DAG_NODES]) -> Result<usize, TerrainDagError> {
        let n = self.nodes.len();
        if n == 0 {
            return Ok(0);
        }
        let mut in_degrees = [0usize; MAX_DAG_NODES];
        for i in 0..n {
            let mut deps = [false; MAX_DAG_NODES];
            for inp in self.nodes[i].inputs.iter().copied().flatten() {
                if inp >= n { return Err(TerrainDagError::InvalidNodeId(inp)); }
                if !deps[inp] { deps[inp] = true; in_degrees[i] += 1; }
            }
        }

        let mut queue = [0usize; MAX_DAG_NODES];
        let (mut head, mut tail) = (0usize, 0usize);
        for (i, &deg) in in_degrees.iter().enumerate().take(n) {
            if deg == 0 { queue[tail] = i; tail += 1; }
        }

        let mut count = 0usize;
        while head < tail {
            let u = queue[head];
            head += 1;
            out_order[count] = u;
            count += 1;
            for v in 0..n {
                if self.nodes[v].inputs.iter().any(|&inp| inp == Some(u)) && in_degrees[v] > 0 {
                    in_degrees[v] -= 1;
                    if in_degrees[v] == 0 { queue[tail] = v; tail += 1; }
                }
            }
        }
        if count < n { Err(TerrainDagError::CycleDetected) } else { Ok(count) }
    }

    /// Mark a node's parameter changed and propagate the dirty flag downstream.
    pub fn mark_param_changed(&mut self, node_id: usize, new_param_hash: u64) -> Result<usize, TerrainDagError> {
        let n = self.nodes.len();
        if node_id >= n {
            return Err(TerrainDagError::InvalidNodeId(node_id));
        }
        self.nodes[node_id].param_hash = new_param_hash;
        let mut dirty_cone = [false; MAX_DAG_NODES];
        dirty_cone[node_id] = true;
        self.nodes[node_id].dirty = true;

        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..n {
                if !dirty_cone[i] && self.nodes[i].inputs.iter().copied().flatten().any(|inp| dirty_cone[inp]) {
                    dirty_cone[i] = true;
                    self.nodes[i].dirty = true;
                    changed = true;
                }
            }
        }
        Ok(dirty_cone.iter().take(n).filter(|&&d| d).count())
    }

    /// Incrementally evaluate dirty nodes in topological order and clear their dirty flags.
    pub fn evaluate_dirty(&mut self, tile_origin: [f32; 2], cell_size: f32) -> Result<usize, TerrainDagError> {
        let mut order = [0usize; MAX_DAG_NODES];
        let total_ordered = self.topological_order(&mut order)?;
        let full_w = self.tile_width + 2 * self.halo_px;
        let full_h = self.tile_height + 2 * self.halo_px;
        let buffer_len = full_w * full_h;
        let mut evaluated_count = 0usize;

        for &node_id in &order[..total_ordered] {
            if !self.nodes[node_id].dirty { continue; }
            let (kernel, inputs) = (self.nodes[node_id].kernel, self.nodes[node_id].inputs);
            let mut target = std::mem::take(&mut self.cached_outputs[node_id]);

            match kernel {
                DagNodeKernel::Constant(v) => target.fill(v),
                DagNodeKernel::GradientX { min, max } => {
                    let span = if self.tile_width > 0 { self.tile_width as f32 } else { 1.0 };
                    for y in 0..full_h {
                        let row = y * full_w;
                        for x in 0..full_w {
                            let t = (x as f32 - self.halo_px as f32) / span;
                            target[row + x] = min + (max - min) * t;
                        }
                    }
                }
                DagNodeKernel::GradientY { min, max } => {
                    let span = if self.tile_height > 0 { self.tile_height as f32 } else { 1.0 };
                    for y in 0..full_h {
                        let val = min + (max - min) * ((y as f32 - self.halo_px as f32) / span);
                        let row = y * full_w;
                        for x in 0..full_w { target[row + x] = val; }
                    }
                }
                DagNodeKernel::RadialPeak { cx, cy, radius, height } => {
                    let inv_r = if radius > 1e-6 { 1.0 / radius } else { 0.0 };
                    for y in 0..full_h {
                        let wy = tile_origin[1] + (y as f32 - self.halo_px as f32) * cell_size;
                        let dy2 = (wy - cy) * (wy - cy);
                        let row = y * full_w;
                        for x in 0..full_w {
                            let wx = tile_origin[0] + (x as f32 - self.halo_px as f32) * cell_size;
                            let dist = ((wx - cx) * (wx - cx) + dy2).sqrt();
                            target[row + x] = if dist < radius { height * (1.0 - dist * inv_r) } else { 0.0 };
                        }
                    }
                }
                DagNodeKernel::Add { weight_a, weight_b } => {
                    let (in_a, in_b) = (inputs[0].and_then(|id| self.cached_outputs.get(id)), inputs[1].and_then(|id| self.cached_outputs.get(id)));
                    for i in 0..buffer_len {
                        target[i] = weight_a * in_a.map(|b| b[i]).unwrap_or(0.0) + weight_b * in_b.map(|b| b[i]).unwrap_or(0.0);
                    }
                }
                DagNodeKernel::Multiply => {
                    let (in_a, in_b) = (inputs[0].and_then(|id| self.cached_outputs.get(id)), inputs[1].and_then(|id| self.cached_outputs.get(id)));
                    for i in 0..buffer_len {
                        target[i] = in_a.map(|b| b[i]).unwrap_or(0.0) * in_b.map(|b| b[i]).unwrap_or(0.0);
                    }
                }
                DagNodeKernel::Min => {
                    let (in_a, in_b) = (inputs[0].and_then(|id| self.cached_outputs.get(id)), inputs[1].and_then(|id| self.cached_outputs.get(id)));
                    for i in 0..buffer_len { target[i] = in_a.map(|b| b[i]).unwrap_or(0.0).min(in_b.map(|b| b[i]).unwrap_or(0.0)); }
                }
                DagNodeKernel::Max => {
                    let (in_a, in_b) = (inputs[0].and_then(|id| self.cached_outputs.get(id)), inputs[1].and_then(|id| self.cached_outputs.get(id)));
                    for i in 0..buffer_len { target[i] = in_a.map(|b| b[i]).unwrap_or(0.0).max(in_b.map(|b| b[i]).unwrap_or(0.0)); }
                }
                DagNodeKernel::BlendMask => {
                    let (in_a, in_b, in_m) = (inputs[0].and_then(|id| self.cached_outputs.get(id)), inputs[1].and_then(|id| self.cached_outputs.get(id)), inputs[2].and_then(|id| self.cached_outputs.get(id)));
                    for i in 0..buffer_len {
                        let (va, vb) = (in_a.map(|b| b[i]).unwrap_or(0.0), in_b.map(|b| b[i]).unwrap_or(0.0));
                        let m = in_m.map(|b| b[i].clamp(0.0, 1.0)).unwrap_or(0.0);
                        target[i] = va * (1.0 - m) + vb * m;
                    }
                }
                DagNodeKernel::Terrace { steps, sharpness } => {
                    let in_a = inputs[0].and_then(|id| self.cached_outputs.get(id));
                    let (num_steps, k) = (steps.max(1) as f32, sharpness.clamp(0.0, 1.0));
                    for i in 0..buffer_len {
                        let s = in_a.map(|b| b[i]).unwrap_or(0.0) * num_steps;
                        let (base, frac) = (s.floor(), s - s.floor());
                        let stepped_frac = frac * frac * (3.0 - 2.0 * frac);
                        target[i] = (base + (1.0 - k) * frac + k * stepped_frac) / num_steps;
                    }
                }
                DagNodeKernel::BoxBlur3x3 => {
                    if let Some(src) = inputs[0].and_then(|id| self.cached_outputs.get(id)) {
                        for y in 0..full_h {
                            let (y_min, y_max) = (if y > 0 { y - 1 } else { 0 }, (y + 1).min(full_h - 1));
                            for x in 0..full_w {
                                let (x_min, x_max) = (if x > 0 { x - 1 } else { 0 }, (x + 1).min(full_w - 1));
                                let (mut sum, mut count) = (0.0f32, 0.0f32);
                                for ny in y_min..=y_max {
                                    let r = ny * full_w;
                                    for nx in x_min..=x_max { sum += src[r + nx]; count += 1.0; }
                                }
                                target[y * full_w + x] = sum / count;
                            }
                        }
                    } else { target.fill(0.0); }
                }
            }

            self.cached_outputs[node_id] = target;
            self.nodes[node_id].dirty = false;
            evaluated_count += 1;
        }
        Ok(evaluated_count)
    }

    /// Extract interior tile heightfield excluding the halo border apron.
    pub fn extract_interior_tile(&self, node_id: usize, out: &mut [f32]) -> Result<(), TerrainDagError> {
        if node_id >= self.nodes.len() { return Err(TerrainDagError::InvalidNodeId(node_id)); }
        let expected = self.tile_width.checked_mul(self.tile_height).ok_or(TerrainDagError::DimensionMismatch)?;
        if out.len() != expected { return Err(TerrainDagError::DimensionMismatch); }
        let full_w = self.tile_width + 2 * self.halo_px;
        let src = &self.cached_outputs[node_id];
        for y in 0..self.tile_height {
            let src_start = (y + self.halo_px) * full_w + self.halo_px;
            let dst_start = y * self.tile_width;
            out[dst_start..dst_start + self.tile_width].copy_from_slice(&src[src_start..src_start + self.tile_width]);
        }
        Ok(())
    }

    /// Stitch adjacent East-West tiles by Hermite-blending the halo overlap zone.
    pub fn stitch_adjacent_tiles_east_west(
        west_halo_tile: &[f32],
        east_halo_tile: &[f32],
        full_w: usize,
        full_h: usize,
        halo_px: usize,
        out_west_interior: &mut [f32],
        out_east_interior: &mut [f32],
    ) -> Result<f32, TerrainDagError> {
        if west_halo_tile.len() != full_w * full_h || east_halo_tile.len() != full_w * full_h {
            return Err(TerrainDagError::DimensionMismatch);
        }
        let tile_w = full_w.checked_sub(2 * halo_px).ok_or(TerrainDagError::DimensionMismatch)?;
        let tile_h = full_h.checked_sub(2 * halo_px).ok_or(TerrainDagError::DimensionMismatch)?;
        if tile_w == 0 || tile_h == 0 || out_west_interior.len() != tile_w * tile_h || out_east_interior.len() != tile_w * tile_h {
            return Err(TerrainDagError::DimensionMismatch);
        }

        for y in 0..tile_h {
            let src = (y + halo_px) * full_w + halo_px;
            let dst = y * tile_w;
            out_west_interior[dst..dst + tile_w].copy_from_slice(&west_halo_tile[src..src + tile_w]);
            out_east_interior[dst..dst + tile_w].copy_from_slice(&east_halo_tile[src..src + tile_w]);
        }
        if halo_px == 0 { return Ok(0.0); }

        let mut max_seam_error = 0.0f32;
        let blend_span = 2 * halo_px;
        for y in 0..tile_h {
            let y_buf = y + halo_px;
            let (w_row, e_row) = (y_buf * full_w, y_buf * full_w);
            let (out_w_row, out_e_row) = (y * tile_w, y * tile_w);

            for k in 0..halo_px {
                let w_int_x = tile_w - halo_px + k;
                let (w_val, e_halo_val) = (west_halo_tile[w_row + halo_px + w_int_x], east_halo_tile[e_row + k]);
                let diff1 = (w_val - e_halo_val).abs();
                if diff1 > max_seam_error { max_seam_error = diff1; }
                let t = (k as f32 + 0.5) / (blend_span as f32);
                let w = t * t * (3.0 - 2.0 * t);
                out_west_interior[out_w_row + w_int_x] = (1.0 - w) * w_val + w * e_halo_val;
            }

            for k in 0..halo_px {
                let (w_halo_val, e_val) = (west_halo_tile[w_row + halo_px + tile_w + k], east_halo_tile[e_row + halo_px + k]);
                let diff2 = (w_halo_val - e_val).abs();
                if diff2 > max_seam_error { max_seam_error = diff2; }
                let t = ((halo_px + k) as f32 + 0.5) / (blend_span as f32);
                let w = t * t * (3.0 - 2.0 * t);
                out_east_interior[out_e_row + k] = (1.0 - w) * w_halo_val + w * e_val;
            }
        }
        Ok(max_seam_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topological_sort_and_cycle_detection() {
        let mut ws = TerrainDagWorkspace::new(8, 8, 1, 64 * 1024).unwrap();
        let n0 = ws.add_node(DagNodeKernel::Constant(10.0), [None, None, None], 1).unwrap();
        let n1 = ws.add_node(DagNodeKernel::Constant(20.0), [None, None, None], 2).unwrap();
        let n2 = ws.add_node(DagNodeKernel::Add { weight_a: 1.0, weight_b: 1.0 }, [Some(n0), Some(n1), None], 3).unwrap();

        let mut order = [0usize; MAX_DAG_NODES];
        let count = ws.topological_order(&mut order).unwrap();
        assert_eq!(count, 3);
        assert_eq!(order[2], n2);

        // Introduce a cycle
        ws.nodes[n0].inputs[0] = Some(n2);
        assert_eq!(ws.topological_order(&mut order), Err(TerrainDagError::CycleDetected));
    }

    #[test]
    fn test_fail_closed_byte_budget_exceeded() {
        // Tile is (4 + 2*1) * (4 + 2*1) = 36 cells * 4 bytes = 144 bytes per node buffer
        let budget = 200; // Allows only 1 node (144 bytes), 2nd node needs 288 bytes
        let mut ws = TerrainDagWorkspace::new(4, 4, 1, budget).unwrap();
        assert!(ws.add_node(DagNodeKernel::Constant(1.0), [None, None, None], 1).is_ok());

        let res = ws.add_node(DagNodeKernel::Constant(2.0), [None, None, None], 2);
        assert_eq!(res, Err(TerrainDagError::ByteBudgetExceeded { requested: 288, budget: 200 }));
    }

    #[test]
    fn test_incremental_dirty_cone_invalidation() {
        let mut ws = TerrainDagWorkspace::new(4, 4, 1, 64 * 1024).unwrap();
        let n0 = ws.add_node(DagNodeKernel::Constant(5.0), [None, None, None], 1).unwrap();
        let n1 = ws.add_node(DagNodeKernel::Constant(10.0), [None, None, None], 2).unwrap();
        let n2 = ws.add_node(DagNodeKernel::Add { weight_a: 1.0, weight_b: 1.0 }, [Some(n0), Some(n1), None], 3).unwrap();
        let n3 = ws.add_node(DagNodeKernel::Multiply, [Some(n2), Some(n0), None], 4).unwrap();

        let evaluated = ws.evaluate_dirty([0.0, 0.0], 1.0).unwrap();
        assert_eq!(evaluated, 4);
        assert!(!ws.nodes[n0].dirty && !ws.nodes[n1].dirty && !ws.nodes[n2].dirty && !ws.nodes[n3].dirty);

        // Changing parameter on n1 should invalidate n1, n2, and n3, but keep n0 clean
        let invalidated = ws.mark_param_changed(n1, 999).unwrap();
        assert_eq!(invalidated, 3);
        assert!(!ws.nodes[n0].dirty && ws.nodes[n1].dirty && ws.nodes[n2].dirty && ws.nodes[n3].dirty);

        let second_eval = ws.evaluate_dirty([0.0, 0.0], 1.0).unwrap();
        assert_eq!(second_eval, 3);
    }

    #[test]
    fn test_stitch_adjacent_tiles_east_west_zero_seam() {
        let (w, h, halo, cell_size) = (8, 8, 2, 1.0f32);
        let mut ws_west = TerrainDagWorkspace::new(w, h, halo, 64 * 1024).unwrap();
        let n_west = ws_west.add_node(
            DagNodeKernel::RadialPeak { cx: 4.0, cy: 4.0, radius: 10.0, height: 100.0 },
            [None, None, None],
            10,
        ).unwrap();
        ws_west.evaluate_dirty([0.0, 0.0], cell_size).unwrap();

        let mut ws_east = TerrainDagWorkspace::new(w, h, halo, 64 * 1024).unwrap();
        let n_east = ws_east.add_node(
            DagNodeKernel::RadialPeak { cx: 4.0, cy: 4.0, radius: 10.0, height: 100.0 },
            [None, None, None],
            10,
        ).unwrap();
        ws_east.evaluate_dirty([w as f32 * cell_size, 0.0], cell_size).unwrap();

        let (full_w, full_h) = (w + 2 * halo, h + 2 * halo);
        let mut out_west = vec![0.0f32; w * h];
        let mut out_east = vec![0.0f32; w * h];

        let seam_err = TerrainDagWorkspace::stitch_adjacent_tiles_east_west(
            &ws_west.cached_outputs[n_west],
            &ws_east.cached_outputs[n_east],
            full_w,
            full_h,
            halo,
            &mut out_west,
            &mut out_east,
        ).unwrap();

        assert!(seam_err < 1e-6, "Seam error should be exactly 0.0, got {seam_err}");
    }
}
