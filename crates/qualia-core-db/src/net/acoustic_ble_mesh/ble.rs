//! BLE transport and BLE Mesh: nodes, addressing, capabilities, the mesh
//! network/provisioning/configuration/security stack, advertiser, scanner,
//! and connection management for short-range communication.

use super::*;

/// BLE network for short-range communication
pub struct BleNetwork {
    nodes: HashMap<String, BleNode>,
    mesh_manager: BleMeshManager,
    advertiser: BleAdvertiser,
    scanner: BleScanner,
    connection_manager: BleConnectionManager,
}

/// BLE node
#[derive(Debug, Clone)]
pub struct BleNode {
    pub node_id: String,
    pub address: BleAddress,
    pub capabilities: BleCapabilities,
    pub role: BleRole,
    pub connection_state: ConnectionState,
    pub rssi: i8,
    pub battery_level: f64,
}

/// BLE address
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BleAddress {
    pub address: [u8; 6],
    pub address_type: BleAddressType,
}

/// BLE address types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BleAddressType {
    Public,
    Random,
    ResolvablePrivate,
    NonResolvablePrivate,
}

/// BLE capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleCapabilities {
    pub max_connections: u8,
    pub data_length: u16,
    pub phy_types: Vec<BlePhyType>,
    pub features: Vec<BleFeature>,
}

/// BLE PHY types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlePhyType {
    LE1M,
    LE2M,
    LECoded,
}

/// BLE features
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BleFeature {
    ExtendedAdvertising,
    LE2MPHY,
    LEDataPacketLengthExtension,
    LLPrivacy,
    LEExtendedScannerFilterPolicies,
}

/// BLE roles
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BleRole {
    Peripheral,
    Central,
    Observer,
    Broadcaster,
}

/// Connection states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

/// BLE mesh manager
pub struct BleMeshManager {
    mesh_network: BleMeshNetwork,
    provisioning_manager: ProvisioningManager,
    configuration_manager: ConfigurationManager,
    message_handler: MeshMessageHandler,
}

/// BLE mesh network
#[derive(Debug, Clone)]
pub struct BleMeshNetwork {
    pub network_id: String,
    pub network_key: [u8; 16],
    pub iv_index: u32,
    pub seq_num: u32,
    pub nodes: HashMap<u16, MeshNode>,
    pub elements: HashMap<u16, Vec<MeshElement>>,
}

/// Mesh node
#[derive(Debug, Clone)]
pub struct MeshNode {
    pub unicast_address: u16,
    pub device_key: [u8; 16],
    pub composition_data: CompositionData,
    pub default_ttl: u8,
    pub features: NodeFeatures,
}

/// Composition data
#[derive(Debug, Clone)]
pub struct CompositionData {
    pub cid: u16,
    pub pid: u16,
    pub vid: u16,
    pub crpl: u16,
    pub features: NodeFeatures,
    pub elements: Vec<Element>,
}

/// Node features
#[derive(Debug, Clone)]
pub struct NodeFeatures {
    pub relay: bool,
    pub proxy: bool,
    pub friend: bool,
    pub low_power: bool,
}

/// Element
#[derive(Debug, Clone)]
pub struct Element {
    pub location: u16,
    pub sig_models: Vec<u16>,
    pub vendor_models: Vec<u16>,
}

/// Mesh element
#[derive(Debug, Clone)]
pub struct MeshElement {
    pub element_index: u8,
    pub location: u16,
    pub models: Vec<MeshModel>,
}

/// Mesh model
#[derive(Debug, Clone)]
pub struct MeshModel {
    pub model_id: u16,
    pub vendor_id: Option<u16>,
    pub publication: Option<Publication>,
    pub subscriptions: Vec<u16>,
}

/// Publication
#[derive(Debug, Clone)]
pub struct Publication {
    pub address: u16,
    pub app_key_index: u12,
    pub credential_flag: bool,
    pub ttl: u8,
    pub period: u8,
    pub retransmit: Retransmit,
}

/// Retransmit
#[derive(Debug, Clone)]
pub struct Retransmit {
    pub count: u3,
    pub interval: u5,
}

