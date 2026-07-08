//! Welfare economics primitives: social welfare functions, inequality and
//! poverty metrics, cost-benefit analysis with distributional weights, and a
//! needs/survival-floor allocation model.
//!
//! # Allocation class
//!
//! All kernels here are `AllocationClass::HotZeroHeap`. Sorting scratch uses
//! fixed-capacity stack arrays (`[f64; MAX_POPULATION]`) or caller-provided
//! output buffers — never `Vec`, `String`, or `Box`. No allocation occurs on
//! any path, hot or cold, within this module.
//!
//! # Rights-affecting use
//!
//! Poverty metrics, distributional weights, and the survival-floor allocation
//! model can inform decisions that affect entitlements, transfers, or access.
//! Those uses MUST be paired with SHACL/deontic checks (see `deontic_logic.rs`
//! opcodes `OP_OBLIGATE`/`OP_FORBID`/`OP_PERMIT`) and capacity-modalities
//! review before any UI exposure or downstream action. The `repr(C)` report
//! structs returned by rights-affecting functions carry diagnostics and
//! assumptions, not just a scalar, so callers can audit the basis of a
//! computed allocation. No function in this module performs or recommends an
//! external action on its own.

/// Maximum population size supported by stack-array scratch buffers.
pub const MAX_POPULATION: usize = 256;

/// Welfare kernel error vocabulary.
///
/// `repr(u8)` for ABI-stable reporting across WASM / edge / Webizen dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WelfareError {
    /// Input failed validation (bad dimensions, negative where non-negative
    /// is required, out-of-range parameter, mismatched lengths, etc.).
    InvalidInput = 0,
    /// Population or series is empty where a non-empty population is required.
    InsufficientData = 1,
    /// A non-finite value (NaN/inf) was encountered in input or during
    /// computation.
    NonFinite = 2,
    /// A caller-supplied output buffer was too small for the request.
    BufferTooSmall = 3,
}

impl WelfareError {
    /// True when the failure is a caller-side buffer/dimension problem.
    #[inline]
    pub fn is_caller_error(self) -> bool {
        matches!(
            self,
            WelfareError::InvalidInput | WelfareError::BufferTooSmall | WelfareError::InsufficientData
        )
    }
}

/// Report returned by rights-affecting welfare kernels.
///
/// `repr(C)` so it can cross the WASM / edge / GPU ABI as a fixed record and
/// be audited by deontic / SHACL layers before any UI exposure. The scalar
/// `value` is never the whole story: `assumptions` and `diagnostics` carry the
/// basis of the computation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct WelfareReport {
    /// The primary scalar result (count, ratio, allocated amount, etc.).
    /// Semantics are documented at each call site.
    pub value: f64,
    /// Secondary scalar (e.g. population size as f64, or a companion metric).
    pub auxiliary: f64,
    /// Bit-packed assumption flags (see `ASSUMPTION_*` constants).
    pub assumptions: u32,
    /// Diagnostic status code (0 = clean; non-zero mirrors `WelfareError` as
    /// u8 plus kernel-specific high bits reserved for future use).
    pub diagnostics: u32,
}

/// Assumption flag: at least one observation was clamped to the survival floor.
pub const ASSUMPTION_FLOOR_CLAMPED: u32 = 1 << 0;
/// Assumption flag: distributional weights were applied (CBA only).
pub const ASSUMPTION_WEIGHTED: u32 = 1 << 1;
/// Assumption flag: result is degenerate (e.g. zero-mean population for Gini).
pub const ASSUMPTION_DEGENERATE: u32 = 1 << 2;
/// Assumption flag: poverty line is above the maximum observed income.
pub const ASSUMPTION_LINE_ABOVE_MAX: u32 = 1 << 3;

impl WelfareReport {
    /// Construct a clean report with no assumption flags.
    pub const fn clean(value: f64, auxiliary: f64) -> Self {
        Self {
            value,
            auxiliary,
            assumptions: 0,
            diagnostics: 0,
        }
    }

