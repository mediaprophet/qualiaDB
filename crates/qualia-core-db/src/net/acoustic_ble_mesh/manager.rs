//! Top-level mesh network manager: orchestrates the acoustic and BLE networks,
//! the router, the data store, and the performance monitor; owns node discovery,
//! message send/receive, routing, and interface selection.

use super::*;

/// Acoustic & BLE Mesh Network Manager
pub struct MeshNetworkManager {
    acoustic_network: AcousticNetwork,
    ble_network: BleNetwork,
    mesh_router: MeshRouter,
    data_store: MeshDataStore,
    performance_monitor: MeshPerformanceMonitor,
}

impl MeshNetworkManager {
    /// Create new mesh network manager
    pub fn new() -> Self {
        Self {
            acoustic_network: AcousticNetwork::new(),
            ble_network: BleNetwork::new(),
            mesh_router: MeshRouter::new(),
            data_store: MeshDataStore::new(),
            performance_monitor: MeshPerformanceMonitor::new(),
        }
    }

    /// Initialize mesh networks
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        // Initialize acoustic network
        self.acoustic_network.initialize()?;

        // Initialize BLE network
        self.ble_network.initialize()?;

        // Initialize mesh router
        self.mesh_router.initialize()?;

        // Initialize data store
        self.data_store.initialize()?;

