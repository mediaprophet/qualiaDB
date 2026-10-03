//! Deterministic 2D Navigation and Group Movement Engine (QG-11).
//!
//! Provides bounded, zero-heap pathfinding and formation movement for:
//! - Multi-agent RTS simulations (Rolling Commons)
//! - POET spatial hyper-canvas auto-routing
//! - Dynamic obstruction updates (buildings, gates, bridges)
//!
//! Adheres strictly to `AGENTS.md`:
//! - Tier 1 zero-heap hot path: caller-supplied output buffers (`&mut [(i32, i32)]`).
//! - Fixed-capacity scratch storage (`[T; N]`).
//! - Bounded queue iteration — guaranteed non-recursive, non-infinite loop.

/// Maximum dimensions for navigation grid.
pub const NAV_GRID_MAX_DIM: usize = 128;
pub const NAV_GRID_MAX_CELLS: usize = NAV_GRID_MAX_DIM * NAV_GRID_MAX_DIM;

/// Maximum path length returned in a single query.
pub const MAX_PATH_LENGTH: usize = 256;

/// Bounded deterministic 2D navigation grid.
#[derive(Clone)]
pub struct NavigationGrid<const W: usize, const H: usize> {
    /// Passability mask: 0 = walkable, 1 = static obstacle, 2 = dynamic obstacle / closed gate.
    pub cells: [u8; NAV_GRID_MAX_CELLS],
    pub width: usize,
    pub height: usize,
}

impl<const W: usize, const H: usize> NavigationGrid<W, H> {
    pub fn new() -> Self {
        assert!(W <= NAV_GRID_MAX_DIM && H <= NAV_GRID_MAX_DIM);
        Self {
            cells: [0u8; NAV_GRID_MAX_CELLS],
            width: W,
            height: H,
        }
    }

    #[inline]
    pub fn index_of(&self, x: i32, y: i32) -> Option<usize> {
        if x >= 0 && (x as usize) < self.width && y >= 0 && (y as usize) < self.height {
            Some((y as usize) * self.width + (x as usize))
        } else {
            None
        }
    }

