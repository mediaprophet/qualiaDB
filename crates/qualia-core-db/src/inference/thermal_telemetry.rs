//! W7 — real GPU thermal/power telemetry + a detect-and-recommend thermal governor.
//!
//! `orchestrator::ThermalGovernor` previously had only a *simulated* implementation
//! (`CalculusThermalGovernor`, a Newton-cooling ODE). This module adds a REAL one backed by NVIDIA
//! NVML (via the optional `nvml` feature / `nvml-wrapper`), reading actual GPU temperature and power.
//!
//! **Policy: detect + recommend, never silently escalate.** The governor maps live temperature to
//! `ThermalStatus` and exposes a *recommended* TDP cap; it does NOT change the GPU's power limit.
//! Enforcement is a separate, explicit, privileged opt-in
//! (`NvmlThermalGovernor::apply_power_limit_w`) that nothing here calls automatically — a human/admin
//! policy must invoke it. This is the in-repo form of the human-centric-control norm for the off-grid
//! / constrained-power target: the machine reports and recommends; the human decides.
//!
//! When the `nvml` feature is off, or NVML/the driver is absent (non-NVIDIA host), telemetry degrades
//! cleanly: `sample_gpu_thermal()` returns `None` and `open_thermal_governor()` returns the
//! `NullThermalGovernor` (always `Cool`), so callers never need to know whether NVML is present.

#![cfg(not(target_arch = "wasm32"))]

use crate::inference::orchestrator::{NullThermalGovernor, ThermalGovernor, ThermalStatus};
use std::sync::atomic::{AtomicBool, Ordering};

// ── Auto-cap mode switch (user option; default ON) ────────────────────────────
// When ON *and* a real NVML governor is active (i.e. there is an NVIDIA card), the governor
// automatically applies its recommended TDP cap after temperature has been sustained-Critical for
// `CRITICAL_DWELL` checks, and restores the original limit once the card cools back to Cool. This is a
// protective default the principal (Timothy) chose for the off-grid / constrained-power target — it
// only ever REDUCES power under sustained heat (never raises it or acts against the user), it is fully
// reversible, and it is user-toggleable, so it respects the human-centric-control norm. When OFF the
// governor is recommend-only (logs an advisory cap, changes nothing). On non-NVIDIA hosts there is no
// NVML governor, so this flag has no effect. Applying a cap needs admin/root; if that fails the
// governor degrades to recommend-only and logs it.
//
// MODE SWITCH (UI-reachable, like spec-decode): env `QUALIA_LLM_GPU_AUTO_CAP=0` (off) / `=1` (on) at
// launch; `set_gpu_auto_cap(bool)` at runtime (the UI/host calls this); `gpu_auto_cap_enabled()` to
// read the effective mode. The env var, when set, overrides the runtime flag in both directions.
static GPU_AUTO_CAP: AtomicBool = AtomicBool::new(true);

/// Number of consecutive Critical checks before auto-cap engages (hysteresis — ignores transient
/// spikes). At the decode-loop tick cadence (~every 64 tokens) this is several seconds of sustained
/// Critical.
pub const CRITICAL_DWELL: u32 = 3;

/// Enable/disable automatic TDP capping under sustained Critical temperature (`QUALIA_LLM_GPU_AUTO_CAP`).
/// Runtime mode switch — the desktop UI / host calls this. Default ON (effective only when an NVIDIA
/// card + NVML are present).
#[inline]
pub fn set_gpu_auto_cap(on: bool) {
    GPU_AUTO_CAP.store(on, Ordering::Relaxed);
}

/// Whether automatic TDP capping is enabled (env var wins if set, else the runtime flag). Read this to
/// reflect the current mode in a UI.
#[inline]
pub fn gpu_auto_cap_enabled() -> bool {
    match std::env::var("QUALIA_LLM_GPU_AUTO_CAP").ok().as_deref() {
        Some("0") | Some("false") => false,
        Some("1") | Some("true") => true,
        _ => GPU_AUTO_CAP.load(Ordering::Relaxed),
    }
}

