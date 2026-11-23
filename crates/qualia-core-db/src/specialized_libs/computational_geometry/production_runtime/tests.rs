use crate::container_10d::cluster_dag_section::{
    cluster_dag_encoded_len, decode_cluster_dag_section, encode_cluster_dag_section,
    encode_oct16_normal, ClusterDagMiniHeader, MeshletClusterNode, FLAG_CDAG_CONE_CULLED,
    FLAG_CDAG_SEAM_LOCKED, MAX_CLUSTER_DAG_NODES, MAX_MESHLET_TRIANGLES, MAX_MESHLET_VERTICES,
};
use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::{
    CapacityPlan, ProductionBudget,
};
use crate::specialized_libs::computational_geometry::geometry_workspace::Cancellation;
use crate::specialized_libs::computational_geometry::production_contracts::ContentId;
use bytemuck::Zeroable;

use super::*;

fn cid(tag: &str) -> ContentId {
    ContentId::sha256(tag.as_bytes())
}

fn with_budget<T>(work: u64, f: impl FnOnce(&mut ProductionBudget<'_>) -> T) -> T {
    let cancel = Cancellation::new();
    let mut budget =
        ProductionBudget::admit_default(CapacityPlan::default(), work, &cancel).unwrap();
    f(&mut budget)
}

fn cluster(
    sphere: [f32; 4],
    parent_sphere: [f32; 4],
    err_self: f32,
    err_parent: f32,
    lod: u8,
    child_group_first: u16,
    vertex_off: u32,
    tri_off: u32,
) -> MeshletClusterNode {
    MeshletClusterNode {
        bounding_sphere: sphere,
        parent_error_sphere: parent_sphere,
        lod_error_self: err_self,
        lod_error_parent: err_parent,
        cone_axis_oct16: encode_oct16_normal([0.0, 0.0, 1.0]),
        cone_cutoff_snorm16: -16383,
        lod_level: lod,
        vertex_count: 8,
        triangle_count: 4,
        material_slot: 0,
        child_group_first,
        vertex_data_offset: vertex_off,
        triangle_data_offset: tri_off,
        semantic_subpart_id: 1,
    }
}

struct Forest {
    nodes: [MeshletClusterNode; 6],
    links: [ReplacementLink; 6],
    children: [u32; 4],
    roots: [u32; 2],
    order: [u32; 8],
    source: ContentId,
}

impl Forest {
    fn new() -> Self {
        let border_a = cid("border-a");
        let border_e = cid("border-e");
        let r0 = [0.0, 0.0, 0.0, 8.0];
        let a = [-2.0, 0.0, 0.0, 4.0];
        let b = [3.0, 0.0, 0.0, 2.0];
        let c = [-3.0, 0.0, 0.0, 2.0];
        let d = [-1.0, 0.0, 0.0, 2.0];
        let e = [20.0, 0.0, 0.0, 4.0];
        Self {
            nodes: [
                cluster(r0, r0, 0.08, f32::INFINITY, 2, 0, 0, 0),
                cluster(a, r0, 0.02, 0.08, 1, 1, 8, 12),
                cluster(b, r0, 0.0, 0.08, 0, 0xFFFF, 16, 24),
                cluster(c, a, 0.0, 0.02, 0, 0xFFFF, 24, 36),
                cluster(d, a, 0.0, 0.02, 0, 0xFFFF, 32, 48),
                cluster(e, e, 0.04, f32::INFINITY, 1, 0xFFFF, 40, 60),
            ],
            links: [
                ReplacementLink {
                    parent: None,
                    children_start: 0,
                    children_count: 2,
                    coverage_start: 0,
                    coverage_count: 6,
                    border_identity: border_a,
                },
                ReplacementLink {
                    parent: Some(0),
                    children_start: 2,
                    children_count: 2,
                    coverage_start: 0,
                    coverage_count: 4,
                    border_identity: border_a,
                },
                ReplacementLink {
                    parent: Some(0),
                    children_start: 0,
                    children_count: 0,
                    coverage_start: 4,
                    coverage_count: 2,
                    border_identity: border_a,
                },
                ReplacementLink {
                    parent: Some(1),
                    children_start: 0,
                    children_count: 0,
                    coverage_start: 0,
                    coverage_count: 2,
                    border_identity: border_a,
                },
                ReplacementLink {
                    parent: Some(1),
                    children_start: 0,
                    children_count: 0,
                    coverage_start: 2,
                    coverage_count: 2,
                    border_identity: border_a,
                },
                ReplacementLink {
                    parent: None,
                    children_start: 0,
                    children_count: 0,
                    coverage_start: 6,
                    coverage_count: 2,
                    border_identity: border_e,
                },
            ],
            children: [1, 2, 3, 4],
            roots: [0, 5],
            order: [0, 1, 2, 3, 4, 5, 6, 7],
            source: cid("source-forest"),
        }
    }

    fn admit<'a>(&'a self, budget: &mut ProductionBudget<'_>) -> ValidatedHierarchy<'a> {
        let mut seen = [false; 8];
        ValidatedHierarchy::admit(
            &self.nodes,
            &self.links,
            &self.children,
            &self.roots,
            self.source,
            &self.order,
            &mut seen,
            budget,
        )
        .expect("forest must admit")
    }
}

fn wide_frustum() -> [[f32; 4]; 6] {
    [
        [1.0, 0.0, 0.0, 1000.0],
        [-1.0, 0.0, 0.0, 1000.0],
        [0.0, 1.0, 0.0, 1000.0],
        [0.0, -1.0, 0.0, 1000.0],
        [0.0, 0.0, 1.0, 1000.0],
        [0.0, 0.0, -1.0, 1000.0],
    ]
}

fn view_at(camera: [f32; 3], tau: f32, occlusion: OcclusionHistory) -> ViewRequest {
    ViewRequest {
        projection: Projection::Perspective {
            viewport_height_px: 720.0,
            fov_y_rad: 60.0_f32.to_radians(),
        },
        camera_pos: camera,
        frustum_planes: wide_frustum(),
        near_plane_index: 4,
        tau_px: tau,
        two_sided: false,
        transform_max_scale: 1.0,
        deformation_radius: 0.0,
        occlusion,
    }
}

#[test]
fn existing_meshlet_and_dag_limits_stay_authoritative() {
    assert_eq!(MAX_MESHLET_VERTICES, 64);
    assert_eq!(MAX_MESHLET_TRIANGLES, 124);
    assert_eq!(MAX_CLUSTER_DAG_NODES, 262_144);
    assert_eq!(core::mem::size_of::<MeshletClusterNode>(), 64);
    assert_eq!(GPU_CLUSTER_NODE_STRIDE, 12);
    assert_eq!(GPU_CULL_PARAM_SCALARS, 28);
    assert_eq!(GPU_CULL_WORKGROUP_SIZE, 64);
}

#[test]
fn admit_forest_and_dependency_identities() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        assert_eq!(hierarchy.roots(), &[0, 5]);
        assert_eq!(hierarchy.coverage(), 8);
        assert_eq!(hierarchy.children_of(0), &[1, 2]);
        assert_eq!(hierarchy.children_of(1), &[3, 4]);
        assert!(hierarchy.children_of(5).is_empty());
        let id = hierarchy.dependency_identity(3);
        assert_eq!(id.parent, Some(1));
        assert_eq!(id.border, forest.links[1].border_identity);
        assert_eq!(id.coverage_start, 0);
        assert_eq!(id.coverage_count, 2);
        assert_eq!(id.source, forest.source);
    });
}

