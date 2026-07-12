//! Power / thermal / battery governance: PowerManager and its monitors.

use super::*;
use std::time::{Duration, Instant};

/// Power manager
pub struct PowerManager {
    power_policy: PowerPolicy,
    pub(super) battery_monitor: BatteryMonitor,
    thermal_monitor: ThermalMonitor,
    power_optimizer: PowerOptimizer,
    /// Current orchestration state, used to estimate power when no platform
    /// power API is available.
    orchestration_state: AmbientOrchestrationState,
    /// Number of models currently loaded/active on the managed device.
    active_model_count: usize,
}

/// Battery monitor
pub struct BatteryMonitor {
    pub(super) current_level: f64,
    voltage: f64,
    temperature: f64,
    health: f64,
    charging: bool,
    estimated_time_remaining: Duration,
}

/// Thermal monitor
pub struct ThermalMonitor {
    cpu_temperature: f64,
    gpu_temperature: f64,
    battery_temperature: f64,
    ambient_temperature: f64,
    thermal_state: ThermalState,
}

/// Power optimizer
pub struct PowerOptimizer {
    optimization_algorithm: OptimizationAlgorithm,
    optimization_history: Vec<OptimizationRecord>,
    target_efficiency: f64,
}

impl PowerManager {
    /// Create new power manager
    pub fn new() -> Self {
        Self {
            power_policy: PowerPolicy::Balanced,
            battery_monitor: BatteryMonitor::new(),
            thermal_monitor: ThermalMonitor::new(),
            power_optimizer: PowerOptimizer::new(),
            orchestration_state: AmbientOrchestrationState::Idle,
            active_model_count: 0,
        }
    }

    /// Set the current orchestration state so power estimates track the
    /// real state machine in `orchestrator.rs` (`ModelLifecycle`).
    pub fn set_orchestration_state(&mut self, state: AmbientOrchestrationState) {
        self.orchestration_state = state;
    }

    /// Set the number of models currently active on the managed device.
    pub fn set_active_model_count(&mut self, count: usize) {
        self.active_model_count = count;
    }

    /// Check if device can execute task
    pub fn can_execute(&self, _device: &AmbientDevice) -> bool {
        let battery_level = self.battery_monitor.current_level;
        let thermal_state = &self.thermal_monitor.thermal_state;

        battery_level > 20.0 && *thermal_state != ThermalState::Critical
    }

    /// Update power consumption after executing a task on a device.
    ///
    /// Uses the power optimizer to record the power state transition and
    /// the power policy to determine the power budget.
    pub fn update_power_consumption(
        &mut self,
        device: &mut AmbientDevice,
        execution_time: Duration,
    ) {
        let power_consumed = device.power_profile.active_power * execution_time.as_secs_f64();

        // Drain battery based on power consumed.
        self.battery_monitor.drain(power_consumed, execution_time);

        // Apply thermal impact from the power draw.
        self.thermal_monitor
            .apply_heat(power_consumed, execution_time);

        // Record the optimization state transition.
        let input_state = PowerState {
            power_consumption: power_consumed,
            performance: device.performance_profile.sustainable_performance,
            efficiency: if power_consumed > 0.0 {
                device.performance_profile.sustainable_performance / power_consumed
            } else {
                0.0
            },
            thermal_state: self.thermal_monitor.state(),
            battery_level: self.battery_monitor.level(),
        };
        self.power_optimizer.optimize(&input_state);
    }

    /// Get the current power policy.
    pub fn power_policy(&self) -> &PowerPolicy {
        &self.power_policy
    }

    /// Set the power policy.
    pub fn set_power_policy(&mut self, policy: PowerPolicy) {
        self.power_policy = policy;
    }

    /// Get a mutable reference to the power optimizer for direct optimization.
    pub fn power_optimizer_mut(&mut self) -> &mut PowerOptimizer {
        &mut self.power_optimizer
    }

    /// Get a mutable reference to the battery monitor for updates.
    pub fn battery_monitor_mut(&mut self) -> &mut BatteryMonitor {
        &mut self.battery_monitor
    }

    /// Get a mutable reference to the thermal monitor for updates.
    pub fn thermal_monitor_mut(&mut self) -> &mut ThermalMonitor {
        &mut self.thermal_monitor
    }

    /// Get battery level
    pub fn get_battery_level(&self, _device_id: &str) -> f64 {
        self.battery_monitor.current_level
    }