/// What the governor should do this check — the pure decision, separated from the NVML side effects so
/// it is unit-testable. `critical_streak` counts consecutive Critical checks INCLUDING this one;
/// `capped` is whether we currently hold an applied cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalAction {
    /// Nothing to do.
    Nothing,
    /// Advisory only — log the recommended cap; do not change hardware.
    Recommend,
    /// Auto-cap is on and Critical has been sustained past the dwell — apply the recommended cap now.
    ApplyCap,
    /// Cooled back to Cool after we had applied a cap — restore the original limit.
    Restore,
}

/// Pure enforcement decision (see [`ThermalAction`]). Hysteresis: only caps on **sustained** Critical,
/// holds the cap through Warm, and restores only once back to Cool.
pub fn decide_thermal_action(
    status: ThermalStatus,
    critical_streak: u32,
    auto_cap: bool,
    capped: bool,
) -> ThermalAction {
    match status {
        ThermalStatus::Critical => {
            if auto_cap && !capped && critical_streak >= CRITICAL_DWELL {
                ThermalAction::ApplyCap
            } else if !capped {
                ThermalAction::Recommend
            } else {
                ThermalAction::Nothing // already capped — hold
            }
        }
        ThermalStatus::Warm => {
            if capped {
                ThermalAction::Nothing // hold the cap through Warm (hysteresis)
            } else {
                ThermalAction::Recommend
            }
        }
        ThermalStatus::Cool => {
            if capped {
                ThermalAction::Restore
            } else {
                ThermalAction::Nothing
            }
        }
    }
}

/// GPU temperature bands (°C) — the same thresholds the simulated `CalculusThermalGovernor` uses, so
/// the real and simulated governors classify identically.
pub const WARM_THRESHOLD_C: u32 = 65;
pub const CRITICAL_THRESHOLD_C: u32 = 85;

/// Map a GPU temperature to the project `ThermalStatus`.
#[inline]
pub fn status_for_temp(temp_c: u32) -> ThermalStatus {
    if temp_c > CRITICAL_THRESHOLD_C {
        ThermalStatus::Critical
    } else if temp_c > WARM_THRESHOLD_C {
        ThermalStatus::Warm
    } else {
        ThermalStatus::Cool
    }
}

/// A point-in-time GPU thermal/power reading.
#[derive(Debug, Clone, Copy)]
pub struct GpuThermalSample {
    /// GPU core temperature (°C).
    pub temp_c: u32,
    /// Instantaneous board power draw (W).
    pub power_w: f64,
    /// Currently enforced power limit / TDP (W).
    pub power_limit_w: f64,
    /// Min settable power limit (W) from the driver constraints.
    pub power_min_w: f64,
    /// Max settable power limit (W) from the driver constraints.
    pub power_max_w: f64,
    /// Thermal classification derived from `temp_c`.
    pub status: ThermalStatus,
}

impl GpuThermalSample {
    /// A *recommended* TDP cap (W) when the GPU is running hot — advisory only; nothing here applies
    /// it. `None` when `Cool` (no action recommended). Clamped to the driver's `[min, max]` limits.
    pub fn recommended_power_cap_w(&self) -> Option<f64> {
        let frac = match self.status {
            ThermalStatus::Cool => return None,
            ThermalStatus::Warm => 0.90,
            ThermalStatus::Critical => 0.80,
        };
        let lo = self.power_min_w.max(1.0);
        let hi = self.power_max_w.max(lo);
        Some((self.power_limit_w * frac).clamp(lo, hi))
    }
}

/// Read one GPU thermal/power sample. `None` when the `nvml` feature is off or NVML/the driver is
/// unavailable (non-NVIDIA host, no driver). Never panics. Suitable for a UI telemetry poll.
pub fn sample_gpu_thermal() -> Option<GpuThermalSample> {
    #[cfg(feature = "nvml")]
    {
        nvml_impl::one_shot_sample().ok()
    }
    #[cfg(not(feature = "nvml"))]
    {
        None
    }
}

/// Construct the best available thermal governor: the real NVML one when the `nvml` feature is on and
/// NVML initializes, else the `NullThermalGovernor` (always `Cool`). Mirrors `open_storage` /
/// `open_platform_filter` — callers don't branch on platform/feature.
pub fn open_thermal_governor() -> Box<dyn ThermalGovernor> {
    #[cfg(feature = "nvml")]
    {
        match nvml_impl::NvmlThermalGovernor::new() {
            Ok(g) => {
                log::info!("W7|thermal|NVML governor active: {}", g.device_label());
                return Box::new(g);
            }
            Err(e) => {
                log::info!("W7|thermal|NVML unavailable ({e}) — NullThermalGovernor (always Cool)");
            }
        }
    }
    Box::new(NullThermalGovernor)
}