#[test]
fn legal_cuts_are_contiguous_coverage_partitions() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        for cut in [&[0u32, 5][..], &[1, 2, 5][..], &[3, 4, 2, 5][..]] {
            hierarchy.validate_cut(cut, budget).unwrap();
        }
    });
}

#[test]
fn illegal_cuts_holes_duplicates_order_and_sibling_subset() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        let illegal: &[&[u32]] = &[
            &[0],
            &[1, 5],
            &[0, 1, 5],
            &[3, 4, 2],
            &[2, 1, 5],
            &[1],
            &[3, 4, 1, 2, 5],
            &[5, 0],
            &[],
        ];
        for cut in illegal {
            assert_eq!(
                hierarchy.validate_cut(cut, budget),
                Err(RuntimeError::InvalidCut)
            );
        }
    });
}

#[test]
fn random_subsets_match_exactly_the_three_legal_cuts() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        let mut legal = 0u32;
        let mut buf = [0u32; 6];
        for mask in 0u32..64 {
            let mut n = 0usize;
            for i in 0..6u32 {
                if mask & (1 << i) != 0 {
                    buf[n] = i;
                    n += 1;
                }
            }
            if hierarchy.validate_cut(&buf[..n], budget).is_ok() {
                legal += 1;
            }
        }
        // Index-sorted subsets only. [0,5] and [1,2,5] are both legal and
        // sorted; [3,4,2,5] is legal in coverage order and is checked above.
        assert_eq!(legal, 2);
    });
}

