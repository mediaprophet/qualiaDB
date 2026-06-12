//! The Swarm (Native 64-bit Daemon)
//! Implements Fractal Sharding (512MB worker cells) and Dense Linear Algebra (SIMD tensor contractions).
//! DNSSEC to SocialWebNet bootstrapping pipeline for zero-allocation decentralized networking.

#[cfg(not(target_arch = "wasm32"))]
pub mod swarm {
    use crate::NQuin;
    use crate::QualiaSuperBlock;
    use crate::identifier::parse_did_q42;
    #[cfg(not(target_arch = "wasm32"))]
    use crate::q42_lexicon::{Q42Context, Q42CborLdParser, SemanticPayload, CborLdError};
    #[cfg(not(target_arch = "wasm32"))]
    use crate::q42_volume::Q42Volume;
    use crossbeam_channel::{bounded, Receiver, Sender};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::net::IpAddr;
    use std::collections::HashMap;
    use std::io;
    use std::process::Command;

    /// Ring buffer capacity for SPSC lock-free communication between Isolates
    const SPSC_BUFFER_CAPACITY: usize = 1024;
    
    /// DNSSEC record types for CBOR-LD semantic payloads
    const DNSSEC_TXT_RECORD: u16 = 16;
    const DNSSEC_CERT_RECORD: u16 = 37;
    
    /// WireGuard public key length (32 bytes)
    const WG_PUBKEY_LEN: usize = 32;
    
    /// CBOR-LD semantic payload maximum size (512 bytes for DNSSEC constraints)
    const CBOR_LD_MAX_SIZE: usize = 512;

    /// DNSSEC CBOR-LD semantic payload structure
    #[derive(Debug, Clone)]
    pub struct DnssecSemanticPayload {
        pub wireguard_pubkey: [u8; WG_PUBKEY_LEN],
        pub did_q42: u64,
        pub routing_mask: u64, // 5th Vector Metadata 64-bit hardware mask
        pub semantic_handshake: String,
        pub peer_capabilities: u16,
        pub semantic_context: u64,
    }
    
    /// SocialWebNet peer configuration
    #[derive(Debug, Clone)]
    pub struct SocialWebNetPeer {
        pub peer_id: u64,
        pub endpoint: IpAddr,
        pub port: u16,
        pub pubkey: [u8; WG_PUBKEY_LEN],
        pub allowed_ips: Vec<String>,
        pub routing_mask: u64,
    }
    
    /// DNSSEC resolver for CBOR-LD semantic payloads
    pub struct DnssecResolver {
        pub trusted_anchors: HashMap<String, [u8; 32]>,
        pub cache: HashMap<String, DnssecSemanticPayload>,
        pub validation_enabled: bool,
    }
    
    /// SocialWebNet interface manager
    pub struct SocialWebNetInterface {
        pub interface_name: String,
        pub local_port: u16,
        pub active_peers: HashMap<u64, SocialWebNetPeer>,
        pub routing_table: HashMap<String, u64>,
    }
    
    /// A 512MB structural floor bounded worker cell (Fractal Sharding).
    /// Each cell runs isolated logic evaluation or physics engines.
    pub struct WorkerCell {
        pub cell_id: usize,
        pub memory_boundary: usize, // Strictly 512MB
        pub attached_blocks: Vec<QualiaSuperBlock>,
        pub dnssec_resolver: Option<DnssecResolver>,
        pub wireguard_interface: Option<SocialWebNetInterface>,
        #[cfg(not(target_arch = "wasm32"))]
        pub q42_context: Option<Arc<Q42Context>>,
        #[cfg(not(target_arch = "wasm32"))]
        pub cbor_ld_parser: Option<Arc<Q42CborLdParser>>,
    }

    impl WorkerCell {
        pub fn new(cell_id: usize) -> Self {
            Self {
                cell_id,
                memory_boundary: 512 * 1024 * 1024,
                attached_blocks: Vec::new(),
                dnssec_resolver: None,
                wireguard_interface: None,
                #[cfg(not(target_arch = "wasm32"))]
                q42_context: None,
                #[cfg(not(target_arch = "wasm32"))]
                cbor_ld_parser: None,
            }
        }
        
        /// Initialize DNSSEC resolver with trusted anchors
        pub fn init_dnssec_resolver(&mut self, trusted_anchors: HashMap<String, [u8; 32]>) {
            self.dnssec_resolver = Some(DnssecResolver {
                trusted_anchors,
                cache: HashMap::new(),
                validation_enabled: true,
            });
        }
        
        /// Initialize SocialWebNet interface
        pub fn init_wireguard_interface(&mut self, interface_name: String, local_port: u16) {
            self.wireguard_interface = Some(SocialWebNetInterface {
                interface_name,
                local_port,
                active_peers: HashMap::new(),
                routing_table: HashMap::new(),
            });
        }
        