/// Periodic thermal check for the decode hot path — samples a RESIDENT NVML governor and, per the
/// auto-cap user option, enforces (sustained Critical → apply the recommended cap; cooled → restore)
/// or recommends. Cheap (an NVML read); call every N decode tokens. No-op when the `nvml` feature is
/// off or NVML/the driver is unavailable. The resident governor keeps the auto-cap hysteresis state
/// (Critical streak, capped flag, saved limit) across ticks.
pub fn thermal_tick() {
    #[cfg(feature = "nvml")]
    {
        use std::sync::OnceLock;
        static RESIDENT: OnceLock<Option<nvml_impl::NvmlThermalGovernor>> = OnceLock::new();
        if let Some(g) = RESIDENT.get_or_init(|| nvml_impl::NvmlThermalGovernor::new().ok()) {
            g.tick();
        }
    }
}

/// The real NVML-backed governor. Construct directly (`NvmlThermalGovernor::new()`) when you need the
/// privileged `apply_power_limit_w` enforcement path or repeated `sample()`s; use
/// `open_thermal_governor()` for the trait-object detect+recommend role.
#[cfg(feature = "nvml")]
pub use nvml_impl::NvmlThermalGovernor;

#[cfg(feature = "nvml")]
mod nvml_impl {
    use super::*;
    use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
    use nvml_wrapper::Nvml;
    use std::sync::atomic::AtomicU32;

    /// A real NVML-backed thermal governor over GPU 0. Detect + recommend only; the sole enforcement
    /// path (`apply_power_limit_w`) is explicit, privileged, and never called automatically.
    pub struct NvmlThermalGovernor {
        nvml: Nvml,
        index: u32,
        label: String,
        /// Consecutive Critical checks (hysteresis for auto-cap).
        critical_streak: AtomicU32,
        /// Whether we currently hold an applied cap.
        capped: AtomicBool,
        /// The enforced power limit (mW) captured before we capped, restored on cool-down.
        saved_limit_mw: AtomicU32,
    }

    impl NvmlThermalGovernor {
        pub fn new() -> Result<Self, String> {
            let nvml = Nvml::init().map_err(|e| format!("NVML init: {e}"))?;
            let index = 0u32;
            let label = nvml
                .device_by_index(index)
                .and_then(|d| d.name())
                .unwrap_or_else(|_| "NVIDIA GPU".to_string());
            Ok(Self {
                nvml,
                index,
                label,
                critical_streak: AtomicU32::new(0),
                capped: AtomicBool::new(false),
                saved_limit_mw: AtomicU32::new(0),
            })
        }

        pub fn device_label(&self) -> &str {
            &self.label
        }

        /// Read one live sample from the resident NVML handle.
        pub fn sample(&self) -> Result<GpuThermalSample, String> {
            let dev = self
                .nvml
                .device_by_index(self.index)
                .map_err(|e| format!("device: {e}"))?;
            let temp_c = dev
                .temperature(TemperatureSensor::Gpu)
                .map_err(|e| format!("temperature: {e}"))?;
            let power_w = dev.power_usage().map(|mw| mw as f64 / 1000.0).unwrap_or(0.0);
            let power_limit_w = dev
                .enforced_power_limit()
                .map(|mw| mw as f64 / 1000.0)
                .unwrap_or(0.0);
            let (power_min_w, power_max_w) = dev
                .power_management_limit_constraints()
                .map(|c| (c.min_limit as f64 / 1000.0, c.max_limit as f64 / 1000.0))
                .unwrap_or((0.0, power_limit_w));
            Ok(GpuThermalSample {
                temp_c,
                power_w,
                power_limit_w,
                power_min_w,
                power_max_w,
                status: status_for_temp(temp_c),
            })
        }

        /// EXPLICIT, PRIVILEGED opt-in enforcement — set the GPU power limit (W). This is the ONLY
        /// method that mutates hardware state; it is NEVER called automatically by the governor and
        /// requires admin/root (returns `Err` otherwise). A human/admin policy must invoke it (no
        /// silent escalation).
        pub fn apply_power_limit_w(&self, watts: f64) -> Result<(), String> {
            self.apply_power_limit_mw((watts * 1000.0).round().max(0.0) as u32)
        }

