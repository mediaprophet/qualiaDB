//! Acoustic network: nodes, channels, modem/PHY, and the acoustic protocol stack
//! (physical / data-link / network / transport layers) for through-wall and
//! underwater communication.

use super::*;

/// Acoustic network for underwater/through-wall communication
pub struct AcousticNetwork {
    nodes: HashMap<String, AcousticNode>,
    channel_manager: AcousticChannelManager,
    modem_controller: AcousticModemController,
    protocol_handler: AcousticProtocolHandler,
    tx_queue: Vec<StoredMessage>,
}

/// Acoustic node in the network
#[derive(Debug, Clone)]
pub struct AcousticNode {
    pub node_id: String,
    pub node_type: NodeType,
    pub capabilities: AcousticCapabilities,
    pub location: Option<Location>,
    pub status: NodeStatus,
    pub signal_strength: f64,
    pub battery_level: f64,
}

/// Acoustic capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcousticCapabilities {
    pub frequency_range: (f64, f64), // Hz
    pub bandwidth: f64,              // Hz
    pub max_range: f64,              // meters
    pub data_rate: f64,              // bps
    pub modulation: ModulationType,
    pub error_correction: ErrorCorrectionType,
}

/// Modulation types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModulationType {
    FSK,
    PSK,
    OFDM,
    DSSS,
    Chirp,
}

/// Error correction types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorCorrectionType {
    None,
    Hamming,
    ReedSolomon,
    Convolutional,
    LDPC,
}

/// Node status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NodeStatus {
    Active,
    Idle,
    Sleeping,
    Error,
    Offline,
}

/// Acoustic channel manager
pub struct AcousticChannelManager {
    available_channels: Vec<AcousticChannel>,
    active_channels: HashMap<String, AcousticChannel>,
    channel_allocation: ChannelAllocationStrategy,
}

/// Acoustic channel
#[derive(Debug, Clone)]
pub struct AcousticChannel {
    pub channel_id: String,
    pub frequency: f64,
    pub bandwidth: f64,
    pub power_level: f64,
    pub modulation: ModulationType,
    pub noise_floor: f64,
    pub interference_level: f64,
}

/// Channel allocation strategies
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelAllocationStrategy {
    Fixed,
    Dynamic,
    Adaptive,
    Opportunistic,
}

/// Acoustic modem controller
pub struct AcousticModemController {
    modem_type: ModemType,
    transmission_power: f64,
    receiver_sensitivity: f64,
    signal_processing: SignalProcessingConfig,
}

/// Modem types
#[derive(Debug, Clone, PartialEq)]
pub enum ModemType {
    SoftwareDefined,
    HardwareBased,
    Hybrid,
}

/// Signal processing configuration
#[derive(Debug, Clone)]
pub struct SignalProcessingConfig {
    pub sampling_rate: f64,
    pub fft_size: usize,
    pub filter_type: FilterType,
    pub noise_reduction: bool,
    pub equalization: bool,
}

/// Filter types
#[derive(Debug, Clone, PartialEq)]
pub enum FilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    Adaptive,
}

/// Packet handler
pub struct PacketHandler {}

/// Flow control
pub struct FlowControl {}

/// Error handling
pub struct ErrorHandling {}

/// Acoustic protocol handler
pub struct AcousticProtocolHandler {
    protocol_stack: AcousticProtocolStack,
    packet_handler: PacketHandler,
    flow_control: FlowControl,
    error_handling: ErrorHandling,
}

/// Acoustic protocol stack
#[derive(Debug, Clone)]
pub struct AcousticProtocolStack {
    pub physical_layer: PhysicalLayer,
    pub data_link_layer: DataLinkLayer,
    pub network_layer: NetworkLayer,
    pub transport_layer: TransportLayer,
}

/// Physical layer
#[derive(Debug, Clone)]
pub struct PhysicalLayer {
    pub modulation: ModulationType,
    pub coding: ErrorCorrectionType,
    pub frequency_hopping: bool,
    pub power_control: bool,
}

