//! Mesh routing: the inter-network router, routing/forwarding tables, route
//! discovery and caching, and congestion control (queue management + rate control).

use super::*;

/// Mesh router for inter-network routing
pub struct MeshRouter {
    routing_table: RoutingTable,
    forwarding_table: ForwardingTable,
    route_discovery: RouteDiscovery,
    congestion_control: CongestionControl,
}

/// Routing table
#[derive(Debug, Clone)]
pub struct RoutingTable {
    pub entries: Vec<RouteEntry>,
}

/// Route entry
#[derive(Debug, Clone)]
pub struct RouteEntry {
    pub destination: u16,
    pub next_hop: u16,
    pub metric: u8,
    pub sequence_number: u16,
}

/// Forwarding table
#[derive(Debug, Clone)]
pub struct ForwardingTable {
    pub entries: Vec<ForwardingEntry>,
}

/// Forwarding entry
#[derive(Debug, Clone)]
pub struct ForwardingEntry {
    pub destination: String,
    pub next_hop: String,
    pub interface: NetworkInterface,
    pub metric: u16,
    pub ttl: u8,
}

/// Route discovery
pub struct RouteDiscovery {
    pub discovery_protocol: DiscoveryProtocol,
    pub route_cache: RouteCache,
    pub discovery_timeout: Duration,
}

/// Discovery protocols
#[derive(Debug, Clone, PartialEq)]
pub enum DiscoveryProtocol {
    Proactive,
    Reactive,
    Hybrid,
}

/// Route cache
#[derive(Debug, Clone)]
pub struct RouteCache {
    pub entries: Vec<CachedRoute>,
}

/// Cached route
#[derive(Debug, Clone)]
pub struct CachedRoute {
    pub destination: String,
    pub route: Vec<String>,
    pub metric: u16,
    pub timestamp: Instant,
    pub ttl: Duration,
}

/// Congestion control
pub struct CongestionControl {
    pub algorithm: CongestionAlgorithm,
    pub queue_management: QueueManagement,
    pub rate_control: RateControl,
}

/// Congestion algorithms
#[derive(Debug, Clone, PartialEq)]
pub enum CongestionAlgorithm {
    DropTail,
    RED,
    ECN,
    Custom,
}

/// Queue management
#[derive(Debug, Clone)]
pub struct QueueManagement {
    pub queue_size: usize,
    pub drop_policy: DropPolicy,
}

/// Drop policies
#[derive(Debug, Clone, PartialEq)]
pub enum DropPolicy {
    DropTail,
    DropHead,
    Random,
    Priority,
}

/// Rate control
#[derive(Debug, Clone)]
pub struct RateControl {
    pub token_bucket: TokenBucket,
    pub leaky_bucket: LeakyBucket,
}

/// Token bucket
#[derive(Debug, Clone)]
pub struct TokenBucket {
    pub capacity: u32,
    pub rate: u32,
    pub tokens: u32,
    pub last_update: Instant,
}

/// Leaky bucket
#[derive(Debug, Clone)]
pub struct LeakyBucket {
    pub capacity: u32,
    pub rate: u32,
    pub level: u32,
    pub last_update: Instant,
}

impl MeshRouter {
    pub fn new() -> Self {
        Self {
            routing_table: RoutingTable::new(),
            forwarding_table: ForwardingTable::new(),
            route_discovery: RouteDiscovery::new(),
            congestion_control: CongestionControl::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MeshError> {
        self.congestion_control.initialize()?;
        Ok(())
    }

    pub fn get_route_count(&self) -> u32 {
        self.routing_table.entries.len() as u32
    }

    pub fn forwarding_table(&self) -> &ForwardingTable {
        &self.forwarding_table
    }

    pub fn route_discovery(&self) -> &RouteDiscovery {
        &self.route_discovery
    }

    pub fn congestion_control(&self) -> &CongestionControl {
        &self.congestion_control
    }

    pub fn optimize_routes(&mut self) -> Result<(), MeshError> {
        // Prune stale forwarding entries (expired TTL or empty next hop).
        self.forwarding_table
            .entries
            .retain(|e| e.ttl > 0 && !e.next_hop.is_empty());
        // Decrement TTL on remaining entries.
        for e in &mut self.forwarding_table.entries {
            e.ttl = e.ttl.saturating_sub(1);
        }
        Ok(())
    }
}

impl RoutingTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl ForwardingTable {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl RouteDiscovery {
    pub fn new() -> Self {
        Self {
            discovery_protocol: DiscoveryProtocol::Hybrid,
            route_cache: RouteCache::new(),
            discovery_timeout: Duration::from_secs(30),
        }
    }
}

impl RouteCache {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl CongestionControl {
    pub fn new() -> Self {
        Self {
            algorithm: CongestionAlgorithm::RED,
            queue_management: QueueManagement::new(),
            rate_control: RateControl::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), MeshError> {
        Ok(())
    }

    pub fn algorithm(&self) -> &CongestionAlgorithm {
        &self.algorithm
    }

    pub fn queue_management(&self) -> &QueueManagement {
        &self.queue_management
    }

    pub fn rate_control(&self) -> &RateControl {
        &self.rate_control
    }
}

impl QueueManagement {
    pub fn new() -> Self {
        Self {
            queue_size: 1000,
            drop_policy: DropPolicy::DropTail,
        }
    }
}

impl RateControl {
    pub fn new() -> Self {
        Self {
            token_bucket: TokenBucket::new(),
            leaky_bucket: LeakyBucket::new(),
        }
    }
}

impl TokenBucket {
    pub fn new() -> Self {
        Self {
            capacity: 1000,
            rate: 100,
            tokens: 1000,
            last_update: Instant::now(),
        }
    }
}

impl LeakyBucket {
    pub fn new() -> Self {
        Self {
            capacity: 1000,
            rate: 100,
            level: 0,
            last_update: Instant::now(),
        }
    }
}