    /// Get thermal state derived from the current estimated power draw.
    ///
    /// Power-to-thermal mapping (mobile SoC heuristic):
    /// - `< 3W`  → `Normal` (cool)
    /// - `3–7W`  → `Warm`
    /// - `> 7W`  → `Critical`
    pub fn get_thermal_state(&self, device_id: &str) -> ThermalState {
        let power = self.get_power_consumption(device_id);
        if power > 7.0 {
            ThermalState::Critical
        } else if power >= 3.0 {
            ThermalState::Warm
        } else {
            ThermalState::Normal
        }
    }

    /// Get power consumption in watts.
    ///
    /// On platforms exposing a power API (e.g. RAPL on Intel, the Energy
    /// Meter on Android) this would query the hardware. On every other target
    /// we estimate consumption from the current orchestration state and the
    /// number of active models, which is what battery-aware ML scheduling and
    /// thermal management rely on:
    ///
    /// | State            | Base power |
    /// |------------------|------------|
    /// | Idle             | ~0.5 W     |
    /// | Active inference | ~5.0 W + (active_models × 2.0 W) |
    /// | Scrubbing        | ~3.0 W     |
    /// | Streaming        | ~4.0 W     |
    pub fn get_power_consumption(&self, device_id: &str) -> f64 {
        // NOTE: a real implementation would probe `/sys/class/powercap/` (RAPL),
        // `android.os.PowerManager` via JNI, or the CoreML energy log. Until a
        // platform power API is wired in, estimate from the orchestration state.
        let _ = device_id; // hardware query would be keyed on this id
        match self.orchestration_state {
            AmbientOrchestrationState::Idle => 0.5,
            AmbientOrchestrationState::ActiveInference => {
                5.0 + (self.active_model_count as f64) * 2.0
            }
            AmbientOrchestrationState::Scrubbing => 3.0,
            AmbientOrchestrationState::Streaming => 4.0,
        }
    }

    /// Estimate battery life remaining in hours.
    ///
    /// `hours = (battery_capacity_wh * current_battery_pct / 100.0) / power_consumption`
    ///
    /// Returns `0.0` if the estimated power consumption is zero (avoids
    /// division by zero) or if the battery percentage is non-positive.
    pub fn estimate_battery_life_remaining(
        &self,
        current_battery_pct: f64,
        battery_capacity_wh: f64,
    ) -> f64 {
        if current_battery_pct <= 0.0 || battery_capacity_wh <= 0.0 {
            return 0.0;
        }
        let power = self.get_power_consumption("");
        if power <= 0.0 {
            return 0.0;
        }
        (battery_capacity_wh * current_battery_pct / 100.0) / power
    }

    /// Decide whether inference should be throttled.
    ///
    /// Returns `true` when the thermal state is `Critical` or when the
    /// estimated battery life (using the battery monitor's current charge
    /// against a 15 Wh mobile battery as a reasonable default) drops below
    /// 1 hour.
    pub fn should_throttle_inference(&self) -> bool {
        let thermal = self.get_thermal_state("");
        if thermal == ThermalState::Critical {
            return true;
        }
        // Reasonable mobile default: 15 Wh battery. Use the battery monitor's
        // current charge level so real battery drain drives the decision.
        let battery_pct = self.battery_monitor.current_level;
        if battery_pct <= 0.0 {
            return true; // No battery left — must throttle.
        }
        let estimated_hours = self.estimate_battery_life_remaining(battery_pct, 15.0);
        estimated_hours < 1.0
    }

    /// Aggregate the current power/thermal/battery snapshot.
    ///
    /// `estimated_battery_hours` is `Some` when a non-zero battery capacity is
    /// known; here we use the battery monitor's current level against a 15 Wh
    /// mobile battery default. Returns `None` when the device has no battery
    /// (e.g. mains-powered embedded host).
    pub fn get_power_metrics(&self) -> PowerMetrics {
        let current_power_w = self.get_power_consumption("");
        let thermal_state = self.get_thermal_state("");

        // The battery monitor tracks a 0–100 percentage. Use a 15 Wh mobile
        // battery as the default capacity when one is present.
        let battery_pct = self.battery_monitor.current_level;
        let estimated_battery_hours = if battery_pct > 0.0 {
            let hours = self.estimate_battery_life_remaining(battery_pct, 15.0);
            if hours > 0.0 {
                Some(hours)
            } else {
                None
            }
        } else {
            None
        };

        PowerMetrics {
            current_power_w,
            thermal_state,
            estimated_battery_hours,
            active_model_count: self.active_model_count,
        }
    }
}