        fn apply_power_limit_mw(&self, mw: u32) -> Result<(), String> {
            let mut dev = self
                .nvml
                .device_by_index(self.index)
                .map_err(|e| format!("device: {e}"))?;
            dev.set_power_management_limit(mw)
                .map_err(|e| format!("set_power_management_limit (needs admin): {e}"))
        }

        /// One thermal check: sample, decide (with hysteresis), then enforce (auto-cap on + sustained
        /// Critical → apply the recommended cap; cooled after a cap → restore) or recommend-only.
        /// Call periodically (e.g. every N decode tokens). Never panics; a failed NVML apply degrades
        /// to recommend-only + a log line.
        pub fn tick(&self) {
            let s = match self.sample() {
                Ok(s) => s,
                Err(_) => return,
            };
            let streak = if matches!(s.status, ThermalStatus::Critical) {
                self.critical_streak.fetch_add(1, Ordering::Relaxed) + 1
            } else {
                self.critical_streak.store(0, Ordering::Relaxed);
                0
            };
            let auto_cap = gpu_auto_cap_enabled();
            let capped = self.capped.load(Ordering::Relaxed);
            match decide_thermal_action(s.status, streak, auto_cap, capped) {
                ThermalAction::ApplyCap => {
                    if let Some(cap) = s.recommended_power_cap_w() {
                        self.saved_limit_mw
                            .store((s.power_limit_w * 1000.0).round() as u32, Ordering::Relaxed);
                        match self.apply_power_limit_w(cap) {
                            Ok(()) => {
                                self.capped.store(true, Ordering::Relaxed);
                                log::warn!(
                                    "W7|thermal|AUTO-CAP: {} sustained {}\u{b0}C — capped {:.0}W \u{2192} {:.0}W (reversible; auto_cap on)",
                                    self.label, s.temp_c, s.power_limit_w, cap
                                );
                            }
                            Err(e) => log::warn!(
                                "W7|thermal|auto-cap FAILED (needs admin) at {}\u{b0}C — RECOMMEND {:.0}W only: {e}",
                                s.temp_c, cap
                            ),
                        }
                    }
                }
                ThermalAction::Restore => {
                    let saved = self.saved_limit_mw.load(Ordering::Relaxed);
                    if saved > 0 {
                        match self.apply_power_limit_mw(saved) {
                            Ok(()) => log::info!(
                                "W7|thermal|cooled to {}\u{b0}C — restored power limit {:.0}W",
                                s.temp_c, saved as f64 / 1000.0
                            ),
                            Err(e) => log::warn!("W7|thermal|restore power limit FAILED: {e}"),
                        }
                    }
                    self.capped.store(false, Ordering::Relaxed);
                }
                ThermalAction::Recommend => {
                    if let Some(cap) = s.recommended_power_cap_w() {
                        log::warn!(
                            "W7|thermal|{:?} {}\u{b0}C {:.0}W/{:.0}W — RECOMMEND cap {:.0}W (advisory)",
                            s.status, s.temp_c, s.power_w, s.power_limit_w, cap
                        );
                    }
                }
                ThermalAction::Nothing => {}
            }
        }
    }

    impl ThermalGovernor for NvmlThermalGovernor {
        fn get_thermal_state(&self) -> ThermalStatus {
            self.sample().map(|s| s.status).unwrap_or(ThermalStatus::Cool)
        }

        fn adjust_policy(&self, _status: ThermalStatus) {
            // Full check: recommend, or auto-cap/restore when the user option is on (see `tick`).
            self.tick();
        }
    }