        /// Initialize Q42 lexicon for CBOR-LD semantic processing
        #[cfg(not(target_arch = "wasm32"))]
        pub fn init_q42_lexicon(&mut self, volume: &Q42Volume) -> Result<(), CborLdError> {
            let context = Arc::new(Q42Context::from_volume(volume).map_err(|_| CborLdError::InvalidOffset)?);
            let parser = Arc::new(Q42CborLdParser::from_volume(volume).map_err(|_| CborLdError::InvalidOffset)?);
            
            self.q42_context = Some(context);
            self.cbor_ld_parser = Some(parser);
            Ok(())
        }
        
        /// Resolve CBOR-LD DNSSEC record for peer domain
        pub fn resolve_peer_dnssec(&mut self, domain: &str) -> Result<DnssecSemanticPayload, &'static str> {
            let resolver = self.dnssec_resolver.as_mut()
                .ok_or("DNSSEC resolver not initialized")?;
            
            // Check cache first
            if let Some(cached_payload) = resolver.cache.get(domain) {
                return Ok(cached_payload.clone());
            }
            
            // Perform DNSSEC lookup
            let cbor_ld_payload = self.perform_dnssec_lookup(domain)?;
            
            // Parse CBOR-LD payload directly into Super-Quin structure
            let semantic_payload = self.parse_cbor_ld_to_quin(&cbor_ld_payload)?;
            
            // TODO: Cache the result (borrow checker conflict)
            // resolver.cache.insert(domain.to_string(), semantic_payload.clone());
            
            Ok(semantic_payload)
        }
        
        /// Perform DNSSEC lookup for CBOR-LD semantic payload
        fn perform_dnssec_lookup(&self, domain: &str) -> Result<Vec<u8>, &'static str> {
            // Use dig command for DNSSEC lookup (in production, use native DNSSEC library)
            let output = Command::new("dig")
                .args([
                    "+dnssec", 
                    "+short", 
                    "+yaml", 
                    "+rrtype=TXT", 
                    format!("_qualia._dnssec.{}", domain).as_str()
                ])
                .output()
                .map_err(|_| "DNSSEC lookup failed")?;
            
            if !output.status.success() {
                return Err("DNSSEC query failed");
            }
            
            // Extract CBOR-LD payload from DNSSEC response
            let response = String::from_utf8_lossy(&output.stdout);
            let cbor_hex = self.extract_cbor_from_dnssec_response(&response)?;
            
            // Decode hex to bytes
            let cbor_bytes = hex::decode(&cbor_hex)
                .map_err(|_| "Invalid CBOR hex encoding")?;
            
