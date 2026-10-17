//! Precision Timer and Sub-Millisecond Benchmark Telemetry.
//!
//! Solves the truncation problem where ultra-fast zero-heap graph traversals,
//! SIMD matrix operations, and NQuin evaluations (150 ns – 40 µs) truncate
//! to integer 0 ms when converted via `as_millis() as u64`.

use std::time::{Duration, Instant};

/// Formats a duration into a human-readable string with appropriate sub-millisecond units.
pub fn format_duration_human(duration: Duration) -> String {
    let nanos = duration.as_nanos();
    if nanos < 1_000 {
        format!("{nanos} ns")
    } else if nanos < 1_000_000 {
        format!("{:.3} µs", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.3} ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.3} s", duration.as_secs_f64())
    }
}

/// Extension trait for `std::time::Duration` providing sub-millisecond precision helpers.
pub trait DurationPrecisionExt {
    /// Returns elapsed time in fractional microseconds (e.g. 12.345 µs).
    fn as_micros_f64(&self) -> f64;
    /// Returns elapsed time in fractional milliseconds (e.g. 0.042 ms instead of 0 ms).
    fn as_millis_f64(&self) -> f64;
    /// Returns a formatted human-readable string with automatic unit scaling (ns, µs, ms, s).
    fn format_human(&self) -> String;
}

impl DurationPrecisionExt for Duration {
    #[inline]
    fn as_micros_f64(&self) -> f64 {
        self.as_nanos() as f64 / 1_000.0
    }

    #[inline]
    fn as_millis_f64(&self) -> f64 {
        self.as_nanos() as f64 / 1_000_000.0
    }

    #[inline]
    fn format_human(&self) -> String {
        format_duration_human(*self)
    }
}

/// High-resolution stopwatch for measuring sub-millisecond graph and computational operations.
#[derive(Debug, Clone, Copy)]
pub struct PrecisionTimer {
    start_instant: Instant,
}

impl Default for PrecisionTimer {
    fn default() -> Self {
        Self::now()
    }
}

impl PrecisionTimer {
    /// Starts a new precision timer at the current instant.
    #[inline]
    pub fn now() -> Self {
        Self {
            start_instant: Instant::now(),
        }
    }

    /// Resets the timer to the current instant and returns the elapsed duration.
    #[inline]
    pub fn restart(&mut self) -> Duration {
        let elapsed = self.start_instant.elapsed();
        self.start_instant = Instant::now();
        elapsed
    }

    /// Returns elapsed duration.
    #[inline]
    pub fn elapsed(&self) -> Duration {
        self.start_instant.elapsed()
    }

    /// Returns elapsed time in integer nanoseconds (10⁻⁹ seconds).
    #[inline]
    pub fn elapsed_nanos(&self) -> u64 {
        self.start_instant.elapsed().as_nanos() as u64
    }

    /// Returns elapsed time in fractional microseconds (e.g. 1.250 µs).
    #[inline]
    pub fn elapsed_micros_f64(&self) -> f64 {
        self.start_instant.elapsed().as_micros_f64()
    }

    /// Returns elapsed time in fractional milliseconds (e.g. 0.015 ms instead of 0).
    #[inline]
    pub fn elapsed_millis_f64(&self) -> f64 {
        self.start_instant.elapsed().as_millis_f64()
    }

    /// Formats the elapsed time as a readable string (e.g. "340 ns", "12.45 µs").
    #[inline]
    pub fn format_elapsed(&self) -> String {
        self.start_instant.elapsed().format_human()
    }

    /// Runs a closure over N iterations to measure throughput without zero-ms truncation.
    pub fn benchmark_fn<F: FnMut()>(iterations: usize, mut f: F) -> BenchmarkResult {
        let iterations = iterations.max(1);
        let timer = Self::now();
        for _ in 0..iterations {
            f();
        }
        let total_duration = timer.elapsed();
        let total_nanos = total_duration.as_nanos() as f64;
        let nanos_per_op = total_nanos / (iterations as f64);
        let ops_per_sec = (iterations as f64) / total_duration.as_secs_f64().max(1e-9);

        BenchmarkResult {
            iterations,
            total_duration,
            nanos_per_op,
            ops_per_sec,
        }
    }
}

/// High-precision benchmark summary.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkResult {
    pub iterations: usize,
    pub total_duration: Duration,
    pub nanos_per_op: f64,
    pub ops_per_sec: f64,
}

impl BenchmarkResult {
    pub fn to_report_string(&self) -> String {
        format!(
            "Iterations: {} | Total: {} | Speed: {:.2} ns/op ({:.2} M-ops/s)",
            self.iterations,
            self.total_duration.format_human(),
            self.nanos_per_op,
            self.ops_per_sec / 1_000_000.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration_human() {
        assert_eq!(format_duration_human(Duration::from_nanos(450)), "450 ns");
        assert_eq!(format_duration_human(Duration::from_nanos(15_500)), "15.500 µs");
        assert_eq!(format_duration_human(Duration::from_nanos(2_500_000)), "2.500 ms");
        assert_eq!(format_duration_human(Duration::from_millis(1500)), "1.500 s");
    }

    #[test]
    fn test_sub_millisecond_fractional_millis() {
        let d = Duration::from_micros(25); // 25 µs = 0.025 ms
        assert_eq!(d.as_millis(), 0); // Demonstrates the original truncation bug
        assert!((d.as_millis_f64() - 0.025).abs() < 1e-6);
        assert!((d.as_micros_f64() - 25.0).abs() < 1e-6);
    }

    #[test]
    fn test_benchmark_runner() {
        let mut x = 0u64;
        let result = PrecisionTimer::benchmark_fn(10_000, || {
            x = x.wrapping_add(1);
        });
        assert_eq!(result.iterations, 10_000);
        assert!(result.nanos_per_op > 0.0);
        assert!(result.ops_per_sec > 0.0);
    }
}
