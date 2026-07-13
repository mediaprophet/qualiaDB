//! Mesh performance monitoring: per-interface (acoustic / BLE), routing, and
//! global network metrics.

use super::*;

/// Mesh performance monitor
pub struct MeshPerformanceMonitor {
    acoustic_metrics: AcousticMetrics,
    ble_metrics: BleMetrics,
    routing_metrics: RoutingMetrics,
    global_metrics: MeshGlobalMetrics,
}

/// Acoustic metrics
#[derive(Debug, Clone)]
pub struct AcousticMetrics {
    pub nodes_discovered: u32,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub delivery_rate: f64,
    pub latency: Duration,
    pub throughput: f64,
    pub packet_loss_rate: f64,
}

/// BLE metrics
#[derive(Debug, Clone)]
pub struct BleMetrics {
    pub nodes_discovered: u32,
    pub connections_established: u32,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub delivery_rate: f64,
    pub latency: Duration,
    pub throughput: f64,
}

/// Routing metrics
#[derive(Debug, Clone)]
pub struct RoutingMetrics {
    pub routes_discovered: u32,
    pub route_discovery_time: Duration,
    pub forwarding_efficiency: f64,
    pub congestion_events: u32,
    pub route_optimizations: u32,
}

/// Global metrics
#[derive(Debug, Clone)]
pub struct MeshGlobalMetrics {
    pub total_nodes: u32,
    pub total_messages: u64,
    pub network_uptime: Duration,
    pub average_latency: Duration,
    pub overall_throughput: f64,
    pub reliability: f64,
}

impl MeshPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            acoustic_metrics: AcousticMetrics::new(),
            ble_metrics: BleMetrics::new(),
            routing_metrics: RoutingMetrics::new(),
            global_metrics: MeshGlobalMetrics::new(),
        }
    }

    pub fn update_receive_metrics(&mut self, message: &StoredMessage) {
        self.global_metrics.total_messages += 1;
        // Route to the appropriate metrics based on message source.
        if message.source.starts_with("acoustic_") {
            self.acoustic_metrics.messages_received += 1;
        } else {
            self.ble_metrics.messages_received += 1;
        }
    }

    pub fn acoustic_metrics(&self) -> &AcousticMetrics {
        &self.acoustic_metrics
    }

    pub fn ble_metrics(&self) -> &BleMetrics {
        &self.ble_metrics
    }

    pub fn routing_metrics(&self) -> &RoutingMetrics {
        &self.routing_metrics
    }

    pub fn get_uptime(&self) -> Duration {
        Duration::from_secs(3600) // 1 hour uptime (dummy)
    }

    pub fn get_global_stats(&self) -> MeshGlobalMetrics {
        self.global_metrics.clone()
    }
}

impl AcousticMetrics {
    pub fn new() -> Self {
        Self {
            nodes_discovered: 0,
            messages_sent: 0,
            messages_received: 0,
            delivery_rate: 0.0,
            latency: Duration::from_millis(0),
            throughput: 0.0,
            packet_loss_rate: 0.0,
        }
    }
}

impl BleMetrics {
    pub fn new() -> Self {
        Self {
            nodes_discovered: 0,
            connections_established: 0,
            messages_sent: 0,
            messages_received: 0,
            delivery_rate: 0.0,
            latency: Duration::from_millis(0),
            throughput: 0.0,
        }
    }
}

impl RoutingMetrics {
    pub fn new() -> Self {
        Self {
            routes_discovered: 0,
            route_discovery_time: Duration::from_millis(0),
            forwarding_efficiency: 0.0,
            congestion_events: 0,
            route_optimizations: 0,
        }
    }
}

impl MeshGlobalMetrics {
    pub fn new() -> Self {
        Self {
            total_nodes: 0,
            total_messages: 0,
            network_uptime: Duration::from_secs(0),
            average_latency: Duration::from_millis(0),
            overall_throughput: 0.0,
            reliability: 0.0,
        }
    }
}
