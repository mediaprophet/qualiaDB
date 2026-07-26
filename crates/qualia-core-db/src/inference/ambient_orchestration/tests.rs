use super::*;
use std::time::Duration;

#[test]
fn test_ambient_orchestration_creation() {
    let manager = AmbientOrchestrationManager::new();
    assert_eq!(manager.list_devices().len(), 0);
}

#[test]
fn test_device_discovery() {
    let mut manager = AmbientOrchestrationManager::new();

    let devices = manager.discover_devices().unwrap();
    assert!(!devices.is_empty());
    assert!(devices.len() <= 9);
    assert_eq!(devices.len(), manager.list_devices().len());
    assert!(devices.iter().any(|id| id == "local_host"));

    let host = manager.devices.get("local_host").unwrap();
    assert!(host.capabilities.compute_units >= 1);

    let cpu_core_count = devices
        .iter()
        .filter(|id| id.starts_with("cpu_core_"))
        .count();
    assert_eq!(devices.len(), cpu_core_count + 1);

    for device_id in &devices {
        let device_status = manager.get_device_status(device_id);
        assert!(device_status.is_some());
    }
}

#[test]
fn test_task_submission() {
    let mut manager = AmbientOrchestrationManager::new();

    let task = Task {
        task_id: "test_task".to_string(),
        task_type: TaskType::NeuralInference,
        priority: TaskPriority::Normal,
        resource_requirements: ResourceRequirements {
            compute_units: 2,
            memory: 1024 * 1024,
            neural_engines: 1,
            power_budget: 2.0,
            thermal_budget: 1.0,
        },
        deadline: None,
        estimated_duration: Duration::from_millis(100),
        dependencies: vec![],
    };

    let task_id = manager.submit_task(task).unwrap();
    assert_eq!(task_id, "test_task");
}

#[test]
fn test_neural_inference() {
    let mut manager = AmbientOrchestrationManager::new();

    let devices = manager.discover_devices().unwrap();
    let device_id = &devices[0];

    let model_data = vec![1u8; 1024];
    let input_data = vec![2u8; 512];

    let result = manager.execute_neural_inference(device_id, &model_data, &input_data);
    assert!(result.is_ok());
}

// ── Ambient power monitoring tests ───────────────────────────────────

#[test]
fn test_power_consumption_changes_with_orchestration_state() {
    let mut pm = PowerManager::new();

    // Idle baseline
    pm.set_orchestration_state(AmbientOrchestrationState::Idle);
    let idle_power = pm.get_power_consumption("local_host");
    assert!(
        (idle_power - 0.5).abs() < f64::EPSILON,
        "idle power should be ~0.5W, got {idle_power}"
    );

    // Active inference scales with active model count
    pm.set_active_model_count(2);
    pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
    let active_power = pm.get_power_consumption("local_host");
    // 5.0 + 2 * 2.0 = 9.0
    assert!(
        (active_power - 9.0).abs() < f64::EPSILON,
        "active inference power with 2 models should be ~9.0W, got {active_power}"
    );

    // Scrubbing
    pm.set_orchestration_state(AmbientOrchestrationState::Scrubbing);
    let scrub_power = pm.get_power_consumption("local_host");
    assert!(
        (scrub_power - 3.0).abs() < f64::EPSILON,
        "scrubbing power should be ~3.0W, got {scrub_power}"
    );

    // Streaming
    pm.set_orchestration_state(AmbientOrchestrationState::Streaming);
    let stream_power = pm.get_power_consumption("local_host");
    assert!(
        (stream_power - 4.0).abs() < f64::EPSILON,
        "streaming power should be ~4.0W, got {stream_power}"
    );

    // Verify ordering: idle < scrubbing < streaming < active(2 models)
    assert!(idle_power < scrub_power);
    assert!(scrub_power < stream_power);
    assert!(stream_power < active_power);
}

#[test]
fn test_power_consumption_scales_with_active_models() {
    let mut pm = PowerManager::new();
    pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);

    pm.set_active_model_count(0);
    assert!((pm.get_power_consumption("d") - 5.0).abs() < f64::EPSILON);

    pm.set_active_model_count(1);
    assert!((pm.get_power_consumption("d") - 7.0).abs() < f64::EPSILON);

    pm.set_active_model_count(3);
    assert!((pm.get_power_consumption("d") - 11.0).abs() < f64::EPSILON);
}

