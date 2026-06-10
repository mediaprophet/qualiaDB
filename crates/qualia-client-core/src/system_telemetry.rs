//! Structured system telemetry events for Flutter HUD (100 ms during activation).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTelemetryEvent {
    pub ram_used_mb: u32,
    pub ram_total_mb: u32,
    pub vram_used_mb: u32,
    pub vram_total_mb: u32,
    pub llm_memory_mb: u32,
    pub kv_cache_mb: u32,
    pub lifecycle: String,
    pub status: String,
    pub activation_in_progress: bool,
}

struct TelemetryBus {
    senders: Mutex<Vec<SyncSender<SystemTelemetryEvent>>>,
}

impl TelemetryBus {
    fn new() -> Self {
        Self {
            senders: Mutex::new(Vec::new()),
        }
    }

    fn subscribe(&self, tx: SyncSender<SystemTelemetryEvent>) {
        if let Ok(mut senders) = self.senders.lock() {
            senders.push(tx);
        }
    }

    fn publish(&self, event: SystemTelemetryEvent) {
        if let Ok(mut senders) = self.senders.lock() {
            senders.retain(|tx| match tx.try_send(event.clone()) {
                Ok(_) => true,
                Err(TrySendError::Full(_)) => true,
                Err(TrySendError::Disconnected(_)) => false,
            });
        }
    }
}

fn bus() -> &'static TelemetryBus {
    static BUS: OnceLock<TelemetryBus> = OnceLock::new();
    BUS.get_or_init(TelemetryBus::new)
}

static ACTIVATION_TICKER: AtomicBool = AtomicBool::new(false);
static TICKER_STOP: AtomicBool = AtomicBool::new(false);

pub fn subscribe_system_telemetry(tx: SyncSender<SystemTelemetryEvent>) {
    bus().subscribe(tx);
}

pub fn publish_system_telemetry(event: SystemTelemetryEvent) {
    bus().publish(event);
}

fn probe_vram_usage_mb() -> (u32, u32) {
    #[cfg(target_os = "windows")]
    {
        if let Ok(memory) = qualia_core_db::directml_bridge::probe_best_adapter_memory() {
            let used = memory.local_usage_bytes / (1024 * 1024);
            let total = memory.local_budget_bytes / (1024 * 1024);
            return (used as u32, total as u32);
        }
    }
    (0, 0)
}

fn sample_event(status: &str, activation_in_progress: bool) -> SystemTelemetryEvent {
    use sysinfo::System;

    let mut sys = System::new_all();
    sys.refresh_memory();
    let (vram_used_mb, vram_total_mb) = probe_vram_usage_mb();
    let ram_total_mb = (sys.total_memory() / (1024 * 1024)).min(u32::MAX as u64) as u32;
    let ram_used_mb = (sys.used_memory() / (1024 * 1024)).min(u32::MAX as u64) as u32;
    let llm_memory_mb =
        (crate::model_lifecycle::get_llm_memory_bytes() / (1024 * 1024)).min(u32::MAX as u64) as u32;

    SystemTelemetryEvent {
        ram_used_mb,
        ram_total_mb,
        vram_used_mb,
        vram_total_mb,
        llm_memory_mb,
        kv_cache_mb: crate::model_lifecycle::get_kv_cache_used_mb(),
        lifecycle: crate::model_lifecycle::lifecycle_label(
            crate::model_lifecycle::get_model_lifecycle_state(),
        )
        .to_string(),
        status: status.to_string(),
        activation_in_progress,
    }
}

pub fn stop_activation_telemetry() {
    TICKER_STOP.store(true, Ordering::Release);
}

/// Push telemetry every 100 ms while model activation runs.
pub fn start_activation_telemetry(status: impl Into<String>) {
    if ACTIVATION_TICKER
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    TICKER_STOP.store(false, Ordering::Release);
    let status = status.into();
    thread::Builder::new()
        .name("qualia-telemetry-ticker".into())
        .spawn(move || {
            bus().publish(sample_event(&status, true));
            while !TICKER_STOP.load(Ordering::Acquire) {
                bus().publish(sample_event(&status, true));
                thread::sleep(Duration::from_millis(100));
            }
            bus().publish(sample_event("Model activation complete", false));
            ACTIVATION_TICKER.store(false, Ordering::Release);
        })
        .ok();
}

pub fn publish_idle_telemetry() {
    bus().publish(sample_event("Idle", false));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_event_has_nonzero_ram_total_on_host() {
        let event = sample_event("test", false);
        assert!(event.ram_total_mb > 0);
    }
}
