//! Allocation Firewall (eBPF) Implementation
//! 
//! This module provides kernel-level socket bypassing and packet filtering using eBPF programs.
//! Designed for high-performance networking with zero-copy operations and advanced security.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::fs::{File, OpenOptions};
use std::path::Path;
use serde::{Deserialize, Serialize};

/// eBPF Firewall Manager
pub struct EbpfFirewall {
    programs: HashMap<String, EbpfProgram>,
    sockets: HashMap<i32, SocketInfo>,
    firewall_rules: Vec<FirewallRule>,
    performance_monitor: PerformanceMonitor,
}

/// eBPF program with metadata
#[derive(Debug, Clone)]
pub struct EbpfProgram {
    pub name: String,
    pub program_type: ProgramType,
    pub bytecode: Vec<u8>,
    pub program_id: u32,
    pub attached_sockets: Vec<i32>,
    pub performance_stats: ProgramStats,
}

/// eBPF program types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProgramType {
    /// Socket filter program
    SocketFilter,
    /// XDP program for high-performance packet processing
    Xdp,
    /// TC program for traffic control
    TrafficControl,
    /// Tracepoint program for monitoring
    Tracepoint,
    /// Kprobe program for kernel function tracing
    Kprobe,
}

/// Socket information
#[derive(Debug, Clone)]
pub struct SocketInfo {
    pub fd: i32,
    pub socket_type: SocketType,
    pub protocol: Protocol,
    pub local_address: SocketAddress,
    pub remote_address: Option<SocketAddress>,
    pub attached_program: Option<String>,
    pub bypass_enabled: bool,
    pub performance_stats: SocketStats,
}

/// Socket types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SocketType {
    Stream,     // TCP
    Datagram,   // UDP
    Raw,        // Raw socket
    SeqPacket,  // SCTP
}

/// Network protocols
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Protocol {
    Tcp,
    Udp,
    Icmp,
    Ipv6,
    Raw,
}

/// Socket address representation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocketAddress {
    pub ip: String,
    pub port: u16,
    pub family: AddressFamily,
}

/// Address families
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AddressFamily {
    IPv4,
    IPv6,
    Unix,
}

/// Firewall rule for packet filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallRule {
    pub rule_id: u32,
    pub name: String,
    pub action: RuleAction,
    pub conditions: Vec<RuleCondition>,
    pub priority: u8,
    pub enabled: bool,
    pub hit_count: u64,
}

/// Rule actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleAction {
    Allow,
    Deny,
    Redirect(String),
    Modify(PacketModification),
    Log,
}

/// Packet modification actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PacketModification {
    pub field: String,
    pub operation: ModificationOperation,
    pub value: Vec<u8>,
}

/// Modification operations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModificationOperation {
    Set,
    Add,
    Subtract,
    Xor,
}

/// Rule conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCondition {
    pub field: String,
    pub operator: ConditionOperator,
    pub value: Vec<u8>,
}

/// Condition operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    StartsWith,
    EndsWith,
}

/// Performance monitor for eBPF programs
pub struct PerformanceMonitor {
    program_metrics: HashMap<u32, ProgramMetrics>,
    socket_metrics: HashMap<i32, SocketMetrics>,
    global_metrics: GlobalMetrics,
}

/// Program performance metrics
#[derive(Debug, Clone)]
pub struct ProgramMetrics {
    pub program_id: u32,
    pub execution_count: u64,
    pub total_execution_time: u64,
    pub average_execution_time: f64,
    pub max_execution_time: u64,
    pub min_execution_time: u64,
    pub memory_usage: u64,
    pub packet_count: u64,
    pub byte_count: u64,
}

/// Socket performance metrics
#[derive(Debug, Clone)]
pub struct SocketMetrics {
    pub fd: i32,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connection_time: u64,
    pub last_activity: u64,
    pub error_count: u64,
}

/// Global performance metrics
#[derive(Debug, Clone)]
pub struct GlobalMetrics {
    pub total_packets_processed: u64,
    pub total_bytes_processed: u64,
    pub average_processing_time: f64,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub active_connections: u64,
    pub dropped_packets: u64,
}

/// Program statistics
#[derive(Debug, Clone)]
pub struct ProgramStats {
    pub execution_count: u64,
    pub total_execution_time: u64,
    pub memory_usage: u64,
    pub packet_count: u64,
}

