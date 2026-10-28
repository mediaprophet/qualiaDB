// Portions adapted from RustGIS-main v0.4.0 (MIT / Apache-2.0 dual license).
// Copyright (c) 2024-2026 RustGIS contributors and QualiaDB developers.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

//! MVT / PMTiles Vector Tile Style Evaluator, Curve-Following Centerline
//! Glyph Placement, and R-Tree Screen-Space Label Collision Solver.
//!
//! Complies with QualiaDB Tier-1 HotZeroHeap and Tier-2 ColdBounded contracts.

/// Error conditions for vector tile styling, glyph layout, and label collision.
#[derive(Debug, Clone, PartialEq)]
pub enum GeoLabelError {
    ByteBudgetExceeded { requested: usize, budget: usize },
    MaxCurvatureExceeded { turn_angle_rad: f32, max_angle_rad: f32 },
    PolylineTooShort,
    EmptyText,
    InvalidGlyphMetrics,
}

impl core::fmt::Display for GeoLabelError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ByteBudgetExceeded { requested, budget } => {
                write!(f, "byte budget exceeded: requested {requested} B, budget {budget} B")
            }
            Self::MaxCurvatureExceeded { turn_angle_rad, max_angle_rad } => {
                write!(f, "max curvature exceeded: deflection {turn_angle_rad:.3} rad > max {max_angle_rad:.3} rad")
            }
            Self::PolylineTooShort => write!(f, "polyline too short for label placement"),
            Self::EmptyText => write!(f, "empty text cannot be placed"),
            Self::InvalidGlyphMetrics => write!(f, "invalid glyph metrics: dimensions must be positive"),
        }
    }
}

impl std::error::Error for GeoLabelError {}

/// A zoom-dependent keyframe stop for vector tile property interpolation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StyleZoomStop {
    pub zoom: f32,
    pub value: f32,
}

/// Evaluates Mapbox/MapLibre-style zoom interpolation for vector layer properties.
///
/// Supports linear (`base_exponential == 1.0`) and exponential (`base_exponential > 1.0`)
/// zoom curve interpolation. Tier-1 HotZeroHeap: zero heap allocations.
pub fn evaluate_style_zoom_interpolated(stops: &[StyleZoomStop], zoom: f32, base_exponential: f32) -> f32 {
    if stops.is_empty() { return 0.0; }
    if stops.len() == 1 || zoom <= stops[0].zoom { return stops[0].value; }
    let last_idx = stops.len() - 1;
    if zoom >= stops[last_idx].zoom { return stops[last_idx].value; }

    let mut idx = 0;
    while idx < last_idx && stops[idx + 1].zoom < zoom { idx += 1; }

    let (z0, z1, v0, v1) = (stops[idx].zoom, stops[idx + 1].zoom, stops[idx].value, stops[idx + 1].value);
    let dz = z1 - z0;
    if dz.abs() < 1e-6 { return v0; }

    let t = if (base_exponential - 1.0).abs() < 1e-5 {
        ((zoom - z0) / dz).clamp(0.0, 1.0)
    } else if base_exponential > 0.0 {
        let num = base_exponential.powf(zoom - z0) - 1.0;
        let den = base_exponential.powf(dz) - 1.0;
        if den.abs() < 1e-6 { 0.0 } else { (num / den).clamp(0.0, 1.0) }
    } else {
        ((zoom - z0) / dz).clamp(0.0, 1.0)
    };

    v0 + t * (v1 - v0)
}

/// Vector tile layer visual styling configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorTileLayerStyle {
    pub layer_name: String,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub fill_rgba8: [u8; 4],
    pub stroke_rgba8: [u8; 4],
    pub width_stops: Vec<StyleZoomStop>,
    pub opacity_stops: Vec<StyleZoomStop>,
}

/// GPU-ready glyph vertex quad layout oriented along a curve tangent.
///
/// Memory layout: strictly 32 bytes, 4-byte aligned.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurvedGlyphQuad {
    pub center_px: [f32; 2],
    pub half_size_px: [f32; 2],
    pub cos_sin_angle: [f32; 2],
    pub codepoint: u32,
    pub color_rgba8: u32,
}

#[inline]
fn angle_deflection(a: f32, b: f32) -> f32 {
    let mut diff = (a - b).abs() % (2.0 * core::f32::consts::PI);
    if diff > core::f32::consts::PI { diff = 2.0 * core::f32::consts::PI - diff; }
    diff
}