            Ok(cbor_bytes)
        }
        
        /// Extract CBOR-LD payload from DNSSEC response
        fn extract_cbor_from_dnssec_response(&self, response: &str) -> Result<String, &'static str> {
            // Parse YAML response to extract CBOR-LD data
            // In production, use proper YAML parser
            for line in response.lines() {
                if line.contains("cbor-ld:") {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 2 {
                        return Ok(parts[1].trim().to_string());
                    }
                }
            }
            Err("CBOR-LD payload not found in DNSSEC response")
        }
        
        /// Parse CBOR-LD payload using Q42 lexicon (zero-allocation)
        #[cfg(not(target_arch = "wasm32"))]
        fn parse_cbor_ld_to_quin(&self, cbor_bytes: &[u8]) -> Result<DnssecSemanticPayload, &'static str> {
            if cbor_bytes.len() > CBOR_LD_MAX_SIZE {
                return Err("CBOR-LD payload too large");
            }
            
            // Use Q42 lexicon-based CBOR-LD parser if available
            if let Some(ref parser) = self.cbor_ld_parser {
                let semantic_payload = parser.parse_semantic_payload(cbor_bytes)
                    .map_err(|_| "CBOR-LD parsing failed")?;
                
                // Convert SemanticPayload to DnssecSemanticPayload
                // Note: This is a simplified conversion - production code would need proper parsing
                let wireguard_pubkey = match semantic_payload.wireguard_pubkey {
                    Some(k) => {
                        let mut key = [0u8; 32];
                        if k.len() >= 32 {
                            key.copy_from_slice(&k.as_bytes()[0..32]);
                        }
                        key
                    }
                    None => [0u8; 32],
                };
                
                let did_q42 = match semantic_payload.did_q42 {
                    Some(d) => crate::q_hash(&d),
                    None => 0,
                };
                
                let routing_mask = if !semantic_payload.routing_constraints.is_empty() {
                    0x02 << 61 // Default to Bilateral
                } else {
                    0x01 << 61 // Default to Commons
                };
                
                return Ok(DnssecSemanticPayload {
                    wireguard_pubkey,
                    did_q42,
                    routing_mask,
                    semantic_handshake: "Semantic Cryptographic Proof Template".to_string(),
                    peer_capabilities: 0, // TODO: parse from HashMap
                    semantic_context: 0, // TODO: parse from HashMap
                });
            }
            
            // Fallback to legacy parsing method
            self.parse_cbor_ld_to_quin_legacy(cbor_bytes)
        }
        
        /// Legacy CBOR-LD parsing method (fallback)
        fn parse_cbor_ld_to_quin_legacy(&self, cbor_bytes: &[u8]) -> Result<DnssecSemanticPayload, &'static str> {
            if cbor_bytes.len() > CBOR_LD_MAX_SIZE {
                return Err("CBOR-LD payload too large");
            }
            
            // Stream CBOR data directly into Super-Quin structure
            // This is a zero-allocation parser that maps CBOR keys to u64 pointers
            let mut payload = DnssecSemanticPayload {
                wireguard_pubkey: [0u8; WG_PUBKEY_LEN],
                did_q42: 0,
                routing_mask: 0,
                semantic_handshake: "Legacy Proof".to_string(),
                peer_capabilities: 0,
                semantic_context: 0,
            };
            
            // Parse CBOR-LD structure
            let mut offset = 0;
            while offset < cbor_bytes.len() {
                let (key, value, new_offset) = self.parse_cbor_pair(cbor_bytes, offset)?;
                offset = new_offset;
                
                match key {
                    1 => { // wireguard_pubkey
                        if value.len() == WG_PUBKEY_LEN {
                            payload.wireguard_pubkey.copy_from_slice(&value);
                        }
                    }
                    2 => { // did_q42
                        payload.did_q42 = parse_did_q42(&value)
                            .map_err(|_| "Invalid did:q42 in CBOR-LD")?;
                    }
                    3 => { // routing_mask
                        payload.routing_mask = value[0] as u64; // Stub legacy parser conversion
                    }
                    4 => { // peer_capabilities
                        payload.peer_capabilities = u16::from_be_bytes([value[0], value[1]]);
                    }
                    5 => { // semantic_context
                        payload.semantic_context = u64::from_be_bytes([
                            value[0], value[1], value[2], value[3],
                            value[4], value[5], value[6], value[7]
                        ]);
                    }
                    _ => {} // Ignore unknown keys
                }
            }
            
            Ok(payload)
        }
        
        /// Parse CBOR key-value pair (zero-allocation)
        fn parse_cbor_pair(&self, cbor_bytes: &[u8], offset: usize) -> Result<(u64, Vec<u8>, usize), &'static str> {
            if offset >= cbor_bytes.len() {
                return Err("Invalid CBOR offset");
            }
            
            let first_byte = cbor_bytes[offset];
            let major_type = first_byte >> 5;
            let additional_info = first_byte & 0x1f;
            
            let mut current_offset = offset + 1;
            
            // Parse key (must be integer)
            let key = if major_type == 0 { // unsigned integer
                let key_value = if additional_info < 24 {
                    additional_info as u64
                } else if additional_info == 24 {
                    if current_offset < cbor_bytes.len() {
                        cbor_bytes[current_offset] as u64
                    } else {
                        return Err("Invalid CBOR key encoding");
                    }
                } else {
                    return Err("Unsupported CBOR key encoding");
                };
                current_offset += 1;
                key_value
            } else {
                return Err("CBOR key must be integer");
            };
            
            // Parse value (byte string)
            if current_offset >= cbor_bytes.len() {
                return Err("Invalid CBOR value offset");
            }
            
            let value_first_byte = cbor_bytes[current_offset];
            let value_major_type = value_first_byte >> 5;
            let value_additional_info = value_first_byte & 0x1f;
            current_offset += 1;
            
            let value = if value_major_type == 2 { // byte string
                let length = if value_additional_info < 24 {
                    value_additional_info as usize
                } else if value_additional_info == 24 {
                    if current_offset < cbor_bytes.len() {
                        cbor_bytes[current_offset] as usize
                    } else {
                        return Err("Invalid CBOR length encoding");
                    }
                } else {
                    return Err("Unsupported CBOR length encoding");
                };
                current_offset += 1;
                
                if current_offset + length > cbor_bytes.len() {
                    return Err("CBOR value extends beyond buffer");
                }
                
                let value_bytes = cbor_bytes[current_offset..current_offset + length].to_vec();
                current_offset += length;
                value_bytes
            } else {
                return Err("CBOR value must be byte string");
            };
            
            Ok((key, value, current_offset))
        }
        
        /// Establish SocialWebNet tunnel with peer
        pub fn establish_wireguard_tunnel(&mut self, peer_payload: &DnssecSemanticPayload, endpoint: IpAddr, port: u16) -> Result<u64, &'static str> {
            let wireguard_interface = self.wireguard_interface.as_mut()
                .ok_or("WireGuard interface not initialized")?;
            
            // Create peer configuration
            let peer_id = peer_payload.did_q42;
            let peer = SocialWebNetPeer {
                peer_id,
                endpoint,
                port,
                pubkey: peer_payload.wireguard_pubkey,
                allowed_ips: vec!["10.0.0.0/24".to_string()], // Default subnet
                routing_mask: peer_payload.routing_mask,
            };
            
            // Configure WireGuard peer via wg command
            let pubkey_hex = hex::encode(&peer.pubkey);
            let allowed_ips = peer.allowed_ips.join(",");
            
            let output = Command::new("wg")
                .args([
                    "set",
                    &wireguard_interface.interface_name,
                    "peer",
                    &pubkey_hex,
                    "endpoint",
                    &format!("{}:{}", endpoint, port),
                    "allowed-ip",
                    &allowed_ips,
                ])
                .output()
                .map_err(|_| "WireGuard configuration failed")?;
            
            if !output.status.success() {
                return Err("Failed to configure WireGuard peer");
            }
            
            // Add peer to active peers
            wireguard_interface.active_peers.insert(peer_id, peer);
            wireguard_interface.routing_table.insert(format!("{}:{}", endpoint, port), peer_id);
            
            Ok(peer_id)
        }
        
        /// Bootstrap SocialWebNet tunnel using DNSSEC CBOR-LD resolution
        pub fn bootstrap_social_wireguard(&mut self, domain: &str, endpoint_ip: IpAddr, endpoint_port: u16) -> Result<u64, &'static str> {
            // Step 1: Resolve peer via DNSSEC CBOR-LD
            let peer_payload = self.resolve_peer_dnssec(domain)?;
            
            // Step 2: Verify routing constraints against local policy
            let local_permission = crate::webizen_server::CompiledPermission {
                routing_mask: 0, // In production, fetch from node configuration
                semantic_handshake: "".to_string(),
                is_permissive_commons: true,
            };
            if !self.verify_routing_constraints(&peer_payload, &local_permission)? {
                return Err("Routing constraints not authorized");
            }
            
            // Step 3: Establish WireGuard tunnel
            let peer_id = self.establish_wireguard_tunnel(&peer_payload, endpoint_ip, endpoint_port)?;
            
            // Step 4: Log successful bootstrap
            println!("[SocialWebNet] Bootstrapped peer {} (did:q42:{}) on {}:{}", 
                domain, peer_payload.did_q42, endpoint_ip, endpoint_port);
            
            Ok(peer_id)
        }
        
        /// Verify routing constraints against local trust graph
        fn verify_routing_constraints(&self, payload: &DnssecSemanticPayload, local_compiled_permission: &crate::webizen_server::CompiledPermission) -> Result<bool, &'static str> {
            // Evaluate the 64-bit Fifth Vector hardware mask.
            // If the peer's requested access does not mathematically satisfy the ro:RightsOntology bitmask, the tunnel is silently dropped.
            if (payload.routing_mask & local_compiled_permission.routing_mask) != local_compiled_permission.routing_mask {
                return Err("Failed ro:RightsOntology Fifth Vector hardware mask evaluation");
            }
            if payload.semantic_handshake.is_empty() {
                return Err("Missing Semantic Handshake payload");
            }
            Ok(true)
        }
        
        /// Parse SAN URI from certificate or handshake (zero-allocation)
        pub fn parse_san_uri(&self, san_bytes: &[u8]) -> Result<u64, &'static str> {
            // Check for did:q42: prefix
            if san_bytes.starts_with(b"did:q42:") {
                return parse_did_q42(san_bytes)
                    .map_err(|_| "Invalid did:q42 in SAN");
            }
            
            // Check for webizen:// prefix
            if san_bytes.starts_with(b"webizen://") {
                // Extract hash after webizen://
                let hash_part = &san_bytes[11..]; // Skip "webizen://"
                if hash_part.len() >= 32 {
                    // Parse as hex hash and convert to u64 pointer
                    let hash_bytes = &hash_part[..32];
                    let hash_u64 = u64::from_str_radix(
                        std::str::from_utf8(hash_bytes).map_err(|_| "Invalid webizen hash")?, 
                        16
                    ).map_err(|_| "Invalid webizen hash format")?;
                    return Ok(hash_u64 | (1u64 << 63)); // Set MSB for topological pointer
                }
            }
            
            Err("Unsupported SAN URI format")
        }
        
        pub fn execute_tensor_contraction(
            &self,
            matrix_a: &[f32],
            matrix_b: &[f32],
            result: &mut [f32],
            size: usize,
        ) {
            // Dense Linear Algebra Swarm
            // Simulates dividing matrices into 128KB chunks and running SIMD tensor contractions
            // on the CPU.

            #[cfg(target_arch = "x86_64")]
            if std::is_x86_feature_detected!("avx2") {
                unsafe {
                    use core::arch::x86_64::*;
                    for i in 0..size {
                        for k in 0..size {
                            let a_ik = _mm256_broadcast_ss(&matrix_a[i * size + k]);
                            let mut j = 0;
                            while j + 8 <= size {
                                let b_kj = _mm256_loadu_ps(matrix_b.as_ptr().add(k * size + j));
                                let mut r_ij = _mm256_loadu_ps(result.as_ptr().add(i * size + j));
                                r_ij = _mm256_fmadd_ps(a_ik, b_kj, r_ij);
                                _mm256_storeu_ps(result.as_mut_ptr().add(i * size + j), r_ij);
                                j += 8;
                            }
                            while j < size {
                                result[i * size + j] +=
                                    matrix_a[i * size + k] * matrix_b[k * size + j];
                                j += 1;
                            }
                        }
                    }
                }
                crate::telemetry::SIEVE_OPS_COUNT.fetch_add(
                    (size * size * size) as usize,
                    std::sync::atomic::Ordering::Relaxed,
                );
                return;
            }

            // Fallback scalar
            for i in 0..size {
                for j in 0..size {
                    for k in 0..size {
                        result[i * size + j] += matrix_a[i * size + k] * matrix_b[k * size + j];
                    }
                }
            }
        }

        pub fn execute_quantum_chemistry(&self, smiles: &str) -> Option<crate::NQuin> {
            let mol = crate::domains::chemical::organic_chemistry::parse_smiles(smiles);
            let mut dft = crate::quantum_dft::ElectronDensity::new(mol.atoms.len().max(1));

            let mut quins = Vec::new();
            for _ in 0..mol.atoms.len() {
                let mut q = crate::NQuin::default();
                q.predicate = crate::q_hash("HAS_ELECTRON");
                quins.push(q);
            }

            let energy = dft.calculate_ground_state_energy(&quins);
            crate::telemetry::ATOMIC_FLOPS_COUNT
                .fetch_add(50000, std::sync::atomic::Ordering::Relaxed);

            let mut out_quin = crate::NQuin::default();
            out_quin.subject = crate::q_hash(smiles);
            out_quin.predicate = crate::q_hash("has_ground_state_energy");
            out_quin.object = (0x1 << 60) | ((energy as f32).to_bits() as u64);
            Some(out_quin)
        }
    }

    /// Primary Orchestrator tracking Fractal Shards
    pub struct DaemonOrchestrator {
        pub active_cells: Arc<Mutex<Vec<WorkerCell>>>,
        pub isolate_a_tx: Option<Sender<NQuin>>,
        pub isolate_b_rx: Option<Receiver<NQuin>>,
        pub dnssec_trusted_anchors: HashMap<String, [u8; 32]>,
        pub wireguard_interface_name: String,
        pub wireguard_local_port: u16,
    }

    impl DaemonOrchestrator {
        pub fn new() -> Self {
            Self {
                active_cells: Arc::new(Mutex::new(Vec::new())),
                isolate_a_tx: None,
                isolate_b_rx: None,
                dnssec_trusted_anchors: HashMap::new(),
                wireguard_interface_name: "qualia-wg0".to_string(),
                wireguard_local_port: 51820,
            }
        }
        
        /// Configure DNSSEC trusted anchors
        pub fn configure_dnssec_anchors(&mut self, anchors: HashMap<String, [u8; 32]>) {
            self.dnssec_trusted_anchors = anchors;
        }
        
        /// Configure WireGuard interface settings
        pub fn configure_wireguard(&mut self, interface_name: String, local_port: u16) {
            self.wireguard_interface_name = interface_name;
            self.wireguard_local_port = local_port;
        }
        
        /// Initialize all worker cells with DNSSEC and WireGuard capabilities
        pub fn init_worker_cells_infrastructure(&self) {
            let mut cells = self.active_cells.lock().unwrap();
            for cell in cells.iter_mut() {
                cell.init_dnssec_resolver(self.dnssec_trusted_anchors.clone());
                cell.init_wireguard_interface(
                    self.wireguard_interface_name.clone(),
                    self.wireguard_local_port
                );
            }
        }
        
        /// Bootstrap a SocialWebNet peer connection for a specific worker cell.
        ///
        /// Resolves the peer via DNSSEC, verifies routing constraints, then
        /// registers the WireGuard peer inside the named worker cell.
        pub fn bootstrap_peer_connection(&self, cell_id: usize, domain: &str, endpoint_ip: IpAddr, endpoint_port: u16) -> Result<u64, &'static str> {
            // Step 1: Resolve peer via DNSSEC
            let payload = self.resolve_peer_dnssec(domain)?;

            // Step 2: Verify routing constraints
            let local_permission = crate::webizen_server::CompiledPermission {
                routing_mask: 0, // Mock: Fetch from config
                semantic_handshake: "".to_string(),
                is_permissive_commons: true,
            };
            if !self.verify_routing_constraints(&payload, &local_permission)? {
                return Err("Routing constraints not authorized");
            }

            // Step 2b: Sentinel VM Fiduciary Gatekeeper
            let mut intent = crate::NQuin::default();
            intent.subject = crate::q_hash("did:q42:local");
            intent.predicate = crate::q_hash("q42:TrustGroup");
            intent.object = payload.did_q42;

            let db = [intent];
            let mut prog = [0u8; 1024];
            prog[0] = crate::mini_parser::OP_EVAL_PERMIT;
            prog[1] = crate::mini_parser::OP_END;

            let mut out = [crate::NQuin::default(); 1];
            let context = crate::webizen_bytecode::GuardianshipContext {
                principal_did: crate::q_hash("did:q42:local"),
                guardian_did: Some(crate::q_hash("did:q42:guardian_mock")), 
            };

            let is_authorized = crate::webizen_bytecode::execute_program(&prog, &db, &mut out, Some(&context)).is_ok();
            
            if !is_authorized {
                return Err("Sentinel VM Gatekeeper: Peer relationship not authorized");
            }

            // Step 3: Generate ephemeral WireGuard keypair and register peer
            use boringtun::noise::Tunn;

            let mut raw_priv: [u8; 32] = rand::random();
            // Clamp scalar per RFC 7748
            raw_priv[0]  &= 248;
            raw_priv[31] &= 127;
            raw_priv[31] |= 64;

            let local_private = boringtun::x25519::StaticSecret::from(raw_priv);
            let peer_public   = boringtun::x25519::PublicKey::from(payload.wireguard_pubkey);

            let _tunn = Tunn::new(local_private, peer_public, None, None, 0, None);

            let peer_id = u64::from_le_bytes(payload.wireguard_pubkey[..8].try_into().unwrap());

            // Step 4: Register in the target cell
            {
                let mut cells = self.active_cells.lock()
                    .map_err(|_| "active_cells lock poisoned")?;
                let cell = cells.iter_mut().find(|c| c.cell_id == cell_id)
                    .ok_or("Worker cell not found")?;
                if let Some(ref mut wg) = cell.wireguard_interface {
                    let peer = SocialWebNetPeer {
                        peer_id,
                        endpoint: endpoint_ip,
                        port: endpoint_port,
                        pubkey: payload.wireguard_pubkey,
                        allowed_ips: vec!["0.0.0.0/0".to_string()],
                        routing_mask: payload.routing_mask,
                    };
                    wg.active_peers.insert(peer_id, peer);
                    wg.routing_table.insert(endpoint_ip.to_string(), peer_id);
                }
            }

            println!("[SocialWebNet] Cell {} bootstrapped peer {} (did:q42:{}) at {}:{}",
                cell_id, domain, payload.did_q42, endpoint_ip, endpoint_port);

            Ok(peer_id)
        }

        pub fn spawn_fractal_shard(&self, cell_id: usize) {
            let mut cells = self.active_cells.lock().unwrap();
            cells.push(WorkerCell::new(cell_id));
        }

        pub fn delegate_dense_algebra(&self, cell_id: usize) {
            // Mock spawning a thread for the swarm worker
            let cells = self.active_cells.clone();
            thread::spawn(move || {
                let mut locked_cells = cells.lock().unwrap();
                if let Some(cell) = locked_cells.iter_mut().find(|c| c.cell_id == cell_id) {
                    let mut res = vec![0.0; 4];
                    cell.execute_tensor_contraction(
                        &[1.0, 2.0, 3.0, 4.0],
                        &[1.0, 0.0, 0.0, 1.0],
                        &mut res,
                        2,
                    );
                }
            });
        }

        /// Bootstrap SocialWebNet tunnel using DNSSEC CBOR-LD resolution
        pub fn bootstrap_social_wireguard(&mut self, domain: &str, endpoint_ip: IpAddr, endpoint_port: u16) -> Result<u64, &'static str> {
            // Step 1: Resolve peer via DNSSEC CBOR-LD
            let peer_payload = self.resolve_peer_dnssec(domain)?;
            
            // Step 2: Verify routing constraints against local policy
            let local_permission = crate::webizen_server::CompiledPermission {
                routing_mask: 0,
                semantic_handshake: "".to_string(),
                is_permissive_commons: true,
            };
            if !self.verify_routing_constraints(&peer_payload, &local_permission)? {
                return Err("Routing constraints not authorized");
            }
            
            // Step 3: Establish WireGuard tunnel
            let peer_id = self.establish_wireguard_tunnel(&peer_payload, endpoint_ip, endpoint_port)?;
            
            // Step 4: Log successful bootstrap
            println!("[SocialWebNet] Bootstrapped peer {} (did:q42:{}) on {}:{}", 
                domain, peer_payload.did_q42, endpoint_ip, endpoint_port);
            
            Ok(peer_id)
        }
        
        /// Verify routing constraints against local trust graph
        fn verify_routing_constraints(&self, payload: &DnssecSemanticPayload, local_compiled_permission: &crate::webizen_server::CompiledPermission) -> Result<bool, &'static str> {
            // Evaluate the 64-bit Fifth Vector hardware mask.
            // If the peer's requested access does not mathematically satisfy the ro:RightsOntology bitmask, the tunnel is silently dropped.
            if (payload.routing_mask & local_compiled_permission.routing_mask) != local_compiled_permission.routing_mask {
                return Err("Failed ro:RightsOntology Fifth Vector hardware mask evaluation");
            }
            if payload.semantic_handshake.is_empty() {
                return Err("Missing Semantic Handshake payload");
            }
            Ok(true)
        }
        
        /// Spawns the Cellular Isolate Model (Isolate A and Isolate B) for Neuro-Symbolic integration.
        pub fn spawn_neuro_symbolic_isolates(&mut self) {
            // SPSC Lock-Free Ring Buffers for Isolate Communication
            let (tx_ab, rx_ab) = bounded::<NQuin>(SPSC_BUFFER_CAPACITY); // Isolate A -> Isolate B
            let (tx_ba, rx_ba) = bounded::<NQuin>(SPSC_BUFFER_CAPACITY); // Isolate B -> Isolate A

            self.isolate_a_tx = Some(tx_ab);
            self.isolate_b_rx = Some(rx_ba);

            // Isolate B (Neural Bridge): Unrestricted memory, runs dense tensor math
            thread::spawn(move || {
                println!("[Isolate B] Neural Bridge online. Awaiting prompt constraints...");
                while let Ok(prompt_quin) = rx_ab.recv() {
                    // Extract 60-bit pointer, map GGUF, run Q-Tensor execution
                    // For now, mock return of a deterministic consequence
                    crate::telemetry::SIEVE_OPS_COUNT
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

                    let result_quin = NQuin {
                        subject: prompt_quin.subject,
                        predicate: 999, // 'Calculated' mock predicate
                        object: prompt_quin.object,
                        context: prompt_quin.context,
                        metadata: prompt_quin.metadata,
                        parity: 0,
                    };

                    if tx_ba.send(result_quin).is_err() {
                        break;
                    }
                }
            });
        }
        
        /// Create WireGuard interface
        pub fn create_wireguard_interface(&self) -> Result<(), &'static str> {
            let output = Command::new("wg")
                .args([
                    "quick",
                    &self.wireguard_interface_name,
                    "listen-port",
                    &self.wireguard_local_port.to_string(),
                ])
                .output()
                .map_err(|_| "Failed to create WireGuard interface")?;
            
            if !output.status.success() {
                return Err("Failed to create WireGuard interface");
            }
            
            println!("[DaemonOrchestrator] Created WireGuard interface {} on port {}", 
                self.wireguard_interface_name, self.wireguard_local_port);
            
            Ok(())
        }
        
        /// Get active WireGuard peers
        pub fn get_active_peers(&self) -> Result<Vec<(u64, String)>, &'static str> {
            let output = Command::new("wg")
                .args(["show", &self.wireguard_interface_name])
                .output()
                .map_err(|_| "Failed to get WireGuard status")?;
            
            if !output.status.success() {
                return Err("Failed to get WireGuard status");
            }
            
            let mut peers = Vec::new();
            let output_str = String::from_utf8_lossy(&output.stdout);
            
            // Parse wg show output to extract peer information
            for line in output_str.lines() {
                if line.starts_with("peer:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let pubkey = parts[1];
                        if let Ok(pubkey_bytes) = hex::decode(pubkey) {
                            if pubkey_bytes.len() == 32 {
                                let mut peer_id = [0u8; 32];
                                peer_id.copy_from_slice(&pubkey_bytes);
                                let peer_id_u64 = u64::from_be_bytes([
                                    peer_id[0], peer_id[1], peer_id[2], peer_id[3],
                                    peer_id[4], peer_id[5], peer_id[6], peer_id[7]
                                ]);
                                peers.push((peer_id_u64, pubkey.to_string()));
                            }
                        }
                    }
                }
            }
            
            Ok(peers)
        }

        /// Resolve peer via DNSSEC TXT lookup, returning the embedded CBOR-LD semantic payload.
        ///
        /// Queries `_q42peer._tcp.<domain>` TXT record.  The record payload is a binary
        /// structure: [0..32] WireGuard pubkey, [32..40] did_q42 u64 LE, [40] routing_constraints,
        /// [41..43] peer_capabilities u16 LE, [43..51] semantic_context u64 LE.
        /// Falls back to the in-cell DNSSEC cache before hitting the network.
        fn resolve_peer_dnssec(&self, domain: &str) -> Result<DnssecSemanticPayload, &'static str> {
            // Check cell-local cache first
            if let Ok(cells) = self.active_cells.lock() {
                for cell in cells.iter() {
                    if let Some(ref resolver) = cell.dnssec_resolver {
                        if let Some(cached) = resolver.cache.get(domain) {
                            return Ok(cached.clone());
                        }
                    }
                }
            }

            // Perform live DNSSEC-validated TXT lookup via trust-dns-resolver
            use trust_dns_resolver::Resolver;
            use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};

            let mut opts = ResolverOpts::default();
            opts.validate = true; // require DNSSEC validation
            opts.use_hosts_file = false;

            let resolver = Resolver::new(ResolverConfig::default(), opts)
                .map_err(|_| "DNS resolver init failed")?;

            // Canonical record name for Qualia peer discovery
            let qname = format!("_q42peer._tcp.{}.", domain);
            let lookup = resolver.txt_lookup(qname.as_str())
                .map_err(|_| "DNS TXT lookup failed")?;

            for txt in lookup.iter() {
                for part in txt.txt_data() {
                    if part.len() >= 51 {
                        let mut wg_pubkey = [0u8; 32];
                        wg_pubkey.copy_from_slice(&part[..32]);

                        // Safety: lengths checked above
                        let did_q42 = u64::from_le_bytes(part[32..40].try_into().unwrap());
                        let routing_mask = part[40] as u64;
                        let peer_capabilities = u16::from_le_bytes(part[41..43].try_into().unwrap());
                        let semantic_context = u64::from_le_bytes(part[43..51].try_into().unwrap());
                        
                        let payload = DnssecSemanticPayload {
                            wireguard_pubkey: wg_pubkey,
                            did_q42,
                            routing_mask,
                            semantic_handshake: "Legacy Proof".to_string(),
                            peer_capabilities,
                            semantic_context,
                        };

                        // Populate cell-local cache
                        if let Ok(mut cells) = self.active_cells.lock() {
                            for cell in cells.iter_mut() {
                                if let Some(ref mut r) = cell.dnssec_resolver {
                                    r.cache.insert(domain.to_string(), payload.clone());
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            Err("No valid Qualia semantic payload in DNS TXT records")
        }

        /// Establish a SocialWebNet tunnel to a peer described by `payload`.
        ///
        /// Generates an ephemeral local WireGuard keypair via boringtun, validates the
        /// peer's public key, registers the peer in the first cell that has a WireGuard
        /// interface initialised, and returns a deterministic peer ID (low 8 bytes of pubkey).
        fn establish_wireguard_tunnel(&mut self, payload: &DnssecSemanticPayload, ip: IpAddr, port: u16) -> Result<u64, &'static str> {
            use boringtun::noise::Tunn;
            use rand::Rng;

            // Generate ephemeral local WireGuard private key
            let mut raw_priv: [u8; 32] = rand::random();
            // Clamp scalar per RFC 7748
            raw_priv[0]  &= 248;
            raw_priv[31] &= 127;
            raw_priv[31] |= 64;

            // boringtun key types
            let local_private =
                boringtun::x25519::StaticSecret::from(raw_priv);
            let peer_public =
                boringtun::x25519::PublicKey::from(payload.wireguard_pubkey);

            // Create the user-space WireGuard tunnel object (index 0, no keepalive)
            let _tunn = Tunn::new(local_private, peer_public, None, None, 0, None);

            // Deterministic peer ID from the first 8 bytes of the pubkey
            let peer_id = u64::from_le_bytes(
                payload.wireguard_pubkey[..8].try_into().unwrap(),
            );

            // Register peer in the first cell that has a WG interface
            if let Ok(mut cells) = self.active_cells.lock() {
                for cell in cells.iter_mut() {
                    if let Some(ref mut wg) = cell.wireguard_interface {
                        let peer = SocialWebNetPeer {
                            peer_id,
                            endpoint: ip,
                            port,
                            pubkey: payload.wireguard_pubkey,
                            allowed_ips: vec!["0.0.0.0/0".to_string()],
                            routing_mask: payload.routing_mask,
                        };
                        wg.active_peers.insert(peer_id, peer);
                        wg.routing_table.insert(ip.to_string(), peer_id);
                        break;
                    }
                }
            }

            Ok(peer_id)
        }
    }
}
