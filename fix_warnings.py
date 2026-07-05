import os
import re

def main():
    base_dir = r"c:\Projects\qualia-27062026\crates\qualia-core-db\src"
    
    replacements = {
        r"container_10d\conformance.rs": [
            (r"use crate::container_10d::crc32c::crc32c;\n", r""),
            (r"AlignmentTier, SectionDescriptor, SECTION_DESCRIPTOR_SIZE, SectionInput, SectionType,", r"SectionDescriptor, SECTION_DESCRIPTOR_SIZE,"),
            (r"read_node, write_node_section_aos, NodeMiniHeader, NODE_MINI_HEADER_SIZE,", r"read_node, write_node_section_aos, NodeMiniHeader,"),
        ],
        r"container_10d\integrity.rs": [
            (r"use crate::container_10d::header::{Container10dHeader, HEADER_BYTE_SIZE};", r"use crate::container_10d::header::HEADER_BYTE_SIZE;"),
        ],
        r"container_10d\node_section.rs": [
            (r"use crate::container_10d::axis_role::AXIS_ORDER;\n", r""),
        ],
        r"specialized_libs\computational_geometry\alpha_shape.rs": [
            (r"fn circle_points\(n: usize, r: f64\) -> Vec<Point2> \{", r"#[allow(dead_code)]\n    fn circle_points(n: usize, r: f64) -> Vec<Point2> {"),
            (r"let \(tc, ec, report\)", r"let (tc, _ec, report)"),
        ],
        r"specialized_libs\computational_geometry\authoring.rs": [
            (r"pub fn asset_encoded_len\(mesh: &Mesh, provenance: &ProvenanceMetadata\) -> usize", r"pub fn asset_encoded_len(mesh: &Mesh, _provenance: &ProvenanceMetadata) -> usize"),
        ],
        r"specialized_libs\computational_geometry\boolean_3.rs": [
            (r"tri: \[Point3; 3\],", r"_tri: [Point3; 3],"),
            (r"let \[a, b, c\] = tri;\n", r""),
            (r"fn vertex_edge_split\(tri: \[Point3; 3\], vertex: usize, edge: usize, vp: Point3", r"fn vertex_edge_split(tri: [Point3; 3], vertex: usize, edge: usize, _vp: Point3"),
        ],
        r"specialized_libs\computational_geometry\box_join.rs": [
            (r"use super::spatial_order::sort_by_morton_3d;\n", r""),
        ],
        r"specialized_libs\computational_geometry\constrained_delaunay.rs": [
            (r"use super::delaunay_2::\{delaunay_triangulation_2, verify_delaunay\};", r"use super::delaunay_2::delaunay_triangulation_2;"),
        ],
        r"specialized_libs\computational_geometry\corpus_4.rs": [
            (r"use super::delaunay_2::\{delaunay_triangulation_2, triangulation_hash, verify_delaunay, DelaunayError\};", r"use super::delaunay_2::{delaunay_triangulation_2, triangulation_hash, verify_delaunay};"),
            (r"use super::hull::\{convex_hull_indices_2, HullError\};\n", r""),
            (r"pub struct OrientationVector \{", r"#[allow(dead_code)]\npub struct OrientationVector {"),
            (r"pub struct HullVector \{", r"#[allow(dead_code)]\npub struct HullVector {"),
            (r"pub struct IncircleVector \{", r"#[allow(dead_code)]\npub struct IncircleVector {"),
            (r"pub struct DelaunayVector \{", r"#[allow(dead_code)]\npub struct DelaunayVector {"),
        ],
        r"specialized_libs\computational_geometry\decimate_3.rs": [
            (r"let mut consider = \|best:", r"let consider = |best:"),
        ],
        r"specialized_libs\computational_geometry\delaunay_2.rs": [
            (r"use super::hull::convex_hull_indices_2;\n", r""),
            (r"let mut hull_out = \[0u32; 0\];", r"let _hull_out = [0u32; 0];"),
            (r"let mid_y =", r"let _mid_y ="),
        ],
        r"specialized_libs\computational_geometry\determinism_corpus.rs": [
            (r"pub const PINNED_CORPUS_HASH: u64", r"#[allow(dead_code)]\npub const PINNED_CORPUS_HASH: u64"),
        ],
        r"specialized_libs\computational_geometry\exact_kernel.rs": [
            (r"two_diff, two_product, two_sum, Sign,", r"two_diff, two_product, Sign,"),
            (r"let cx_num =", r"let _cx_num ="),
            (r"let big_val =", r"let _big_val ="),
        ],
        r"specialized_libs\computational_geometry\exact_test_helper.rs": [
            (r"pub fn equals", r"#[allow(dead_code)]\n    pub fn equals"),
            (r"fn normalize\(mut self\)", r"#[allow(dead_code)]\n    fn normalize(mut self)"),
            (r"pub fn expansion_to_exact", r"#[allow(dead_code)]\npub fn expansion_to_exact"),
        ],
        r"specialized_libs\computational_geometry\gpu.rs": [
            (r"use super::bvh::\{BvhNode, MAX_BVH_DEPTH\};\n", r""),
        ],
        r"specialized_libs\computational_geometry\hull.rs": [
            (r"use super::\{FilteredF64Kernel, GeometryKernel\};", r"use super::FilteredF64Kernel;"),
        ],
        r"specialized_libs\computational_geometry\kd_tree.rs": [
            (r"use super::primitives::Point3;\n", r""),
            (r"let split_val =", r"let _split_val ="),
        ],
        r"specialized_libs\computational_geometry\persistence.rs": [
            (r"use super::super::vr_filtration::\{vr_filtration, spatial_distance\};", r"use super::super::vr_filtration::vr_filtration;"),
        ],
        r"specialized_libs\computational_geometry\polygon_soup.rs": [
            (r"fn run_pipeline\(", r"#[allow(dead_code)]\n    fn run_pipeline("),
            (r"fn report_only\(", r"#[allow(dead_code)]\n    fn report_only("),
        ],
        r"specialized_libs\computational_geometry\query_frontend.rs": [
            (r"use super::bvh::\{BvhNode, MAX_BVH_DEPTH\};\n", r""),
            (r"use super::kd_tree::\{KdNode, MAX_KD_DEPTH\};\n", r""),
            (r"build_bvh_recursive, build_kd_tree_3d, Aabb, Point3, BVH_NODE_SIZE, KD_NODE_SIZE,", r"build_bvh_recursive, build_kd_tree_3d, Aabb, Point3,"),
        ],
        r"specialized_libs\computational_geometry\reconstruct_3d.rs": [
            (r"let phi = ", r"let _phi = "),
        ],
        r"specialized_libs\computational_geometry\statistical_manifold.rs": [
            (r"let q = ", r"let _q = "),
        ],
        r"specialized_libs\computational_geometry\tri_tri_3.rs": [
            (r"let mut g = \|rng:", r"let g = |rng:"),
        ],
    }

    for rel_path, reps in replacements.items():
        path = os.path.join(base_dir, rel_path)
        with open(path, "r", encoding="utf-8") as f:
            content = f.read()
        
        for search, replace in reps:
            content = re.sub(search, replace, content)
            
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
            
    print("Done")

if __name__ == "__main__":
    main()