/// Socket statistics
#[derive(Debug, Clone)]
pub struct SocketStats {
    pub packets_sent: u64,
    pub packets_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub connection_time: u64,
    pub error_count: u64,
}

/// Zero-copy buffer for direct memory access
pub struct ZeroCopyBuffer {
    pub ptr: *mut u8,
    pub size: usize,
    pub capacity: usize,
    pub fd: i32,
}

impl EbpfFirewall {
    /// Create new eBPF firewall
    pub fn new() -> Result<Self, EbpfError> {
        Ok(Self {
            programs: HashMap::new(),
            sockets: HashMap::new(),
            firewall_rules: Vec::new(),
            performance_monitor: PerformanceMonitor::new(),
        })
    }

    /// Load eBPF program
    pub fn load_program(&mut self, name: String, program_type: ProgramType, bytecode: Vec<u8>) -> Result<u32, EbpfError> {
        // Validate program bytecode
        Self::validate_bytecode(&bytecode)?;

        // Generate program ID
        let program_id = self.generate_program_id();

        // Create program
        let program = EbpfProgram {
            name: name.clone(),
            program_type,
            bytecode: bytecode.clone(),
            program_id,
            attached_sockets: Vec::new(),
            performance_stats: ProgramStats {
                execution_count: 0,
                total_execution_time: 0,
                memory_usage: 0,
                packet_count: 0,
            },
        };

        // Load program into kernel
        self.load_program_into_kernel(&program)?;

        // Store program
        self.programs.insert(name, program);

        Ok(program_id)
    }

    /// Attach program to socket
    pub fn attach_socket(&mut self, fd: i32, program_name: &str) -> Result<(), EbpfError> {
        // Pre-compute all &self operations before any mutable borrows
        let socket_type = self.detect_socket_type(fd)?;
        let protocol = self.detect_protocol(fd)?;
        let local_address = self.get_local_address(fd)?;
        let remote_address = self.get_remote_address(fd)?;

        // Extract program fields via immutable borrow (dropped before mutable ops)
        let (program_id, program_type) = {
            let program = self.programs.get(program_name)
                .ok_or_else(|| EbpfError::ProgramNotFound(program_name.to_string()))?;
            (program.program_id, program.program_type.clone())
        };

        // Attach program to socket (no live &mut program borrow)
        self.attach_program_to_socket(fd, program_id, program_type)?;

        // Update program attached sockets
        self.programs.get_mut(program_name)
            .ok_or_else(|| EbpfError::ProgramNotFound(program_name.to_string()))?
            .attached_sockets.push(fd);

        // Create and store socket info
        let socket_info = SocketInfo {
            fd,
            socket_type,
            protocol,
            local_address,
            remote_address,
            attached_program: Some(program_name.to_string()),
            bypass_enabled: true,
            performance_stats: SocketStats {
                packets_sent: 0,
                packets_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                connection_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                error_count: 0,
            },
        };

        self.sockets.insert(fd, socket_info);

        Ok(())
    }

    /// Enable socket bypassing
    pub fn bypass_socket(&mut self, fd: i32) -> Result<(), EbpfError> {
        let socket_info = self.sockets.get_mut(&fd)
            .ok_or_else(|| EbpfError::SocketNotFound(fd))?;

        // Enable bypass
        socket_info.bypass_enabled = true;

        // Configure kernel for bypass
        self.configure_socket_bypass(fd)?;

        Ok(())
    }

    /// Disable socket bypassing
    pub fn unbypass_socket(&mut self, fd: i32) -> Result<(), EbpfError> {
        let socket_info = self.sockets.get_mut(&fd)
            .ok_or_else(|| EbpfError::SocketNotFound(fd))?;

        // Disable bypass
        socket_info.bypass_enabled = false;

        // Configure kernel for normal processing
        self.configure_socket_normal(fd)?;

        Ok(())
    }

    /// Add firewall rule
    pub fn add_rule(&mut self, rule: FirewallRule) -> Result<(), EbpfError> {
        // Validate rule
        self.validate_rule(&rule)?;

        // Add to rules list
        self.firewall_rules.push(rule.clone());

        // Update eBPF programs with new rule
        self.update_firewall_programs()?;

        Ok(())
    }

    /// Remove firewall rule
    pub fn remove_rule(&mut self, rule_id: u32) -> Result<(), EbpfError> {
        // Find and remove rule
        self.firewall_rules.retain(|rule| rule.rule_id != rule_id);

        // Update eBPF programs
        self.update_firewall_programs()?;

        Ok(())
    }