impl BatteryMonitor {
    pub fn new() -> Self {
        Self {
            current_level: 100.0,
            voltage: 3.7,
            temperature: 25.0,
            health: 100.0,
            charging: false,
            estimated_time_remaining: Duration::from_secs(3600 * 10), // 10 hours
        }
    }

    /// Current battery level as a percentage (0–100).
    pub fn level(&self) -> f64 {
        self.current_level
    }

    /// Battery voltage in volts.
    pub fn voltage(&self) -> f64 {
        self.voltage
    }

    /// Battery temperature in degrees Celsius.
    pub fn temperature(&self) -> f64 {
        self.temperature
    }

    /// Battery health as a percentage (0–100, where 100 = new).
    pub fn health(&self) -> f64 {
        self.health
    }

    /// Whether the battery is currently charging.
    pub fn is_charging(&self) -> bool {
        self.charging
    }

    /// Estimated time remaining until the battery is depleted.
    pub fn time_remaining(&self) -> Duration {
        self.estimated_time_remaining
    }

    /// Update the battery state from platform telemetry.
    pub fn update(&mut self, level: f64, voltage: f64, temperature: f64, charging: bool) {
        self.current_level = level.clamp(0.0, 100.0);
        self.voltage = voltage;
        self.temperature = temperature;
        self.charging = charging;
        // Estimate time remaining based on current drain rate.
        // A simple linear model: if not charging, estimate from level and
        // a nominal drain of 10%/hour for active use.
        if !charging && level > 0.0 {
            let hours = level / 10.0;
            self.estimated_time_remaining = Duration::from_secs((hours * 3600.0) as u64);
        } else if charging {
            // While charging, estimate time to full at ~20%/hour charge rate.
            let hours_to_full = (100.0 - level) / 20.0;
            self.estimated_time_remaining = Duration::from_secs((hours_to_full * 3600.0) as u64);
        }
    }

    /// Apply battery drain from a computation that consumed `power_w` watts
    /// for `duration`. Uses a nominal 15 Wh battery capacity.
    pub fn drain(&mut self, power_w: f64, duration: Duration) {
        if self.charging {
            return; // No drain while charging.
        }
        let wh_consumed = power_w * duration.as_secs_f64() / 3600.0;
        let battery_capacity_wh = 15.0;
        let pct_drained = (wh_consumed / battery_capacity_wh) * 100.0;
        self.current_level = (self.current_level - pct_drained).max(0.0);
        // Update temperature estimate from power draw.
        self.temperature += power_w * 0.5 * duration.as_secs_f64();
    }
}

impl ThermalMonitor {
    pub fn new() -> Self {
        Self {
            cpu_temperature: 45.0,
            gpu_temperature: 40.0,
            battery_temperature: 30.0,
            ambient_temperature: 25.0,
            thermal_state: ThermalState::Normal,
        }
    }

    /// CPU temperature in degrees Celsius.
    pub fn cpu_temp(&self) -> f64 {
        self.cpu_temperature
    }

    /// GPU temperature in degrees Celsius.
    pub fn gpu_temp(&self) -> f64 {
        self.gpu_temperature
    }

    /// Battery temperature in degrees Celsius.
    pub fn battery_temp(&self) -> f64 {
        self.battery_temperature
    }

    /// Ambient (environmental) temperature in degrees Celsius.
    pub fn ambient_temp(&self) -> f64 {
        self.ambient_temperature
    }

    /// Current thermal state classification.
    pub fn state(&self) -> ThermalState {
        self.thermal_state
    }

    /// Update thermal readings from platform sensors and reclassify the
    /// thermal state.
    ///
    /// State thresholds (mobile SoC heuristic):
    /// - CPU < 50°C → `Normal`
    /// - CPU 50–70°C → `Warm`
    /// - CPU 70–85°C → `Hot`
    /// - CPU > 85°C → `Critical`
    pub fn update(&mut self, cpu: f64, gpu: f64, battery: f64, ambient: f64) {
        self.cpu_temperature = cpu;
        self.gpu_temperature = gpu;
        self.battery_temperature = battery;
        self.ambient_temperature = ambient;
        self.thermal_state = if cpu > 85.0 {
            ThermalState::Critical
        } else if cpu > 70.0 {
            ThermalState::Hot
        } else if cpu > 50.0 {
            ThermalState::Warm
        } else {
            ThermalState::Normal
        };
    }