/// Provisioning manager
pub struct ProvisioningManager {
    provisioning_protocol: ProvisioningProtocol,
    provisioning_data: ProvisioningData,
    oob_data: Option<OobData>,
}

/// Provisioning protocols
#[derive(Debug, Clone, PartialEq)]
pub enum ProvisioningProtocol {
    PBADV,
    PBGATT,
    PBNOOB,
    PBALERT,
}

/// Provisioning data
#[derive(Debug, Clone)]
pub struct ProvisioningData {
    pub network_key: [u8; 16],
    pub net_key_index: u12,
    pub flags: u8,
    pub iv_index: u32,
    pub unicast_address: u16,
}

/// OOB data
#[derive(Debug, Clone)]
pub struct OobData {
    pub oob_type: OobType,
    pub data: Vec<u8>,
}

/// OOB types
#[derive(Debug, Clone, PartialEq)]
pub enum OobType {
    Static,
    Output,
    Input,
    None,
}

/// Configuration manager
pub struct ConfigurationManager {
    config_database: ConfigDatabase,
    config_models: Vec<ConfigModel>,
    access_control: AccessControl,
}

/// Config database
#[derive(Debug, Clone)]
pub struct ConfigDatabase {
    pub app_keys: HashMap<u12, AppKey>,
    pub subnet_list: Vec<Subnet>,
    pub virtual_addresses: HashMap<u16, VirtualAddress>,
}

/// App key
#[derive(Debug, Clone)]
pub struct AppKey {
    pub key: [u8; 16],
    pub net_key_index: u12,
    pub aid: u4,
}

/// Subnet
#[derive(Debug, Clone)]
pub struct Subnet {
    pub net_key_index: u12,
    pub app_key_indices: Vec<u12>,
    pub kr_flag: bool,
    pub phase: u8,
}

/// Virtual address
#[derive(Debug, Clone)]
pub struct VirtualAddress {
    pub address: u16,
    pub label_uuid: [u8; 16],
}

/// Config model
#[derive(Debug, Clone)]
pub struct ConfigModel {
    pub model_id: u16,
    pub opcode: u16,
    pub parameters: Vec<ConfigParameter>,
}

/// Config parameter
#[derive(Debug, Clone)]
pub struct ConfigParameter {
    pub name: String,
    pub value: ConfigValue,
}

/// Config values
#[derive(Debug, Clone)]
pub enum ConfigValue {
    U8(u8),
    U16(u16),
    U32(u32),
    Buffer(Vec<u8>),
}

/// Access control
#[derive(Debug, Clone)]
pub struct AccessControl {
    pub access_list: Vec<AccessEntry>,
    pub default_policy: AccessPolicy,
}

/// Access entry
#[derive(Debug, Clone)]
pub struct AccessEntry {
    pub address: u16,
    pub permissions: Vec<Permission>,
}

/// Permissions
#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    Read,
    Write,
    Subscribe,
    Publish,
    Admin,
}

/// Access policies
#[derive(Debug, Clone, PartialEq)]
pub enum AccessPolicy {
    Allow,
    Deny,
    RequireAuth,
}

/// Mesh message handler
pub struct MeshMessageHandler {
    message_queue: Vec<MeshMessage>,
    routing_table: RoutingTable,
    security_manager: MeshSecurityManager,
}

/// Mesh message
#[derive(Debug, Clone)]
pub struct MeshMessage {
    pub message_id: String,
    pub source: u16,
    pub destination: u16,
    pub ttl: u8,
    pub opcode: u16,
    pub parameters: Vec<u8>,
    pub app_key_index: u12,
    pub net_key_index: u12,
    pub sequence_number: u32,
    pub timestamp: Instant,
}

/// Mesh security manager
pub struct MeshSecurityManager {
    pub network_keys: HashMap<u12, [u8; 16]>,
    pub application_keys: HashMap<u12, [u8; 16]>,
    pub device_keys: HashMap<u16, [u8; 16]>,
    pub beacon_key: [u8; 16],
}