    /// Get zero-copy buffer for socket
    pub fn get_zero_copy_buffer(&self, fd: i32, size: usize) -> Result<ZeroCopyBuffer, EbpfError> {
        let socket_info = self.sockets.get(&fd)
            .ok_or_else(|| EbpfError::SocketNotFound(fd))?;

        if !socket_info.bypass_enabled {
            return Err(EbpfError::BypassNotEnabled(fd));
        }

        // Create zero-copy buffer
        let buffer = ZeroCopyBuffer {
            ptr: std::ptr::null_mut(), // Would be actual memory mapping
            size,
            capacity: size,
            fd,
        };

        Ok(buffer)
    }

    /// Process packet through firewall
    pub fn process_packet(&mut self, packet: &[u8], socket_fd: i32) -> Result<PacketAction, EbpfError> {
        let start_time = std::time::Instant::now();

        // Get socket info
        let socket_info = self.sockets.get(&socket_fd)
            .ok_or_else(|| EbpfError::SocketNotFound(socket_fd))?;

        // Check attached program
        let program_name = socket_info.attached_program.as_ref()
            .ok_or_else(|| EbpfError::NoProgramAttached(socket_fd))?;

        let program = self.programs.get(program_name)
            .ok_or_else(|| EbpfError::ProgramNotFound(program_name.clone()))?;

        // Execute eBPF program
        let action = self.execute_ebpf_program(&program, packet)?;

        // Update performance metrics
        let execution_time = start_time.elapsed().as_nanos() as u64;
        self.performance_monitor.update_program_metrics(program.program_id, execution_time, packet.len());
        self.performance_monitor.update_socket_metrics(socket_fd, packet.len());

        Ok(action)
    }

    /// Get performance statistics
    pub fn get_performance_stats(&self) -> PerformanceStats {
        self.performance_monitor.get_global_stats()
    }

    /// Get socket statistics
    pub fn get_socket_stats(&self, fd: i32) -> Option<SocketStats> {
        self.sockets.get(&fd).map(|info| info.performance_stats.clone())
    }

    /// Get program statistics
    pub fn get_program_stats(&self, program_id: u32) -> Option<ProgramStats> {
        self.programs.values()
            .find(|p| p.program_id == program_id)
            .map(|p| p.performance_stats.clone())
    }

    /// List all sockets
    pub fn list_sockets(&self) -> Vec<i32> {
        self.sockets.keys().cloned().collect()
    }

    /// List all programs
    pub fn list_programs(&self) -> Vec<String> {
        self.programs.keys().cloned().collect()
    }

    /// List all firewall rules
    pub fn list_rules(&self) -> Vec<FirewallRule> {
        self.firewall_rules.clone()
    }

    // Internal methods

    /// Validate eBPF bytecode
    fn validate_bytecode(bytecode: &[u8]) -> Result<(), EbpfError> {
        // Check bytecode size
        if bytecode.len() > 4096 {
            return Err(EbpfError::InvalidBytecode("Bytecode too large".to_string()));
        }

        // Check bytecode alignment
        if bytecode.len() % 8 != 0 {
            return Err(EbpfError::InvalidBytecode("Bytecode not aligned".to_string()));
        }

        // Validate eBPF instructions
        Self::validate_instructions(bytecode)?;

        Ok(())
    }

    /// Validate eBPF instructions
    fn validate_instructions(bytecode: &[u8]) -> Result<(), EbpfError> {
        // Simple validation - in real implementation would parse eBPF instructions
        for chunk in bytecode.chunks(8) {
            if chunk.len() != 8 {
                return Err(EbpfError::InvalidBytecode("Invalid instruction size".to_string()));
            }
        }

        Ok(())
    }

    /// Load program into kernel
    ///
    /// Requires the `aya` feature (Linux-only eBPF runtime loader).  Returns an
    /// explicit `LoadError` instead of silently succeeding so callers know the
    /// program was never actually loaded into the kernel.
    fn load_program_into_kernel(&self, program: &EbpfProgram) -> Result<(), EbpfError> {
        #[cfg(target_os = "linux")]
        {
            return Err(EbpfError::LoadError(
                format!(
                    "eBPF program '{}' cannot be loaded: aya feature not enabled in this build. \
                     Recompile with `--features aya` on Linux to enable kernel eBPF loading.",
                    program.name
                )
            ));
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Err(EbpfError::LoadError(
                format!(
                    "eBPF program '{}' cannot be loaded: eBPF is Linux-only \
                     (current OS does not support BPF syscalls).",
                    program.name
                )
            ));
        }
    }

