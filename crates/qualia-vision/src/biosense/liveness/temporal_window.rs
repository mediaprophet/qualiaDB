//! Strict millisecond challenge windows (TTS / TTC).
//!
//! Prevents deepfake/replay brute-forcing by requiring the geometric action
//! to *start* and *complete* inside fixed local-clock bounds.

/// Default time-to-start: subject must begin geometric motion within this many ms.
pub const DEFAULT_TTS_MS: u32 = 800;
/// Default time-to-complete: action must cross threshold by this many ms.
pub const DEFAULT_TTC_MS: u32 = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalGate {
    /// Within both TTS and TTC windows for the observed events.
    Ok,
    /// No motion / action onset observed by TTS.
    TimeToStartExceeded,
    /// Action did not reach threshold by TTC.
    TimeToCompleteExceeded,
    /// Sample times empty or ill-ordered.
    InvalidTimeline,
}

#[derive(Debug, Clone, Copy)]
pub struct TemporalWindow {
    pub tts_ms: u32,
    pub ttc_ms: u32,
}

impl Default for TemporalWindow {
    fn default() -> Self {
        Self {
            tts_ms: DEFAULT_TTS_MS,
            ttc_ms: DEFAULT_TTC_MS,
        }
    }
}

/// Evaluate TTS/TTC against observed onset and completion times (ms since issue).
///
/// * `onset_ms` — first frame where action trajectory is detectably underway
///   (e.g. |Δyaw| > 0.15 × threshold). `None` if never started.
/// * `complete_ms` — first frame where full action threshold is met. `None` if never.
/// * `last_sample_ms` — last landmark sample time (for incomplete timelines).
pub fn check_temporal_window(
    window: TemporalWindow,
    onset_ms: Option<u32>,
    complete_ms: Option<u32>,
    last_sample_ms: u32,
) -> TemporalGate {
    if window.tts_ms == 0 || window.ttc_ms == 0 || window.tts_ms > window.ttc_ms {
        return TemporalGate::InvalidTimeline;
    }

    match onset_ms {
        None => {
            if last_sample_ms >= window.tts_ms {
                TemporalGate::TimeToStartExceeded
            } else {
                // Still inside TTS — caller should keep sampling, not fail yet.
                TemporalGate::Ok
            }
        }
        Some(onset) if onset > window.tts_ms => TemporalGate::TimeToStartExceeded,
        Some(_) => match complete_ms {
            Some(done) if done <= window.ttc_ms => TemporalGate::Ok,
            Some(_) => TemporalGate::TimeToCompleteExceeded,
            None if last_sample_ms >= window.ttc_ms => TemporalGate::TimeToCompleteExceeded,
            None => TemporalGate::Ok, // still inside TTC
        },
    }
}

/// Whether the session must hard-fail now (vs keep collecting frames).
pub fn temporal_is_terminal_fail(gate: TemporalGate) -> bool {
    matches!(
        gate,
        TemporalGate::TimeToStartExceeded
            | TemporalGate::TimeToCompleteExceeded
            | TemporalGate::InvalidTimeline
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_time_pass() {
        let g = check_temporal_window(TemporalWindow::default(), Some(400), Some(1200), 1200);
        assert_eq!(g, TemporalGate::Ok);
    }

    #[test]
    fn late_start_fails() {
        let g = check_temporal_window(TemporalWindow::default(), Some(900), Some(1500), 1500);
        assert_eq!(g, TemporalGate::TimeToStartExceeded);
    }

    #[test]
    fn never_complete_by_ttc_fails() {
        let g = check_temporal_window(TemporalWindow::default(), Some(200), None, 2100);
        assert_eq!(g, TemporalGate::TimeToCompleteExceeded);
    }

    #[test]
    fn still_inside_tts_ok() {
        let g = check_temporal_window(TemporalWindow::default(), None, None, 500);
        assert_eq!(g, TemporalGate::Ok);
    }
}