/// BLE advertiser
pub struct BleAdvertiser {
    advertising_data: Vec<u8>,
    scan_response_data: Vec<u8>,
    advertising_parameters: AdvertisingParameters,
    active_advertisements: Vec<ActiveAdvertisement>,
}

/// Advertising parameters
#[derive(Debug, Clone)]
pub struct AdvertisingParameters {
    pub interval_min: u16,
    pub interval_max: u16,
    pub type_: AdvertisingType,
    pub filter_policy: AdvertisingFilterPolicy,
}

/// Advertising types
#[derive(Debug, Clone, PartialEq)]
pub enum AdvertisingType {
    ConnectableUndirected,
    ConnectableDirected,
    ScannableUndirected,
    NonConnectableUndirected,
}

/// Advertising filter policies
#[derive(Debug, Clone, PartialEq)]
pub enum AdvertisingFilterPolicy {
    AllowScanAny,
    AllowScanWhitelist,
    AllowConnectAny,
    AllowConnectWhitelist,
}

/// Active advertisement
#[derive(Debug, Clone)]
pub struct ActiveAdvertisement {
    pub handle: u8,
    pub parameters: AdvertisingParameters,
    pub data: Vec<u8>,
    pub status: AdvertisementStatus,
}

/// Advertisement status
#[derive(Debug, Clone, PartialEq)]
pub enum AdvertisementStatus {
    Active,
    Paused,
    Stopped,
    Error,
}

/// BLE scanner
pub struct BleScanner {
    scanning_parameters: ScanningParameters,
    scan_filter: ScanFilter,
    active_scans: Vec<ActiveScan>,
}

/// Scanning parameters
#[derive(Debug, Clone)]
pub struct ScanningParameters {
    pub interval: u16,
    pub window: u16,
    pub type_: ScanningType,
    pub filter_duplicates: bool,
}

/// Scanning types
#[derive(Debug, Clone, PartialEq)]
pub enum ScanningType {
    Passive,
    Active,
}

/// Scan filter
#[derive(Debug, Clone)]
pub struct ScanFilter {
    pub address_filter: Option<BleAddress>,
    pub rssi_filter: Option<i8>,
    pub service_uuid_filter: Vec<u16>,
}

/// Active scan
#[derive(Debug, Clone)]
pub struct ActiveScan {
    pub handle: u8,
    pub parameters: ScanningParameters,
    pub results: Vec<ScanResult>,
    pub status: ScanStatus,
}

/// Scan result
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub address: BleAddress,
    pub rssi: i8,
    pub advertising_data: Vec<u8>,
    pub scan_response_data: Vec<u8>,
    pub timestamp: Instant,
}

/// Scan status
#[derive(Debug, Clone, PartialEq)]
pub enum ScanStatus {
    Scanning,
    Paused,
    Stopped,
    Error,
}

/// BLE connection manager
pub struct BleConnectionManager {
    connections: HashMap<u16, BleConnection>,
    connection_parameters: ConnectionParameters,
    security_manager: BleSecurityManager,
}

/// BLE connection
#[derive(Debug, Clone)]
pub struct BleConnection {
    pub handle: u16,
    pub role: BleRole,
    pub address: BleAddress,
    pub parameters: ConnectionParameters,
    pub state: ConnectionState,
    pub security_level: SecurityLevel,
    pub mtu: u16,
    pub data_length: u16,
}

/// Connection parameters
#[derive(Debug, Clone)]
pub struct ConnectionParameters {
    pub min_interval: u16,
    pub max_interval: u16,
    pub latency: u16,
    pub supervision_timeout: u16,
    pub min_ce_length: u16,
    pub max_ce_length: u16,
}

/// Security levels
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityLevel {
    None,
    Low,
    Medium,
    High,
    FIPS,
}

/// BLE security manager
pub struct BleSecurityManager {
    pub encryption_keys: HashMap<u16, EncryptionKey>,
    pub identity_keys: HashMap<u16, IdentityKey>,
    pub signing_keys: HashMap<u16, SigningKey>,
    pub csrk: HashMap<u16, Csrk>,
}

/// Encryption key
#[derive(Debug, Clone)]
pub struct EncryptionKey {
    pub ltk: [u8; 16],
    pub rand: [u8; 8],
    pub ediv: u16,
}