#[test]
fn admit_rejects_meshlet_overflow_and_child_outside_parent() {
    with_budget(10_000, |budget| {
        let mut forest = Forest::new();
        forest.nodes[2].vertex_count = 65;
        let mut seen = [false; 8];
        assert_eq!(
            match ValidatedHierarchy::admit(
                &forest.nodes,
                &forest.links,
                &forest.children,
                &forest.roots,
                forest.source,
                &forest.order,
                &mut seen,
                budget,
            ) {
                Err(e) => e,
                Ok(_) => panic!("expected invalid hierarchy"),
            },
            RuntimeError::InvalidHierarchy
        );
        forest.nodes[2].vertex_count = 8;
        forest.nodes[2].bounding_sphere = [20.0, 0.0, 0.0, 2.0];
        assert_eq!(
            match ValidatedHierarchy::admit(
                &forest.nodes,
                &forest.links,
                &forest.children,
                &forest.roots,
                forest.source,
                &forest.order,
                &mut seen,
                budget,
            ) {
                Err(e) => e,
                Ok(_) => panic!("expected invalid bounds"),
            },
            RuntimeError::InvalidBounds
        );
        forest.nodes[2].bounding_sphere = [3.0, 0.0, 0.0, 2.0];
        forest.links[2].border_identity = cid("other-seam");
        assert_eq!(
            match ValidatedHierarchy::admit(
                &forest.nodes,
                &forest.links,
                &forest.children,
                &forest.roots,
                forest.source,
                &forest.order,
                &mut seen,
                budget,
            ) {
                Err(e) => e,
                Ok(_) => panic!("expected invalid hierarchy"),
            },
            RuntimeError::InvalidHierarchy
        );
    });
}

#[test]
fn quantized_cdag_codec_roundtrip_readmits() {
    with_budget(10_000, |budget| {
        let mut forest = Forest::new();
        let needed = cluster_dag_encoded_len(forest.nodes.len()).unwrap();
        let mut bytes = [0u8; 512];
        let header = ClusterDagMiniHeader {
            max_lod_depth: 2,
            cluster_count: 6,
            group_count: 2,
            flags: FLAG_CDAG_SEAM_LOCKED | FLAG_CDAG_CONE_CULLED,
            ..ClusterDagMiniHeader::default()
        };
        let written = encode_cluster_dag_section(&header, &forest.nodes, &mut bytes).unwrap();
        assert_eq!(written, needed);
        let mut decoded = [MeshletClusterNode::zeroed(); 6];
        let (decoded_header, count) =
            decode_cluster_dag_section(&bytes[..written], &mut decoded).unwrap();
        assert_eq!(count, 6);
        assert_eq!(decoded_header.cluster_count, 6);
        assert_eq!(decoded, forest.nodes);
        forest.nodes = decoded;
        let _rehearsed = forest.admit(budget);
    });
}

#[test]
fn cpu_cut_reference_refines_by_projected_error() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        let mut stack = [0u32; 6];
        let mut cut = [0u32; 6];
        let coarse = hierarchy
            .cpu_cut_reference([0.0, 0.0, 40.0], 100.0, 1000.0, &mut stack, &mut cut, budget)
            .unwrap();
        assert_eq!(&cut[..coarse], &[0, 5]);
        let fine = hierarchy
            .cpu_cut_reference([0.0, 0.0, 40.0], 100.0, 0.0, &mut stack, &mut cut, budget)
            .unwrap();
        assert_eq!(&cut[..fine], &[3, 4, 2, 5]);
    });
}

