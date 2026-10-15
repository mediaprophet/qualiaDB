# Comprehensive Algorithmic Geometry Review: `openalea/plantgl` vs. QualiaDB (`computational_geometry`)

**Date:** 2026-10-14  
**Scope:** Comparative architectural and algorithmic audit of [`openalea/plantgl`](https://github.com/openalea/plantgl) (`pglmath`, `pglsg`, `pglalgo`, `light`) against QualiaDB's `crates/qualia-core-db/src/specialized_libs/computational_geometry` (P0–P18), `render/`, and `crates/vibe/` Host Catalog.  
**Licence Note:** PlantGL is licensed under **CeCILL-C** (weak copyleft, LGPL-like). Per QualiaDB's P10.6 fixture/oracle registry rules (`fixture_registry.rs`), **no code may be copied from PlantGL**; all recommended additions must be clean-room Rust implementations derived from public mathematical/academic literature (e.g., Runions et al. 2007, Xu et al. 2007, Livny et al. 2010, Tagliasacchi et al. 2009, Prusinkiewicz & Lindenmayer 1990, Piegl & Tiller *The NURBS Book*, Cescatti 1997).

---

## 1. Executive Summary

QualiaDB's `computational_geometry` library (P0–P18) already **surpasses** PlantGL in general-purpose exact computational geometry, topological rigor, and mesh generation:
- **Exactness & Robustness:** QualiaDB provides filtered + FMA-compensated + arbitrary-precision expansion predicates (`orient_2d`, `orient_3d`, `orient_3d_sos`, `incircle`, `insphere`), exact rational constructions (`ExactPoint2`, `tri_tri_intersect_3_exact`), Weiler 3D arrangements (`arrangement_3d.rs`), exact 3D mesh co-refinement & booleans (`corefine_3d.rs`, `boolean_3_exact`), and DCEL planar overlays (`dcel_overlay.rs`). PlantGL relies on floating-point approximations and optional external CGAL/ANN wrappers (`cgalwrap.h`, `annkdtree_p.h`).
- **Volumetric & Surface Meshing:** QualiaDB includes Ruppert Delaunay refinement (`delaunay_refine.rs`), 2D/3D advancing-front meshing (`advancing_front.rs`), 2:1 balanced quadtree/octree hex/tet meshing (`quad_octree_mesh.rs`), tetrahedral sliver exudation (`tet_quality_improve.rs`), anisotropic metric-tensor remeshing (`anisotropic_remesh.rs`), Screened-Poisson reconstruction (`screened_poisson.rs`), discrete differential geometry (`ddg_operators.rs`), and 10D persistent homology (`vr_filtration.rs`, `persistence.rs`).
- **Execution Architecture:** QualiaDB uses a Two-Tier Zero-Heap ABI (`HotZeroHeap` caller-buffered slices under the 42 MiB core Sentinel ceiling; `ColdBounded` construction with explicit, potentially larger byte budgets) with native + WASM + `wgpu` parity.

However, **PlantGL excels in domain-specific botanical, morphological, allometric, and canopy-physics geometry**—capabilities that are largely absent or only partially represented in QualiaDB's `parametric_cad.rs` and `authoring.rs`. To support procedural flora, ecological digital twins, LiDAR tree reconstruction, and micro-climate canopy simulation in QualiaDB and VibeScript, **7 major algorithmic families (28 concrete kernels/operators)** need to be added.

---

## 2. Architecture & Subsystem Matrix: PlantGL vs. QualiaDB

| PlantGL Module / File | PlantGL Capability | QualiaDB Equivalent (`computational_geometry`) | Coverage Status |
| :--- | :--- | :--- | :--- |
| **`pglmath`** (`util_vector`, `util_matrix`, `util_deformation`) | `Vector2/3/4`, `Matrix2/3/4`, Cylindrical/Spherical coords, 2D engineering `strain` & Hooke's law `stress` | `primitives.rs`, `authoring.rs`, `math_geometry.rs`, `engineering_analysis/` | **Covered** (except inline cylindrical/spherical coordinate helpers in `math_geometry.rs`) |
| **`pglsg::geometry` (Standard Primitives)** | `Box`, `Sphere`, `Cylinder`, `Cone`, `Disc`, `Frustum`, `Paraboloid`, `Plane`, `ElevationGrid` | `authoring.rs` (`unit_box`, `box_mesh`, `uv_sphere`, `cylinder`, `plane`, `torus`, `grid`) | **Partial** — Missing `Cone`, `Frustum`, `Disc`, `Paraboloid`, and `ElevationGrid` heightfield builder |
| **`pglsg::geometry` (Botanical Hulls)** | `AsymmetricHull` (6-point crown envelope + shape exponents), `ExtrudedHull` (orthogonal horizontal + vertical 2D profile intersection) | None | **Missing** (High priority for ecological/botanical crowns) |
| **`pglsg::geometry` (Splines & Sweeps)** | `BezierCurve`, `BezierPatch`, `NurbsCurve`, `NurbsPatch`, `Extrusion` (3D path + 2D cross-section + scale/orientation functions + Frenet/Bishop frame), `Swung` (rotational profile interpolation) | `parametric_cad.rs` (`bezier_eval`, `bspline_eval`, `nurbs_eval`, `tensor_surface_eval`, `tube_along_polyline`, `revolve_profile`, `loft_profiles`), `authoring_loft.rs` | **Partial** — `parametric_cad` assumes uniform knots and fixed-radius Z-offset tubes; lacks non-uniform knot vectors, rotation-minimizing (Bishop) frames along 3D curves, variable scale/twist `Extrusion`, and `Swung` surfaces |
| **`pglsg::transformation`** | `Translated`, `Scaled`, `AxisRotated`, `EulerRotated`, `Oriented`, `OrthonormalBasis3D`, `Tapered` (axial radius deformation), `IFS` (Iterated Function System fractal transform) | `authoring.rs` (`compose_trs`, `transform_mesh`), `math_geometry.rs` | **Partial** — Missing spatial deformations (`Taper`, bend, twist) and bounded `IFS` fractal instancing |
| **`pglalgo::modelling::Turtle` & `PglTurtleDrawer`** | 3D L-system geometric turtle (`F`, `f`, `+`, `-`, `&`, `^`, `\`, `/`, `[`, `]`), **tropism/elasticity** bending (`rollToVert`, `setTropism`), generalized cylinder mode (`startGC`/`stopGC`), custom path guides (`setGuide`, `sweep`) | None | **Missing** (Core procedural branching / L-system geometry engine) |
| **`pglalgo::modelling::SpaceColonization` & `GraphColonization`** | Runions (2007) Space Colonization (attractor grid, perception cone/radius, kill radius, whorls, latent buds) & Graph Colonization | None | **Missing** (Core procedural tree/venation/root growth algorithm) |
| **`pglalgo::base::pointmanipulation`** | LiDAR tree point-cloud skeletonization: Xu (2007) root-distance quotient graph (`skeleton_from_distance_to_root_clusters`), Livny (2010) lobe/branch orientation & position optimization, **Pipe Model** radius estimation (`estimate_radii_from_pipemodel`), Tagliasacchi (2009) adaptive cross-section contraction, Riemannian MST normal orientation, Weiszfeld geometric median, Monge jet principal curvatures | `point_set_3d.rs` (kNN/CkNN, spacing, density, outlier removal), `deterministic_geometry.rs` (EMST, spanners), `ddg_operators.rs` (mesh curvature) | **Missing** (Point-cloud-to-tree-skeleton pipeline, Pipe Model allometry, Weiszfeld median, Monge point-cloud curvature) |
| **`pglalgo::fitting::Skeleton`** | 2D constrained Delaunay **Chordal Axis Transform (CAT)** (`getChordalAxisTransform`): Sleeve / Junction / Termination triangle classification, pseudo-circumcenters, and area/length branch pruning (`filterLittleBranchesOnAreaSize`) | `constrained_delaunay.rs`, `delaunay_refine.rs` | **Missing** (2D medial/chordal skeleton extraction & branch pruning for leaf/silhouette analysis) |
| **`pglalgo::fitting::Fit` & `BranchCompressor`** | Point-cloud primitive fitting (PCA/inertia axis `box`, `cylinder`, `ellipsoid`, `frustum`, `extrudedHull`, `asymmetricHull`), NURBS least-squares curve fitting with chord-length parameterization (`fit.h`), and hierarchical branch geometry compression (`branchcompressor.h`) | `calipers_enclosing_disk.rs` (2D Welzl disk, rotating calipers) | **Missing** (3D Miniball, PCA bounding/average primitives, NURBS least-squares curve fitting, branch compression) |
| **`pglalgo::base` (Measures & Clipping)** | `SurfComputer`, `VolComputer`, `BBoxComputer`, `BSphereComputer`, `WireComputer`, `PlaneClipping`, `CurveManipulation` (2D polyline overlay) | `surface_mesh_processing.rs` (`surface_area`, `signed_volume`), `bvh.rs`, `dcel_overlay.rs`, `boolean_3_exact` | **Covered** (QualiaDB is strictly more exact; minor helper gap for standalone mesh-plane slicing) |
| **`pglalgo::raycasting` & `grid`** | `Ray`, `RayIntersection`, `Octree`, `RegularPointGrid`, `VoxelIntersection` | `bvh.rs`, `kd_tree.rs`, `quad_octree_mesh.rs`, `exact_construct_3.rs` | **Covered** |
| **`pglalgo::projection` & `plantgl.light`** | `ZBufferEngine` (hemispherical, spherical, equirectangular, cylindrical cameras; periodic boundary buffer wrapping `periodizeBuffer`), `DepthSortEngine`, Solar & Sky Dome (`soc_sky_sources`, `sun_sources`) radiative interception (STAR / silhouette-to-total-area ratio per organ) | `render/gpu/` (wgpu 30 renderer), `vibe/src/bind/` (astronomy/cosmic helpers) | **Missing** (Canopy radiative transfer / hemispherical gap-fraction & per-organ PAR light interception kernel) |

---

## 3. Detailed Analysis of Missing Algorithmic Capabilities

### 3.1 Botanical & Morphological Envelope Primitives (`pglsg/geometry`)
PlantGL was designed specifically around botanical envelopes (Cescatti 1997; Boudon et al.):
1. **`AsymmetricHull` (`asymmetrichull.cpp`)**:
   - Models an asymmetric tree crown using 6 morphological extreme points: `Bottom`, `Top`, and 4 cardinal peripheral points `(±X, ±Y)` each with independent radius and height `(negXRadius, posXRadius, negYRadius, posYRadius, negXHeight, posXHeight, negYHeight, posYHeight)` plus super-ellipsoidal shape exponents `bottomShape` and `topShape` satisfying $(x/r)^e + (y/h)^e = 1$.
   - Crucial for compact representation of forest canopies and individual tree crowns from field measurements.
2. **`ExtrudedHull` (`extrudedhull.cpp`)**:
   - Constructs a closed 3D envelope from the intersection of two orthogonal 2D profiles (a `Vertical` profile and a `Horizontal` cross-sectional profile).
3. **Generalized `Extrusion` with Rotation-Minimizing Frames (`extrusion.cpp`)**:
   - QualiaDB's `parametric_cad::tube_along_polyline` currently places circular rings in the XY plane (`Point3::new(p.x + r*cos, p.y + r*sin, p.z)`), which collapses when the path tangent is not parallel to Z and lacks profile scaling or torsion.
   - PlantGL's `Extrusion` sweeps an arbitrary 2D `CrossSection` curve along a 3D `Axis` curve using a continuous **Frenet or Rotation-Minimizing (Bishop / Double-Reflection) Frame**, modulated by per-station 2D `Scale(u)` and twist `Orientation(u)` profiles.
4. **`Swung` Surface (`swung.cpp`)**:
   - Generalizes a Surface of Revolution (`revolve_profile` in `parametric_cad.rs`) by interpolating between multiple 2D profiles at specified azimuthal angles $\theta_0, \theta_1, \dots, \theta_k$ around the Z-axis (used for fluted trunks, buttress roots, petals, and fruits).
5. **Basic Analytic Primitives & Spatial Deformations**:
   - `Cone`, `Frustum` (truncated cone—essential for internode segments), `Paraboloid`, `Disc`, and `Taper` / `Bend` / `Twist` vertex-space deformations, plus bounded `IFS` (Iterated Function System) matrix instancing.

---

### 3.2 Procedural Growth & 3D L-System Geometry (`pglalgo/modelling`)
1. **3D Geometric Turtle with Tropism & Generalized Cylinders (`turtle.cpp`, `pglturtledrawer.cpp`)**:
   - Maintains an oriented 3D frame $(\mathbf{P}, \mathbf{H}, \mathbf{L}, \mathbf{U})$ with a bounded state stack (`push` / `pop`), step length, radius/width, color/material ID, and cross-section profile.
   - **Gravitropism / Phototropism Bending:** At each step `F(dl)`, applies torque $\alpha (\mathbf{H} \times \mathbf{T})$ where $\mathbf{T}$ is the tropism vector and $\alpha$ is the elasticity coefficient (`setTropism`, `setElasticity`, `rollToVert`, `rollToHorizontal`).
   - **Generalized Cylinder (`startGC` / `stopGC`) & Leaf Polygon (`startPolygon` / `stopPolygon`) modes:** Seamlessly welds consecutive turtle segments into continuous tubular branch meshes with smooth mitred joints.
2. **Space Colonization & Graph Colonization (`spacecolonization.cpp`)**:
   - Implements Runions et al. (2007) for 3D tree crowns, root networks, and leaf venation:
     - Spatial hashing / grid indexing of 3D attractor points.
     - Active skeleton nodes query attractors within `perception_radius` and `coneangle`.
     - Computes normalized average attraction direction; spawns new skeleton nodes of length `nodelength` (plus optional lateral whorls via `nb_buds_per_whorl`, `insertion_angle`, and `latentbudlist`).
     - Removes attractors within `kill_radius` of newly grown nodes.
   - **`GraphColonization`** extends this over an existing kNN/Delaunay adjacency graph using geodesic distances from the root (`distances_from_root`).

---

### 3.3 Point-Cloud Tree Skeletonization & Allometry (`pglalgo/base/pointmanipulation.cpp`)
PlantGL has one of the richest open-source C++ implementations of LiDAR-to-tree reconstruction algorithms:
1. **Xu et al. (2007) Geodesic Quotient-Graph Skeletonization (`skeleton_from_distance_to_root_clusters`)**:
   - Builds a kNN or 3D Delaunay adjacency graph over a point cloud, reconnects disconnected components (`connect_all_connex_components`), computes Dijkstra shortest-path distances from the root (`points_dijkstra_shortest_path` with edge weight $\|p_i - p_j\|^p$), slices points into geodesic distance bins of width `binsize` (`quotient_points_from_adjacency_graph`), extracts connected components per bin, and links component centroids into a directed tree `(nodes, parents)`.
2. **Livny et al. (2010) Global Branch Optimization & Leonardo's Pipe Model**:
   - `carried_length` and `subtrees_size`: Bottom-up tree accumulation of supported branch lengths and node counts.
   - `optimize_orientations` & `optimize_positions`: Variational smoothing of skeleton node tangents and positions weighted by carried subtree mass.
   - **`estimate_radii_from_pipemodel`**: Computes physiologically accurate branch radii from tips to trunk using Shinozaki's / Leonardo da Vinci's Pipe Model law:
     $$r_i^{\eta} = \sum_{j \in \text{children}(i)} r_j^{\eta} \quad (\text{default } \eta = 2.5)$$
   - Topology cleaning: `detect_short_nodes`, `detect_similar_nodes`, `merge_nodes`, `node_continuity_test`.
3. **Tagliasacchi et al. (2009) Anisotropic Contraction (`adaptive_section_contration`)**:
   - Contracts noisy cylindrical point clouds onto their medial curve using anisotropic $r$-neighborhoods (`r_anisotropic_neighborhood`) and local cross-sectional circle fitting (`pointset_circle`).
4. **Point-Cloud Differential & Robust Statistics**:
   - **Monge Patch / Jet Fitting (`principal_curvatures`)**: Fits a degree-$d$ polynomial height surface in the local tangent frame to extract principal curvatures $(\kappa_1, \kappa_2)$ and principal directions on raw point clouds (QualiaDB currently only computes curvature on triangle meshes in `ddg_operators.rs`).
   - **Riemannian Graph Normal Orientation (`pointsets_orient_normals`)**: Propagates consistent normal sign across a point cloud via Minimum Spanning Tree traversal (Hoppe et al. 1992).
   - **Weiszfeld Geometric Median (`approx_pointset_median`)**: $L_1$ robust centroid of a 3D point set.

---

### 3.4 2D Shape Skeletonization: Chordal Axis Transform (`pglalgo/fitting/skeleton.cpp`)
- Given a discretized 2D closed contour (e.g., a leaf blade or building footprint), PlantGL computes the Constrained Delaunay Triangulation (CDT) and classifies every interior triangle by its number of boundary edges:
  - **Termination triangle** (2 boundary edges): leaf tip / lobe extremity.
  - **Sleeve triangle** (1 boundary edge): tubular corridor; skeleton segment connects midpoints of the two interior edges.
  - **Junction triangle** (0 boundary edges): branch fork; connects the three interior edge midpoints to the triangle's **pseudo-circumcenter** (circumcenter if acute, midpoint of longest edge if obtuse).
- Provides topological pruning of spurious boundary noise via `filterLittleBranchesOnAreaSize` and `filterLittleBranchesOnBranchSize`, plus bump/lobe extraction (`getSkeletonInformation`).
- QualiaDB already has `constrained_delaunay.rs`; adding the CAT classifier and pruning pass on top of CDT is a natural, high-leverage extension.

---

### 3.5 Geometric Fitting & NURBS Approximation (`pglalgo/fitting/fit.cpp`, `branchcompressor.cpp`)
1. **Least-Squares B-Spline / NURBS Curve Fitting (`Fit::leastSquares`, `chordLengthParam`)**:
   - Given ordered 3D points $Q_0, \dots, Q_m$, computes chord-length parameterization $\bar{u}_k$, generates a clamped knot vector, and solves the banded linear system minimizing $\sum_{k=1}^{m-1} \|Q_k - C(\bar{u}_k)\|^2$ with pinned endpoints $C(0)=Q_0, C(1)=Q_m$ (Piegl & Tiller §9.4.1).
   - `discretizeWithCurvature` & `compressPolyline` (`branchcompressor.h`): Adaptive curve sampling weighted by local curvature and rate-distortion compression of branch polylines into compact NURBS extrusions.
2. **3D Bounding & PCA Primitive Fitting (`Fit`, `miniball.cpp`)**:
   - 3D Welzl smallest enclosing sphere (`miniball.cpp`; QualiaDB currently has 2D Welzl in `calipers_enclosing_disk.rs` but not 3D).
   - Inertia-axis (PCA covariance) fitting of oriented bounding boxes (`box`), cylinders (`cylinder`), ellipsoids (`ellipsoid`), and botanical hulls (`extrudedHull`, `asymmetricHull`).

---

### 3.6 Canopy Radiative Transfer & Hemispherical Projection (`pglalgo/projection`, `plantgl.light`)
- PlantGL computes photosynthetically active radiation (PAR) and silhouette-to-total-area ratio (STAR) per plant organ using:
  - Orthographic directional projection over solar tracks (`sun_sources`) and Standard Overcast Sky (SOC) dome turtle discretizations (`soc_sky_sources`, typically 16 or 46 directions).
  - **Periodic Boundary Buffer Wrapping (`ZBufferEngine::periodizeBuffer`)**: Wraps projected depth/ID buffers across $(dx, dy, dz)$ domain bounds to simulate an **infinite periodic canopy** without duplicating geometry.
  - Analytical Depth-Sorted Triangle Projection (`DepthSortEngine`) and Hemispherical/Fisheye/Cylindrical camera projections (`ProjectionCamera`) to compute gap fraction and Leaf Area Index (LAI) directly from 3D scenes.

---

## 4. Recommended Additions to QualiaDB (`computational_geometry`)

All proposed modules must honour QualiaDB's immovable rules (`AGENTS.md` §0):
- **Tier-1 (`HotZeroHeap`)**: Per-point/per-segment predicates, frame updates, Pipe-Model sweeps, and coordinate projections take caller-owned `&mut [T]` slices with zero heap allocation (`assert_zero_alloc`).
- **Tier-2 (`ColdBounded`)**: Complex builders (Space Colonization, L-System Turtle mesh generation, Xu/Livny skeletonization, NURBS least-squares fitting, Canopy ID-buffer projection) use explicit caller-selected byte budgets. High-detail construction may exceed the 42 MiB core Sentinel ceiling; each producer must fail closed at its own budget. Keep new implementation files under 500 lines, with the ownership review and exceptions described in `AGENTS.md` §0-B.

### Phase A — Parametric Sweeps, Non-Uniform NURBS & Botanical Primitives
_Target directory: `crates/qualia-core-db/src/specialized_libs/computational_geometry/`_

1. **`sweep_extrusion.rs`** (extends `parametric_cad.rs` & `authoring.rs`):
   - `compute_bishop_frames(path: &[Point3], initial_normal: Option<Point3>, out_frames: &mut [AffineFrame3])` — Tier-1 zero-heap double-reflection rotation-minimizing frame (RMF) along a 3D polyline (avoids Frenet-frame flipping on inflection points).
   - `extrude_profile_along_path(...)` — Sweeps an arbitrary 2D cross-section with per-station `(scale_x, scale_y)` and twist angle `orientation` into caller-supplied vertex/triangle slices, with optional end-caps.
   - `swung_surface(...)` — Rotational sweep interpolating multiple 2D profiles across azimuthal angles.
   - Analytic frustum/cone/paraboloid/disc generators (`frustum_mesh`, `cone_mesh`, `paraboloid_mesh`).
2. **`botanical_hulls.rs`**:
   - `asymmetric_hull_into(params: &AsymmetricHullParams, out_vertices: &mut [Point3], out_triangles: &mut [[u32; 3]])` — 6-point asymmetric crown envelope with super-ellipsoid top/bottom shape exponents.
   - `extruded_hull_into(vertical_profile: &[Point2], horizontal_profile: &[Point2], ...)` — Orthogonal silhouette intersection hull.
   - `fit_asymmetric_hull(points: &[Point3]) -> AsymmetricHullParams` — Fits crown morphological parameters to a 3D point cluster.
3. **`nurbs_fitting.rs`** (extends `parametric_cad.rs`):
   - Non-uniform knot vector Cox–de Boor evaluation (`bspline_eval_knots`, `nurbs_eval_knots`) and rational surface patches (`nurbs_patch_eval`).
   - `chord_length_parameterize(points: &[Point3], out_u: &mut [f64])` (Tier-1 zero-heap).
   - `fit_bspline_least_squares(points: &[Point3], degree: usize, num_ctrl: usize, workspace: &mut GeometryWorkspace, out_ctrl: &mut [Point3], out_knots: &mut [f64])` (Piegl & Tiller §9.4.1).
   - `discretize_curve_by_curvature(...)` — Adaptive polyline sampling concentrated at high-curvature regions.

### Phase B — 3D Geometric Turtle & Space Colonization Growth Engine
_Target directory: `crates/qualia-core-db/src/specialized_libs/computational_geometry/`_

4. **`turtle_3d.rs`**:
   - Stack-bounded (`[TurtleState; 64]`), zero-heap 3D turtle state machine (`Turtle3D`) supporting standard L-system opcodes (`ForwardDraw`, `ForwardMove`, `Yaw`, `Pitch`, `Roll`, `RollToVert`, `Push`, `Pop`, `SetWidth`, `StartGeneralizedCylinder`, `StopGeneralizedCylinder`, `StartPolygon`, `PolygonVertex`, `StopPolygon`).
   - Built-in gravitropism/phototropism bending: $\Delta \mathbf{H} = \alpha (\mathbf{H} \times \mathbf{T})$ per segment step.
   - Emits directly into caller-provided `Mesh` buffers or `.10d` `QuantizedMesh` + `SkeletonNodes` sections.
5. **`space_colonization.rs`**:
   - Deterministic, arena-backed 3D Space Colonization (`SpaceColonizationConfig`: `node_length`, `kill_radius`, `perception_radius`, `cone_angle`, `insertion_angle`, `whorl_count`, `max_iterations`).
   - Uses QualiaDB's existing spatial hashing / `kd_tree.rs` for attractor lookup, with canonical index tie-breaking for bit-exact reproducibility across native and WASM.
   - Outputs a flat parent-indexed tree (`nodes: &mut [Point3]`, `parents: &mut [u32]`, `radii: &mut [f64]`) ready for Pipe-Model sizing and `extrude_profile_along_path` meshing.

### Phase C — Point-Cloud Skeletonization, Pipe Model & 2D Chordal Axis
_Target directory: `crates/qualia-core-db/src/specialized_libs/computational_geometry/`_

6. **`tree_skeleton_3d.rs`**:
   - **Xu (2007) Geodesic Quotient Skeletonizer**: kNN graph → Dijkstra geodesic distances from root → distance-bin quotient clustering → centroid tree extraction.
   - **Livny (2010) Allometry & Pipe Model**:
     - `pipe_model_radii(parents: &[u32], tip_radius: f64, exponent: f64, out_radii: &mut [f64])` (Tier-1 `HotZeroHeap`).
     - `optimize_skeleton_orientations_and_positions(...)` — Subtree-weighted branch smoothing.
   - **Tagliasacchi (2009) Cross-Section Contraction**: `adaptive_section_contraction(...)` for noisy trunk/branch scans.
7. **`chordal_axis_2d.rs`**:
   - Builds on `constrained_delaunay.rs` to classify CDT triangles into `Termination`, `Sleeve`, and `Junction` (using acute circumcenter / obtuse longest-edge midpoint pseudo-circumcenters).
   - Extracts the 2D medial/chordal skeleton graph and prunes noisy boundary spurs by triangle-area or branch-length thresholds (`prune_chordal_axis`).
8. **`point_cloud_diff_geom.rs`** (extends `point_set_3d.rs` & `calipers_enclosing_disk.rs`):
   - `smallest_enclosing_sphere_3d` (3D Welzl Miniball with support-set certificate).
   - `pca_bounding_primitives_3d` (oriented bounding box, bounding cylinder, bounding ellipsoid via 3×3 Jacobi eigensolver).
   - `weiszfeld_geometric_median_3d` (Tier-1 `HotZeroHeap` iterative $L_1$ median).
   - `monge_jet_principal_curvatures_3d` (local polynomial surface fit for $\kappa_1, \kappa_2$ on point clouds) and `orient_normals_riemannian_mst`.

### Phase D — Canopy Radiative Interception & Hemispherical Sensor Kernel
_Target directory: `crates/qualia-core-db/src/specialized_libs/computational_geometry/`_

9. **`canopy_radiation.rs`**:
   - CPU/WGSL orthographic ID-buffer & depth-buffer rasterizer (`project_scene_orthographic_ids`) with **periodic boundary wrapping** (`periodize_buffer`) for infinite-canopy simulation.
   - Standard Overcast Sky (`soc_sky_dome_directions`) and solar-track directional integrator computing per-organ / per-triangle intercepted irradiance, silhouette area, and hemispherical gap fraction (LAI).

---

## 5. VibeScript (`vibe-0.1`) & `.10d` Container Integration

To make these capabilities first-class in Poet / VibeScript (`using Geometry;` or `using Ecology;`):

| VibeScript / MCP Capability ID | Effect / Tier | Description |
| :--- | :--- | :--- |
| `Geometry.extrude_along_path` | `Cold` (Tier-2) | Sweep 2D profile along 3D curve with Bishop frames, scale & twist |
| `Geometry.asymmetric_hull` | `Cold` (Tier-2) | Generate 6-point botanical crown envelope mesh |
| `Geometry.swung_surface` | `Cold` (Tier-2) | Generate multi-profile rotational swung surface |
| `Geometry.turtle_eval` | `Cold` (Tier-2) | Execute 3D L-system turtle command stream with tropism into a mesh |
| `Geometry.space_colonization` | `Cold` (Tier-2) | Run 3D space colonization on attractors → skeleton + Pipe-Model radii |
| `Geometry.skeletonize_point_cloud` | `Cold` (Tier-2) | Xu (2007) + Livny (2010) LiDAR point-cloud to branching tree skeleton |
| `Geometry.pipe_model_radii` | `Hot` (Tier-1) | Compute branch radii from parent array via da Vinci / Shinozaki exponent |
| `Geometry.chordal_axis_2d` | `Cold` (Tier-2) | 2D CDT Chordal Axis Transform + area-based branch pruning |
| `Geometry.fit_nurbs_curve` | `Cold` (Tier-2) | Chord-length parameterized least-squares B-spline/NURBS curve fit |
| `Geometry.miniball_3d` | `Hot` / `Cold` | 3D Welzl smallest enclosing sphere with support certificate |
| `Geometry.canopy_light_interception` | `Cold` (Tier-2) | Directional / sky-dome PAR interception & periodic canopy buffer |

---

## 6. Prioritized Implementation Roadmap

| Priority | Work Package | Effort | Key Deliverables & Verification Gates |
| :--- | :--- | :--- | :--- |
| **P1 (Immediate)** | **Botanical Envelopes & RMF Extrusions** (`sweep_extrusion.rs`, `botanical_hulls.rs`) | Small–Med | Bishop frame (RMF) tube/profile extrusion, `AsymmetricHull`, `ExtrudedHull`, `Swung`, `Frustum`/`Cone`. Verified with watertight half-edge topology check (`build_triangle_half_edges`) and `signed_volume` oracle. |
| **P2 (High)** | **Procedural Growth: Space Colonization + Pipe Model + 3D Turtle** (`space_colonization.rs`, `turtle_3d.rs`) | Medium | Seeded-deterministic `SpaceColonization`, `pipe_model_radii` (`assert_zero_alloc`), and tropism-aware `Turtle3D` generalized cylinder mesher. |
| **P3 (Medium)** | **2D Chordal Axis & 3D LiDAR Tree Skeletonization** (`chordal_axis_2d.rs`, `tree_skeleton_3d.rs`) | Medium | CDT Sleeve/Junction/Termination CAT skeleton with area pruning; Xu (2007) geodesic quotient graph + Livny (2010) branch smoothing. |
| **P4 (Medium)** | **Non-Uniform NURBS Fitting & 3D Point-Cloud Geometry** (`nurbs_fitting.rs`, `point_cloud_diff_geom.rs`) | Medium | Piegl & Tiller §9.4.1 least-squares curve fit, 3D Welzl Miniball, Weiszfeld median (`assert_zero_alloc`), Monge jet curvatures, Riemannian MST normal orientation. |
| **P5 (Targeted)** | **Canopy Radiative Transfer & Periodic Projection** (`canopy_radiation.rs`) | Small–Med | Orthographic ID/depth projection with `periodize_buffer` and SOC sky-dome integration; cross-checked against analytical sphere/disc projection areas. |

---

## 7. Post-PlantGL Horizon — Modern Flora / Fauna / Biosphere Geometry

_Addendum, 2026-10-14. Phases A–D (P20.1–P20.9) close the PlantGL-era gaps
(L-systems, string rewriting, fixed extrusion). Modern biological and ecological
modelling additionally requires environment-aware, topologically dynamic, and
physics-informed structures — volumetric growth, spatial competition, and
non-rigid deformation. This section audits the requested modern algorithm
families against the current `computational_geometry` stack and proposes the
next wave (P21.x). References are the canonical public literature; all
implementations remain clean-room._

### 7.1 Flora — Morphogenesis & Botanical Architecture

| Requested algorithm | Existing QualiaDB coverage | Gap → proposed module |
| :--- | :--- | :--- |
| **Space Colonization (SCA)** | `space_colonization.rs` (Runions 2007, perception cones, kill radii, whorls, latent buds) | **Covered** (P20.5). Extensions: obstacle repulsion field, resource-gradient bias |
| **Laplace–Beltrami differential growth** | `ddg_operators.rs` (cotangent Laplacian, `heat_step`, `mean_curvature_flow_step`), `anisotropic_remesh.rs` (metric-tensor remeshing), `cknn_laplacian.rs` | **Partial** → `differential_growth.rs`: per-vertex anisotropic growth-rate field → prescribed target metric tensor → growth step via conformal metric map, coupled to edge-split/collapse remesh pass (leaf ruffling, petal/fungal curling) |
| **Curve skeletonization & topological simplification** | `tree_skeleton_3d.rs` (geodesic quotient bins for point clouds), `chordal_axis_2d.rs` (2-D CAT), `tda.rs`/`persistence.rs` (persistence-based noise filtering) | **Partial** → `curve_skeleton.rs`: mesh-based mean-curvature-flow skeletonization (Au, Tai, Chu, Cohen-Or, Lee 2008) + persistence-guided branch pruning. Also consider GMM/EM skeleton refinement (stochastic plant skeletonization, Front. Plant Sci. 2020) |
| **Laguerre–Voronoi / power diagrams** | `voronoi_variants.rs` (segment/farthest/higher-order sites), `delaunay_2/3`, `incircle` | **Missing** → `power_diagram.rs`: `power_test` weighted-in-circle predicate (exact via `expansion.rs` kernel), regular triangulation (2-D and 3-D weighted Delaunay), weighted dual cells for meristem/cellular tissue of non-uniform cell volumes |

### 7.2 Fauna — Anatomy, Articulation & Soft Tissue

| Requested algorithm | Existing coverage | Gap → proposed module |
| :--- | :--- | :--- |
| **Signed Distance Fields (SDF)** | `parametric_cad.rs` (`SdfPrimitive`, `sdf_eval`, `sdf_compose` hard CSG union/intersect/difference), `isosurface.rs` (marching cubes) | **Partial** → `sdf_ops.rs`: smooth-min boolean blends (polynomial/quadratic smin), mesh→narrow-band SDF rasterization (fast-marching), sphere-tracing ray march for continuous collision detection |
| **Dual-Quaternion Skinning (DQS)** | `math_geometry.rs` (Quaternion, SLERP, SO(3) exp/log, quaternion→matrix) | **Missing** → `dual_quaternion_skinning.rs`: `DualQuat` (real+dual parts), unit dual-quat normalize/ScLERP, point transform, `dqs_blend_skin` per-vertex over bone palette — fixes candy-wrapper volume loss (Kavan et al. 2007) |
| **Position-Based Dynamics (PBD/XPBD)** | `motion_planning.rs` (kinodynamic propagation — not soft body) | **Missing** → `pbd_softbody.rs`: deterministic-iteration PBD/XPBD with distance, dihedral bending, and volume constraints; ground/obstacle projection; substep loop for muscle/fat jiggle (Müller et al. 2007; Macklin et al. 2016 XPBD) |
| **Medial Axis Transform (3-D)** | `chordal_axis_2d.rs` (2-D CAT), `calipers_enclosing_disk.rs` | **Missing** → `medial_axis_3d.rs`: Voronoi-pole medial axis (Amenta & Bern power crust) built on `delaunay_3`, or voxel-thinning fallback; feeds automatic rigging hierarchies + wall-thickness bounds |

### 7.3 Biosphere — Ecology, Terrain & Spatial Interaction

| Requested algorithm | Existing coverage | Gap → proposed module |
| :--- | :--- | :--- |
| **Poisson-disk / blue-noise sampling** | `deterministic_geometry.rs::generate_spatial_points` (uniform/clustered/skew generators) | **Missing** → `poisson_disk.rs`: Bridson (2007) seeded dart-throwing with background acceleration grid, 2-D/3-D, optional spatially-varying radius field (seed-dispersal density gradients), canonical index tie-break for bit-exact determinism |
| **Heat-method geodesics** | `ddg_operators.rs` already ships all three ingredients: `heat_step` (implicit Euler), `face_vector_areas`/gradients, `solve_poisson_jacobi`, plus `geodesic_distances_dijkstra` baseline | **Partial** → `heat_geodesics.rs`: assemble the Crane, Weischedel & Wardetzky (2013) pipeline (diffuse δ → normalize −∇u → Poisson solve for ∇·X) as a single verified entry point + cross-check vs Dijkstra |
| **Sparse voxel octree (SVO) + volumetric ray cast** | `quad_octree_mesh.rs` (`OctTree` — meshing-oriented), `bvh.rs`, `kd_tree.rs` | **Partial** → `sparse_voxel_octree.rs`: occupancy SVO with deterministic DFS ray traversal, used as the acceleration structure for `canopy_radiation.rs` at biome scale (thousands of crowns) |
| **Spatial hashing & BVH for agent swarms** | `bvh.rs`, `kd_tree.rs`, `box_join.rs`, `deterministic_geometry.rs` (spatial hash pack), `point_set_3d.rs` (kNN), `nn_query.rs`, `motion_planning.rs` | **Covered** — uniform-hash O(1) neighbour query for flock/swarm perception is already available via `deterministic_pack_u32` + `knn_brute_force_3d`/`nn_query`; only a thin `spatial_hash_grid` convenience layer would be new |

### 7.4 Additional modern gaps surfaced by the literature review

- **4-D growth tracking**: budding/bifurcation event detection across time-series point clouds (forward–backward event analysis, Li et al. 2024 "Analyzing Growing Plants from 4D Point Cloud Data") — no temporal registration layer exists today; would extend `point_cloud_diff_geom` with inter-scan correspondence.
- **Medial meshes / Medial Skeletal Diagram**: compact medial representations between plain curve skeletons and full MAT (Livesu et al.; medial-mesh reconstruction) — candidate companion to `medial_axis_3d`.
- **SRVF-tree morphometrics**: elastic shape spaces over tree-structured objects for comparing branching morphologies across time (spatiotemporal registration) — natural fit for `statistical_manifold.rs` + skeleton graphs.
- **Simulation-ready tree generation from images** (Tree-D Fusion, ECCV 2024): diffusion-prior reconstruction feeding `space_colonization_3d` — ML side is out of geometry scope, but the geometry seam (envelope point markers → colonization attractors) is already expressible.

### 7.5 Extended roadmap

| Priority | Work package | Effort | Deliverables & gates |
| :--- | :--- | :--- | :--- |
| **P6 (High)** | **Biosphere foundations**: `poisson_disk.rs`, `heat_geodesics.rs`, `sparse_voxel_octree.rs` | Medium | Bridson blue-noise (seeded, `assert_zero_alloc` sample query); heat geodesics cross-checked vs `geodesic_distances_dijkstra`; SVO ray traversal verified vs brute-force ray·triangle on corpus meshes |
| **P7 (High)** | **Tissue & growth dynamics**: `power_diagram.rs`, `differential_growth.rs`, `curve_skeleton.rs` | Med–Large | `power_test` predicate parity vs expanded arithmetic oracle; growth step preserves enclosed volume within tolerance + manifold topology; MCF skeleton on genus-0 test meshes, branch count stable under noise |
| **P8 (Medium)** | **Fauna kinematics & soft tissue**: `dual_quaternion_skinning.rs`, `pbd_softbody.rs`, `medial_axis_3d.rs`, `sdf_ops.rs` | Medium | DQS vs LBS volume conservation on articulated test rigs; XPBD convergence + constraint residuals bounded; Voronoi-pole MAT inside tolerance ball of exact medial axis on convex meshes; smin blends C¹-continuous |

_Dependency order: `power_diagram` reuses the `expansion.rs` exact kernel and
`delaunay_3` machinery; `sparse_voxel_octree` slots under `canopy_radiation` as
an acceleration structure; `differential_growth` composes
`ddg_operators` + `anisotropic_remesh` rather than duplicating operators._

### 7.6 P6–P8 implementation status (Wave 28)

All ten P6–P8 modules are implemented, exported, manifested, and host-wired:

| Module | Invoke ID | Allocation class |
| :--- | :--- | :--- |
| `poisson_disk.rs` | `ComputationalGeometry.poisson_disk_2d` / `.poisson_disk_3d` | `HotZeroHeap` (seeded Bridson; Bridson-bound grid sizing: ≤1 sample/cell) |
| `heat_geodesics.rs` | `ComputationalGeometry.heat_geodesics` | `ColdBounded` (composes bounded DDG operators + Jacobi solve) |
| `sparse_voxel_octree.rs` | `ComputationalGeometry.svo_raycast` | `ColdBounded` construction, zero-heap traversal |
| `power_diagram.rs` | `ComputationalGeometry.power_diagram_2d` | `ColdBounded` (Bowyer–Watson regular triangulation) |
| `differential_growth.rs` | `ComputationalGeometry.differential_growth_step` | `HotZeroHeap` (caller rest-length + vertex buffers) |
| `curve_skeleton.rs` | `ComputationalGeometry.curve_skeleton` | `ColdBounded` (iterative contraction + graph extraction) |
| `dual_quaternion_skinning.rs` | `ComputationalGeometry.dqs_skin` | `HotZeroHeap` (stack dual-quaternion blend per vertex) |
| `pbd_softbody.rs` | `ComputationalGeometry.pbd_step` | `HotZeroHeap` (constraint projection, caller positions/previous buffers) |
| `medial_axis_3d.rs` | `ComputationalGeometry.medial_axis_3d` | `ColdBounded` (Voronoi-pole extraction over `delaunay_3`) |
| `sdf_ops.rs` | `ComputationalGeometry.sdf_smooth` | `HotZeroHeap` (smin union/intersect/subtract, capsule/rounded-box/metaball fields) |

Host seams live in `poet_host/invoke/geometry/wave28_host.rs`; ID constants,
`ALL_BOUND`, `seam_for` ("geometry"), and dispatch arms are registered in
`poet_host/invoke/ids.rs` and `invoke/mod.rs`; all 11 IDs are in
`crates/vibe/src/catalog/ids.rs` (`vibe_catalog_contains_every_bound_invoke_id`
green). Verified: 1,563 `computational_geometry` lib tests + 647
`vibe_host::invoke` tests + 25 `vibe::catalog` tests passed in the 2026-10-14
targeted run. This is an implementation status, not a full-suite acceptance
claim; the same run reported failures outside geometry and a process access
violation in the full library suite.

Remaining roadmap items not yet built (documented gaps, no implementation
planned in this wave): 4-D growth event tracking, medial meshes/MSD,
SRVF-tree morphometrics, Tree-D-Fusion-style image→attractor seams.

### 7.7 Follow-up audit (2026-10-15)

The three Rust kernels listed in §5 without direct Vibe invoke IDs now have
`ComputationalGeometry.swung_surface`, `.pipe_model_radii`, and `.miniball_3d`
adapters and catalog entries. `ComputationalGeometry.canopy_integrate` remains
the canopy light entry point. A caller-buffered `mesh_plane_slice` Rust helper
now emits triangle/plane intersection segments through
`ComputationalGeometry.mesh_plane_slice`; coplanar-region boundary extraction
remains a separate operation.

The core `SlgArena` and Tier-1 kernels retain the 42 MiB rule. Cold geometry
and high-detail asset construction are explicitly budgeted outside that arena:
the default is 256 MiB, and SVO accepts a caller-selected node budget up to
16 GiB. It reports exhaustion rather than silently reducing model detail. The
Poisson Host adapter has a caller-selected grid budget (256 MiB default,
16 GiB maximum) and uses fallible reservations. Remaining P20/P21 builders
still need individual budget enforcement audits; their existing 128 MiB
manifest entries are no longer interpreted as violations of the core ceiling.

The four §7.4 research items remain proposed capabilities. As of this update,
the `miniball_3d` kernel (`smallest_enclosing_sphere_3d`) has been upgraded to a
textbook 4-level iterative Welzl algorithm (zero heap, non-recursive, bit-exact)
guaranteeing minimal radius and a genuine 1–4 point boundary support certificate.
The weighted power-triangulation predicate now incorporates `power_test_2d_sign`
backed by the exact Shewchuk `orient_3d` adaptive-precision ladder (filtered →
compensated → exact expansion), providing exact topological decisions for
Bowyer–Watson regular triangulation.
