//! Workspace state — serializable workspace with device assignments and sync.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use super::device::{DeviceProfile, DeviceStatus};
use super::registry::ManifoldSeed;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// DeviceRole
// ---------------------------------------------------------------------------

/// The role a device plays in the workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceRole {
    /// Full manifold canvas + all containers.
    Primary,
    /// Extended canvas — additional containers on a second display.
    Secondary,
    /// Compact control surface — manifold switcher, command triggers.
    Remote,
    /// Dock panels only — inspector, property sheet, toolbox.
    ControlSurface,
    /// Headless — runs compute, streams results.
    Compute,
    /// Read-only canvas view — presentations, monitoring.
    DisplayOnly,
}

impl Default for DeviceRole {
    fn default() -> Self {
        Self::Primary
    }
}

impl DeviceRole {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Secondary => "Secondary Display",
            Self::Remote => "Remote Control",
            Self::ControlSurface => "Control Surface",
            Self::Compute => "Compute Node",
            Self::DisplayOnly => "Display Only",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Primary => "Full manifold canvas with all containers and panels.",
            Self::Secondary => "Extended canvas for additional containers on extra displays.",
            Self::Remote => {
                "Compact control surface for triggering commands and switching manifolds."
            }
            Self::ControlSurface => "Dock panels only \u{2014} inspector, property sheet, toolbox.",
            Self::Compute => "Headless compute node \u{2014} runs processing, streams results.",
            Self::DisplayOnly => "Read-only canvas view for presentations and monitoring.",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Primary => "\u{1F5A5}",
            Self::Secondary => "\u{1F5A5}\u{2795}",
            Self::Remote => "\u{1F4F1}",
            Self::ControlSurface => "\u{1F4F2}",
            Self::Compute => "\u{1F916}",
            Self::DisplayOnly => "\u{1F4FA}",
        }
    }
}

// ---------------------------------------------------------------------------
// DeviceAssignment
// ---------------------------------------------------------------------------

/// Assignment of a container to a specific device and display.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeviceAssignment {
    /// Container identifier (type + position hash).
    pub container_id: String,
    /// Target device DID.
    pub device_id: String,
    /// Role the device plays for this container.
    pub role: DeviceRole,
    /// Specific display on the device (if multi-monitor).
    pub display_id: Option<String>,
}

// ---------------------------------------------------------------------------
// ContainerOverride
// ---------------------------------------------------------------------------

/// A per-device override for a container (e.g. different position on laptop).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ContainerOverride {
    /// Container identifier.
    pub container_id: String,
    /// Device DID this override applies to.
    pub device_id: String,
    /// Override position x.
    pub x: Option<f32>,
    /// Override position y.
    pub y: Option<f32>,
    /// Override width.
    pub width: Option<f32>,
    /// Override height.
    pub height: Option<f32>,
    /// Override display.
    pub display_id: Option<String>,
}

// ---------------------------------------------------------------------------
// WorkspaceDelta
// ---------------------------------------------------------------------------

/// A change to the workspace state, signed by a device.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceDelta {
    /// Workspace version this delta applies to.
    pub base_version: u64,
    /// Resulting version after applying.
    pub new_version: u64,
    /// Device that produced this delta.
    pub device_id: String,
    /// Timestamp (Unix seconds).
    pub timestamp: i64,
    /// The change itself.
    pub change: DeltaChange,
    /// Cryptographic signature (mock — empty in P0).
    pub signature: String,
}

/// The kind of change in a workspace delta.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeltaChange {
    /// Container moved to new position.
    ContainerMoved {
        container_id: String,
        x: f32,
        y: f32,
    },
    /// Container added.
    ContainerAdded {
        container_type: String,
        title: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Container removed.
    ContainerRemoved { container_id: String },
    /// Active manifold changed.
    ManifoldChanged { manifold_id: String },
    /// Container assigned to a device.
    DeviceAssigned { assignment: DeviceAssignment },
    /// Container unassigned from a device.
    DeviceUnassigned {
        container_id: String,
        device_id: String,
    },
    /// Device paired.
    DevicePaired { device: DeviceProfile },
    /// Device revoked.
    DeviceRevoked { device_id: String },
}

// ---------------------------------------------------------------------------
// WorkspaceState
// ---------------------------------------------------------------------------

/// The complete workspace state — serializable and syncable across devices.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct WorkspaceState {
    /// Unique workspace ID.
    pub workspace_id: String,
    /// Owner DID — e.g. `did:qualia:timothy_charles_holborn`.
    pub owner_did: String,
    /// All manifold seeds in the workspace.
    pub manifolds: Vec<ManifoldSeed>,
    /// Currently active manifold ID.
    pub active_manifold: String,
    /// Per-device container overrides.
    pub container_overrides: Vec<ContainerOverride>,
    /// Container-to-device assignments.
    pub device_assignments: Vec<DeviceAssignment>,
    /// All known paired devices.
    pub devices: Vec<DeviceProfile>,
    /// Monotonic version counter.
    pub version: u64,
    /// Last modified timestamp (Unix seconds).
    pub last_modified: i64,
    /// Crypto chain signature (mock — empty in P0).
    pub signature: String,
}