        Ok(())
    }

    /// Discover nearby nodes
    pub fn discover_nodes(&mut self) -> Result<Vec<DiscoveredNode>, MeshError> {
        let mut discovered_nodes = Vec::new();

        // Discover acoustic nodes
        let acoustic_nodes = self.acoustic_network.discover_nodes()?;
        for node in acoustic_nodes {
            discovered_nodes.push(DiscoveredNode {
                node_id: node.node_id.clone(),
                node_type: node.node_type.clone(),
                interface: NetworkInterface::Acoustic,
                capabilities: NodeCapabilities::Acoustic(node.capabilities.clone()),
                signal_strength: node.signal_strength,
                location: node.location,
            });
        }

        // Discover BLE nodes
        let ble_nodes = self.ble_network.discover_nodes()?;
        for node in ble_nodes {
            discovered_nodes.push(DiscoveredNode {
                node_id: node.node_id.clone(),
                node_type: NodeType::Mobile, // BLE nodes are typically mobile
                interface: NetworkInterface::Ble,
                capabilities: NodeCapabilities::Ble(node.capabilities.clone()),
                signal_strength: node.rssi as f64,
                location: None, // BLE nodes typically don't have location info
            });
        }

        Ok(discovered_nodes)
    }

    /// Discover nearby nodes into a caller-owned zero-heap buffer.
    pub fn discover_nodes_into(
        &mut self,
        out: &mut [DiscoveredNodeHandle],
    ) -> Result<usize, MeshError> {
        let mut written = 0usize;

        let acoustic_nodes = self.acoustic_network.discover_nodes()?;
        for node in acoustic_nodes {
            if written >= out.len() {
                return Err(MeshError::BufferTooSmall(
                    "discovered node output buffer exhausted".to_string(),
                ));
            }
            out[written] = Self::discovered_node_handle(
                &node.node_id,
                node.node_type,
                NetworkInterface::Acoustic,
                Self::acoustic_capability_tag(&node.capabilities),
                node.signal_strength,
                node.location,
            );
            written += 1;
        }

        let ble_nodes = self.ble_network.discover_nodes()?;
        for node in ble_nodes {
            if written >= out.len() {
                return Err(MeshError::BufferTooSmall(
                    "discovered node output buffer exhausted".to_string(),
                ));
            }
            out[written] = Self::discovered_node_handle(
                &node.node_id,
                NodeType::Mobile,
                NetworkInterface::Ble,
                Self::ble_capability_tag(&node.capabilities),
                node.rssi as f64,
                None,
            );
            written += 1;
        }

        Ok(written)
    }

    /// Send message through mesh network
    pub fn send_message(
        &mut self,
        destination: String,
        payload: Vec<u8>,
        priority: MessagePriority,
    ) -> Result<String, MeshError> {
        // Create message
        let message_id = self.generate_message_id();
        let message = StoredMessage {
            message_id: message_id.clone(),
            source: "local_node".to_string(),
            destination: destination.clone(),
            payload,
            priority,
            timestamp: Instant::now(),
            ttl: Duration::from_secs(3600), // 1 hour TTL
            delivery_attempts: 0,
            status: MessageStatus::Pending,
        };

        // Store message
        self.data_store.store_message(message.clone())?;

        // Route message
        self.route_message(&message)?;

        Ok(message_id)
    }

    /// Route a transient payload without cloning it into the heap-backed persistence layer.
    pub fn send_message_ephemeral(
        &mut self,
        destination: &str,
        payload: &[u8],
        priority: MessagePriority,
    ) -> Result<u64, MeshError> {
        let message_hash = self.generate_message_hash(destination, payload, priority);
        self.route_payload(destination, payload, priority)?;
        Ok(message_hash)
    }

    /// Receive message from mesh network
    pub fn receive_message(&mut self, message: StoredMessage) -> Result<(), MeshError> {
        // Store received message
        self.data_store.store_message(message.clone())?;

        // Update performance metrics
        self.performance_monitor.update_receive_metrics(&message);

        // Forward if not destined for this node
        if message.destination != "local_node" && message.ttl.as_secs() > 0 {
            self.route_message(&message)?;
        }

        Ok(())
    }

    /// Get network status
    pub fn get_network_status(&self) -> NetworkStatus {
        NetworkStatus {
            acoustic_nodes: self.acoustic_network.get_node_count(),
            ble_nodes: self.ble_network.get_node_count(),
            total_nodes: self.acoustic_network.get_node_count() + self.ble_network.get_node_count(),
            active_routes: self.mesh_router.get_route_count(),
            pending_messages: self.data_store.get_pending_message_count(),
            network_uptime: self.performance_monitor.get_uptime(),
        }
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> MeshGlobalMetrics {
        self.performance_monitor.get_global_stats()
    }

    /// Optimize network performance
    pub fn optimize_network(&mut self) -> Result<(), MeshError> {
        // Optimize routing
        self.mesh_router.optimize_routes()?;

        // Optimize buffer management
        self.data_store.optimize_buffers()?;

        // Optimize discovery
        self.acoustic_network.optimize_discovery()?;
        self.ble_network.optimize_discovery()?;

        Ok(())
    }

    // Internal methods

    /// Route message through network
    fn route_message(&mut self, message: &StoredMessage) -> Result<(), MeshError> {
        // Determine best interface for routing
        let interface =
            self.select_best_interface_for_payload(message.payload.len(), message.priority)?;

        match interface {
            NetworkInterface::Acoustic => {
                self.acoustic_network.send_message(message)?;
            }
            NetworkInterface::Ble => {
                self.ble_network.send_message(message)?;
            }
            NetworkInterface::Hybrid => {
                // Use both interfaces for redundancy
                self.acoustic_network.send_message(message)?;
                self.ble_network.send_message(message)?;
            }
        }

        Ok(())
    }

    /// Route a transient payload without constructing a heap-backed message envelope.
    fn route_payload(
        &mut self,
        destination: &str,
        payload: &[u8],
        priority: MessagePriority,
    ) -> Result<(), MeshError> {
        let interface = self.select_best_interface_for_payload(payload.len(), priority)?;

        match interface {
            NetworkInterface::Acoustic => {
                self.acoustic_network
                    .send_payload(destination, payload, priority)?
            }
            NetworkInterface::Ble => {
                self.ble_network
                    .send_payload(destination, payload, priority)?
            }
            NetworkInterface::Hybrid => {
                self.acoustic_network
                    .send_payload(destination, payload, priority)?;
                self.ble_network
                    .send_payload(destination, payload, priority)?;
            }
        }

        Ok(())
    }

    /// Select best interface for message routing
    fn select_best_interface_for_payload(
        &self,
        payload_len: usize,
        priority: MessagePriority,
    ) -> Result<NetworkInterface, MeshError> {
        // Simple selection logic - in real implementation would be more sophisticated
        if payload_len > 1000 {
            // Large payload - use acoustic
            Ok(NetworkInterface::Acoustic)
        } else if priority == MessagePriority::Critical {
            // Critical message - use both for redundancy
            Ok(NetworkInterface::Hybrid)
        } else {
            // Default to BLE for small messages
            Ok(NetworkInterface::Ble)
        }
    }

    /// Generate unique message ID
    fn generate_message_id(&self) -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        format!("msg_{}", COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    fn generate_message_hash(
        &self,
        destination: &str,
        payload: &[u8],
        priority: MessagePriority,
    ) -> u64 {
        let mut payload_hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in payload {
            payload_hash ^= *byte as u64;
            payload_hash = payload_hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        q_hash(destination) ^ payload_hash ^ (priority as u64)
    }

    fn discovered_node_handle(
        node_id: &str,
        node_type: NodeType,
        interface: NetworkInterface,
        capability_tag: u8,
        signal_strength: f64,
        location: Option<Location>,
    ) -> DiscoveredNodeHandle {
        DiscoveredNodeHandle {
            node_id_hash: q_hash(node_id),
            node_type,
            interface,
            capability_tag,
            signal_strength,
            location,
        }
    }

    fn acoustic_capability_tag(capabilities: &AcousticCapabilities) -> u8 {
        match capabilities.modulation {
            ModulationType::FSK => 0x01,
            ModulationType::PSK => 0x02,
            ModulationType::OFDM => 0x03,
            ModulationType::DSSS => 0x04,
            ModulationType::Chirp => 0x05,
        }
    }

    fn ble_capability_tag(capabilities: &BleCapabilities) -> u8 {
        let mut tag = 0u8;
        if capabilities
            .features
            .iter()
            .any(|feature| *feature == BleFeature::ExtendedAdvertising)
        {
            tag |= 0x01;
        }
        if capabilities
            .features
            .iter()
            .any(|feature| *feature == BleFeature::LE2MPHY)
        {
            tag |= 0x02;
        }
        if capabilities
            .features
            .iter()
            .any(|feature| *feature == BleFeature::LEDataPacketLengthExtension)
        {
            tag |= 0x04;
        }
        if tag == 0 {
            0x10
        } else {
            tag
        }
    }
}