#[test]
fn selection_is_tied_to_a_legal_cut_and_records_error_contract() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        let contract = cpu_vs_gpu_error_contract();
        assert!(contract.cpu_projects_world_error);
        assert!(contract.gpu_consumes_preprojected_px);
        assert_eq!(contract.gpu_node_stride_scalars, 12);
        assert!(CPU_VS_GPU_ERROR_CONTRACT.contains("preprojected"));
        let view = view_at([0.0, 0.0, 40.0], 0.0, OcclusionHistory::Frozen);
        let mut stack = [0u32; 6];
        let mut cut = [0u32; 6];
        let mut mask = [0u32; 6];
        let n = select_conservative(
            &hierarchy,
            &[view],
            &[],
            &mut stack,
            &mut cut,
            &mut mask,
            budget,
        )
        .unwrap();
        hierarchy.validate_cut(&cut[..n], budget).unwrap();
        assert_eq!(&cut[..n], &[3, 4, 2, 5]);
        assert!(mask[..n].iter().all(|&bit| bit == 1));
    });
}

#[test]
fn frozen_domain_does_not_disappear_under_stale_occlusion() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        let mut hzb = [None; 6];
        hzb[0] = Some([0.95, 0.10]);
        let mut stack = [0u32; 6];
        let mut cut = [0u32; 6];
        let mut mask = [0u32; 6];
        let frozen = view_at([0.0, 0.0, 40.0], 1000.0, OcclusionHistory::Frozen);
        let n = select_conservative(
            &hierarchy,
            &[frozen],
            &hzb,
            &mut stack,
            &mut cut,
            &mut mask,
            budget,
        )
        .unwrap();
        assert_eq!(&cut[..n], &[0, 5]);
        assert_eq!(mask[0], 1);
        let live = view_at(
            [0.0, 0.0, 40.0],
            1000.0,
            OcclusionHistory::Live {
                history_generation: 3,
                view_generation: 3,
            },
        );
        let _n = select_conservative(
            &hierarchy,
            &[live],
            &hzb,
            &mut stack,
            &mut cut,
            &mut mask,
            budget,
        )
        .unwrap();
        assert_eq!(mask[0], 0);
        let stale = view_at(
            [0.0, 0.0, 40.0],
            1000.0,
            OcclusionHistory::Live {
                history_generation: 1,
                view_generation: 9,
            },
        );
        let n = select_conservative(
            &hierarchy,
            &[stale],
            &hzb,
            &mut stack,
            &mut cut,
            &mut mask,
            budget,
        )
        .unwrap();
        assert_eq!(mask[0], 1);
        assert_eq!(n, 2);
    });
}