/// Layout glyph quads along a curve centerline (roads, rivers).
///
/// Tier-2 ColdBounded: enforces byte budget fail-closed and rejects hairpin curves.
pub fn layout_glyphs_along_centerline(
    polyline_px: &[[f32; 2]],
    text: &str,
    glyph_advance_px: f32,
    glyph_height_px: f32,
    max_turn_angle_rad: f32,
    color_rgba8: u32,
    byte_budget: usize,
) -> Result<Vec<CurvedGlyphQuad>, GeoLabelError> {
    if text.is_empty() { return Err(GeoLabelError::EmptyText); }
    if polyline_px.len() < 2 { return Err(GeoLabelError::PolylineTooShort); }
    if glyph_advance_px <= 0.0 || glyph_height_px <= 0.0 { return Err(GeoLabelError::InvalidGlyphMetrics); }

    let char_count = text.chars().count();
    let estimated_bytes = char_count * core::mem::size_of::<CurvedGlyphQuad>() + polyline_px.len() * 16 + 128;
    if estimated_bytes > byte_budget {
        return Err(GeoLabelError::ByteBudgetExceeded { requested: estimated_bytes, budget: byte_budget });
    }

    let seg_count = polyline_px.len() - 1;
    let mut cum_dist = Vec::with_capacity(polyline_px.len());
    cum_dist.push(0.0f32);
    let mut total_length = 0.0f32;

    for i in 0..seg_count {
        let (dx, dy) = (polyline_px[i + 1][0] - polyline_px[i][0], polyline_px[i + 1][1] - polyline_px[i][1]);
        total_length += (dx * dx + dy * dy).sqrt();
        cum_dist.push(total_length);
    }

    let text_length = (char_count as f32) * glyph_advance_px;
    if text_length > total_length { return Err(GeoLabelError::PolylineTooShort); }

    let start_s = (total_length - text_length) * 0.5;
    let mut quads = Vec::with_capacity(char_count);
    let mut prev_angle: Option<f32> = None;

    for (idx, ch) in text.chars().enumerate() {
        let s_clamped = (start_s + (idx as f32 + 0.5) * glyph_advance_px).clamp(0.0, total_length);
        let mut seg_idx = 0;
        while seg_idx < seg_count - 1 && cum_dist[seg_idx + 1] < s_clamped { seg_idx += 1; }

        let (p0, p1) = (polyline_px[seg_idx], polyline_px[seg_idx + 1]);
        let seg_d = cum_dist[seg_idx + 1] - cum_dist[seg_idx];

        let (pos, cos_val, sin_val) = if seg_d > 1e-6 {
            let t = ((s_clamped - cum_dist[seg_idx]) / seg_d).clamp(0.0, 1.0);
            let px = p0[0] + t * (p1[0] - p0[0]);
            let py = p0[1] + t * (p1[1] - p0[1]);
            let dx = (p1[0] - p0[0]) / seg_d;
            let dy = (p1[1] - p0[1]) / seg_d;
            ([px, py], dx, dy)
        } else {
            (p0, 1.0, 0.0)
        };

        let angle = sin_val.atan2(cos_val);
        if let Some(prev) = prev_angle {
            let turn = angle_deflection(angle, prev);
            if turn > max_turn_angle_rad {
                return Err(GeoLabelError::MaxCurvatureExceeded { turn_angle_rad: turn, max_angle_rad: max_turn_angle_rad });
            }
        }
        prev_angle = Some(angle);

        quads.push(CurvedGlyphQuad {
            center_px: pos,
            half_size_px: [glyph_advance_px * 0.5, glyph_height_px * 0.5],
            cos_sin_angle: [cos_val, sin_val],
            codepoint: ch as u32,
            color_rgba8,
        });
    }

    Ok(quads)
}

/// Point-label placement candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelCandidate {
    pub id: u32,
    pub text: String,
    pub anchor_px: [f32; 2],
    pub bbox_half_px: [f32; 2],
    pub priority: f32,
    pub allow_8_way_offset: bool,
}

/// Accepted label with placed coordinates and Imhof position code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedLabel {
    pub id: u32,
    pub final_center_px: [f32; 2],
    pub aabb_min_max_px: [f32; 4],
    pub offset_position_code: u8,
}

pub const OFFSET_TR: u8 = 0;
pub const OFFSET_TL: u8 = 1;
pub const OFFSET_BR: u8 = 2;
pub const OFFSET_BL: u8 = 3;
pub const OFFSET_R: u8 = 4;
pub const OFFSET_L: u8 = 5;
pub const OFFSET_T: u8 = 6;
pub const OFFSET_B: u8 = 7;
pub const OFFSET_CENTER: u8 = 8;