#[test]
fn test_thermal_state_mapping_cool_warm_critical() {
    let mut pm = PowerManager::new();

    // < 3W → Normal (Cool)
    pm.set_orchestration_state(AmbientOrchestrationState::Idle);
    assert_eq!(pm.get_thermal_state("d"), ThermalState::Normal);

    // 3W (boundary) → Warm
    pm.set_orchestration_state(AmbientOrchestrationState::Scrubbing);
    assert_eq!(pm.get_thermal_state("d"), ThermalState::Warm);

    // 4W → Warm
    pm.set_orchestration_state(AmbientOrchestrationState::Streaming);
    assert_eq!(pm.get_thermal_state("d"), ThermalState::Warm);

    // > 7W → Critical
    pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
    pm.set_active_model_count(2); // 9.0W
    assert_eq!(pm.get_thermal_state("d"), ThermalState::Critical);

    // Exactly 7W boundary → Warm (>= 3.0 and <= 7.0)
    pm.set_active_model_count(1); // 7.0W
    assert_eq!(pm.get_thermal_state("d"), ThermalState::Warm);
}

#[test]
fn test_estimate_battery_life_remaining() {
    let mut pm = PowerManager::new();
    pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
    pm.set_active_model_count(0); // 5.0W

    // 50% of a 10Wh battery = 5Wh / 5W = 1.0 hour
    let hours = pm.estimate_battery_life_remaining(50.0, 10.0);
    assert!((hours - 1.0).abs() < 1e-9, "expected 1.0 hour, got {hours}");

    // 100% of 15Wh / 5W = 3.0 hours
    let hours = pm.estimate_battery_life_remaining(100.0, 15.0);
    assert!(
        (hours - 3.0).abs() < 1e-9,
        "expected 3.0 hours, got {hours}"
    );

    // Zero power → 0.0 (avoid div-by-zero). Idle is 0.5W, so use a
    // contrived zero-battery case instead.
    assert_eq!(pm.estimate_battery_life_remaining(0.0, 10.0), 0.0);
    assert_eq!(pm.estimate_battery_life_remaining(50.0, 0.0), 0.0);
}

#[test]
fn test_should_throttle_inference_thermal_critical() {
    let mut pm = PowerManager::new();
    // Force Critical thermal: > 7W
    pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
    pm.set_active_model_count(2); // 9.0W → Critical
    assert!(
        pm.should_throttle_inference(),
        "should throttle when thermal state is Critical"
    );
}

#[test]
fn test_should_throttle_inference_low_battery() {
    let mut pm = PowerManager::new();
    // Idle: 0.5W, thermal Normal. Drain battery so estimated life < 1h.
    // With 0.5W and 15Wh default, full charge = 30h. To get < 1h we need
    // battery_pct such that (15 * pct/100) / 0.5 < 1 → pct < 3.33%.
    pm.set_orchestration_state(AmbientOrchestrationState::Idle);
    pm.battery_monitor.current_level = 2.0; // ~0.6h remaining
    assert!(
        pm.should_throttle_inference(),
        "should throttle when estimated battery life < 1 hour"
    );
}

#[test]
fn test_should_not_throttle_when_healthy() {
    let mut pm = PowerManager::new();
    // Idle, 0.5W, full battery → 30h remaining, Normal thermal.
    pm.set_orchestration_state(AmbientOrchestrationState::Idle);
    pm.battery_monitor.current_level = 100.0;
    assert!(
        !pm.should_throttle_inference(),
        "should not throttle when thermal is cool and battery is healthy"
    );
}

#[test]
fn test_power_metrics_aggregation() {
    let mut pm = PowerManager::new();
    pm.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
    pm.set_active_model_count(1); // 7.0W → Warm
    pm.battery_monitor.current_level = 50.0;

    let metrics = pm.get_power_metrics();
    assert!((metrics.current_power_w - 7.0).abs() < f64::EPSILON);
    assert_eq!(metrics.thermal_state, ThermalState::Warm);
    assert_eq!(metrics.active_model_count, 1);
    // 50% of 15Wh / 7W = 1.0714...h
    let expected = (15.0 * 50.0 / 100.0) / 7.0;
    assert!(metrics.estimated_battery_hours.is_some());
    assert!((metrics.estimated_battery_hours.unwrap() - expected).abs() < 1e-9);
}

#[test]
fn test_power_metrics_no_battery() {
    let mut pm = PowerManager::new();
    pm.battery_monitor.current_level = 0.0;
    let metrics = pm.get_power_metrics();
    assert!(metrics.estimated_battery_hours.is_none());
}

#[test]
fn test_manager_power_monitoring_integration() {
    let mut manager = AmbientOrchestrationManager::new();

    // Default idle state
    let metrics = manager.get_power_metrics();
    assert!((metrics.current_power_w - 0.5).abs() < f64::EPSILON);
    assert_eq!(metrics.thermal_state, ThermalState::Normal);

    // Transition to active inference with 3 models
    manager.set_orchestration_state(AmbientOrchestrationState::ActiveInference);
    manager.set_active_model_count(3); // 11.0W → Critical

    let metrics = manager.get_power_metrics();
    assert!((metrics.current_power_w - 11.0).abs() < f64::EPSILON);
    assert_eq!(metrics.thermal_state, ThermalState::Critical);
    assert!(manager.should_throttle_inference());
}