/// Data link layer
#[derive(Debug, Clone)]
pub struct DataLinkLayer {
    pub mac_protocol: MacProtocol,
    pub frame_format: FrameFormat,
    pub error_detection: ErrorDetection,
    pub retransmission: RetransmissionStrategy,
}

/// MAC protocols
#[derive(Debug, Clone, PartialEq)]
pub enum MacProtocol {
    CSMA,
    TDMA,
    FDMA,
    CDMA,
    Hybrid,
}

/// Frame formats
#[derive(Debug, Clone, PartialEq)]
pub enum FrameFormat {
    Fixed,
    Variable,
    Adaptive,
}

/// Error detection
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorDetection {
    CRC,
    Checksum,
    Parity,
    None,
}

/// Retransmission strategies
#[derive(Debug, Clone, PartialEq)]
pub enum RetransmissionStrategy {
    StopAndWait,
    GoBackN,
    SelectiveRepeat,
    Adaptive,
}

/// Network layer
#[derive(Debug, Clone)]
pub struct NetworkLayer {
    pub routing_protocol: RoutingProtocol,
    pub addressing_scheme: AddressingScheme,
    pub fragmentation: bool,
    pub congestion_control: bool,
}

/// Routing protocols
#[derive(Debug, Clone, PartialEq)]
pub enum RoutingProtocol {
    Flooding,
    DistanceVector,
    LinkState,
    Geographic,
    Opportunistic,
}

/// Addressing schemes
#[derive(Debug, Clone, PartialEq)]
pub enum AddressingScheme {
    Hierarchical,
    Flat,
    Geographic,
    ContentBased,
}

/// Transport layer
#[derive(Debug, Clone)]
pub struct TransportLayer {
    pub transport_protocol: TransportProtocol,
    pub reliability: ReliabilityLevel,
    pub flow_control: FlowControlType,
    pub congestion_control: CongestionControlType,
}

/// Transport protocols
#[derive(Debug, Clone, PartialEq)]
pub enum TransportProtocol {
    UDP,
    TCP,
    DTN,
    Custom,
}

/// Reliability levels
#[derive(Debug, Clone, PartialEq)]
pub enum ReliabilityLevel {
    BestEffort,
    Reliable,
    SemiReliable,
    Adaptive,
}

/// Flow control types
#[derive(Debug, Clone, PartialEq)]
pub enum FlowControlType {
    None,
    WindowBased,
    RateBased,
    CreditBased,
}

/// Congestion control types
#[derive(Debug, Clone, PartialEq)]
pub enum CongestionControlType {
    None,
    AIMD,
    RED,
    Custom,
}

impl AcousticNetwork {
    /// Construct a `AcousticNetwork` with default field values.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            channel_manager: AcousticChannelManager::new(),
            modem_controller: AcousticModemController::new(),
            protocol_handler: AcousticProtocolHandler::new(),
            tx_queue: Vec::new(),
        }
    }

    /// Register an acoustic node in the node registry.
    pub fn register_node(&mut self, node: AcousticNode) {
        self.nodes.insert(node.node_id.clone(), node);
    }

    /// Initialize channel manager, modem controller, and protocol handler.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        // Initialize acoustic network components
        self.channel_manager.initialize()?;
        self.modem_controller.initialize()?;
        self.protocol_handler.initialize()?;

        // Register default local transducer node interface if none registered
        if self.nodes.is_empty() {
            let local_transducer = AcousticNode {
                node_id: "acoustic_local_transducer".to_string(),
                node_type: NodeType::Gateway,
                capabilities: AcousticCapabilities {
                    frequency_range: (20000.0, 50000.0),
                    bandwidth: 5000.0,
                    max_range: 2000.0,
                    data_rate: 2400.0,
                    modulation: ModulationType::OFDM,
                    error_correction: ErrorCorrectionType::ReedSolomon,
                },
                location: None,
                status: NodeStatus::Active,
                signal_strength: -30.0,
                battery_level: 100.0,
            };
            self.nodes.insert(local_transducer.node_id.clone(), local_transducer);
        }
        Ok(())
    }

    /// Return the discovered acoustic nodes from the node registry.
    pub fn discover_nodes(&mut self) -> Result<Vec<AcousticNode>, MeshError> {
        Ok(self.nodes.values().cloned().collect())
    }

    /// Enqueue message into the acoustic modem transmission buffer.
    pub fn send_message(&mut self, message: &StoredMessage) -> Result<(), MeshError> {
        self.tx_queue.push(message.clone());
        Ok(())
    }

    /// Enqueue payload into the acoustic transmission buffer.
    pub fn send_payload(
        &mut self,
        destination: &str,
        payload: &[u8],
        priority: MessagePriority,
    ) -> Result<(), MeshError> {
        let message = StoredMessage {
            message_id: format!("acoustic_{}", self.tx_queue.len() + 1),
            source: "local_node".to_string(),
            destination: destination.to_string(),
            payload: payload.to_vec(),
            priority,
            timestamp: Instant::now(),
            ttl: Duration::from_secs(3600),
            delivery_attempts: 0,
            status: MessageStatus::Pending,
        };
        self.tx_queue.push(message);
        Ok(())
    }

    /// Inspect the current acoustic transmission queue.
    pub fn tx_queue(&self) -> &[StoredMessage] {
        &self.tx_queue
    }

    /// Number of nodes in the registry.
    pub fn get_node_count(&self) -> u32 {
        self.nodes.len() as u32
    }

    /// Optimize acoustic discovery across available channels.
    pub fn optimize_discovery(&mut self) -> Result<(), MeshError> {
        if self.channel_manager.active_channel_count() == 0 && self.channel_manager.available_channel_count() > 0 {
            let ch_id = self.channel_manager.available_channels[0].channel_id.clone();
            self.channel_manager.allocate_channel(&ch_id);
        }
        Ok(())
    }
}

