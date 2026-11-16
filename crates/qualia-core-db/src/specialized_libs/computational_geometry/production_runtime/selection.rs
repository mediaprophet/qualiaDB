//! P16: conservative view/deformation selection bound to a P11 legal cut.
//! Inspects, but does not change, the existing 12-scalar GPU cull/LOD ABI.

use crate::container_10d::cluster_dag_section::{
    decode_oct16_normal, project_sphere_error_px,
};
use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::ProductionBudget;

use super::{RuntimeError, ValidatedHierarchy};

/// Existing Etalon `cull_and_lod` cluster node stride in f32 scalars.
pub const GPU_CLUSTER_NODE_STRIDE: usize = 12;
/// Existing Etalon `params` buffer: 6 planes + eye + tau_px.
pub const GPU_CULL_PARAM_SCALARS: usize = 28;
/// Existing Etalon compute workgroup size. Do not silently change it.
pub const GPU_CULL_WORKGROUP_SIZE: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    Perspective {
        viewport_height_px: f32,
        fov_y_rad: f32,
    },
    Orthographic {
        viewport_height_px: f32,
        world_height: f32,
    },
}

/// Occlusion history is ignored when frozen or when the generation is stale.
/// Stale HZB must not cause disappearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcclusionHistory {
    None,
    Frozen,
    Live {
        history_generation: u64,
        view_generation: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewRequest {
    pub projection: Projection,
    pub camera_pos: [f32; 3],
    pub frustum_planes: [[f32; 4]; 6],
    pub near_plane_index: u8,
    pub tau_px: f32,
    pub two_sided: bool,
    pub transform_max_scale: f32,
    pub deformation_radius: f32,
    pub occlusion: OcclusionHistory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorContractDifference {
    pub cpu_projects_world_error: bool,
    pub gpu_consumes_preprojected_px: bool,
    pub gpu_node_stride_scalars: usize,
    pub gpu_param_scalars: usize,
    pub gpu_workgroup_size: u32,
}

/// CPU projects world-space metres; the GPU kernel consumes preprojected pixels.
pub const CPU_VS_GPU_ERROR_CONTRACT: &str = "CPU cut reference projects world-space lod_error through project_sphere_error_px (perspective: error * proj_scale / surface_dist; orthographic: error * viewport_height / world_height). The existing Etalon GPU ABI stores 12 f32 scalars per node and consumes preprojected pixel errors at slots [8]=err_self_px and [9]=err_parent_px; it does not re-project world metres. Independent GPU per-node masks are not a legal cut; P16 binds visibility to P11 contiguous coverage cuts.";

pub fn cpu_vs_gpu_error_contract() -> ErrorContractDifference {
    ErrorContractDifference {
        cpu_projects_world_error: true,
        gpu_consumes_preprojected_px: true,
        gpu_node_stride_scalars: GPU_CLUSTER_NODE_STRIDE,
        gpu_param_scalars: GPU_CULL_PARAM_SCALARS,
        gpu_workgroup_size: GPU_CULL_WORKGROUP_SIZE,
    }
}

pub fn projection_scale(projection: Projection) -> Result<f32, RuntimeError> {
    match projection {
        Projection::Perspective {
            viewport_height_px,
            fov_y_rad,
        } => {
            if !viewport_height_px.is_finite()
                || viewport_height_px <= 0.0
                || !fov_y_rad.is_finite()
                || fov_y_rad <= 0.0
                || fov_y_rad >= core::f32::consts::PI
            {
                return Err(RuntimeError::InvalidView);
            }
            let half_tan = (0.5 * fov_y_rad).tan();
            if !(half_tan > 1e-6) || !half_tan.is_finite() {
                return Err(RuntimeError::InvalidView);
            }
            Ok(viewport_height_px / (2.0 * half_tan))
        }
        Projection::Orthographic {
            viewport_height_px,
            world_height,
        } => {
            if !viewport_height_px.is_finite()
                || viewport_height_px <= 0.0
                || !world_height.is_finite()
                || world_height <= 0.0
            {
                return Err(RuntimeError::InvalidView);
            }
            Ok(viewport_height_px / world_height)
        }
    }
}

pub fn projected_world_error(
    sphere: [f32; 4],
    world_error: f32,
    camera_pos: [f32; 3],
    projection: Projection,
) -> Result<f32, RuntimeError> {
    let scale = projection_scale(projection)?;
    match projection {
        Projection::Perspective { .. } => {
            Ok(project_sphere_error_px(sphere, world_error, camera_pos, scale))
        }
        Projection::Orthographic { .. } => {
            if world_error.is_infinite() {
                Ok(f32::INFINITY)
            } else if !world_error.is_finite() || world_error < 0.0 {
                Err(RuntimeError::InvalidView)
            } else {
                Ok(world_error * scale)
            }
        }
    }
}

pub fn validate_view(view: &ViewRequest) -> Result<f32, RuntimeError> {
    if view.near_plane_index >= 6
        || !view.tau_px.is_finite()
        || view.tau_px < 0.0
        || !view.transform_max_scale.is_finite()
        || view.transform_max_scale <= 0.0
        || !view.deformation_radius.is_finite()
        || view.deformation_radius < 0.0
        || !view.camera_pos.iter().all(|x| x.is_finite())
        || !view
            .frustum_planes
            .iter()
            .all(|p| p.iter().all(|x| x.is_finite()))
    {
        return Err(RuntimeError::InvalidView);
    }
    projection_scale(view.projection)
}

fn inflated_radius(node_radius: f32, view: &ViewRequest) -> f32 {
    node_radius * view.transform_max_scale + view.deformation_radius
}

fn plane_distance(plane: [f32; 4], center: [f32; 3]) -> f32 {
    plane[0] * center[0] + plane[1] * center[1] + plane[2] * center[2] + plane[3]
}

fn conservative_visible(
    center: [f32; 3],
    radius: f32,
    cone_axis: [f32; 3],
    cone_cutoff: f32,
    view: &ViewRequest,
    hzb: Option<[f32; 2]>,
) -> bool {
    let radius = inflated_radius(radius, view);
    for plane in &view.frustum_planes {
        if plane_distance(*plane, center) < -radius {
            return false;
        }
    }
    let to_eye = [
        view.camera_pos[0] - center[0],
        view.camera_pos[1] - center[1],
        view.camera_pos[2] - center[2],
    ];
    let cam_dist = (to_eye[0] * to_eye[0] + to_eye[1] * to_eye[1] + to_eye[2] * to_eye[2]).sqrt();
    let near = view.frustum_planes[view.near_plane_index as usize];
    let intersects_near = plane_distance(near, center).abs() <= radius;
    let camera_inside = cam_dist <= radius;
    if !view.two_sided && !intersects_near && !camera_inside {
        let dist = cam_dist.max(1.0e-6);
        let view_dir = [to_eye[0] / dist, to_eye[1] / dist, to_eye[2] / dist];
        let facing =
            view_dir[0] * cone_axis[0] + view_dir[1] * cone_axis[1] + view_dir[2] * cone_axis[2];
        if facing < cone_cutoff {
            return false;
        }
    }
    match view.occlusion {
        OcclusionHistory::None | OcclusionHistory::Frozen => {}
        OcclusionHistory::Live {
            history_generation,
            view_generation,
        } if history_generation != view_generation => {}
        OcclusionHistory::Live { .. } => {
            if let Some([min_depth, occluder]) = hzb {
                if min_depth > occluder {
                    return false;
                }
            }
        }
    }
    true
}

/// Finest legal cut wanted by any view, then conservative visibility (union).
/// A cluster is dropped only when every view can prove it is invisible.
pub fn select_conservative(
    hierarchy: &ValidatedHierarchy<'_>,
    views: &[ViewRequest],
    hzb_by_node: &[Option<[f32; 2]>],
    stack: &mut [u32],
    out_cut: &mut [u32],
    out_mask: &mut [u32],
    budget: &mut ProductionBudget<'_>,
) -> Result<usize, RuntimeError> {
    if views.is_empty() {
        return Err(RuntimeError::InvalidView);
    }
    for view in views {
        validate_view(view)?;
    }
    if !hzb_by_node.is_empty() && hzb_by_node.len() < hierarchy.nodes().len() {
        return Err(RuntimeError::Capacity);
    }
    let count = hierarchy.cpu_cut_by_refine(
        |index| {
            let node = &hierarchy.nodes()[index];
            let mut refine = false;
            for view in views {
                let error = projected_world_error(
                    node.bounding_sphere,
                    node.lod_error_self,
                    view.camera_pos,
                    view.projection,
                )?;
                refine |= error > view.tau_px;
            }
            Ok(refine)
        },
        stack,
        out_cut,
        budget,
    )?;
    if out_mask.len() < count {
        return Err(RuntimeError::Capacity);
    }
    for i in 0..count {
        budget.checkpoint(views.len() as u64)?;
        let index = out_cut[i] as usize;
        let node = &hierarchy.nodes()[index];
        let cone = decode_oct16_normal(node.cone_axis_oct16);
        let cutoff = node.cone_cutoff_snorm16 as f32 / 32767.0;
        let hzb = hzb_by_node.get(index).copied().flatten();
        let center = [
            node.bounding_sphere[0],
            node.bounding_sphere[1],
            node.bounding_sphere[2],
        ];
        let keep = views.iter().any(|view| {
            conservative_visible(center, node.bounding_sphere[3], cone, cutoff, view, hzb)
        });
        out_mask[i] = u32::from(keep);
    }
    Ok(count)
}

pub fn pack_gpu_cull_params(
    frustum_planes: &[[f32; 4]; 6],
    camera_eye: [f32; 3],
    tau_px: f32,
    out: &mut [f32],
) -> Result<(), RuntimeError> {
    if out.len() < GPU_CULL_PARAM_SCALARS {
        return Err(RuntimeError::Capacity);
    }
    for (plane_index, plane) in frustum_planes.iter().enumerate() {
        let base = plane_index * 4;
        out[base] = plane[0];
        out[base + 1] = plane[1];
        out[base + 2] = plane[2];
        out[base + 3] = plane[3];
    }
    out[24] = camera_eye[0];
    out[25] = camera_eye[1];
    out[26] = camera_eye[2];
    out[27] = tau_px;
    Ok(())
}

/// Pack the existing 12-scalar GPU node. `err_*_px` are preprojected pixels.
pub fn pack_gpu_cluster_node(
    node: &crate::container_10d::cluster_dag_section::MeshletClusterNode,
    err_self_px: f32,
    err_parent_px: f32,
    min_depth: f32,
    hzb_occluder_depth: f32,
    out: &mut [f32],
) -> Result<(), RuntimeError> {
    if out.len() < GPU_CLUSTER_NODE_STRIDE {
        return Err(RuntimeError::Capacity);
    }
    let cone = decode_oct16_normal(node.cone_axis_oct16);
    out[0] = node.bounding_sphere[0];
    out[1] = node.bounding_sphere[1];
    out[2] = node.bounding_sphere[2];
    out[3] = node.bounding_sphere[3];
    out[4] = cone[0];
    out[5] = cone[1];
    out[6] = cone[2];
    out[7] = node.cone_cutoff_snorm16 as f32 / 32767.0;
    out[8] = err_self_px;
    out[9] = err_parent_px;
    out[10] = min_depth;
    out[11] = hzb_occluder_depth;
    Ok(())
}

/// Bind GPU independent masks to a legal cut: non-cut nodes fail `err_self <= tau < err_parent`.
pub fn pack_gpu_clusters_for_legal_cut(
    hierarchy: &ValidatedHierarchy<'_>,
    cut: &[u32],
    view: &ViewRequest,
    hzb_by_node: &[Option<[f32; 2]>],
    out_scalars: &mut [f32],
    on_cut: &mut [bool],
    budget: &mut ProductionBudget<'_>,
) -> Result<(), RuntimeError> {
    validate_view(view)?;
    hierarchy.validate_cut(cut, budget)?;
    let n = hierarchy.nodes().len();
    if out_scalars.len() < n * GPU_CLUSTER_NODE_STRIDE || on_cut.len() < n {
        return Err(RuntimeError::Capacity);
    }
    on_cut[..n].fill(false);
    for &index in cut {
        on_cut[index as usize] = true;
    }
    for (index, node) in hierarchy.nodes().iter().enumerate() {
        budget.checkpoint(1)?;
        let slot = &mut out_scalars[index * GPU_CLUSTER_NODE_STRIDE
            ..index * GPU_CLUSTER_NODE_STRIDE + GPU_CLUSTER_NODE_STRIDE];
        let hzb = hzb_by_node.get(index).copied().flatten().unwrap_or([0.0, 1.0]);
        if on_cut[index] {
            let err_self = projected_world_error(
                node.bounding_sphere,
                node.lod_error_self,
                view.camera_pos,
                view.projection,
            )?;
            let err_parent = projected_world_error(
                node.parent_error_sphere,
                node.lod_error_parent,
                view.camera_pos,
                view.projection,
            )?;
            pack_gpu_cluster_node(node, err_self, err_parent, hzb[0], hzb[1], slot)?;
        } else {
            pack_gpu_cluster_node(node, view.tau_px + 1.0, view.tau_px, hzb[0], hzb[1], slot)?;
        }
    }
    Ok(())
}
