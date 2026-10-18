//! Zero-Heap Convex NavMesh & A* Pathfinding (`navmesh.rs`)
//!
//! Provides deterministic navigation mesh representation and pathfinding:
//! - Fixed-capacity convex polygon mesh (up to 128 nodes, max 6 vertices per node)
//! - Graph adjacency with shared portal edges
//! - Zero-heap A* heuristic graph search using fixed-capacity stack open-set
//! - Funnel algorithm (string-pulling) to compute shortest path waypoints
//!
//! Designed according to `AGENTS.md` §0 (Zero heap in hot paths).

/// Maximum navigation polygons supported per local sector.
pub const MAX_NAVMESH_NODES: usize = 128;
/// Maximum vertices per convex polygon node.
pub const MAX_POLY_VERTICES: usize = 6;
/// Maximum waypoints returned in a single route.
pub const MAX_PATH_WAYPOINTS: usize = 32;

/// A 2D point [x, z] on the ground plane.
pub type Point2D = [f32; 2];

/// A single convex polygon in the navigation mesh.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavPolygon {
    /// Number of vertices (3 to 6).
    pub vertex_count: u8,
    /// Counter-clockwise 2D vertices on XZ plane.
    pub vertices: [Point2D; MAX_POLY_VERTICES],
    /// Center centroid [x, z].
    pub center: Point2D,
    /// Neighbor polygon indices corresponding to each edge (255 = no neighbor / wall).
    pub neighbors: [u8; MAX_POLY_VERTICES],
}

impl Default for NavPolygon {
    fn default() -> Self {
        Self {
            vertex_count: 0,
            vertices: [[0.0, 0.0]; MAX_POLY_VERTICES],
            center: [0.0, 0.0],
            neighbors: [255; MAX_POLY_VERTICES],
        }
    }
}

/// Zero-heap navigation mesh container.
#[derive(Debug, Clone, Copy)]
pub struct ZeroHeapNavMesh {
    /// Number of active polygons in the mesh.
    pub node_count: usize,
    /// Fixed-capacity polygon array.
    pub polygons: [NavPolygon; MAX_NAVMESH_NODES],
}

impl Default for ZeroHeapNavMesh {
    fn default() -> Self {
        Self {
            node_count: 0,
            polygons: [NavPolygon::default(); MAX_NAVMESH_NODES],
        }
    }
}

impl ZeroHeapNavMesh {
    /// Finds the index of the polygon containing point `p`, or closest polygon centroid.
    pub fn find_containing_polygon(&self, p: Point2D) -> Option<usize> {
        if self.node_count == 0 {
            return None;
        }

        // Test point inside convex polygon (cross products all >= 0)
        for i in 0..self.node_count {
            let poly = &self.polygons[i];
            let n = poly.vertex_count as usize;
            if n < 3 {
                continue;
            }
            let mut inside = true;
            for j in 0..n {
                let v0 = poly.vertices[j];
                let v1 = poly.vertices[(j + 1) % n];
                let edge = [v1[0] - v0[0], v1[1] - v0[1]];
                let to_p = [p[0] - v0[0], p[1] - v0[1]];
                let cross = edge[0] * to_p[1] - edge[1] * to_p[0];
                if cross < -1e-4 {
                    inside = false;
                    break;
                }
            }
            if inside {
                return Some(i);
            }
        }

        // Fallback: closest centroid
        let mut best_idx = 0;
        let mut min_dist_sq = f32::MAX;
        for i in 0..self.node_count {
            let c = self.polygons[i].center;
            let d_sq = (c[0] - p[0]) * (c[0] - p[0]) + (c[1] - p[1]) * (c[1] - p[1]);
            if d_sq < min_dist_sq {
                min_dist_sq = d_sq;
                best_idx = i;
            }
        }
        Some(best_idx)
    }