#[test]
fn near_plane_two_sided_deformation_and_orthographic_are_conservative() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        let mut stack = [0u32; 6];
        let mut cut = [0u32; 6];
        let mut mask = [0u32; 6];
        let mut tight = view_at([0.0, 0.0, 40.0], 1000.0, OcclusionHistory::None);
        tight.frustum_planes[1] = [-1.0, 0.0, 0.0, 10.0];
        let clipped = select_conservative(
            &hierarchy,
            &[tight],
            &[],
            &mut stack,
            &mut cut,
            &mut mask,
            budget,
        )
        .unwrap();
        assert_eq!(clipped, 2);
        assert_eq!(mask[1], 0);
        tight.deformation_radius = 20.0;
        let deformed = select_conservative(
            &hierarchy,
            &[tight],
            &[],
            &mut stack,
            &mut cut,
            &mut mask,
            budget,
        )
        .unwrap();
        assert_eq!(mask[1], 1);
        assert_eq!(deformed, 2);
        let mut back = view_at([0.0, 0.0, -40.0], 1000.0, OcclusionHistory::None);
        let culled = select_conservative(
            &hierarchy,
            &[back],
            &[],
            &mut stack,
            &mut cut,
            &mut mask,
            budget,
        )
        .unwrap();
        assert!(mask[..culled].iter().all(|&bit| bit == 0));
        back.two_sided = true;
        let kept = select_conservative(
            &hierarchy,
            &[back],
            &[],
            &mut stack,
            &mut cut,
            &mut mask,
            budget,
        )
        .unwrap();
        assert!(mask[..kept].iter().all(|&bit| bit == 1));
        let mut near = view_at([0.0, 0.0, -40.0], 1000.0, OcclusionHistory::None);
        near.frustum_planes[4] = [0.0, 0.0, 1.0, 1.0];
        let near_keep = select_conservative(
            &hierarchy,
            &[near],
            &[],
            &mut stack,
            &mut cut,
            &mut mask,
            budget,
        )
        .unwrap();
        assert!(mask[..near_keep].iter().all(|&bit| bit == 1));
        let ortho = ViewRequest {
            projection: Projection::Orthographic {
                viewport_height_px: 720.0,
                world_height: 24.0,
            },
            ..view_at([0.0, 0.0, 40.0], 1000.0, OcclusionHistory::None)
        };
        let perspective_err = projected_world_error(
            hierarchy.nodes()[0].bounding_sphere,
            0.08,
            [0.0, 0.0, 40.0],
            Projection::Perspective {
                viewport_height_px: 720.0,
                fov_y_rad: 60.0_f32.to_radians(),
            },
        )
        .unwrap();
        let ortho_err = projected_world_error(
            hierarchy.nodes()[0].bounding_sphere,
            0.08,
            [0.0, 0.0, 40.0],
            ortho.projection,
        )
        .unwrap();
        assert_ne!(perspective_err, ortho_err);
        select_conservative(
            &hierarchy,
            &[ortho],
            &[],
            &mut stack,
            &mut cut,
            &mut mask,
            budget,
        )
        .unwrap();
    });
}

#[test]
fn gpu_abi_pack_binds_independent_masks_to_the_legal_cut() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        let view = view_at([0.0, 0.0, 40.0], 1000.0, OcclusionHistory::None);
        let cut = [0u32, 5];
        let mut scalars = [0.0f32; 6 * GPU_CLUSTER_NODE_STRIDE];
        let mut on_cut = [false; 6];
        pack_gpu_clusters_for_legal_cut(
            &hierarchy,
            &cut,
            &view,
            &[],
            &mut scalars,
            &mut on_cut,
            budget,
        )
        .unwrap();
        assert!(on_cut[0] && on_cut[5] && !on_cut[1]);
        let tau = view.tau_px;
        for i in 0..6 {
            let err_self = scalars[i * GPU_CLUSTER_NODE_STRIDE + 8];
            let err_parent = scalars[i * GPU_CLUSTER_NODE_STRIDE + 9];
            let gpu_selected = err_self <= tau && tau < err_parent;
            assert_eq!(gpu_selected, on_cut[i]);
        }
        let mut params = [0.0f32; GPU_CULL_PARAM_SCALARS];
        pack_gpu_cull_params(&view.frustum_planes, view.camera_pos, tau, &mut params).unwrap();
        assert_eq!(params[27], tau);
        assert_eq!(params[24], 0.0);
    });
}

