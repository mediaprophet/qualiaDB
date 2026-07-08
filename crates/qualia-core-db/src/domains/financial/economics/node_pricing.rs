//! Node-local economic pricing helpers.

/// Context regarding the physical state of the node.
pub struct SystemContext {
    pub current_battery_level: f32,
    pub cpu_temperature: f32,
    pub network_congestion_index: f64,
}

/// Get mock system context.
pub fn get_current_system_context() -> SystemContext {
    SystemContext {
        current_battery_level: 0.8,
        cpu_temperature: 45.0,
        network_congestion_index: 0.2,
    }
}

/// Calculates bandwidth liability in USD based on routed bytes and context.
pub fn calculate_bandwidth_liability(bytes: usize, context: &SystemContext) -> f64 {
    let gb_routed = bytes as f64 / 1_073_741_824.0;
    let mut base_rate = 0.05;

    base_rate += context.network_congestion_index * 0.05;
    if context.current_battery_level < 0.2 {
        base_rate += 0.05;
    }
    if context.cpu_temperature > 70.0 {
        base_rate += 0.02;
    }

    gb_routed * base_rate
}