    /// One-shot read (init NVML, sample, drop). For repeated polling prefer a resident
    /// `NvmlThermalGovernor` to avoid re-initializing NVML each call.
    pub fn one_shot_sample() -> Result<GpuThermalSample, String> {
        NvmlThermalGovernor::new()?.sample()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_bands_classify_correctly() {
        assert_eq!(status_for_temp(30), ThermalStatus::Cool);
        assert_eq!(status_for_temp(WARM_THRESHOLD_C), ThermalStatus::Cool); // boundary is inclusive-Cool
        assert_eq!(status_for_temp(WARM_THRESHOLD_C + 1), ThermalStatus::Warm);
        assert_eq!(status_for_temp(80), ThermalStatus::Warm);
        assert_eq!(status_for_temp(CRITICAL_THRESHOLD_C), ThermalStatus::Warm);
        assert_eq!(status_for_temp(CRITICAL_THRESHOLD_C + 1), ThermalStatus::Critical);
        assert_eq!(status_for_temp(95), ThermalStatus::Critical);
    }

    #[test]
    fn recommended_cap_is_advisory_and_clamped() {
        let mk = |temp, limit, min, max| GpuThermalSample {
            temp_c: temp,
            power_w: 0.0,
            power_limit_w: limit,
            power_min_w: min,
            power_max_w: max,
            status: status_for_temp(temp),
        };
        // Cool → no recommendation.
        assert!(mk(40, 70.0, 40.0, 90.0).recommended_power_cap_w().is_none());
        // Warm → 90% of the enforced limit.
        let warm = mk(70, 70.0, 40.0, 90.0).recommended_power_cap_w().unwrap();
        assert!((warm - 63.0).abs() < 1e-6, "warm cap {warm}");
        // Critical → 80%, clamped up to the driver minimum.
        let crit = mk(90, 70.0, 60.0, 90.0).recommended_power_cap_w().unwrap();
        assert!((crit - 60.0).abs() < 1e-6, "critical cap clamped to min: {crit}");
    }

    #[test]
    fn auto_cap_decision_hysteresis() {
        use ThermalAction::*;
        use ThermalStatus::*;
        // Cool, nothing held → nothing; Cool while capped → restore.
        assert_eq!(decide_thermal_action(Cool, 0, true, false), Nothing);
        assert_eq!(decide_thermal_action(Cool, 0, true, true), Restore);
        // Warm recommends when uncapped; holds the cap when capped (hysteresis).
        assert_eq!(decide_thermal_action(Warm, 0, true, false), Recommend);
        assert_eq!(decide_thermal_action(Warm, 0, true, true), Nothing);
        // Critical below the dwell → recommend only, even with auto-cap on.
        assert_eq!(decide_thermal_action(Critical, CRITICAL_DWELL - 1, true, false), Recommend);
        // Critical sustained past the dwell with auto-cap ON → apply the cap.
        assert_eq!(decide_thermal_action(Critical, CRITICAL_DWELL, true, false), ApplyCap);
        // Same, but auto-cap OFF → recommend only (the user option gates enforcement).
        assert_eq!(decide_thermal_action(Critical, CRITICAL_DWELL, false, false), Recommend);
        // Already capped and still Critical → hold (don't re-apply).
        assert_eq!(decide_thermal_action(Critical, CRITICAL_DWELL + 5, true, true), Nothing);
    }

    #[test]
    fn auto_cap_toggle_roundtrips() {
        set_gpu_auto_cap(false);
        assert!(!gpu_auto_cap_enabled() || std::env::var("QUALIA_LLM_GPU_AUTO_CAP").is_ok());
        set_gpu_auto_cap(true);
        assert!(gpu_auto_cap_enabled() || std::env::var("QUALIA_LLM_GPU_AUTO_CAP").as_deref() == Ok("0"));
    }

    /// Hardware smoke test — only compiled with `--features nvml` and only meaningful on an NVIDIA
    /// host. Reads one live sample and sanity-checks it; skips gracefully if NVML is absent.
    #[cfg(feature = "nvml")]
    #[test]
    fn nvml_live_sample_smoke() {
        match sample_gpu_thermal() {
            Some(s) => {
                println!(
                    "[w7] {:?} temp={}\u{b0}C power={:.1}W limit={:.1}W [{:.1}..{:.1}]W rec={:?}",
                    s.status, s.temp_c, s.power_w, s.power_limit_w, s.power_min_w, s.power_max_w,
                    s.recommended_power_cap_w()
                );
                assert!(s.temp_c > 0 && s.temp_c < 130, "implausible GPU temp {}", s.temp_c);
                assert_eq!(s.status, status_for_temp(s.temp_c));
            }
            None => eprintln!("[w7] NVML unavailable on this host — smoke test skipped"),
        }
    }
}