fn compute_imhof_candidate_center(anchor: [f32; 2], half_size: [f32; 2], margin: f32, code: u8) -> [f32; 2] {
    let (dx, dy) = (half_size[0] + margin, half_size[1] + margin);
    match code {
        OFFSET_TR => [anchor[0] + dx, anchor[1] - dy],
        OFFSET_TL => [anchor[0] - dx, anchor[1] - dy],
        OFFSET_BR => [anchor[0] + dx, anchor[1] + dy],
        OFFSET_BL => [anchor[0] - dx, anchor[1] + dy],
        OFFSET_R  => [anchor[0] + dx, anchor[1]],
        OFFSET_L  => [anchor[0] - dx, anchor[1]],
        OFFSET_T  => [anchor[0], anchor[1] - dy],
        OFFSET_B  => [anchor[0], anchor[1] + dy],
        _ => anchor,
    }
}

#[inline]
fn aabb_intersects(a: &[f32; 4], b: &[f32; 4]) -> bool {
    !(a[2] <= b[0] || a[0] >= b[2] || a[3] <= b[1] || a[1] >= b[3])
}

#[inline]
fn expand_aabb(target: &mut [f32; 4], other: &[f32; 4]) {
    target[0] = target[0].min(other[0]);
    target[1] = target[1].min(other[1]);
    target[2] = target[2].max(other[2]);
    target[3] = target[3].max(other[3]);
}

/// Internal R-Tree node for bounding-box screen spatial index.
#[derive(Debug, Clone, PartialEq)]
pub struct RTreeNode {
    pub bbox: [f32; 4],
    pub children: Vec<usize>,
    pub label: Option<PlacedLabel>,
}

/// Screen-space R-Tree collision structure.
#[derive(Debug, Clone, Default)]
pub struct RTreeScreenIndex {
    pub nodes: Vec<RTreeNode>,
    pub root: Option<usize>,
}

impl RTreeScreenIndex {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), root: None }
    }

    pub fn collides(&self, bbox: &[f32; 4]) -> bool {
        let Some(root_idx) = self.root else { return false; };
        let mut stack = [0usize; 64];
        let mut top = 1;
        stack[0] = root_idx;
        while top > 0 {
            top -= 1;
            let node = &self.nodes[stack[top]];
            if aabb_intersects(&node.bbox, bbox) {
                if node.label.is_some() { return true; }
                for &child_idx in &node.children {
                    if top < stack.len() {
                        stack[top] = child_idx;
                        top += 1;
                    }
                }
            }
        }
        false
    }

    pub fn insert(&mut self, label: PlacedLabel) {
        let leaf_idx = self.nodes.len();
        self.nodes.push(RTreeNode { bbox: label.aabb_min_max_px, children: Vec::new(), label: Some(label) });
        match self.root {
            None => {
                let root_idx = self.nodes.len();
                self.nodes.push(RTreeNode { bbox: label.aabb_min_max_px, children: vec![leaf_idx], label: None });
                self.root = Some(root_idx);
            }
            Some(r_idx) => {
                expand_aabb(&mut self.nodes[r_idx].bbox, &label.aabb_min_max_px);
                self.nodes[r_idx].children.push(leaf_idx);
            }
        }
    }
}