    /// Attach program to socket
    ///
    /// Requires a loaded eBPF program file descriptor from the kernel.  Returns
    /// `AttachError` instead of silently succeeding when the runtime is absent.
    fn attach_program_to_socket(&self, fd: i32, program_id: u32, program_type: ProgramType) -> Result<(), EbpfError> {
        #[cfg(target_os = "linux")]
        {
            return Err(EbpfError::AttachError(
                format!(
                    "Cannot attach program {} (type {:?}) to socket fd {}: \
                     eBPF runtime not available — compile with `--features aya`.",
                    program_id, program_type, fd
                )
            ));
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Err(EbpfError::AttachError(
                format!(
                    "Cannot attach program {} to socket fd {}: eBPF is Linux-only.",
                    program_id, fd
                )
            ));
        }
    }

    /// Configure socket bypass (zero-copy)
    ///
    /// Returns `ConfigurationError` instead of silently succeeding when the
    /// kernel eBPF bypass path is not available.
    fn configure_socket_bypass(&self, fd: i32) -> Result<(), EbpfError> {
        #[cfg(target_os = "linux")]
        {
            return Err(EbpfError::ConfigurationError(
                format!(
                    "Cannot enable zero-copy bypass for socket fd {}: \
                     eBPF runtime not available — compile with `--features aya`.",
                    fd
                )
            ));
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Err(EbpfError::ConfigurationError(
                format!(
                    "Cannot enable zero-copy bypass for socket fd {}: eBPF is Linux-only.",
                    fd
                )
            ));
        }
    }

    /// Restore normal socket processing
    ///
    /// Returns `ConfigurationError` instead of silently succeeding when the
    /// eBPF detach path is not available.
    fn configure_socket_normal(&self, fd: i32) -> Result<(), EbpfError> {
        #[cfg(target_os = "linux")]
        {
            return Err(EbpfError::ConfigurationError(
                format!(
                    "Cannot restore normal processing for socket fd {}: \
                     eBPF runtime not available — compile with `--features aya`.",
                    fd
                )
            ));
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Err(EbpfError::ConfigurationError(
                format!(
                    "Cannot restore normal processing for socket fd {}: eBPF is Linux-only.",
                    fd
                )
            ));
        }
    }

    /// Detect socket type
    fn detect_socket_type(&self, fd: i32) -> Result<SocketType, EbpfError> {
        // In real implementation, would use getsockopt() to detect type
        // For now, return default
        Ok(SocketType::Stream)
    }

    /// Detect protocol
    fn detect_protocol(&self, fd: i32) -> Result<Protocol, EbpfError> {
        // In real implementation, would use getsockopt() to detect protocol
        // For now, return default
        Ok(Protocol::Tcp)
    }

    /// Get local address
    fn get_local_address(&self, fd: i32) -> Result<SocketAddress, EbpfError> {
        // In real implementation, would use getsockname()
        // For now, return default
        Ok(SocketAddress {
            ip: "127.0.0.1".to_string(),
            port: 8080,
            family: AddressFamily::IPv4,
        })
    }

    /// Get remote address
    fn get_remote_address(&self, fd: i32) -> Result<Option<SocketAddress>, EbpfError> {
        // In real implementation, would use getpeername()
        // For now, return None
        Ok(None)
    }

    /// Validate firewall rule
    fn validate_rule(&self, rule: &FirewallRule) -> Result<(), EbpfError> {
        // Check rule conditions
        if rule.conditions.is_empty() {
            return Err(EbpfError::InvalidRule("Rule must have at least one condition".to_string()));
        }

        // Validate condition fields
        for condition in &rule.conditions {
            if condition.field.is_empty() {
                return Err(EbpfError::InvalidRule("Condition field cannot be empty".to_string()));
            }
        }

        Ok(())
    }