#[test]
fn residency_pins_fallback_and_refuses_incomplete_replacement() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        let mut states = [PageState::Absent; 6];
        let mut bytes = [100u32, 50, 50, 25, 25, 100];
        let mut table = ResidencyTable::admit(&hierarchy, &mut states, &mut bytes, 400, budget)
            .unwrap();
        assert_eq!(table.state(0), PageState::Pinned);
        assert_eq!(table.state(5), PageState::Pinned);
        let mut stack = [0u32; 6];
        let mut cut = [0u32; 6];
        let n = table
            .renderable_cut(&hierarchy, &mut stack, &mut cut, budget)
            .unwrap();
        assert_eq!(&cut[..n], &[0, 5]);
        table.begin_transfer(1, 11, 50, budget).unwrap();
        table.begin_transfer(2, 12, 50, budget).unwrap();
        assert_eq!(
            table.activate_replacement(&hierarchy, 0, budget),
            Err(RuntimeError::MissingFallback)
        );
        table.complete_transfer(1, 11, budget).unwrap();
        table.complete_transfer(2, 12, budget).unwrap();
        table.activate_replacement(&hierarchy, 0, budget).unwrap();
        let n = table
            .renderable_cut(&hierarchy, &mut stack, &mut cut, budget)
            .unwrap();
        assert_eq!(&cut[..n], &[1, 2, 5]);
        table.begin_transfer(3, 13, 25, budget).unwrap();
        table.cancel_transfer(3, 13, budget).unwrap();
        assert_eq!(table.state(3), PageState::Cancelled);
        assert_eq!(
            table.activate_replacement(&hierarchy, 1, budget),
            Err(RuntimeError::MissingFallback)
        );
        table.evict(&hierarchy, 2, budget).unwrap();
        let n = table
            .renderable_cut(&hierarchy, &mut stack, &mut cut, budget)
            .unwrap();
        assert_eq!(&cut[..n], &[0, 5]);
        assert_eq!(
            table.evict(&hierarchy, 0, budget),
            Err(RuntimeError::MissingFallback)
        );
    });
}

#[test]
fn residency_cap_and_random_inflight_completion_keep_a_cut() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        let mut states = [PageState::Absent; 6];
        let mut bytes = [100u32, 50, 50, 25, 25, 100];
        let mut table =
            ResidencyTable::admit(&hierarchy, &mut states, &mut bytes, 250, budget).unwrap();
        assert_eq!(
            table.begin_transfer(1, 1, 51, budget),
            Err(RuntimeError::Capacity)
        );
        let mut states = [PageState::Absent; 6];
        let sizes = [100u32, 40, 40, 20, 20, 100];
        let mut bytes = sizes;
        let mut table =
            ResidencyTable::admit(&hierarchy, &mut states, &mut bytes, 400, budget).unwrap();
        for (i, id) in [(1u32, 1u32), (2, 2), (3, 3), (4, 4)] {
            table
                .begin_transfer(i as usize, id, sizes[i as usize], budget)
                .unwrap();
        }
        let order = [4u32, 1, 3, 2];
        let mut stack = [0u32; 6];
        let mut cut = [0u32; 6];
        for id in order {
            table.complete_transfer(id as usize, id, budget).unwrap();
            table
                .renderable_cut(&hierarchy, &mut stack, &mut cut, budget)
                .unwrap();
        }
        table
            .activate_replacement(&hierarchy, 0, budget)
            .unwrap();
        table
            .activate_replacement(&hierarchy, 1, budget)
            .unwrap();
    });
}

#[test]
fn compaction_zero_all_alternating_exact_and_overflow() {
    with_budget(10_000, |budget| {
        let forest = Forest::new();
        let hierarchy = forest.admit(budget);
        let cut = [3u32, 4, 2, 5];
        hierarchy.validate_cut(&cut, budget).unwrap();
        let mut prefix = [0u32; 4];
        let mut compact = [0u32; 4];
        let mut commands = [DrawIndexedCommand::EMPTY; 4];
        for cmd in &mut commands {
            cmd.index_count = u32::MAX;
            cmd.instance_count = u32::MAX;
        }
        let zero_mask = [0u32; 4];
        let gen = compact_and_draw(
            &hierarchy,
            &cut,
            &zero_mask,
            &mut prefix,
            &mut compact,
            &mut commands,
            budget,
        )
        .unwrap();
        assert_eq!(gen.compacted, 0);
        assert_eq!(gen.dispatch_x, 0);
        assert!(commands.iter().all(|c| *c == DrawIndexedCommand::EMPTY));
        let all = [1u32; 4];
        let gen = compact_and_draw(
            &hierarchy,
            &cut,
            &all,
            &mut prefix,
            &mut compact,
            &mut commands,
            budget,
        )
        .unwrap();
        assert_eq!(gen.compacted, 4);
        assert_eq!(gen.dispatch_x, 1);
        assert_eq!(commands[0].index_count, 12);
        assert_eq!(commands[0].instance_count, 1);
        let alt = [1u32, 0, 1, 0];
        let gen = compact_and_draw(
            &hierarchy,
            &cut,
            &alt,
            &mut prefix,
            &mut compact,
            &mut commands,
            budget,
        )
        .unwrap();
        assert_eq!(gen.compacted, 2);
        assert_eq!(&compact[..2], &[0, 2]);
        let mut small = [0u32; 1];
        let mut commands = [DrawIndexedCommand::EMPTY; 4];
        for command in &mut commands {
            command.index_count = 9;
            command.instance_count = 9;
            command.first_index = 9;
            command.base_vertex = 9;
            command.first_instance = 9;
        }
        assert_eq!(
            compact_and_draw(
                &hierarchy,
                &cut,
                &all,
                &mut prefix,
                &mut small,
                &mut commands,
                budget,
            ),
            Err(RuntimeError::Capacity)
        );
        assert!(commands.iter().all(|c| *c == DrawIndexedCommand::EMPTY));
        let mut exact = [0u32; 4];
        compact_and_draw(
            &hierarchy,
            &cut,
            &all,
            &mut prefix,
            &mut exact,
            &mut commands,
            budget,
        )
        .unwrap();
        assert_eq!(dispatch_workgroups(64).unwrap(), 1);
        assert_eq!(dispatch_workgroups(65).unwrap(), 2);
        assert_eq!(
            dispatch_workgroups(MAX_DISPATCH_WORKGROUPS.saturating_mul(GPU_CULL_WORKGROUP_SIZE) + 1),
            Err(RuntimeError::Capacity)
        );
    });
}

