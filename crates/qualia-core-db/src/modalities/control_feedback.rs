// Control Theory & Feedback Modality
// Provides self-stabilizing agents for power systems and sanctuary management

use crate::NQuin;
use std::collections::HashMap;

pub const CONTROL_BIT: u64 = 1u64 << 52;
pub const FEEDBACK_BIT: u64 = 1u64 << 51;
pub const STABILIZATION_BIT: u64 = 1u64 << 50;

/// Control system state for feedback loops
#[derive(Debug, Clone)]
pub struct ControlState {
    pub setpoint: f64,        // Target value
    pub process_variable: f64, // Current measured value
    pub error: f64,           // Difference between setpoint and PV
    pub integral: f64,        // Accumulated error for integral control
    pub derivative: f64,       // Rate of change for derivative control
    pub last_error: f64,      // Previous error for derivative calculation
    pub last_time: u64,        // Timestamp for derivative calculation
}

impl ControlState {
    /// Create a new control state
    pub fn new(setpoint: f64, initial_value: f64) -> Self {
        let error = setpoint - initial_value;
        Self {
            setpoint,
            process_variable: initial_value,
            error,
            integral: 0.0,
            derivative: 0.0,
            last_error: error,
            last_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
    
    /// Update control state with new measurement
    pub fn update(&mut self, new_value: f64, current_time: u64) {
        self.process_variable = new_value;
        self.last_error = self.error;
        self.error = self.setpoint - new_value;
        
        // Calculate derivative (rate of change)
        let dt = (current_time - self.last_time) as f64;
        if dt > 0.0 {
            self.derivative = (self.error - self.last_error) / dt;
            self.integral += self.error * dt;
        }
        
        self.last_time = current_time;
    }
    
    /// Reset integral term to prevent windup
    pub fn reset_integral(&mut self) {
        self.integral = 0.0;
    }
}

/// PID controller parameters
#[derive(Debug, Clone)]
pub struct PidParameters {
    pub kp: f64, // Proportional gain
    pub ki: f64, // Integral gain
    pub kd: f64, // Derivative gain
    pub output_min: f64,
    pub output_max: f64,
}

impl PidParameters {
    /// Create conservative PID parameters for power systems
    pub fn conservative_power_system() -> Self {
        Self {
            kp: 0.5,
            ki: 0.1,
            kd: 0.05,
            output_min: 0.0,
            output_max: 100.0,
        }
    }
    
    /// Create aggressive PID parameters for fast response
    pub fn aggressive_response() -> Self {
        Self {
            kp: 1.0,
            ki: 0.5,
            kd: 0.2,
            output_min: 0.0,
            output_max: 100.0,
        }
    }
}

/// Feedback controller using PID algorithm
#[derive(Debug, Clone)]
pub struct FeedbackController {
    pub name: String,
    pub parameters: PidParameters,
    pub state: ControlState,
    pub enabled: bool,
}

impl FeedbackController {
    /// Create a new feedback controller
    pub fn new(name: String, setpoint: f64, initial_value: f64, params: PidParameters) -> Self {
        Self {
            name,
            parameters: params,
            state: ControlState::new(setpoint, initial_value),
            enabled: true,
        }
    }
    
    /// Compute control output using PID algorithm
    pub fn compute_output(&mut self) -> f64 {
        if !self.enabled {
            return 0.0;
        }
        
        // PID calculation: output = Kp*error + Ki*integral + Kd*derivative
        let proportional = self.parameters.kp * self.state.error;
        let integral = self.parameters.ki * self.state.integral;
        let derivative = self.parameters.kd * self.state.derivative;
        
        let mut output = proportional + integral + derivative;
        
        // Clamp output to limits
        output = output.clamp(self.parameters.output_min, self.parameters.output_max);
        
        // Anti-windup: reset integral if output is saturated
        if output >= self.parameters.output_max && self.state.error > 0.0 {
            self.state.reset_integral();
        } else if output <= self.parameters.output_min && self.state.error < 0.0 {
            self.state.reset_integral();
        }
        
        output
    }
    
    /// Update controller with new measurement
    pub fn update(&mut self, new_value: f64) -> f64 {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        self.state.update(new_value, current_time);
        self.compute_output()
    }
    
    /// Change setpoint
    pub fn set_setpoint(&mut self, new_setpoint: f64) {
        self.state.setpoint = new_setpoint;
        self.state.error = new_setpoint - self.state.process_variable;
    }
    
    /// Enable/disable controller
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.state.reset_integral();
        }
    }
}

/// Power system controller for 12V battery management
#[derive(Debug, Clone)]
pub struct PowerSystemController {
    pub battery_voltage_controller: FeedbackController,
    pub solar_current_controller: FeedbackController,
    pub load_balance_controller: FeedbackController,
    pub system_state: PowerSystemState,
}