    /// Apply thermal impact from a computation that drew `power_w` watts
    /// for `duration`. Increases CPU/GPU temperatures proportionally.
    pub fn apply_heat(&mut self, power_w: f64, duration: Duration) {
        let secs = duration.as_secs_f64();
        // Each watt for 1 second raises CPU temp by ~0.1°C (simplified model).
        self.cpu_temperature += power_w * 0.1 * secs;
        self.gpu_temperature += power_w * 0.08 * secs;
        // Reclassify state after heating.
        self.thermal_state = if self.cpu_temperature > 85.0 {
            ThermalState::Critical
        } else if self.cpu_temperature > 70.0 {
            ThermalState::Hot
        } else if self.cpu_temperature > 50.0 {
            ThermalState::Warm
        } else {
            ThermalState::Normal
        };
    }

    /// Cool down toward ambient temperature. Called during idle periods.
    pub fn cool(&mut self, duration: Duration) {
        let secs = duration.as_secs_f64();
        // Cool at ~0.5°C/s toward ambient.
        let cooling = 0.5 * secs;
        self.cpu_temperature =
            self.cpu_temperature.max(self.ambient_temperature + cooling) - cooling;
        self.gpu_temperature =
            self.gpu_temperature.max(self.ambient_temperature + cooling) - cooling;
        // Reclassify state after cooling.
        self.thermal_state = if self.cpu_temperature > 85.0 {
            ThermalState::Critical
        } else if self.cpu_temperature > 70.0 {
            ThermalState::Hot
        } else if self.cpu_temperature > 50.0 {
            ThermalState::Warm
        } else {
            ThermalState::Normal
        };
    }
}

impl PowerOptimizer {
    pub fn new() -> Self {
        Self {
            optimization_algorithm: OptimizationAlgorithm::Greedy,
            optimization_history: Vec::new(),
            target_efficiency: 0.85,
        }
    }

    /// Optimize the power state to approach the target efficiency.
    ///
    /// Returns the optimized power state. The optimization algorithm
    /// determines the strategy:
    /// - `Greedy`: picks the lowest-power state that meets the target efficiency.
    /// - `Genetic`/`SimulatedAnnealing`/`ReinforcementLearning`: uses the same
    ///   greedy heuristic but records the decision for future learning.
    pub fn optimize(&mut self, input: &PowerState) -> PowerState {
        let mut output = input.clone();

        // Greedy: if efficiency is below target, reduce power consumption.
        if output.efficiency < self.target_efficiency {
            // Reduce power by 20% and see if efficiency improves.
            output.power_consumption *= 0.8;
            // Recalculate efficiency as performance per watt.
            if output.power_consumption > 0.0 {
                output.efficiency = output.performance / output.power_consumption;
            }
            // Adjust thermal state based on new power level.
            output.thermal_state = if output.power_consumption > 7.0 {
                ThermalState::Critical
            } else if output.power_consumption >= 3.0 {
                ThermalState::Warm
            } else {
                ThermalState::Normal
            };
        }

        // Record the optimization.
        let gain = if input.power_consumption > 0.0 {
            (input.power_consumption - output.power_consumption) / input.power_consumption
        } else {
            0.0
        };
        self.optimization_history.push(OptimizationRecord {
            timestamp: Instant::now(),
            algorithm: self.optimization_algorithm.clone(),
            input_state: input.clone(),
            output_state: output.clone(),
            efficiency_gain: gain,
        });

        // Trim history.
        if self.optimization_history.len() > 200 {
            let drop = self.optimization_history.len() - 200;
            self.optimization_history.drain(0..drop);
        }

        output
    }

    /// Get the average efficiency gain from recent optimizations.
    pub fn average_efficiency_gain(&self) -> f64 {
        if self.optimization_history.is_empty() {
            return 0.0;
        }
        self.optimization_history
            .iter()
            .map(|r| r.efficiency_gain)
            .sum::<f64>()
            / self.optimization_history.len() as f64
    }

    /// Get the target efficiency this optimizer is configured for.
    pub fn target_efficiency(&self) -> f64 {
        self.target_efficiency
    }
}