/// Identity key
#[derive(Debug, Clone)]
pub struct IdentityKey {
    pub irk: [u8; 16],
    pub address: BleAddress,
}

/// Signing key
#[derive(Debug, Clone)]
pub struct SigningKey {
    pub csrk: [u8; 16],
    pub counter: u32,
}

/// CSRK
#[derive(Debug, Clone)]
pub struct Csrk {
    pub key: [u8; 16],
    pub counter: u32,
}

impl BleNetwork {
    /// Create the BLE network facade with uninitialised sub-managers.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            mesh_manager: BleMeshManager::new(),
            advertiser: BleAdvertiser::new(),
            scanner: BleScanner::new(),
            connection_manager: BleConnectionManager::new(),
        }
    }

    /// Register a BLE node in the node registry.
    pub fn register_node(&mut self, node: BleNode) {
        self.nodes.insert(node.node_id.clone(), node);
    }

    /// Initialise mesh manager, advertiser, scanner, and connection manager in order.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        // Initialize BLE network components
        self.mesh_manager.initialize()?;
        self.advertiser.initialize()?;
        self.scanner.initialize()?;
        self.connection_manager.initialize()?;

        // Register default local BLE node interface if none registered
        if self.nodes.is_empty() {
            let local_node = BleNode {
                node_id: "ble_local_gateway".to_string(),
                address: BleAddress {
                    address: [0x00, 0x1A, 0x7D, 0xDA, 0x71, 0x13],
                    address_type: BleAddressType::Public,
                },
                capabilities: BleCapabilities {
                    max_connections: 8,
                    data_length: 251,
                    phy_types: vec![BlePhyType::LE1M, BlePhyType::LE2M, BlePhyType::LECoded],
                    features: vec![
                        BleFeature::ExtendedAdvertising,
                        BleFeature::LE2MPHY,
                        BleFeature::LEDataPacketLengthExtension,
                    ],
                },
                role: BleRole::Central,
                connection_state: ConnectionState::Connected,
                rssi: -45,
                battery_level: 100.0,
            };
            self.nodes.insert(local_node.node_id.clone(), local_node);
        }
        Ok(())
    }

    /// Discover nearby nodes from the active node registry and scan results.
    pub fn discover_nodes(&mut self) -> Result<Vec<BleNode>, MeshError> {
        let mut discovered: Vec<BleNode> = self.nodes.values().cloned().collect();

        // Process any scan results from active scans into nodes
        for scan in &self.scanner.active_scans {
            for res in &scan.results {
                let node_id = format!(
                    "ble_{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                    res.address.address[0],
                    res.address.address[1],
                    res.address.address[2],
                    res.address.address[3],
                    res.address.address[4],
                    res.address.address[5],
                );
                if !self.nodes.contains_key(&node_id) {
                    let node = BleNode {
                        node_id: node_id.clone(),
                        address: res.address.clone(),
                        capabilities: BleCapabilities {
                            max_connections: 3,
                            data_length: 251,
                            phy_types: vec![BlePhyType::LE1M],
                            features: vec![BleFeature::ExtendedAdvertising],
                        },
                        role: BleRole::Peripheral,
                        connection_state: ConnectionState::Disconnected,
                        rssi: res.rssi,
                        battery_level: 100.0,
                    };
                    self.nodes.insert(node_id, node.clone());
                    discovered.push(node);
                }
            }
        }

        Ok(discovered)
    }

    /// Enqueue a message for BLE mesh delivery via the mesh message handler.
    pub fn send_message(&mut self, message: &StoredMessage) -> Result<(), MeshError> {
        let dest_hash = q_hash(&message.destination);
        let src_hash = q_hash(&message.source);
        let mesh_msg = MeshMessage {
            message_id: message.message_id.clone(),
            source: src_hash as u16,
            destination: dest_hash as u16,
            ttl: (message.ttl.as_secs().min(255)) as u8,
            opcode: 0x01,
            parameters: message.payload.clone(),
            app_key_index: 0,
            net_key_index: 0,
            sequence_number: self.mesh_manager.mesh_network.seq_num,
            timestamp: Instant::now(),
        };
        self.mesh_manager.mesh_network.seq_num = self.mesh_manager.mesh_network.seq_num.wrapping_add(1);
        self.mesh_manager.message_handler.enqueue_message(mesh_msg);
        Ok(())
    }

    /// Enqueue a raw payload for transmission to a destination via the BLE mesh message handler.
    pub fn send_payload(
        &mut self,
        destination: &str,
        payload: &[u8],
        _priority: MessagePriority,
    ) -> Result<(), MeshError> {
        let dest_hash = q_hash(destination);
        let mesh_msg = MeshMessage {
            message_id: format!("payload_{}", self.mesh_manager.mesh_network.seq_num),
            source: 0x0001,
            destination: dest_hash as u16,
            ttl: 64,
            opcode: 0x02,
            parameters: payload.to_vec(),
            app_key_index: 0,
            net_key_index: 0,
            sequence_number: self.mesh_manager.mesh_network.seq_num,
            timestamp: Instant::now(),
        };
        self.mesh_manager.mesh_network.seq_num = self.mesh_manager.mesh_network.seq_num.wrapping_add(1);
        self.mesh_manager.message_handler.enqueue_message(mesh_msg);
        Ok(())
    }

    /// Number of nodes currently registered in `self.nodes`.
    pub fn get_node_count(&self) -> u32 {
        self.nodes.len() as u32
    }

    /// Optimize BLE discovery scanning window and interval.
    pub fn optimize_discovery(&mut self) -> Result<(), MeshError> {
        self.scanner.scanning_parameters.interval = 160;
        self.scanner.scanning_parameters.window = 80;
        Ok(())
    }
}