#[derive(Debug, Clone)]
pub struct PowerSystemState {
    pub battery_voltage: f64,    // Volts
    pub solar_current: f64,     // Amps
    pub load_current: f64,      // Amps
    pub battery_soc: f64,       // State of charge (0-100%)
    pub solar_irradiance: f64,   // W/m²
    pub ambient_temperature: f64, // Celsius
}

impl PowerSystemController {
    /// Create a new power system controller for Jayco Songbird setup
    pub fn new() -> Self {
        // Battery voltage controller (target 12.6V for LiFePO4)
        let battery_voltage_controller = FeedbackController::new(
            "Battery Voltage".to_string(),
            12.6, // Target voltage for LiFePO4
            12.5, // Initial reading
            PidParameters::conservative_power_system(),
        );
        
        // Solar current controller (target based on irradiance)
        let solar_current_controller = FeedbackController::new(
            "Solar Current".to_string(),
            10.0, // Target current (will be updated based on conditions)
            5.0,  // Initial reading
            PidParameters::aggressive_response(),
        );
        
        // Load balance controller (target 0 net current)
        let load_balance_controller = FeedbackController::new(
            "Load Balance".to_string(),
            0.0,  // Target: balanced current
            2.0,  // Initial load current
            PidParameters::conservative_power_system(),
        );
        
        Self {
            battery_voltage_controller,
            solar_current_controller,
            load_balance_controller,
            system_state: PowerSystemState {
                battery_voltage: 12.5,
                solar_current: 5.0,
                load_current: 2.0,
                battery_soc: 80.0,
                solar_irradiance: 800.0,
                ambient_temperature: 25.0,
            },
        }
    }
    
    /// Update all controllers with current measurements
    pub fn update(&mut self, measurements: PowerSystemState) -> Vec<ControlAction> {
        self.system_state = measurements.clone();
        
        let mut actions = Vec::new();
        
        // Update battery voltage controller
        let voltage_output = self.battery_voltage_controller.update(measurements.battery_voltage);
        if voltage_output > 50.0 {
            actions.push(ControlAction::IncreaseCharging);
        } else if voltage_output < -50.0 {
            actions.push(ControlAction::DecreaseCharging);
        }
        
        // Update solar current controller based on irradiance
        let solar_target = self.calculate_solar_target(measurements.solar_irradiance);
        self.solar_current_controller.set_setpoint(solar_target);
        let solar_output = self.solar_current_controller.update(measurements.solar_current);
        if solar_output > 50.0 {
            actions.push(ControlAction::IncreaseSolarOutput);
        } else if solar_output < -50.0 {
            actions.push(ControlAction::DecreaseSolarOutput);
        }
        
        // Update load balance controller
        let net_current = measurements.solar_current - measurements.load_current;
        let balance_output = self.load_balance_controller.update(net_current);
        if balance_output > 50.0 {
            actions.push(ControlAction::ReduceLoad);
        } else if balance_output < -50.0 {
            actions.push(ControlAction::IncreaseLoad);
        }
        
        actions
    }
    
    /// Calculate optimal solar current target based on irradiance
    fn calculate_solar_target(&self, irradiance: f64) -> f64 {
        // Simple linear model: max 20A at 1000 W/m²
        let max_current = 20.0;
        let efficiency_factor = 0.85; // Account for system losses
        (irradiance / 1000.0) * max_current * efficiency_factor
    }
    
    /// Get system health status
    pub fn get_health_status(&self) -> SystemHealth {
        let voltage_error = (self.system_state.battery_voltage - 12.6).abs();
        let soc_low = self.system_state.battery_soc < 20.0;
        let high_temp = self.system_state.ambient_temperature > 35.0;
        
        if voltage_error > 1.0 || soc_low || high_temp {
            SystemHealth::Warning
        } else if voltage_error > 0.5 {
            SystemHealth::Caution
        } else {
            SystemHealth::Good
        }
    }
}

/// Control actions for power system
#[derive(Debug, Clone, PartialEq)]
pub enum ControlAction {
    IncreaseCharging,
    DecreaseCharging,
    IncreaseSolarOutput,
    DecreaseSolarOutput,
    IncreaseLoad,
    ReduceLoad,
    EmergencyShutdown,
}

/// System health status
#[derive(Debug, Clone, PartialEq)]
pub enum SystemHealth {
    Good,
    Caution,
    Warning,
    Critical,
}

/// Sanctuary perimeter controller for geofencing
#[derive(Debug, Clone)]
pub struct SanctuaryController {
    pub perimeter_controller: FeedbackController,
    pub intrusion_detection: bool,
    pub sanctuary_radius: f64, // meters
    pub current_position: (f64, f64),
}

