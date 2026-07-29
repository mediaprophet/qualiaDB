//! Cross-crate activity flags for ambient telemetry (ontology jobs, inference, …).

use std::sync::atomic::{AtomicU32, Ordering};

static ONTOLOGY_JOBS_RUNNING: AtomicU32 = AtomicU32::new(0);
static ONTOLOGY_JOB_PULSE: AtomicU32 = AtomicU32::new(0);

/// Mark an ontology / cold-path job as active (increment running count).
pub fn begin_ontology_job() {
    ONTOLOGY_JOBS_RUNNING.fetch_add(1, Ordering::Relaxed);
    ONTOLOGY_JOB_PULSE.fetch_add(1, Ordering::Relaxed);
}

/// Mark an ontology job finished.
pub fn end_ontology_job() {
    ONTOLOGY_JOBS_RUNNING
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| {
            Some(n.saturating_sub(1))
        })
        .ok();
}

/// Normalized baking pressure for ambient shaders (0.0–1.0).
pub fn ontology_baking_pressure() -> f32 {
    let running = ONTOLOGY_JOBS_RUNNING.load(Ordering::Relaxed);
    let pulse = ONTOLOGY_JOB_PULSE.load(Ordering::Relaxed);
    if running > 0 {
        return 1.0;
    }
    if pulse == 0 {
        return 0.0;
    }
    // Decay visual after job completes.
    let decayed = (pulse as f32 * 0.92).max(0.0) as u32;
    ONTOLOGY_JOB_PULSE.store(decayed, Ordering::Relaxed);
    (decayed as f32 / 64.0).min(1.0)
}
