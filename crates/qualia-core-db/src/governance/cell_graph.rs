//! Cellular Logic Graph Partitioning & Routing Kernel (WP-3.2).
//!
//! Partitions continuous hypermedia / simulation space into a topological
//! network of independent logical cells.
//!
//! ### The 42 MiB Sentinel Contract
//! - Each active cell executes its deterministic rules within its own
//!   isolated 42 MiB `SlgArena` (917,504 `NQuin` capacity).
//! - Cross-cell communication is strictly routed through Permissive Routing Lanes:
//!   - `Spatial` (`0b11`): Proximity broadcasts, physical collisions, neighbor notifications.
//!   - `Commons` (`0b01`): Ecological balances, shared stewardship rules, resource flows.
//! - Cold geometry, high-resolution heightfields, and manifold weights live outside
//!   in the decoupled 512 MiB caller-budgeted tier.

use crate::{NQuin, PermissiveRoutingLane};
use serde::{Deserialize, Serialize};

/// Maximum adjacent topological neighbor cells tracked per cell.
pub const MAX_CELL_NEIGHBORS: usize = 8;
/// Maximum pending inter-cell routing messages per frame.
pub const MAX_INTER_CELL_MESSAGES: usize = 64;

/// Unique 64-bit topological identifier for a spatial cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct CellId(pub u64);

/// Spatial descriptor of a logical cell in the world graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogicalCellDescriptor {
    pub id: CellId,
    pub name: String,
    pub center_x: f64,
    pub center_z: f64,
    pub radius_meters: f64,
    pub neighbor_count: usize,
    pub neighbors: [CellId; MAX_CELL_NEIGHBORS],
    pub is_active: bool,
    pub resident_quin_count: u32,
}

/// An inter-cell message routed across logical cells via Permissive Routing Lanes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterCellMessage {
    pub source_cell_id: u64,
    pub target_cell_id: u64,
    pub lane_bits: u8,
    pub reserved_u8: u8,
    pub payload_kind: u16,
    pub reserved_u32: u32,
    pub quin: NQuin,
}

/// Manager tracking the topological network of 42 MiB cellular logical units.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CellularLogicGraph {
    pub cells: Vec<LogicalCellDescriptor>,
    pub message_queue: Vec<InterCellMessage>,
}

impl CellularLogicGraph {
    /// Creates a default `CellularLogicGraph`.
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            message_queue: Vec::new(),
        }
    }

    /// Register a new logical cell in the graph.
    pub fn register_cell(
        &mut self,
        id: CellId,
        name: impl Into<String>,
        center_x: f64,
        center_z: f64,
        radius_meters: f64,
    ) -> usize {
        let descriptor = LogicalCellDescriptor {
            id,
            name: name.into(),
            center_x,
            center_z,
            radius_meters,
            neighbor_count: 0,
            neighbors: [CellId(0); MAX_CELL_NEIGHBORS],
            is_active: false,
            resident_quin_count: 0,
        };
        let idx = self.cells.len();
        self.cells.push(descriptor);
        idx
    }

    /// Link two cells as topological neighbors.
    pub fn link_neighbors(&mut self, a: CellId, b: CellId) -> bool {
        let pos_a = self.cells.iter().position(|c| c.id == a);
        let pos_b = self.cells.iter().position(|c| c.id == b);
        if let (Some(ia), Some(ib)) = (pos_a, pos_b) {
            let ca = &mut self.cells[ia];
            if ca.neighbor_count < MAX_CELL_NEIGHBORS
                && !ca.neighbors[..ca.neighbor_count].contains(&b)
            {
                ca.neighbors[ca.neighbor_count] = b;
                ca.neighbor_count += 1;
            }
            let cb = &mut self.cells[ib];
            if cb.neighbor_count < MAX_CELL_NEIGHBORS
                && !cb.neighbors[..cb.neighbor_count].contains(&a)
            {
                cb.neighbors[cb.neighbor_count] = a;
                cb.neighbor_count += 1;
            }
            true
        } else {
            false
        }
    }

    /// Enqueue an inter-cell routing message. Enforces valid PermissiveRoutingLane bits.
    pub fn route_message(&mut self, msg: InterCellMessage) -> bool {
        if self.message_queue.len() >= MAX_INTER_CELL_MESSAGES {
            return false;
        }
        // Verify lane is either SpatiotemporalAmbiguous (11) or Commons (01)
        if msg.lane_bits != PermissiveRoutingLane::SpatiotemporalAmbiguous as u8
            && msg.lane_bits != PermissiveRoutingLane::EnforcePermissiveCommons as u8
        {
            return false;
        }
        self.message_queue.push(msg);
        true
    }

    /// Drain pending messages destined for a specific cell.
    pub fn drain_messages_for(&mut self, target_cell: CellId) -> Vec<InterCellMessage> {
        let mut target_messages = Vec::new();
        let mut remaining = Vec::with_capacity(self.message_queue.len());
        for msg in self.message_queue.drain(..) {
            if msg.target_cell_id == target_cell.0 {
                target_messages.push(msg);
            } else {
                remaining.push(msg);
            }
        }
        self.message_queue = remaining;
        target_messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cellular_graph_topology_and_routing() {
        let mut graph = CellularLogicGraph::new();
        let kestrel = CellId(101);
        let saltwind = CellId(102);

        graph.register_cell(kestrel, "Kestrel Flats", 0.0, 0.0, 150.0);
        graph.register_cell(saltwind, "Saltwind Reach", 300.0, 50.0, 200.0);
        assert!(graph.link_neighbors(kestrel, saltwind));

        let msg = InterCellMessage {
            source_cell_id: kestrel.0,
            target_cell_id: saltwind.0,
            lane_bits: PermissiveRoutingLane::SpatiotemporalAmbiguous as u8,
            reserved_u8: 0,
            payload_kind: 1,
            reserved_u32: 0,
            quin: NQuin::default(),
        };

        assert!(graph.route_message(msg));
        let delivered = graph.drain_messages_for(saltwind);
        assert_eq!(delivered.len(), 1);
        assert_eq!(delivered[0].source_cell_id, kestrel.0);
        assert!(graph.message_queue.is_empty());
    }
}