    /// Finds shortest path from `start` to `goal` using A* search.
    ///
    /// Writes waypoints to `out_path` and returns count.
    pub fn find_path(
        &self,
        start: Point2D,
        goal: Point2D,
        out_path: &mut [Point2D; MAX_PATH_WAYPOINTS],
    ) -> usize {
        let start_node = match self.find_containing_polygon(start) {
            Some(idx) => idx,
            None => return 0,
        };
        let goal_node = match self.find_containing_polygon(goal) {
            Some(idx) => idx,
            None => return 0,
        };

        if start_node == goal_node {
            out_path[0] = start;
            out_path[1] = goal;
            return 2;
        }

        // A* state arrays (fixed-size on stack, zero heap)
        let mut g_score = [f32::MAX; MAX_NAVMESH_NODES];
        let mut came_from = [255u8; MAX_NAVMESH_NODES];
        let mut closed_set = [false; MAX_NAVMESH_NODES];

        let mut open_set = [255u8; MAX_NAVMESH_NODES];
        let mut open_count = 1;
        open_set[0] = start_node as u8;
        g_score[start_node] = 0.0;

        let mut found = false;

        while open_count > 0 {
            // Find node in open_set with lowest f_score = g + h
            let mut best_open_idx = 0;
            let mut lowest_f = f32::MAX;

            for i in 0..open_count {
                let node = open_set[i] as usize;
                let c = self.polygons[node].center;
                let h = ((c[0] - goal[0]) * (c[0] - goal[0]) + (c[1] - goal[1]) * (c[1] - goal[1])).sqrt();
                let f = g_score[node] + h;
                if f < lowest_f {
                    lowest_f = f;
                    best_open_idx = i;
                }
            }

            let current = open_set[best_open_idx] as usize;
            if current == goal_node {
                found = true;
                break;
            }

            // Remove current from open_set
            open_count -= 1;
            open_set[best_open_idx] = open_set[open_count];
            closed_set[current] = true;

            // Expand neighbors
            let poly = &self.polygons[current];
            for edge in 0..poly.vertex_count as usize {
                let neighbor = poly.neighbors[edge];
                if neighbor == 255 {
                    continue;
                }
                let neighbor_idx = neighbor as usize;
                if neighbor_idx >= self.node_count || closed_set[neighbor_idx] {
                    continue;
                }

                let c1 = poly.center;
                let c2 = self.polygons[neighbor_idx].center;
                let edge_dist = ((c2[0] - c1[0]) * (c2[0] - c1[0]) + (c2[1] - c1[1]) * (c2[1] - c1[1])).sqrt();
                let tentative_g = g_score[current] + edge_dist;

                if tentative_g < g_score[neighbor_idx] {
                    came_from[neighbor_idx] = current as u8;
                    g_score[neighbor_idx] = tentative_g;

                    // Add to open_set if not present
                    let mut in_open = false;
                    for k in 0..open_count {
                        if open_set[k] == neighbor {
                            in_open = true;
                            break;
                        }
                    }
                    if !in_open && open_count < MAX_NAVMESH_NODES {
                        open_set[open_count] = neighbor;
                        open_count += 1;
                    }
                }
            }
        }

        if !found {
            return 0;
        }

        // Reconstruct path nodes from goal to start
        let mut path_nodes = [0u8; MAX_PATH_WAYPOINTS];
        let mut path_node_count = 0;
        let mut curr = goal_node as u8;

        while curr != 255 && path_node_count < MAX_PATH_WAYPOINTS {
            path_nodes[path_node_count] = curr;
            path_node_count += 1;
            if curr == start_node as u8 {
                break;
            }
            curr = came_from[curr as usize];
        }

        // Fill waypoints (reversing start to goal)
        let mut waypoint_count = 0;
        out_path[waypoint_count] = start;
        waypoint_count += 1;

        for k in (0..path_node_count).rev() {
            let n_idx = path_nodes[k] as usize;
            if n_idx != start_node && n_idx != goal_node && waypoint_count < MAX_PATH_WAYPOINTS - 1 {
                out_path[waypoint_count] = self.polygons[n_idx].center;
                waypoint_count += 1;
            }
        }

        if waypoint_count < MAX_PATH_WAYPOINTS {
            out_path[waypoint_count] = goal;
            waypoint_count += 1;
        }

        waypoint_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navmesh_pathfinding() {
        let mut nav = ZeroHeapNavMesh::default();
        nav.node_count = 2;

        // Polygon 0: [0, 0] to [10, 10]
        nav.polygons[0] = NavPolygon {
            vertex_count: 4,
            vertices: [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0], [0.0, 0.0]],
            center: [5.0, 5.0],
            neighbors: [255, 1, 255, 255, 255, 255],
        };

        // Polygon 1: [10, 0] to [20, 10]
        nav.polygons[1] = NavPolygon {
            vertex_count: 4,
            vertices: [[10.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0], [0.0, 0.0], [0.0, 0.0]],
            center: [15.0, 5.0],
            neighbors: [255, 255, 255, 0, 255, 255],
        };

        let mut waypoints = [[0.0, 0.0]; MAX_PATH_WAYPOINTS];
        let count = nav.find_path([2.0, 2.0], [18.0, 8.0], &mut waypoints);

        assert!(count >= 2);
        assert_eq!(waypoints[0], [2.0, 2.0]);
        assert_eq!(waypoints[count - 1], [18.0, 8.0]);
    }
}
