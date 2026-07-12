//! Shared types for the acoustic & BLE mesh: common enums, discovery results,
//! network status, error types, and compact bit-width aliases.

use super::*;

/// Node types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    Sensor,
    Processor,
    Gateway,
    Mobile,
    Fixed,
}

/// Location information
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    pub accuracy: f64,
}

/// Network interfaces
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkInterface {
    Acoustic,
    Ble,
    Hybrid,
}

/// Message priorities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessagePriority {
    Critical,
    High,
    Normal,
    Low,
    Background,
}

/// Message status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MessageStatus {
    Pending,
    InTransit,
    Delivered,
    Expired,
    Failed,
}

// Supporting types

#[derive(Debug, Clone)]
pub struct DiscoveredNode {
    pub node_id: String,
    pub node_type: NodeType,
    pub interface: NetworkInterface,
    pub capabilities: NodeCapabilities,
    pub signal_strength: f64,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscoveredNodeHandle {
    pub node_id_hash: u64,
    pub node_type: NodeType,
    pub interface: NetworkInterface,
    pub capability_tag: u8,
    pub signal_strength: f64,
    pub location: Option<Location>,
}

#[derive(Debug, Clone)]
pub enum NodeCapabilities {
    Acoustic(AcousticCapabilities),
    Ble(BleCapabilities),
    Hybrid(AcousticCapabilities, BleCapabilities),
}

#[derive(Debug, Clone)]
pub struct NetworkStatus {
    pub acoustic_nodes: u32,
    pub ble_nodes: u32,
    pub total_nodes: u32,
    pub active_routes: u32,
    pub pending_messages: u32,
    pub network_uptime: Duration,
}

/// Mesh error types
#[derive(Debug, Clone)]
pub enum MeshError {
    InitializationError(String),
    DiscoveryError(String),
    RoutingError(String),
    TransmissionError(String),
    StorageError(String),
    ConfigurationError(String),
    SecurityError(String),
    BufferTooSmall(String),
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshError::InitializationError(msg) => write!(f, "Initialization error: {}", msg),
            MeshError::DiscoveryError(msg) => write!(f, "Discovery error: {}", msg),
            MeshError::RoutingError(msg) => write!(f, "Routing error: {}", msg),
            MeshError::TransmissionError(msg) => write!(f, "Transmission error: {}", msg),
            MeshError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            MeshError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            MeshError::SecurityError(msg) => write!(f, "Security error: {}", msg),
            MeshError::BufferTooSmall(msg) => write!(f, "Buffer too small: {}", msg),
        }
    }
}

impl std::error::Error for MeshError {}

// Bit type aliases for compact representation
#[allow(non_camel_case_types)]
pub type u4 = u8;
#[allow(non_camel_case_types)]
pub type u3 = u8;
#[allow(non_camel_case_types)]
pub type u5 = u8;
#[allow(non_camel_case_types)]
pub type u12 = u16;