    /// Construct a report with assumption flags set.
    pub const fn with_assumptions(value: f64, auxiliary: f64, assumptions: u32) -> Self {
        Self {
            value,
            auxiliary,
            assumptions,
            diagnostics: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

#[inline]
fn finite(x: f64) -> bool {
    x.is_finite()
}

#[inline]
fn finite_nonnegative(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

#[inline]
fn finite_strictly_positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

/// Validate a population slice: non-empty, within capacity, all finite and
/// non-negative. Returns the population count on success.
fn validate_population(incomes: &[f64]) -> Result<usize, WelfareError> {
    if incomes.is_empty() {
        return Err(WelfareError::InsufficientData);
    }
    if incomes.len() > MAX_POPULATION {
        return Err(WelfareError::BufferTooSmall);
    }
    for &x in incomes {
        if !finite_nonnegative(x) {
            if !finite(x) {
                return Err(WelfareError::NonFinite);
            }
            return Err(WelfareError::InvalidInput);
        }
    }
    Ok(incomes.len())
}

/// Validate a population slice where strictly positive values are required
/// (e.g. geometric-mean based Atkinson index).
fn validate_positive_population(incomes: &[f64]) -> Result<usize, WelfareError> {
    if incomes.is_empty() {
        return Err(WelfareError::InsufficientData);
    }
    if incomes.len() > MAX_POPULATION {
        return Err(WelfareError::BufferTooSmall);
    }
    for &x in incomes {
        if !finite(x) {
            return Err(WelfareError::NonFinite);
        }
        if x <= 0.0 {
            return Err(WelfareError::InvalidInput);
        }
    }
    Ok(incomes.len())
}

/// Copy `incomes` into a stack scratch array and sort ascending. Returns the
/// count (== `incomes.len()`). Zero-heap: scratch is a fixed `[f64; MAX_POPULATION]`.
fn copy_sort_ascending(incomes: &[f64]) -> Result<[f64; MAX_POPULATION], WelfareError> {
    let n = validate_population(incomes)?;
    let mut scratch = [0.0f64; MAX_POPULATION];
    for (i, &x) in incomes.iter().enumerate() {
        scratch[i] = x;
    }
    scratch[..n].sort_by(|a, b| a.total_cmp(b));
    Ok(scratch)
}

// ---------------------------------------------------------------------------
// Inequality metrics
// ---------------------------------------------------------------------------

/// Gini coefficient in `[0, 1]` via the mean-absolute-difference formula:
///
/// `G = sum_i sum_j |x_i - x_j| / (2 * n^2 * mean)`
///
/// Returns `WelfareError::InvalidInput` when the mean is zero (degenerate
/// population of all zeros) since the Gini is undefined there. Inputs must be
/// non-negative and finite; an empty population returns `InsufficientData`.
///
/// Allocation class: `HotZeroHeap`. The O(n^2) mean-absolute-difference sum
/// is computed in place over the input slice — no sort is needed for this
/// formula, so no scratch is used.
pub fn gini_coefficient(incomes: &[f64]) -> Result<f64, WelfareError> {
    let n = validate_population(incomes)?;
    let mut sum: f64 = 0.0;
    let mut total: f64 = 0.0;
    for i in 0..n {
        total += incomes[i];
        for j in 0..n {
            let d = incomes[i] - incomes[j];
            sum += d.abs();
        }
    }
    if total <= 0.0 {
        // All-zero (or all-degenerate) population: Gini undefined.
        return Err(WelfareError::InvalidInput);
    }
    let mean = total / n as f64;
    let denom = 2.0 * (n as f64).powi(2) * mean;
    Ok(sum / denom)
}

/// Write the Lorenz curve as interleaved `(cumulative_population_share,
/// cumulative_income_share)` pairs into `out`: `[pop0, inc0, pop1, inc1, ...]`,
/// sorted ascending by income. Returns the number of points written.
///
/// Each point corresponds to one observation (no binning), so `out` must hold
/// at least `2 * n` `f64` slots. The first point is `(1/n, x_min/total)` and
/// the last is `(1, 1)`.
///
/// Allocation class: `HotZeroHeap`. Sorting uses a fixed `[f64; MAX_POPULATION]`
/// stack scratch array.
pub fn lorenz_curve_into(incomes: &[f64], out: &mut [f64]) -> Result<usize, WelfareError> {
    let n = validate_population(incomes)?;
    if out.len() < 2 * n {
        return Err(WelfareError::BufferTooSmall);
    }
    let sorted = copy_sort_ascending(incomes)?;
    let mut total = 0.0f64;
    for i in 0..n {
        total += sorted[i];
    }
    if total <= 0.0 {
        return Err(WelfareError::InvalidInput);
    }
    let mut cum = 0.0f64;
    for i in 0..n {
        cum += sorted[i];
        out[2 * i] = (i + 1) as f64 / n as f64;
        out[2 * i + 1] = cum / total;
    }
    Ok(n)
}

/// Atkinson inequality index with inequality-aversion parameter `epsilon > 0`.
///
/// - For `epsilon != 1`: `A = 1 - (geometric_mean / arithmetic_mean)^(1-epsilon)`.
/// - For `epsilon == 1`: `A = 1 - geometric_mean / arithmetic_mean`.
///
/// All incomes must be strictly positive (the geometric mean is undefined for
/// zero/negative values). `epsilon` must be finite and `> 0`. Returns a value
/// in `[0, 1)`: `0` under perfect equality, increasing toward `1` as
/// inequality rises and as `epsilon` (inequality aversion) rises.
///
/// Allocation class: `HotZeroHeap`.
pub fn atkinson_inequality(incomes: &[f64], epsilon: f64) -> Result<f64, WelfareError> {
    let n = validate_positive_population(incomes)?;
    if !finite(epsilon) || epsilon <= 0.0 {
        return Err(WelfareError::InvalidInput);
    }

    let mut sum = 0.0f64;
    for i in 0..n {
        if incomes[i] <= 0.0 {
            return Err(WelfareError::InvalidInput);
        }
        sum += incomes[i];
    }
    let arithmetic_mean = sum / n as f64;
    if arithmetic_mean <= 0.0 {
        return Err(WelfareError::InvalidInput);
    }

    if (epsilon - 1.0).abs() < f64::EPSILON {
        // Limit case: 1 - geo / arith
        let mut log_sum = 0.0f64;
        for i in 0..n {
            log_sum += incomes[i].ln();
        }
        let geo = (log_sum / n as f64).exp();
        let a = 1.0 - (geo / arithmetic_mean);
        return Ok(a.clamp(0.0, 1.0));
    }

    // General case: 1 - ( power mean of order (1-eps) / arith )
    let one_minus_eps = 1.0 - epsilon;
    let mut sum_pow = 0.0f64;
    for i in 0..n {
        sum_pow += incomes[i].powf(one_minus_eps);
    }
    let mean_pow = sum_pow / n as f64;
    let power_mean = if one_minus_eps.abs() > 1e-12 {
        mean_pow.powf(1.0 / one_minus_eps)
    } else {
        arithmetic_mean
    };
    let a = 1.0 - (power_mean / arithmetic_mean);
    if !a.is_finite() {
        return Err(WelfareError::NonFinite);
    }
    Ok(a.clamp(0.0, 1.0))
}

/// Headcount poverty: returns `(count_poor, headcount_ratio)` where
/// `count_poor` is the number of observations strictly below `poverty_line`
/// and `headcount_ratio = count_poor / n`.
///
/// `poverty_line` must be finite and strictly positive. Incomes must be
/// non-negative and finite.
///
/// Allocation class: `HotZeroHeap`.
pub fn headcount_poverty(incomes: &[f64], poverty_line: f64) -> Result<(usize, f64), WelfareError> {
    let n = validate_population(incomes)?;
    if !finite_strictly_positive(poverty_line) {
        return Err(WelfareError::InvalidInput);
    }
    let mut count = 0usize;
    for &x in incomes {
        if x < poverty_line {
            count += 1;
        }
    }
    let ratio = count as f64 / n as f64;
    Ok((count, ratio))
}

/// Poverty gap ratio: `sum(max(0, line - income)) / (n * line)`.
///
/// Returns a value in `[0, 1]`: `0` when nobody is below the line, `1` when
/// every observation is zero. `poverty_line` must be finite and strictly
/// positive. Incomes must be non-negative and finite.
///
/// Allocation class: `HotZeroHeap`.
pub fn poverty_gap_ratio(incomes: &[f64], poverty_line: f64) -> Result<f64, WelfareError> {
    let n = validate_population(incomes)?;
    if !finite_strictly_positive(poverty_line) {
        return Err(WelfareError::InvalidInput);
    }
    let mut gap = 0.0f64;
    for &x in incomes {
        if x < poverty_line {
            gap += poverty_line - x;
        }
    }
    Ok(gap / (n as f64 * poverty_line))
}

// ---------------------------------------------------------------------------
// Social welfare functions
// ---------------------------------------------------------------------------

/// Utilitarian (sum) social welfare: `sum_i u_i`.
///
/// Utilities must be finite. Negative utilities are permitted (welfare can be
/// negative). An empty population returns `InsufficientData`.
///
/// Allocation class: `HotZeroHeap`.
pub fn utilitarian_welfare(utilities: &[f64]) -> Result<f64, WelfareError> {
    if utilities.is_empty() {
        return Err(WelfareError::InsufficientData);
    }
    if utilities.len() > MAX_POPULATION {
        return Err(WelfareError::BufferTooSmall);
    }
    let mut sum = 0.0f64;
    for &u in utilities {
        if !finite(u) {
            return Err(WelfareError::NonFinite);
        }
        sum += u;
    }
    Ok(sum)
}

/// Rawlsian (minimax / maximin) social welfare: `min_i u_i`.
///
/// The welfare of a society is the welfare of its worst-off member. Utilities
/// must be finite. An empty population returns `InsufficientData`.
///
/// Allocation class: `HotZeroHeap`.
pub fn rawlsian_welfare(utilities: &[f64]) -> Result<f64, WelfareError> {
    if utilities.is_empty() {
        return Err(WelfareError::InsufficientData);
    }
    if utilities.len() > MAX_POPULATION {
        return Err(WelfareError::BufferTooSmall);
    }
    let mut min = utilities[0];
    if !finite(min) {
        return Err(WelfareError::NonFinite);
    }
    for &u in &utilities[1..] {
        if !finite(u) {
            return Err(WelfareError::NonFinite);
        }
        if u < min {
            min = u;
        }
    }
    Ok(min)
}

/// Nash social welfare: the **product** of utilities, `prod_i u_i`.
///
/// This is the unnormalised Nash product (the sum of logs is the log of the
/// product). It is *not* the geometric mean; divide by `n` externally if the
/// geometric mean is required. All utilities must be strictly positive so the
/// product is well-defined and non-degenerate; a single zero utility would
/// collapse the product to zero and is rejected as `InvalidInput`. An empty
/// population returns `InsufficientData`.
///
/// Allocation class: `HotZeroHeap`.
pub fn nash_welfare(utilities: &[f64]) -> Result<f64, WelfareError> {
    let n = validate_positive_population(utilities)?;
    let mut product = 1.0f64;
    for i in 0..n {
        product *= utilities[i];
    }
    if !product.is_finite() {
        return Err(WelfareError::NonFinite);
    }
    Ok(product)
}

// ---------------------------------------------------------------------------
// Cost-benefit analysis
// ---------------------------------------------------------------------------

/// Net present value: `NPV = sum_{t=0}^{n_periods-1} (B_t - C_t) / (1+r)^t`.
///
/// `benefits` and `costs` must each have length `>= n_periods`; only the first
/// `n_periods` entries are consumed. `discount_rate` must be finite and `> -1`
/// (a rate of `-1` or below makes the discount factor non-positive). The
/// period-0 cash flow is discounted by `(1+r)^0 = 1` (i.e. not discounted).
///
/// `n_periods` must be `> 0` and `<= MAX_POPULATION`.
///
/// Allocation class: `HotZeroHeap`.
pub fn net_present_value(
    benefits: &[f64],
    costs: &[f64],
    discount_rate: f64,
    n_periods: usize,
) -> Result<f64, WelfareError> {
    if n_periods == 0 {
        return Err(WelfareError::InsufficientData);
    }
    if n_periods > MAX_POPULATION {
        return Err(WelfareError::BufferTooSmall);
    }
    if benefits.len() < n_periods || costs.len() < n_periods {
        return Err(WelfareError::InvalidInput);
    }
    if !finite(discount_rate) || discount_rate <= -1.0 {
        return Err(WelfareError::InvalidInput);
    }
    let one_plus_r = 1.0 + discount_rate;
    let mut npv = 0.0f64;
    let mut discount = 1.0f64; // (1+r)^0
    for t in 0..n_periods {
        let b = benefits[t];
        let c = costs[t];
        if !finite(b) || !finite(c) {
            return Err(WelfareError::NonFinite);
        }
        npv += (b - c) * discount;
        discount *= one_plus_r;
    }
    if !npv.is_finite() {
        return Err(WelfareError::NonFinite);
    }
    Ok(npv)
}

/// Distributional NPV: NPV with per-period distributional weights.
///
/// `NPV_w = sum_{t=0}^{n_periods-1} w_t * (B_t - C_t) / (1+r)^t`.
///
/// `weights` must have length `>= n_periods`. Weights must be finite and
/// non-negative (a zero weight simply zeroes that period's contribution).
/// `benefits`, `costs`, `discount_rate`, and `n_periods` follow the same
/// rules as [`net_present_value`].
///
/// Returns a [`WelfareReport`] carrying the `ASSUMPTION_WEIGHTED` flag so
/// downstream deontic / SHACL auditors can see that distributional weights
/// were applied. `value` is the weighted NPV; `auxiliary` is the unweighted
/// NPV for comparison.
///
/// Rights-affecting: distributional weights encode value judgements about
/// whose benefits count how much. Pair with deontic / SHACL review before UI
/// exposure.
///
/// Allocation class: `HotZeroHeap`.
pub fn distributional_npv(
    benefits: &[f64],
    costs: &[f64],
    weights: &[f64],
    discount_rate: f64,
    n_periods: usize,
) -> Result<WelfareReport, WelfareError> {
    if n_periods == 0 {
        return Err(WelfareError::InsufficientData);
    }
    if n_periods > MAX_POPULATION {
        return Err(WelfareError::BufferTooSmall);
    }
    if benefits.len() < n_periods
        || costs.len() < n_periods
        || weights.len() < n_periods
    {
        return Err(WelfareError::InvalidInput);
    }
    if !finite(discount_rate) || discount_rate <= -1.0 {
        return Err(WelfareError::InvalidInput);
    }
    let one_plus_r = 1.0 + discount_rate;
    let mut weighted_npv = 0.0f64;
    let mut unweighted_npv = 0.0f64;
    let mut discount = 1.0f64;
    for t in 0..n_periods {
        let b = benefits[t];
        let c = costs[t];
        let w = weights[t];
        if !finite(b) || !finite(c) || !finite(w) {
            return Err(WelfareError::NonFinite);
        }
        if w < 0.0 {
            return Err(WelfareError::InvalidInput);
        }
        let flow = (b - c) * discount;
        weighted_npv += w * flow;
        unweighted_npv += flow;
        discount /= one_plus_r;
    }
    if !weighted_npv.is_finite() || !unweighted_npv.is_finite() {
        return Err(WelfareError::NonFinite);
    }
    Ok(WelfareReport {
        value: weighted_npv,
        auxiliary: unweighted_npv,
        assumptions: ASSUMPTION_WEIGHTED,
        diagnostics: 0,
    })
}

// ---------------------------------------------------------------------------
// Needs / survival-floor allocation model
// ---------------------------------------------------------------------------

/// Allocate a fixed budget `budget` across `needs` so that each recipient
/// first receives their survival floor `floors[i]`, then the residual is
/// distributed proportionally to surplus need `needs[i] - floors[i]`.
///
/// Composes with deontic and capacity modalities: the returned
/// [`WelfareReport`] carries assumption flags so a downstream deontic / SHACL
/// layer can verify that floors were honoured (`ASSUMPTION_FLOOR_CLAMPED` is
/// set when any allocation was clamped to the floor) and that the budget was
/// sufficient (`ASSUMPTION_DEGENERATE` is set when the budget could not cover
/// all floors, in which case floors are scaled proportionally).
///
/// `value` is the total actually allocated; `auxiliary` is the residual after
/// floors (the amount distributed proportionally). Results are written into
/// the caller-owned `out` buffer (`out.len() >= n`).
///
/// Rights-affecting: this model can determine access to subsistence resources.
/// Pair with `OP_OBLIGATE`/`OP_FORBID` deontic checks and capacity-modalities
/// review before any UI exposure or downstream transfer.
///
/// Allocation class: `HotZeroHeap`.
pub fn survival_floor_allocation_into(
    needs: &[f64],
    floors: &[f64],
    budget: f64,
    out: &mut [f64],
) -> Result<WelfareReport, WelfareError> {
    if needs.is_empty() {
        return Err(WelfareError::InsufficientData);
    }
    let n = needs.len();
    if n > MAX_POPULATION {
        return Err(WelfareError::BufferTooSmall);
    }
    if floors.len() != n || out.len() < n {
        return Err(WelfareError::InvalidInput);
    }
    if !finite(budget) || budget < 0.0 {
        return Err(WelfareError::InvalidInput);
    }

    let mut total_floor = 0.0f64;
    let mut total_surplus_need = 0.0f64;
    for i in 0..n {
        if !finite(needs[i]) || !finite(floors[i]) {
            return Err(WelfareError::NonFinite);
        }
        if needs[i] < 0.0 || floors[i] < 0.0 {
            return Err(WelfareError::InvalidInput);
        }
        if floors[i] > needs[i] {
            // Floor cannot exceed total need.
            return Err(WelfareError::InvalidInput);
        }
        total_floor += floors[i];
        total_surplus_need += needs[i] - floors[i];
    }

    let mut assumptions: u32 = 0;
    let mut total_allocated = 0.0f64;
    let mut residual_distributed = 0.0f64;

    if budget >= total_floor {
        // Floors fully covered; distribute residual proportionally to surplus need.
        let residual = budget - total_floor;
        for i in 0..n {
            let base = floors[i];
            let extra = if total_surplus_need > 0.0 {
                residual * (needs[i] - floors[i]) / total_surplus_need
            } else {
                0.0
            };
            let alloc = base + extra;
            out[i] = alloc;
            total_allocated += alloc;
            residual_distributed += extra;
        }
    } else {
        // Insufficient budget: scale floors proportionally so no recipient is
        // arbitrarily zeroed out. This is a degenerate (under-budget) case.
        assumptions |= ASSUMPTION_DEGENERATE;
        let scale = if total_floor > 0.0 {
            budget / total_floor
        } else {
            0.0
        };
        for i in 0..n {
            let alloc = floors[i] * scale;
            out[i] = alloc;
            total_allocated += alloc;
        }
        assumptions |= ASSUMPTION_FLOOR_CLAMPED;
    }

    if !total_allocated.is_finite() || !residual_distributed.is_finite() {
        return Err(WelfareError::NonFinite);
    }

    Ok(WelfareReport {
        value: total_allocated,
        auxiliary: residual_distributed,
        assumptions,
        diagnostics: 0,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    // --- Gini ---------------------------------------------------------------

    #[test]
    fn gini_perfect_equality_is_zero() {
        let incomes = [10.0, 10.0, 10.0];
        let g = gini_coefficient(&incomes).unwrap();
        assert!(approx(g, 0.0));
    }

    #[test]
    fn gini_extreme_inequality_approaches_one_minus_one_over_n() {
        // [0, 0, 100]: G = 1 - 1/n = 2/3 for n = 3.
        let incomes = [0.0, 0.0, 100.0];
        let g = gini_coefficient(&incomes).unwrap();
        let expected = 1.0 - 1.0 / 3.0;
        assert!(approx(g, expected));
    }

    #[test]
    fn gini_two_person_split_is_one_half() {
        // [0, 1]: G = 1 - 1/2 = 0.5.
        let incomes = [0.0, 1.0];
        let g = gini_coefficient(&incomes).unwrap();
        assert!(approx(g, 0.5));
    }

    #[test]
    fn gini_empty_is_insufficient_data() {
        assert_eq!(gini_coefficient(&[]), Err(WelfareError::InsufficientData));
    }

    #[test]
    fn gini_all_zero_is_invalid() {
        assert_eq!(gini_coefficient(&[0.0, 0.0]), Err(WelfareError::InvalidInput));
    }

    #[test]
    fn gini_nan_is_non_finite() {
        assert_eq!(
            gini_coefficient(&[1.0, f64::NAN]),
            Err(WelfareError::NonFinite)
        );
    }

    #[test]
    fn gini_negative_is_invalid() {
        assert_eq!(
            gini_coefficient(&[1.0, -1.0]),
            Err(WelfareError::InvalidInput)
        );
    }

    // --- Lorenz -------------------------------------------------------------

    #[test]
    fn lorenz_curve_cumulative_shares() {
        let incomes = [1.0, 2.0, 3.0];
        let mut out = [0.0f64; 6];
        let n = lorenz_curve_into(&incomes, &mut out).unwrap();
        assert_eq!(n, 3);
        // Sorted: [1, 2, 3], total = 6.
        // Points: (1/3, 1/6), (2/3, 3/6=1/2), (3/3=1, 6/6=1).
        assert!(approx(out[0], 1.0 / 3.0));
        assert!(approx(out[1], 1.0 / 6.0));
        assert!(approx(out[2], 2.0 / 3.0));
        assert!(approx(out[3], 0.5));
        assert!(approx(out[4], 1.0));
        assert!(approx(out[5], 1.0));
    }

    #[test]
    fn lorenz_buffer_too_small() {
        let incomes = [1.0, 2.0, 3.0];
        let mut out = [0.0f64; 5]; // need 6
        assert_eq!(
            lorenz_curve_into(&incomes, &mut out),
            Err(WelfareError::BufferTooSmall)
        );
    }

    #[test]
    fn lorenz_empty_is_insufficient_data() {
        let mut out = [0.0f64; 2];
        assert_eq!(
            lorenz_curve_into(&[], &mut out),
            Err(WelfareError::InsufficientData)
        );
    }

    // --- Atkinson -----------------------------------------------------------

    #[test]
    fn atkinson_equality_is_zero() {
        let incomes = [10.0, 10.0, 10.0];
        let a = atkinson_inequality(&incomes, 0.5).unwrap();
        assert!(approx(a, 0.0));
    }

    #[test]
    fn atkinson_higher_epsilon_is_more_inequality_averse() {
        let incomes = [1.0, 2.0, 10.0];
        let a_low = atkinson_inequality(&incomes, 0.5).unwrap();
        let a_high = atkinson_inequality(&incomes, 2.0).unwrap();
        assert!(a_high > a_low, "higher epsilon must yield higher A");
    }

    #[test]
    fn atkinson_epsilon_one_uses_log_form() {
        let incomes = [1.0, 2.0, 4.0];
        let a = atkinson_inequality(&incomes, 1.0).unwrap();
        // geometric mean = (1*2*4)^(1/3) = 8^(1/3) = 2
        // arithmetic mean = 7/3
        // A = 1 - 2 / (7/3) = 1 - 6/7 = 1/7
        assert!(approx(a, 1.0 / 7.0));
    }

    #[test]
    fn atkinson_zero_income_is_invalid() {
        assert_eq!(
            atkinson_inequality(&[0.0, 1.0], 0.5),
            Err(WelfareError::InvalidInput)
        );
    }

    #[test]
    fn atkinson_nonpositive_epsilon_is_invalid() {
        assert_eq!(
            atkinson_inequality(&[1.0, 2.0], 0.0),
            Err(WelfareError::InvalidInput)
        );
        assert_eq!(
            atkinson_inequality(&[1.0, 2.0], -1.0),
            Err(WelfareError::InvalidInput)
        );
    }

    // --- Headcount ----------------------------------------------------------

    #[test]
    fn headcount_counts_poor_and_ratio() {
        let incomes = [5.0, 15.0, 25.0];
        let (count, ratio) = headcount_poverty(&incomes, 10.0).unwrap();
        assert_eq!(count, 1);
        assert!(approx(ratio, 1.0 / 3.0));
    }

    #[test]
    fn headcount_none_poor() {
        let incomes = [20.0, 30.0];
        let (count, ratio) = headcount_poverty(&incomes, 10.0).unwrap();
        assert_eq!(count, 0);
        assert!(approx(ratio, 0.0));
    }

    #[test]
    fn headcount_line_at_income_excludes_boundary() {
        // Strictly below the line: an income equal to the line is not poor.
        let incomes = [10.0, 20.0];
        let (count, _) = headcount_poverty(&incomes, 10.0).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn headcount_zero_line_is_invalid() {
        assert_eq!(
            headcount_poverty(&[1.0, 2.0], 0.0),
            Err(WelfareError::InvalidInput)
        );
    }

    // --- Poverty gap --------------------------------------------------------

    #[test]
    fn poverty_gap_ratio_basic() {
        let incomes = [5.0, 15.0, 25.0];
        let g = poverty_gap_ratio(&incomes, 10.0).unwrap();
        // gap = 5; n*line = 30; 5/30 = 1/6.
        assert!(approx(g, 1.0 / 6.0));
    }

    #[test]
    fn poverty_gap_none_poor_is_zero() {
        let incomes = [20.0, 30.0];
        let g = poverty_gap_ratio(&incomes, 10.0).unwrap();
        assert!(approx(g, 0.0));
    }

    #[test]
    fn poverty_gap_all_zero_is_one() {
        let incomes = [0.0, 0.0];
        let g = poverty_gap_ratio(&incomes, 10.0).unwrap();
        assert!(approx(g, 1.0));
    }

    // --- Utilitarian / Rawlsian / Nash --------------------------------------

    #[test]
    fn utilitarian_is_sum() {
        let u = [1.0, 2.0, 3.0];
        assert!(approx(utilitarian_welfare(&u).unwrap(), 6.0));
    }

    #[test]
    fn utilitarian_allows_negative() {
        let u = [-1.0, 2.0, 3.0];
        assert!(approx(utilitarian_welfare(&u).unwrap(), 4.0));
    }

    #[test]
    fn rawlsian_is_min() {
        let u = [1.0, 2.0, 3.0];
        assert!(approx(rawlsian_welfare(&u).unwrap(), 1.0));
    }

    #[test]
    fn rawlsian_negative_min() {
        let u = [-5.0, 2.0, 3.0];
        assert!(approx(rawlsian_welfare(&u).unwrap(), -5.0));
    }

    #[test]
    fn nash_is_product() {
        let u = [1.0, 2.0, 3.0];
        assert!(approx(nash_welfare(&u).unwrap(), 6.0));
    }

    #[test]
    fn nash_zero_utility_is_invalid() {
        assert_eq!(nash_welfare(&[0.0, 2.0]), Err(WelfareError::InvalidInput));
    }

    #[test]
    fn nash_negative_utility_is_invalid() {
        assert_eq!(nash_welfare(&[-1.0, 2.0]), Err(WelfareError::InvalidInput));
    }

    #[test]
    fn welfare_empty_is_insufficient_data() {
        assert_eq!(utilitarian_welfare(&[]), Err(WelfareError::InsufficientData));
        assert_eq!(rawlsian_welfare(&[]), Err(WelfareError::InsufficientData));
        assert_eq!(nash_welfare(&[]), Err(WelfareError::InsufficientData));
    }

    #[test]
    fn welfare_nan_is_non_finite() {
        assert_eq!(
            utilitarian_welfare(&[1.0, f64::NAN]),
            Err(WelfareError::NonFinite)
        );
        assert_eq!(
            rawlsian_welfare(&[1.0, f64::NAN]),
            Err(WelfareError::NonFinite)
        );
    }

    // --- NPV ----------------------------------------------------------------

    #[test]
    fn npv_basic_discounting() {
        let benefits = [10.0, 10.0];
        let costs = [5.0, 5.0];
        let r = 0.1;
        let npv = net_present_value(&benefits, &costs, r, 2).unwrap();
        let expected = 5.0 / 1.1 + 5.0 / 1.21;
        assert!(approx(npv, expected));
    }

    #[test]
    fn npv_period_zero_not_discounted() {
        let benefits = [100.0, 0.0];
        let costs = [0.0, 0.0];
        let npv = net_present_value(&benefits, &costs, 0.5, 2).unwrap();
        assert!(approx(npv, 100.0));
    }

    #[test]
    fn npv_zero_rate_is_sum() {
        let benefits = [10.0, 10.0, 10.0];
        let costs = [1.0, 2.0, 3.0];
        let npv = net_present_value(&benefits, &costs, 0.0, 3).unwrap();
        assert!(approx(npv, 24.0));
    }

    #[test]
    fn npv_length_mismatch_is_invalid() {
        let benefits = [10.0];
        let costs = [5.0, 5.0];
        assert_eq!(
            net_present_value(&benefits, &costs, 0.1, 2),
            Err(WelfareError::InvalidInput)
        );
    }

    #[test]
    fn npv_zero_periods_is_insufficient_data() {
        assert_eq!(
            net_present_value(&[1.0], &[1.0], 0.1, 0),
            Err(WelfareError::InsufficientData)
        );
    }

    #[test]
    fn npv_rate_at_minus_one_is_invalid() {
        assert_eq!(
            net_present_value(&[1.0], &[1.0], -1.0, 1),
            Err(WelfareError::InvalidInput)
        );
    }

    // --- Distributional NPV -------------------------------------------------

    #[test]
    fn distributional_npv_applies_weights() {
        let benefits = [10.0, 10.0];
        let costs = [5.0, 5.0];
        let weights = [1.0, 2.0];
        let r = 0.1;
        let report = distributional_npv(&benefits, &costs, &weights, r, 2).unwrap();
        // t=0: *1.0 , t=1: /1.1
        let expected_weighted = 1.0 * 5.0 + 2.0 * 5.0 / 1.1;
        let expected_unweighted = 5.0 + 5.0 / 1.1;
        assert!(approx(report.value, expected_weighted));
        assert!(approx(report.auxiliary, expected_unweighted));
        assert!(report.assumptions & ASSUMPTION_WEIGHTED != 0);
    }

    #[test]
    fn distributional_npv_negative_weight_is_invalid() {
        assert_eq!(
            distributional_npv(&[10.0], &[5.0], &[-1.0], 0.1, 1),
            Err(WelfareError::InvalidInput)
        );
    }

    #[test]
    fn distributional_npv_weights_length_mismatch() {
        assert_eq!(
            distributional_npv(&[10.0, 10.0], &[5.0, 5.0], &[1.0], 0.1, 2),
            Err(WelfareError::InvalidInput)
        );
    }

    // --- Survival-floor allocation ------------------------------------------

    #[test]
    fn allocation_covers_floors_then_distributes_residual() {
        let needs = [10.0, 20.0, 30.0];
        let floors = [4.0, 5.0, 6.0];
        let budget = 25.0; // total_floor = 15, residual = 10
        let mut out = [0.0f64; 3];
        let report = survival_floor_allocation_into(&needs, &floors, budget, &mut out).unwrap();
        // surplus needs: 6, 15, 24 -> total 45
        // extras: 10*6/45, 10*15/45, 10*24/45
        let expected = [4.0 + 10.0 * 6.0 / 45.0, 5.0 + 10.0 * 15.0 / 45.0, 6.0 + 10.0 * 24.0 / 45.0];
        for i in 0..3 {
            assert!(approx(out[i], expected[i]));
        }
        assert!(approx(report.value, budget));
        assert!(approx(report.auxiliary, 10.0));
        assert_eq!(report.assumptions, 0);
    }

    #[test]
    fn allocation_under_budget_scales_floors() {
        let needs = [10.0, 20.0];
        let floors = [4.0, 6.0];
        let budget = 5.0; // total_floor = 10 > budget -> degenerate
        let mut out = [0.0f64; 2];
        let report = survival_floor_allocation_into(&needs, &floors, budget, &mut out).unwrap();
        // scale = 5/10 = 0.5 -> [2, 3]
        assert!(approx(out[0], 2.0));
        assert!(approx(out[1], 3.0));
        assert!(approx(report.value, 5.0));
        assert!(report.assumptions & ASSUMPTION_DEGENERATE != 0);
        assert!(report.assumptions & ASSUMPTION_FLOOR_CLAMPED != 0);
    }

    #[test]
    fn allocation_floor_exceeds_need_is_invalid() {
        assert_eq!(
            survival_floor_allocation_into(&[5.0], &[10.0], 100.0, &mut [0.0]),
            Err(WelfareError::InvalidInput)
        );
    }

    #[test]
    fn allocation_buffer_too_small() {
        let needs = [10.0, 20.0];
        let floors = [4.0, 6.0];
        let mut out = [0.0f64; 1];
        assert_eq!(
            survival_floor_allocation_into(&needs, &floors, 100.0, &mut out),
            Err(WelfareError::InvalidInput)
        );
    }

    #[test]
    fn allocation_empty_is_insufficient_data() {
        assert_eq!(
            survival_floor_allocation_into(&[], &[], 100.0, &mut []),
            Err(WelfareError::InsufficientData)
        );
    }

    // --- Error classification -----------------------------------------------

    #[test]
    fn error_is_caller_error_classification() {
        assert!(WelfareError::InvalidInput.is_caller_error());
        assert!(WelfareError::InsufficientData.is_caller_error());
        assert!(WelfareError::BufferTooSmall.is_caller_error());
        assert!(!WelfareError::NonFinite.is_caller_error());
    }

    #[test]
    fn welfare_report_clean_constructor() {
        let r = WelfareReport::clean(1.0, 2.0);
        assert_eq!(r.value, 1.0);
        assert_eq!(r.auxiliary, 2.0);
        assert_eq!(r.assumptions, 0);
        assert_eq!(r.diagnostics, 0);
    }

    #[test]
    fn welfare_report_with_assumptions_constructor() {
        let r = WelfareReport::with_assumptions(1.0, 2.0, ASSUMPTION_WEIGHTED);
        assert!(r.assumptions & ASSUMPTION_WEIGHTED != 0);
    }

    #[test]
    fn over_capacity_population_is_buffer_too_small() {
        let big = [1.0f64; MAX_POPULATION + 1];
        // Use a small slice that exceeds capacity.
        let slice = &big[..MAX_POPULATION + 1];
        assert_eq!(
            gini_coefficient(slice),
            Err(WelfareError::BufferTooSmall)
        );
    }
}