impl AcousticChannelManager {
    /// Construct a `AcousticChannelManager` with default field values.
    pub fn new() -> Self {
        Self {
            available_channels: Vec::new(),
            active_channels: HashMap::new(),
            channel_allocation: ChannelAllocationStrategy::Adaptive,
        }
    }

    /// Initialize `AcousticChannelManager`; `Ok` when ready.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        // Pre-allocate standard acoustic channels (20-50 kHz range).
        for i in 0..5 {
            let freq = 20000.0 + i as f64 * 6000.0;
            self.available_channels.push(AcousticChannel {
                channel_id: format!("acoustic_ch_{}", i),
                frequency: freq,
                bandwidth: 1000.0,
                power_level: 100.0,
                modulation: ModulationType::FSK,
                noise_floor: -80.0,
                interference_level: 0.0,
            });
        }
        Ok(())
    }

    /// Allocate `channel_id` if free; `None` when already taken.
    pub fn allocate_channel(&mut self, channel_id: &str) -> Option<&AcousticChannel> {
        if let Some(pos) = self
            .available_channels
            .iter()
            .position(|c| c.channel_id == channel_id)
        {
            let channel = self.available_channels.remove(pos);
            self.active_channels
                .insert(channel.channel_id.clone(), channel);
        }
        self.active_channels.get(channel_id)
    }

    /// Free `channel_id` if allocated.
    pub fn release_channel(&mut self, channel_id: &str) {
        if let Some(channel) = self.active_channels.remove(channel_id) {
            self.available_channels.push(channel);
        }
    }

    /// Channels currently unallocated.
    pub fn available_channel_count(&self) -> usize {
        self.available_channels.len()
    }

    /// Channels currently allocated.
    pub fn active_channel_count(&self) -> usize {
        self.active_channels.len()
    }

    /// The channel allocation strategy.
    pub fn allocation_strategy(&self) -> &ChannelAllocationStrategy {
        &self.channel_allocation
    }
}

impl AcousticModemController {
    /// Construct a `AcousticModemController` with default field values.
    pub fn new() -> Self {
        Self {
            modem_type: ModemType::SoftwareDefined,
            transmission_power: 100.0,    // 100W
            receiver_sensitivity: -120.0, // -120dBm
            signal_processing: SignalProcessingConfig {
                sampling_rate: 192000.0, // 192 kHz
                fft_size: 1024,
                filter_type: FilterType::BandPass,
                noise_reduction: true,
                equalization: true,
            },
        }
    }