impl SanctuaryController {
    /// Create a new sanctuary controller
    pub fn new(radius: f64) -> Self {
        Self {
            perimeter_controller: FeedbackController::new(
                "Sanctuary Perimeter".to_string(),
                radius,    // Target: stay within radius
                radius - 10.0, // Current position (slightly inside)
                PidParameters::conservative_power_system(),
            ),
            intrusion_detection: true,
            sanctuary_radius: radius,
            current_position: (0.0, 0.0),
        }
    }
    
    /// Update sanctuary controller with current position
    pub fn update(&mut self, position: (f64, f64)) -> Vec<SanctuaryAction> {
        self.current_position = position;
        
        // Calculate distance from center
        let distance = (position.0.powi(2) + position.1.powi(2)).sqrt();
        
        // Update controller (negative error means outside perimeter)
        let error = self.sanctuary_radius - distance;
        let output = self.perimeter_controller.update(error);
        
        let mut actions = Vec::new();
        
        if distance > self.sanctuary_radius {
            actions.push(SanctuaryAction::IntrusionAlert);
            if output > 70.0 {
                actions.push(SanctuaryAction::ReturnToPerimeter);
            }
        } else if distance < self.sanctuary_radius * 0.5 {
            actions.push(SanctuaryAction::NearingCenter);
        }
        
        actions
    }
}

/// Sanctuary control actions
#[derive(Debug, Clone, PartialEq)]
pub enum SanctuaryAction {
    IntrusionAlert,
    ReturnToPerimeter,
    NearingCenter,
    PerimeterBreach,
}

/// Convert controller state to NQuin for storage
pub fn controller_to_quin(controller: &FeedbackController, context: u64) -> NQuin {
    let mut quin = NQuin {
        subject: crate::q_hash(&controller.name),
        predicate: crate::q_hash("has_control_state"),
        object: ((controller.state.setpoint * 1000.0) as u64) << 32 |
                ((controller.state.process_variable * 1000.0) as u64 & 0xFFFFFFFF),
        context,
        metadata: CONTROL_BIT | if controller.enabled { 1 } else { 0 },
        parity: 0,
    };
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
    quin
}

/// Create a power management scenario for testing
pub fn create_power_scenario() -> PowerSystemController {
    let mut controller = PowerSystemController::new();
    
    // Simulate high solar irradiance scenario
    let measurements = PowerSystemState {
        battery_voltage: 13.2,  // Slightly high (overcharging)
        solar_current: 15.0,    // Good solar output
        load_current: 8.0,      // Moderate load
        battery_soc: 85.0,      // Good charge level
        solar_irradiance: 900.0, // Bright sun
        ambient_temperature: 30.0, // Warm day
    };
    
    let actions = controller.update(measurements);
    println!("Power actions: {:?}", actions);
    
    controller
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pid_controller() {
        let mut controller = FeedbackController::new(
            "Test".to_string(),
            10.0,
            8.0,
            PidParameters::conservative_power_system(),
        );
        
        // First update
        let output1 = controller.update(8.0);
        assert!(output1 > 0.0); // Should increase output to reach setpoint
        
        // Second update (closer to setpoint)
        let output2 = controller.update(9.0);
        assert!(output2 < output1); // Output should decrease as error reduces
    }
    
    #[test]
    fn test_power_system_controller() {
        let mut controller = PowerSystemController::new();
        
        let measurements = PowerSystemState {
            battery_voltage: 12.0,  // Low voltage
            solar_current: 5.0,
            load_current: 10.0,
            battery_soc: 60.0,
            solar_irradiance: 600.0,
            ambient_temperature: 20.0,
        };
        
        let actions = controller.update(measurements);
        assert!(!actions.is_empty());
        
        let health = controller.get_health_status();
        assert_eq!(health, SystemHealth::Warning); // Low voltage should trigger warning
    }
    
    #[test]
    fn test_sanctuary_controller() {
        let mut controller = SanctuaryController::new(100.0);
        
        // Test position inside sanctuary
        let actions1 = controller.update((50.0, 50.0));
        assert!(actions1.is_empty());
        
        // Test position outside sanctuary
        let actions2 = controller.update((110.0, 110.0));
        assert!(actions2.contains(&SanctuaryAction::IntrusionAlert));
    }
    
    #[test]
    fn test_control_state_update() {
        let mut state = ControlState::new(10.0, 8.0);
        assert_eq!(state.error, 2.0);
        
        state.update(9.0, 1);
        assert_eq!(state.error, 1.0);
        assert_eq!(state.process_variable, 9.0);
        assert!(state.integral > 0.0); // Should accumulate error
    }
}