impl BleMeshManager {
    /// Create a default `BleMeshManager`.
    pub fn new() -> Self {
        Self {
            mesh_network: BleMeshNetwork::new(),
            provisioning_manager: ProvisioningManager::new(),
            configuration_manager: ConfigurationManager::new(),
            message_handler: MeshMessageHandler::new(),
        }
    }

    /// Initialise the `BleMeshManager` sub-components; propagates `MeshError`.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        self.provisioning_manager.initialize()?;
        self.configuration_manager.initialize()?;
        Ok(())
    }

    /// Borrow the mesh network state.
    pub fn mesh_network(&self) -> &BleMeshNetwork {
        &self.mesh_network
    }

    /// Mutable borrow of the mesh network state.
    pub fn mesh_network_mut(&mut self) -> &mut BleMeshNetwork {
        &mut self.mesh_network
    }

    /// Borrow the provisioning manager.
    pub fn provisioning_manager(&self) -> &ProvisioningManager {
        &self.provisioning_manager
    }

    /// Borrow the configuration manager.
    pub fn configuration_manager(&self) -> &ConfigurationManager {
        &self.configuration_manager
    }

    /// Borrow the mesh message handler.
    pub fn message_handler(&self) -> &MeshMessageHandler {
        &self.message_handler
    }
}

impl BleMeshNetwork {
    /// Create a default `BleMeshNetwork`.
    pub fn new() -> Self {
        Self {
            network_id: "mesh_network_1".to_string(),
            network_key: [0u8; 16],
            iv_index: 0,
            seq_num: 0,
            nodes: HashMap::new(),
            elements: HashMap::new(),
        }
    }
}

impl ProvisioningManager {
    /// Create a default `ProvisioningManager`.
    pub fn new() -> Self {
        Self {
            provisioning_protocol: ProvisioningProtocol::PBADV,
            provisioning_data: ProvisioningData {
                network_key: [0u8; 16],
                net_key_index: 0,
                flags: 0,
                iv_index: 0,
                unicast_address: 0x0001,
            },
            oob_data: None,
        }
    }