    #[inline]
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        match self.index_of(x, y) {
            Some(idx) => self.cells[idx] == 0,
            None => false,
        }
    }

    pub fn set_obstacle(&mut self, x: i32, y: i32, obstacle_code: u8) -> bool {
        if let Some(idx) = self.index_of(x, y) {
            self.cells[idx] = obstacle_code;
            true
        } else {
            false
        }
    }

    pub fn clear_obstacle(&mut self, x: i32, y: i32) -> bool {
        if let Some(idx) = self.index_of(x, y) {
            self.cells[idx] = 0;
            true
        } else {
            false
        }
    }

    /// Deterministic Breadth-First-Search / A* pathfinder with bounded stack scratch.
    ///
    /// Writes coordinates (x, y) into `out_path` from start to target.
    /// Returns the number of waypoints written.
    pub fn find_path(
        &self,
        start: (i32, i32),
        target: (i32, i32),
        out_path: &mut [(i32, i32)],
    ) -> usize {
        let (sx, sy) = start;
        let (tx, ty) = target;

        if !self.is_walkable(tx, ty) || out_path.is_empty() {
            return 0;
        }

        if sx == tx && sy == ty {
            out_path[0] = start;
            return 1;
        }

        let total_cells = self.width * self.height;
        if total_cells > NAV_GRID_MAX_CELLS {
            return 0;
        }

        // Bounded scratch structures on the stack:
        // came_from encodes (parent_x, parent_y) packed as (u16, u16) or 0xFFFF_FFFF for unvisited.
        // Queue size bounded to NAV_GRID_MAX_CELLS.
        let mut came_from = [0xFFFF_FFFFu32; NAV_GRID_MAX_CELLS];
        let mut queue = [0u32; NAV_GRID_MAX_CELLS];
        let mut q_head = 0;
        let mut q_tail = 0;

        let start_idx = match self.index_of(sx, sy) {
            Some(i) => i,
            None => return 0,
        };

        // Mark start as visited (pointing to itself)
        let packed_start = ((sx as u16 as u32) << 16) | (sy as u16 as u32);
        came_from[start_idx] = packed_start;
        queue[q_tail] = packed_start;
        q_tail += 1;

        let mut reached = false;

        // 4-way orthogonal neighborhood (deterministic evaluation order: N, E, S, W)
        const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

        while q_head < q_tail && !reached {
            let curr_packed = queue[q_head];
            q_head += 1;

            let cx = (curr_packed >> 16) as i16 as i32;
            let cy = (curr_packed & 0xFFFF) as i16 as i32;

            for &(dx, dy) in &DIRS {
                let nx = cx + dx;
                let ny = cy + dy;

                if nx == tx && ny == ty {
                    if let Some(n_idx) = self.index_of(nx, ny) {
                        came_from[n_idx] = curr_packed;
                        reached = true;
                        break;
                    }
                }

                if self.is_walkable(nx, ny) {
                    if let Some(n_idx) = self.index_of(nx, ny) {
                        if came_from[n_idx] == 0xFFFF_FFFF {
                            came_from[n_idx] = curr_packed;
                            if q_tail < NAV_GRID_MAX_CELLS {
                                queue[q_tail] = ((nx as u16 as u32) << 16) | (ny as u16 as u32);
                                q_tail += 1;
                            }
                        }
                    }
                }
            }
        }

        if !reached {
            return 0;
        }

        // Trace back from target to start
        let mut rev_path = [(0i32, 0i32); MAX_PATH_LENGTH];
        let mut rev_count = 0;
        let mut curr_x = tx;
        let mut curr_y = ty;

        while rev_count < MAX_PATH_LENGTH {
            rev_path[rev_count] = (curr_x, curr_y);
            rev_count += 1;

            if curr_x == sx && curr_y == sy {
                break;
            }

            let c_idx = match self.index_of(curr_x, curr_y) {
                Some(i) => i,
                None => break,
            };

            let p_packed = came_from[c_idx];
            if p_packed == 0xFFFF_FFFF {
                break;
            }

            let px = (p_packed >> 16) as i16 as i32;
            let py = (p_packed & 0xFFFF) as i16 as i32;

            if px == curr_x && py == curr_y {
                break;
            }

            curr_x = px;
            curr_y = py;
        }

        // Write reversed path into caller-supplied buffer
        let copy_len = rev_count.min(out_path.len());
        for i in 0..copy_len {
            out_path[i] = rev_path[rev_count - 1 - i];
        }

        copy_len
    }

    /// Compute deterministic grid offsets for a group of `agent_count` agents ordered
    /// to move towards `(target_x, target_y)`.
    ///
    /// Writes offsets into `out_offsets` around `(target_x, target_y)` such that
    /// agents form a compact grid formation without overlap.
    pub fn compute_group_formation_destinations(
        &self,
        target_x: i32,
        target_y: i32,
        agent_count: usize,
        out_destinations: &mut [(i32, i32)],
    ) -> usize {
        let count = agent_count.min(out_destinations.len());
        if count == 0 {
            return 0;
        }

        let mut written = 0;
        // Spiral outwards from (target_x, target_y) to find walkable slots
        let mut radius = 0i32;

        while written < count && radius < (self.width.max(self.height) as i32) {
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    if dx.abs() == radius || dy.abs() == radius {
                        let gx = target_x + dx;
                        let gy = target_y + dy;
                        if self.is_walkable(gx, gy) {
                            out_destinations[written] = (gx, gy);
                            written += 1;
                            if written >= count {
                                return written;
                            }
                        }
                    }
                }
            }
            radius += 1;
        }

        written
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_straight_line_path() {
        let grid = NavigationGrid::<16, 16>::new();
        let mut path = [(0i32, 0i32); 32];
        let len = grid.find_path((2, 2), (2, 6), &mut path);
        assert_eq!(len, 5);
        assert_eq!(path[0], (2, 2));
        assert_eq!(path[len - 1], (2, 6));
    }

    #[test]
    fn test_path_around_wall() {
        let mut grid = NavigationGrid::<16, 16>::new();
        // Wall from y=1 to y=4 at x=3
        grid.set_obstacle(3, 1, 1);
        grid.set_obstacle(3, 2, 1);
        grid.set_obstacle(3, 3, 1);
        grid.set_obstacle(3, 4, 1);

        let mut path = [(0i32, 0i32); 64];
        let len = grid.find_path((1, 2), (5, 2), &mut path);
        assert!(len > 0);
        // Verify path does not cross wall
        for i in 0..len {
            let (x, y) = path[i];
            assert!(grid.is_walkable(x, y));
        }
    }

    #[test]
    fn test_dynamic_bridge_obstacle() {
        let mut grid = NavigationGrid::<16, 16>::new();
        // Bridge at (4, 4) is closed
        grid.set_obstacle(4, 4, 2);
        let mut path = [(0i32, 0i32); 32];
        let blocked = grid.find_path((3, 4), (4, 4), &mut path);
        assert_eq!(blocked, 0);

        // Open bridge
        grid.clear_obstacle(4, 4);
        let opened = grid.find_path((3, 4), (4, 4), &mut path);
        assert_eq!(opened, 2);
        assert_eq!(path[1], (4, 4));
    }

    #[test]
    fn test_group_formation_destinations() {
        let grid = NavigationGrid::<16, 16>::new();
        let mut dests = [(0i32, 0i32); 5];
        let count = grid.compute_group_formation_destinations(5, 5, 5, &mut dests);
        assert_eq!(count, 5);
        // Ensure all destinations are distinct
        for i in 0..count {
            for j in (i + 1)..count {
                assert_ne!(dests[i], dests[j]);
            }
        }
    }
}