#[test]
fn activation_faults_and_rollback_do_not_undo_simulation() {
    with_budget(10_000, |budget| {
        let mut store = ActivationStore::new();
        store.declare_boundary(ActivationBoundary::Frame(7));
        let good = QualifiedSet {
            mesh: cid("mesh-v1"),
            shader: cid("shader-v1"),
            transfer: cid("xfer-v1"),
            evidence: cid("ev-v1"),
            mesh_generation: 1,
            shader_generation: 1,
            transfer_generation: 1,
            assets_complete: true,
            evidence_valid: true,
            boundary: ActivationBoundary::Frame(7),
        };
        store.activate(good, true, None, budget).unwrap();
        store.consume_simulation(4).unwrap();
        let next = QualifiedSet {
            mesh: cid("mesh-v2"),
            shader: cid("shader-v2"),
            transfer: cid("xfer-v2"),
            evidence: cid("ev-v2"),
            mesh_generation: 2,
            shader_generation: 2,
            transfer_generation: 2,
            assets_complete: true,
            evidence_valid: true,
            boundary: ActivationBoundary::Frame(7),
        };
        store.activate(next, true, None, budget).unwrap();
        assert_eq!(store.current().unwrap().mesh, cid("mesh-v2"));
        let restored = store.rollback(budget).unwrap();
        assert_eq!(restored.mesh, cid("mesh-v1"));
        assert_eq!(store.simulation_consumed(), 4);
        assert!(ROLLBACK_DOES_NOT_UNDO_SIMULATION);
        let mut incomplete = next;
        incomplete.assets_complete = false;
        assert_eq!(
            store.activate(incomplete, true, None, budget),
            Err(RuntimeError::MissingFallback)
        );
        let mut stale = next;
        stale.evidence_valid = false;
        assert_eq!(
            store.activate(stale, true, None, budget),
            Err(RuntimeError::StaleIdentity)
        );
        let mut mixed = next;
        mixed.shader_generation = 9;
        assert_eq!(
            store.activate(mixed, true, None, budget),
            Err(RuntimeError::StaleIdentity)
        );
        assert_eq!(
            store.activate(next, false, Some(ActivationFault::DeviceLost), budget),
            Err(RuntimeError::DeviceLost)
        );
        assert!(store.device_lost());
        assert_eq!(store.current().unwrap().mesh, cid("mesh-v1"));
        let mut wrong = next;
        wrong.boundary = ActivationBoundary::Job(1);
        store.declare_boundary(ActivationBoundary::Frame(7));
        assert_eq!(
            store.activate(wrong, true, None, budget),
            Err(RuntimeError::Unsupported)
        );
    });
}