    /// Initialize `AcousticModemController`; `Ok` when ready.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        Ok(())
    }

    /// The modem type.
    pub fn modem_type(&self) -> &ModemType {
        &self.modem_type
    }

    /// Configured transmit power.
    pub fn transmission_power(&self) -> f64 {
        self.transmission_power
    }

    /// Set transmit power.
    pub fn set_transmission_power(&mut self, power: f64) {
        self.transmission_power = power.max(0.0);
    }

    /// Configured receiver sensitivity.
    pub fn receiver_sensitivity(&self) -> f64 {
        self.receiver_sensitivity
    }

    /// The signal-processing configuration.
    pub fn signal_processing(&self) -> &SignalProcessingConfig {
        &self.signal_processing
    }

    /// Estimate maximum communication range based on transmission power,
    /// receiver sensitivity, and acoustic spreading loss (15 dB/km).
    pub fn estimated_range_km(&self) -> f64 {
        let snr_margin = self.transmission_power + self.receiver_sensitivity.abs();
        snr_margin / 15.0
    }
}

impl AcousticProtocolHandler {
    /// Construct a `AcousticProtocolHandler` with default field values.
    pub fn new() -> Self {
        Self {
            protocol_stack: AcousticProtocolStack::new(),
            packet_handler: PacketHandler::new(),
            flow_control: FlowControl::new(),
            error_handling: ErrorHandling::new(),
        }
    }

    /// Initialize `AcousticProtocolHandler`; `Ok` when ready.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        Ok(())
    }

    /// The acoustic protocol stack.
    pub fn protocol_stack(&self) -> &AcousticProtocolStack {
        &self.protocol_stack
    }

    /// The packet handler.
    pub fn packet_handler(&self) -> &PacketHandler {
        &self.packet_handler
    }

    /// The flow-control config.
    pub fn flow_control(&self) -> &FlowControl {
        &self.flow_control
    }

    /// The error-handling config.
    pub fn error_handling(&self) -> &ErrorHandling {
        &self.error_handling
    }
}

impl AcousticProtocolStack {
    /// Construct a `AcousticProtocolStack` with default field values.
    pub fn new() -> Self {
        Self {
            physical_layer: PhysicalLayer::new(),
            data_link_layer: DataLinkLayer::new(),
            network_layer: NetworkLayer::new(),
            transport_layer: TransportLayer::new(),
        }
    }
}

impl PhysicalLayer {
    /// Construct a `PhysicalLayer` with default field values.
    pub fn new() -> Self {
        Self {
            modulation: ModulationType::FSK,
            coding: ErrorCorrectionType::ReedSolomon,
            frequency_hopping: true,
            power_control: true,
        }
    }
}

impl DataLinkLayer {
    /// Construct a `DataLinkLayer` with default field values.
    pub fn new() -> Self {
        Self {
            mac_protocol: MacProtocol::CSMA,
            frame_format: FrameFormat::Adaptive,
            error_detection: ErrorDetection::CRC,
            retransmission: RetransmissionStrategy::Adaptive,
        }
    }
}

impl NetworkLayer {
    /// Construct a `NetworkLayer` with default field values.
    pub fn new() -> Self {
        Self {
            routing_protocol: RoutingProtocol::Geographic,
            addressing_scheme: AddressingScheme::Geographic,
            fragmentation: true,
            congestion_control: true,
        }
    }
}

impl TransportLayer {
    /// Construct a `TransportLayer` with default field values.
    pub fn new() -> Self {
        Self {
            transport_protocol: TransportProtocol::DTN,
            reliability: ReliabilityLevel::SemiReliable,
            flow_control: FlowControlType::CreditBased,
            congestion_control: CongestionControlType::RED,
        }
    }
}

impl PacketHandler {
    /// Construct a `PacketHandler` with default field values.
    pub fn new() -> Self {
        Self {}
    }
}

impl FlowControl {
    /// Construct a `FlowControl` with default field values.
    pub fn new() -> Self {
        Self {}
    }
}

impl ErrorHandling {
    /// Construct a `ErrorHandling` with default field values.
    pub fn new() -> Self {
        Self {}
    }
}