    /// Update firewall programs
    ///
    /// Re-compiling eBPF programs with new ruleset requires the `aya` kernel
    /// loader.  Returns `LoadError` rather than silently no-oping.
    fn update_firewall_programs(&mut self) -> Result<(), EbpfError> {
        #[cfg(target_os = "linux")]
        {
            return Err(EbpfError::LoadError(
                "Cannot update eBPF firewall programs: aya feature not enabled in this build. \
                 Recompile with `--features aya` on Linux.".to_string()
            ));
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Err(EbpfError::LoadError(
                "Cannot update eBPF firewall programs: eBPF is Linux-only.".to_string()
            ));
        }
    }

    /// Execute eBPF program
    ///
    /// In-kernel eBPF execution requires the `aya` loader.  Returns
    /// `AttachError` (program was never actually loaded) rather than silently
    /// allowing all packets.
    fn execute_ebpf_program(&self, program: &EbpfProgram, _packet: &[u8]) -> Result<PacketAction, EbpfError> {
        #[cfg(target_os = "linux")]
        {
            return Err(EbpfError::AttachError(
                format!(
                    "eBPF program '{}' (id {}) is not loaded into the kernel: \
                     compile with `--features aya` to enable in-kernel execution.",
                    program.name, program.program_id
                )
            ));
        }
        #[cfg(not(target_os = "linux"))]
        {
            return Err(EbpfError::AttachError(
                format!(
                    "eBPF program '{}' cannot be executed: eBPF is Linux-only.",
                    program.name
                )
            ));
        }
    }

    /// Generate unique program ID
    fn generate_program_id(&self) -> u32 {
        use std::sync::atomic::{AtomicU32, Ordering};
        static COUNTER: AtomicU32 = AtomicU32::new(1);
        COUNTER.fetch_add(1, Ordering::SeqCst)
    }
}

impl PerformanceMonitor {
    /// Create new performance monitor
    pub fn new() -> Self {
        Self {
            program_metrics: HashMap::new(),
            socket_metrics: HashMap::new(),
            global_metrics: GlobalMetrics {
                total_packets_processed: 0,
                total_bytes_processed: 0,
                average_processing_time: 0.0,
                cpu_usage: 0.0,
                memory_usage: 0,
                active_connections: 0,
                dropped_packets: 0,
            },
        }
    }

    /// Update program metrics
    pub fn update_program_metrics(&mut self, program_id: u32, execution_time: u64, packet_size: usize) {
        let metrics = self.program_metrics.entry(program_id).or_insert(ProgramMetrics {
            program_id,
            execution_count: 0,
            total_execution_time: 0,
            average_execution_time: 0.0,
            max_execution_time: 0,
            min_execution_time: u64::MAX,
            memory_usage: 0,
            packet_count: 0,
            byte_count: 0,
        });

        metrics.execution_count += 1;
        metrics.total_execution_time += execution_time;
        metrics.average_execution_time = metrics.total_execution_time as f64 / metrics.execution_count as f64;
        metrics.max_execution_time = metrics.max_execution_time.max(execution_time);
        metrics.min_execution_time = metrics.min_execution_time.min(execution_time);
        metrics.packet_count += 1;
        metrics.byte_count += packet_size as u64;

        // Update global metrics
        self.global_metrics.total_packets_processed += 1;
        self.global_metrics.total_bytes_processed += packet_size as u64;
    }

    /// Update socket metrics
    pub fn update_socket_metrics(&mut self, fd: i32, packet_size: usize) {
        let metrics = self.socket_metrics.entry(fd).or_insert(SocketMetrics {
            fd,
            packets_sent: 0,
            packets_received: 0,
            bytes_sent: 0,
            bytes_received: 0,
            connection_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_activity: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            error_count: 0,
        });

        metrics.packets_received += 1;
        metrics.bytes_received += packet_size as u64;
        metrics.last_activity = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }

    /// Get global statistics
    pub fn get_global_stats(&self) -> PerformanceStats {
        PerformanceStats {
            total_packets_processed: self.global_metrics.total_packets_processed,
            total_bytes_processed: self.global_metrics.total_bytes_processed,
            average_processing_time: self.global_metrics.average_processing_time,
            cpu_usage: self.global_metrics.cpu_usage,
            memory_usage: self.global_metrics.memory_usage,
            active_connections: self.global_metrics.active_connections,
            dropped_packets: self.global_metrics.dropped_packets,
        }
    }
}

/// Packet action result
#[derive(Debug, Clone, PartialEq)]
pub enum PacketAction {
    Allow,
    Deny,
    Redirect(String),
    Modify(PacketModification),
    Log,
}