impl WorkspaceState {
    /// Create a new workspace for a user.
    pub fn new(workspace_id: &str, owner_did: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            owner_did: owner_did.to_string(),
            manifolds: Vec::new(),
            active_manifold: String::new(),
            container_overrides: Vec::new(),
            device_assignments: Vec::new(),
            devices: Vec::new(),
            version: 1,
            last_modified: 0,
            signature: String::new(),
        }
    }

    /// Find a device by ID.
    pub fn device(&self, device_id: &str) -> Option<&DeviceProfile> {
        self.devices.iter().find(|d| d.device_id == device_id)
    }

    /// All online devices.
    pub fn online_devices(&self) -> Vec<&DeviceProfile> {
        self.devices
            .iter()
            .filter(|d| d.status == DeviceStatus::Online)
            .collect()
    }

    /// Assignments for a specific device.
    pub fn assignments_for(&self, device_id: &str) -> Vec<&DeviceAssignment> {
        self.device_assignments
            .iter()
            .filter(|a| a.device_id == device_id)
            .collect()
    }

    /// Apply a delta to the workspace state.
    pub fn apply_delta(&mut self, delta: &WorkspaceDelta) {
        match &delta.change {
            DeltaChange::ContainerMoved { container_id, x, y } => {
                for m in &mut self.manifolds {
                    for c in &mut m.containers {
                        if format!("{}:{}", c.container_type, c.title) == *container_id {
                            c.x = *x;
                            c.y = *y;
                        }
                    }
                }
            }
            DeltaChange::ManifoldChanged { manifold_id } => {
                self.active_manifold = manifold_id.clone();
            }
            DeltaChange::DevicePaired { device } => {
                if !self.devices.iter().any(|d| d.device_id == device.device_id) {
                    self.devices.push(device.clone());
                }
            }
            DeltaChange::DeviceRevoked { device_id } => {
                self.devices.retain(|d| d.device_id != *device_id);
                self.device_assignments
                    .retain(|a| a.device_id != *device_id);
            }
            DeltaChange::DeviceAssigned { assignment } => {
                self.device_assignments.retain(|a| {
                    !(a.container_id == assignment.container_id
                        && a.device_id == assignment.device_id)
                });
                self.device_assignments.push(assignment.clone());
            }
            DeltaChange::DeviceUnassigned {
                container_id,
                device_id,
            } => {
                self.device_assignments
                    .retain(|a| !(a.container_id == *container_id && a.device_id == *device_id));
            }
            DeltaChange::ContainerAdded {
                container_type,
                title,
                x,
                y,
                width,
                height,
            } => {
                if let Some(m) = self
                    .manifolds
                    .iter_mut()
                    .find(|m| m.id == self.active_manifold)
                {
                    m.containers.push(super::registry::SeedContainer {
                        container_type: container_type.clone(),
                        title: title.clone(),
                        x: *x,
                        y: *y,
                        width: *width,
                        height: *height,
                        ..Default::default()
                    });
                }
            }
            DeltaChange::ContainerRemoved { container_id } => {
                for m in &mut self.manifolds {
                    m.containers
                        .retain(|c| format!("{}:{}", c.container_type, c.title) != *container_id);
                }
            }
        }
        self.version = delta.new_version;
        self.last_modified = delta.timestamp;
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::device::{DeviceProfile, DeviceType};
    use super::*;

    #[test]
    fn workspace_construction() {
        let ws = WorkspaceState::new("ws1", "did:qualia:timothy_charles_holborn");
        assert_eq!(ws.version, 1);
        assert_eq!(ws.owner_did, "did:qualia:timothy_charles_holborn");
    }

    #[test]
    fn device_pair_unpair() {
        let mut ws = WorkspaceState::new("ws1", "did:qualia:user");
        let dev = DeviceProfile::new("did:qualia:device:1", DeviceType::Phone, "Phone");
        ws.apply_delta(&WorkspaceDelta {
            base_version: 1,
            new_version: 2,
            device_id: "did:qualia:device:0".into(),
            timestamp: 100,
            change: DeltaChange::DevicePaired {
                device: dev.clone(),
            },
            signature: String::new(),
        });
        assert_eq!(ws.devices.len(), 1);
        assert_eq!(ws.version, 2);

        ws.apply_delta(&WorkspaceDelta {
            base_version: 2,
            new_version: 3,
            device_id: "did:qualia:device:0".into(),
            timestamp: 200,
            change: DeltaChange::DeviceRevoked {
                device_id: "did:qualia:device:1".into(),
            },
            signature: String::new(),
        });
        assert_eq!(ws.devices.len(), 0);
        assert_eq!(ws.version, 3);
    }

    #[test]
    fn manifold_switch() {
        let mut ws = WorkspaceState::new("ws1", "did:qualia:user");
        ws.active_manifold = "social".into();
        ws.apply_delta(&WorkspaceDelta {
            base_version: 1,
            new_version: 2,
            device_id: "d0".into(),
            timestamp: 100,
            change: DeltaChange::ManifoldChanged {
                manifold_id: "ontology".into(),
            },
            signature: String::new(),
        });
        assert_eq!(ws.active_manifold, "ontology");
    }

    #[test]
    fn device_assignment() {
        let mut ws = WorkspaceState::new("ws1", "did:qualia:user");
        let assignment = DeviceAssignment {
            container_id: "graph_canvas:Semantic Graph".into(),
            device_id: "did:qualia:device:1".into(),
            role: DeviceRole::Primary,
            display_id: None,
        };
        ws.apply_delta(&WorkspaceDelta {
            base_version: 1,
            new_version: 2,
            device_id: "d0".into(),
            timestamp: 100,
            change: DeltaChange::DeviceAssigned { assignment },
            signature: String::new(),
        });
        assert_eq!(ws.device_assignments.len(), 1);
        let for_dev = ws.assignments_for("did:qualia:device:1");
        assert_eq!(for_dev.len(), 1);
    }
}