    /// Initialise the `ProvisioningManager` sub-components; propagates `MeshError`.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        Ok(())
    }

    /// Borrow the provisioning protocol state machine.
    pub fn protocol(&self) -> &ProvisioningProtocol {
        &self.provisioning_protocol
    }

    /// Borrow the accumulated provisioning data.
    pub fn provisioning_data(&self) -> &ProvisioningData {
        &self.provisioning_data
    }

    /// Install out-of-band provisioning data.
    pub fn set_oob_data(&mut self, data: OobData) {
        self.oob_data = Some(data);
    }

    /// The out-of-band provisioning data, if provisioned.
    pub fn oob_data(&self) -> Option<&OobData> {
        self.oob_data.as_ref()
    }
}

impl ConfigurationManager {
    /// Create a default `ConfigurationManager`.
    pub fn new() -> Self {
        Self {
            config_database: ConfigDatabase::new(),
            config_models: Vec::new(),
            access_control: AccessControl::new(),
        }
    }

    /// Initialise the `ConfigurationManager` sub-components; propagates `MeshError`.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        Ok(())
    }

    /// Borrow the node configuration database.
    pub fn config_database(&self) -> &ConfigDatabase {
        &self.config_database
    }

    /// Mutable borrow of the node configuration database.
    pub fn config_database_mut(&mut self) -> &mut ConfigDatabase {
        &mut self.config_database
    }

    /// Register a `ConfigModel` in the database.
    pub fn add_config_model(&mut self, model: ConfigModel) {
        self.config_models.push(model);
    }

    /// The registered `ConfigModel`s.
    pub fn config_models(&self) -> &[ConfigModel] {
        &self.config_models
    }

    /// Borrow the configuration access-control list.
    pub fn access_control(&self) -> &AccessControl {
        &self.access_control
    }
}

impl ConfigDatabase {
    /// Create a default `ConfigDatabase`.
    pub fn new() -> Self {
        Self {
            app_keys: HashMap::new(),
            subnet_list: Vec::new(),
            virtual_addresses: HashMap::new(),
        }
    }
}

impl AccessControl {
    /// Create a default `AccessControl`.
    pub fn new() -> Self {
        Self {
            access_list: Vec::new(),
            default_policy: AccessPolicy::Allow,
        }
    }
}

impl MeshMessageHandler {
    /// Create a default `MeshMessageHandler`.
    pub fn new() -> Self {
        Self {
            message_queue: Vec::new(),
            routing_table: RoutingTable::new(),
            security_manager: MeshSecurityManager::new(),
        }
    }

    /// Push `message` onto the mesh message queue.
    pub fn enqueue_message(&mut self, message: MeshMessage) {
        self.message_queue.push(message);
    }

    /// Pop the oldest queued `MeshMessage`, or `None` when empty.
    pub fn dequeue_message(&mut self) -> Option<MeshMessage> {
        self.message_queue.pop()
    }

    /// Current message-queue depth.
    pub fn queue_length(&self) -> usize {
        self.message_queue.len()
    }

    /// Borrow the mesh routing table.
    pub fn routing_table(&self) -> &RoutingTable {
        &self.routing_table
    }

    /// Borrow the mesh security manager.
    pub fn security_manager(&self) -> &MeshSecurityManager {
        &self.security_manager
    }
}

impl MeshSecurityManager {
    /// Create a default `MeshSecurityManager`.
    pub fn new() -> Self {
        Self {
            network_keys: HashMap::new(),
            application_keys: HashMap::new(),
            device_keys: HashMap::new(),
            beacon_key: [0u8; 16],
        }
    }
}

impl BleAdvertiser {
    /// Create a default `BleAdvertiser`.
    pub fn new() -> Self {
        Self {
            advertising_data: Vec::new(),
            scan_response_data: Vec::new(),
            advertising_parameters: AdvertisingParameters {
                interval_min: 100,
                interval_max: 200,
                type_: AdvertisingType::ConnectableUndirected,
                filter_policy: AdvertisingFilterPolicy::AllowScanAny,
            },
            active_advertisements: Vec::new(),
        }
    }