/// Screen-space label collision solver with 8-position Imhof offset resolution.
///
/// Tier-2 ColdBounded: respects priority ordering, enforces byte budget fail-closed.
pub fn solve_screen_space_label_collisions(
    candidates: &[LabelCandidate],
    viewport_wh_px: [f32; 2],
    margin_px: f32,
    byte_budget: usize,
) -> Result<Vec<PlacedLabel>, GeoLabelError> {
    let estimated_bytes = candidates.len() * (core::mem::size_of::<PlacedLabel>() + core::mem::size_of::<RTreeNode>() + 32) + 128;
    if estimated_bytes > byte_budget {
        return Err(GeoLabelError::ByteBudgetExceeded { requested: estimated_bytes, budget: byte_budget });
    }

    let mut sorted_indices: Vec<usize> = (0..candidates.len()).collect();
    sorted_indices.sort_by(|&a, &b| {
        candidates[b].priority
            .partial_cmp(&candidates[a].priority)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then_with(|| candidates[a].id.cmp(&candidates[b].id))
    });

    let mut rtree = RTreeScreenIndex::new();
    let mut placed_labels = Vec::with_capacity(candidates.len());

    for &idx in &sorted_indices {
        let cand = &candidates[idx];
        let (hw, hh) = (cand.bbox_half_px[0], cand.bbox_half_px[1]);

        let trial_codes: &[u8] = if cand.allow_8_way_offset {
            &[OFFSET_TR, OFFSET_TL, OFFSET_BR, OFFSET_BL, OFFSET_R, OFFSET_L, OFFSET_T, OFFSET_B]
        } else {
            &[OFFSET_CENTER]
        };

        for &code in trial_codes {
            let center = compute_imhof_candidate_center(cand.anchor_px, cand.bbox_half_px, margin_px, code);
            let aabb = [center[0] - hw, center[1] - hh, center[0] + hw, center[1] + hh];

            if aabb[0] >= 0.0 && aabb[1] >= 0.0 && aabb[2] <= viewport_wh_px[0] && aabb[3] <= viewport_wh_px[1] {
                if !rtree.collides(&aabb) {
                    let placed = PlacedLabel {
                        id: cand.id,
                        final_center_px: center,
                        aabb_min_max_px: aabb,
                        offset_position_code: code,
                    };
                    rtree.insert(placed);
                    placed_labels.push(placed);
                    break;
                }
            }
        }
    }

    Ok(placed_labels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curved_glyph_quad_size() {
        assert_eq!(core::mem::size_of::<CurvedGlyphQuad>(), 32);
        assert_eq!(core::mem::align_of::<CurvedGlyphQuad>(), 4);
    }

    #[test]
    fn test_zoom_stop_interpolation() {
        let stops = [
            StyleZoomStop { zoom: 10.0, value: 2.0 },
            StyleZoomStop { zoom: 12.0, value: 8.0 },
            StyleZoomStop { zoom: 14.0, value: 20.0 },
        ];
        assert_eq!(evaluate_style_zoom_interpolated(&stops, 8.0, 1.0), 2.0);
        assert_eq!(evaluate_style_zoom_interpolated(&stops, 15.0, 1.0), 20.0);

        let linear_val = evaluate_style_zoom_interpolated(&stops, 11.0, 1.0);
        assert!((linear_val - 5.0).abs() < 1e-4);

        let exp_val = evaluate_style_zoom_interpolated(&stops, 11.0, 2.0);
        assert!((exp_val - 4.0).abs() < 1e-4);
    }

    #[test]
    fn test_curve_following_glyph_placement_and_hairpin_rejection() {
        let gentle_river = [[0.0, 0.0], [50.0, 5.0], [100.0, 15.0], [150.0, 30.0], [200.0, 50.0]];
        let max_turn = core::f32::consts::PI / 4.0;
        let res_gentle = layout_glyphs_along_centerline(&gentle_river, "RIVER", 16.0, 12.0, max_turn, 0xFFFFFFFF, 65536);
        assert!(res_gentle.is_ok());
        let quads = res_gentle.unwrap();
        assert_eq!(quads.len(), 5);

        let hairpin_bend = [[0.0, 0.0], [100.0, 0.0], [10.0, 30.0]];
        let res_hairpin = layout_glyphs_along_centerline(&hairpin_bend, "RIVER", 20.0, 12.0, max_turn, 0xFFFFFFFF, 65536);
        match res_hairpin {
            Err(GeoLabelError::MaxCurvatureExceeded { turn_angle_rad, .. }) => {
                assert!(turn_angle_rad > 2.0);
            }
            other => panic!("expected MaxCurvatureExceeded, got {other:?}"),
        }
    }

    #[test]
    fn test_screen_space_label_collision_and_imhof_offsets() {
        let candidates = vec![
            LabelCandidate {
                id: 1,
                text: "Alpha".to_string(),
                anchor_px: [200.0, 200.0],
                bbox_half_px: [20.0, 10.0],
                priority: 10.0,
                allow_8_way_offset: true,
            },
            LabelCandidate {
                id: 2,
                text: "Beta".to_string(),
                anchor_px: [200.0, 200.0],
                bbox_half_px: [20.0, 10.0],
                priority: 9.0,
                allow_8_way_offset: true,
            },
        ];

        let placed = solve_screen_space_label_collisions(&candidates, [800.0, 600.0], 4.0, 65536).unwrap();
        assert_eq!(placed.len(), 2);
        assert_eq!(placed[0].id, 1);
        assert_eq!(placed[0].offset_position_code, OFFSET_TR);
        assert_eq!(placed[1].id, 2);
        assert_eq!(placed[1].offset_position_code, OFFSET_TL);

        let budget_err = solve_screen_space_label_collisions(&candidates, [800.0, 600.0], 4.0, 10);
        match budget_err {
            Err(GeoLabelError::ByteBudgetExceeded { .. }) => {}
            other => panic!("expected ByteBudgetExceeded, got {other:?}"),
        }
    }
}