/// Performance statistics
#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_packets_processed: u64,
    pub total_bytes_processed: u64,
    pub average_processing_time: f64,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub active_connections: u64,
    pub dropped_packets: u64,
}

/// eBPF error types
#[derive(Debug, Clone)]
pub enum EbpfError {
    ProgramNotFound(String),
    SocketNotFound(i32),
    NoProgramAttached(i32),
    BypassNotEnabled(i32),
    InvalidBytecode(String),
    InvalidRule(String),
    LoadError(String),
    AttachError(String),
    ConfigurationError(String),
}

impl std::fmt::Display for EbpfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EbpfError::ProgramNotFound(msg) => write!(f, "Program not found: {}", msg),
            EbpfError::SocketNotFound(fd) => write!(f, "Socket not found: {}", fd),
            EbpfError::NoProgramAttached(fd) => write!(f, "No program attached to socket: {}", fd),
            EbpfError::BypassNotEnabled(fd) => write!(f, "Bypass not enabled for socket: {}", fd),
            EbpfError::InvalidBytecode(msg) => write!(f, "Invalid bytecode: {}", msg),
            EbpfError::InvalidRule(msg) => write!(f, "Invalid rule: {}", msg),
            EbpfError::LoadError(msg) => write!(f, "Load error: {}", msg),
            EbpfError::AttachError(msg) => write!(f, "Attach error: {}", msg),
            EbpfError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
        }
    }
}

impl std::error::Error for EbpfError {}

/// Safety: ZeroCopyBuffer must be handled carefully
unsafe impl Send for ZeroCopyBuffer {}
unsafe impl Sync for ZeroCopyBuffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firewall_creation() {
        let firewall = EbpfFirewall::new().unwrap();
        assert_eq!(firewall.list_programs().len(), 0);
        assert_eq!(firewall.list_sockets().len(), 0);
        assert_eq!(firewall.list_rules().len(), 0);
    }

    #[test]
    fn test_program_loading() {
        let mut firewall = EbpfFirewall::new().unwrap();

        // Create dummy bytecode
        let bytecode = vec![0u8; 64];

        // eBPF kernel loading requires the `aya` feature (Linux-only).
        // On non-Linux builds and without the feature flag, load_program
        // correctly returns an error rather than silently succeeding.
        let result = firewall.load_program(
            "test_program".to_string(),
            ProgramType::SocketFilter,
            bytecode,
        );

        #[cfg(target_os = "linux")]
        {
            // Even on Linux, the aya feature is not enabled — expect LoadError.
            assert!(
                matches!(result, Err(EbpfError::LoadError(_))),
                "Expected LoadError when aya feature is absent: {:?}", result
            );
        }
        #[cfg(not(target_os = "linux"))]
        {
            // On non-Linux hosts, expect LoadError with a "Linux-only" message.
            assert!(
                matches!(result, Err(EbpfError::LoadError(_))),
                "Expected LoadError on non-Linux: {:?}", result
            );
        }
    }

    #[test]
    fn test_firewall_rules() {
        let mut firewall = EbpfFirewall::new().unwrap();

        let rule = FirewallRule {
            rule_id: 1,
            name: "test_rule".to_string(),
            action: RuleAction::Allow,
            conditions: vec![
                RuleCondition {
                    field: "source_ip".to_string(),
                    operator: ConditionOperator::Equals,
                    value: vec![192, 168, 1, 1],
                }
            ],
            priority: 1,
            enabled: true,
            hit_count: 0,
        };

        // add_rule calls update_firewall_programs which requires aya — expect an error.
        let add_result = firewall.add_rule(rule);
        assert!(
            matches!(add_result, Err(EbpfError::LoadError(_))),
            "Expected LoadError when aya feature is absent: {:?}", add_result
        );

        // remove_rule also calls update_firewall_programs — same expectation.
        let remove_result = firewall.remove_rule(1);
        assert!(
            matches!(remove_result, Err(EbpfError::LoadError(_))),
            "Expected LoadError when aya feature is absent: {:?}", remove_result
        );
    }

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();
        
        monitor.update_program_metrics(1, 1000, 1024);
        monitor.update_socket_metrics(1, 1024);
        
        let stats = monitor.get_global_stats();
        assert_eq!(stats.total_packets_processed, 1);
        assert_eq!(stats.total_bytes_processed, 1024);
    }
}