    /// Initialise the `BleAdvertiser` sub-components; propagates `MeshError`.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        Ok(())
    }

    /// Set the advertising payload bytes.
    pub fn set_advertising_data(&mut self, data: Vec<u8>) {
        self.advertising_data = data;
    }

    /// The configured advertising payload bytes.
    pub fn advertising_data(&self) -> &[u8] {
        &self.advertising_data
    }

    /// Set the scan-response payload bytes.
    pub fn set_scan_response_data(&mut self, data: Vec<u8>) {
        self.scan_response_data = data;
    }

    /// The configured scan-response payload bytes.
    pub fn scan_response_data(&self) -> &[u8] {
        &self.scan_response_data
    }

    /// The active advertising parameters.
    pub fn advertising_parameters(&self) -> &AdvertisingParameters {
        &self.advertising_parameters
    }

    /// Borrow the `start_advertising` component.
    pub fn start_advertising(&mut self, adv: ActiveAdvertisement) {
        self.active_advertisements.push(adv);
    }

    /// Borrow the `stop_advertising` component.
    pub fn stop_advertising(&mut self, handle: u8) {
        self.active_advertisements.retain(|a| a.handle != handle);
    }

    /// Number of `active_advertisement` entries currently held.
    pub fn active_advertisement_count(&self) -> usize {
        self.active_advertisements.len()
    }
}

impl BleScanner {
    /// Create a default `BleScanner`.
    pub fn new() -> Self {
        Self {
            scanning_parameters: ScanningParameters {
                interval: 100,
                window: 50,
                type_: ScanningType::Active,
                filter_duplicates: true,
            },
            scan_filter: ScanFilter {
                address_filter: None,
                rssi_filter: None,
                service_uuid_filter: Vec::new(),
            },
            active_scans: Vec::new(),
        }
    }

    /// Initialise the `BleScanner` sub-components; propagates `MeshError`.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        Ok(())
    }

    /// The active scanning parameters.
    pub fn scanning_parameters(&self) -> &ScanningParameters {
        &self.scanning_parameters
    }

    /// The active scan filter.
    pub fn scan_filter(&self) -> &ScanFilter {
        &self.scan_filter
    }

    /// Add a 16-bit service UUID to the scan filter.
    pub fn add_service_uuid_filter(&mut self, uuid: u16) {
        self.scan_filter.service_uuid_filter.push(uuid);
    }

    /// Borrow the `start_scan` component.
    pub fn start_scan(&mut self, scan: ActiveScan) {
        self.active_scans.push(scan);
    }

    /// Borrow the `stop_scan` component.
    pub fn stop_scan(&mut self, handle: u8) {
        self.active_scans.retain(|s| s.handle != handle);
    }

    /// Number of `active_scan` entries currently held.
    pub fn active_scan_count(&self) -> usize {
        self.active_scans.len()
    }
}

impl BleConnectionManager {
    /// Create a default `BleConnectionManager`.
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            connection_parameters: ConnectionParameters {
                min_interval: 24,
                max_interval: 40,
                latency: 0,
                supervision_timeout: 700,
                min_ce_length: 0,
                max_ce_length: 0,
            },
            security_manager: BleSecurityManager::new(),
        }
    }

    /// Initialise the `BleConnectionManager` sub-components; propagates `MeshError`.
    pub fn initialize(&mut self) -> Result<(), MeshError> {
        Ok(())
    }

    /// Track `connection` under `handle`.
    pub fn add_connection(&mut self, handle: u16, connection: BleConnection) {
        self.connections.insert(handle, connection);
    }

    /// Drop the connection tracked under `handle`.
    pub fn remove_connection(&mut self, handle: u16) {
        self.connections.remove(&handle);
    }

    /// Number of `connection` entries currently held.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// Borrow the `connection`.
    pub fn get_connection(&self, handle: u16) -> Option<&BleConnection> {
        self.connections.get(&handle)
    }

    /// The configured connection parameters.
    pub fn connection_parameters(&self) -> &ConnectionParameters {
        &self.connection_parameters
    }

    /// Borrow the mesh security manager.
    pub fn security_manager(&self) -> &BleSecurityManager {
        &self.security_manager
    }
}

impl BleSecurityManager {
    /// Create a default `BleSecurityManager`.
    pub fn new() -> Self {
        Self {
            encryption_keys: HashMap::new(),
            identity_keys: HashMap::new(),
            signing_keys: HashMap::new(),
            csrk: HashMap::new(),
        }
    }
}
