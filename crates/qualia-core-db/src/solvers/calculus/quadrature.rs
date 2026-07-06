//! Adaptive scalar quadrature with bounded, non-recursive work stacks.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuadratureError {
    InvalidDomain,
    NonFiniteIntegrand { x: f64 },
    EvaluationBudgetExceeded,
    WorkspaceExceeded,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadratureResult {
    pub value: f64,
    pub absolute_error: f64,
    pub evaluations: u32,
    pub intervals: u32,
}

#[derive(Clone, Copy, Default)]
struct SimpsonPanel {
    a: f64,
    b: f64,
    fa: f64,
    fm: f64,
    fb: f64,
    whole: f64,
    tolerance: f64,
}

const MAX_ADAPTIVE_PANELS: usize = 512;

fn checked_eval<F>(function: &F, x: f64) -> Result<f64, QuadratureError>
where
    F: Fn(f64) -> f64,
{
    let value = function(x);
    if value.is_finite() {
        Ok(value)
    } else {
        Err(QuadratureError::NonFiniteIntegrand { x })
    }
}

pub fn adaptive_simpson<F>(
    function: F,
    a: f64,
    b: f64,
    absolute_tolerance: f64,
    max_evaluations: u32,
) -> Result<QuadratureResult, QuadratureError>
where
    F: Fn(f64) -> f64,
{
    if !a.is_finite()
        || !b.is_finite()
        || b <= a
        || !absolute_tolerance.is_finite()
        || absolute_tolerance <= 0.0
        || max_evaluations < 3
    {
        return Err(QuadratureError::InvalidDomain);
    }

    let midpoint = 0.5 * (a + b);
    let fa = checked_eval(&function, a)?;
    let fm = checked_eval(&function, midpoint)?;
    let fb = checked_eval(&function, b)?;
    let whole = (b - a) * (fa + 4.0 * fm + fb) / 6.0;
    let mut evaluations = 3_u32;
    let mut stack = [SimpsonPanel::default(); MAX_ADAPTIVE_PANELS];
    stack[0] = SimpsonPanel {
        a,
        b,
        fa,
        fm,
        fb,
        whole,
        tolerance: absolute_tolerance,
    };
    let mut stack_len = 1;
    let mut value = 0.0;
    let mut error = 0.0;
    let mut intervals = 0;

    while stack_len > 0 {
        stack_len -= 1;
        let panel = stack[stack_len];
        if evaluations + 2 > max_evaluations {
            return Err(QuadratureError::EvaluationBudgetExceeded);
        }
        let midpoint = 0.5 * (panel.a + panel.b);
        let left_midpoint = 0.5 * (panel.a + midpoint);
        let right_midpoint = 0.5 * (midpoint + panel.b);
        let flm = checked_eval(&function, left_midpoint)?;
        let frm = checked_eval(&function, right_midpoint)?;
        evaluations += 2;

        let left = (midpoint - panel.a) * (panel.fa + 4.0 * flm + panel.fm) / 6.0;
        let right = (panel.b - midpoint) * (panel.fm + 4.0 * frm + panel.fb) / 6.0;
        let delta = left + right - panel.whole;
        let local_error = delta.abs() / 15.0;
        if local_error <= panel.tolerance {
            value += left + right + delta / 15.0;
            error += local_error;
            intervals += 2;
            continue;
        }

        if stack_len + 2 > stack.len() {
            return Err(QuadratureError::WorkspaceExceeded);
        }
        let child_tolerance = panel.tolerance * 0.5;
        stack[stack_len] = SimpsonPanel {
            a: midpoint,
            b: panel.b,
            fa: panel.fm,
            fm: frm,
            fb: panel.fb,
            whole: right,
            tolerance: child_tolerance,
        };
        stack[stack_len + 1] = SimpsonPanel {
            a: panel.a,
            b: midpoint,
            fa: panel.fa,
            fm: flm,
            fb: panel.fm,
            whole: left,
            tolerance: child_tolerance,
        };
        stack_len += 2;
    }

    Ok(QuadratureResult {
        value,
        absolute_error: error,
        evaluations,
        intervals,
    })
}

const GK15_ABSCISSAE: [f64; 8] = [
    0.991_455_371_120_812_6,
    0.949_107_912_342_758_5,
    0.864_864_423_359_769_1,
    0.741_531_185_599_394_5,
    0.586_087_235_467_691_1,
    0.405_845_151_377_397_2,
    0.207_784_955_007_898_48,
    0.0,
];
const GK15_WEIGHTS: [f64; 8] = [
    0.022_935_322_010_529_224,
    0.063_092_092_629_978_55,
    0.104_790_010_322_250_19,
    0.140_653_259_715_525_92,
    0.169_004_726_639_267_9,
    0.190_350_578_064_785_42,
    0.204_432_940_075_298_89,
    0.209_482_141_084_727_82,
];
const G7_WEIGHTS: [f64; 4] = [
    0.129_484_966_168_869_7,
    0.279_705_391_489_276_64,
    0.381_830_050_505_118_9,
    0.417_959_183_673_469_4,
];

#[derive(Clone, Copy, Default)]
struct IntervalPanel {
    a: f64,
    b: f64,
    tolerance: f64,
}

fn gauss_kronrod_15_panel<F>(
    function: &F,
    a: f64,
    b: f64,
) -> Result<(f64, f64, u32), QuadratureError>
where
    F: Fn(f64) -> f64,
{
    let center = 0.5 * (a + b);
    let half = 0.5 * (b - a);
    let center_value = checked_eval(function, center)?;
    let mut kronrod = GK15_WEIGHTS[7] * center_value;
    let mut gauss = G7_WEIGHTS[3] * center_value;

    for index in 0..7 {
        let offset = half * GK15_ABSCISSAE[index];
        let pair =
            checked_eval(function, center - offset)? + checked_eval(function, center + offset)?;
        kronrod += GK15_WEIGHTS[index] * pair;
        match index {
            1 => gauss += G7_WEIGHTS[0] * pair,
            3 => gauss += G7_WEIGHTS[1] * pair,
            5 => gauss += G7_WEIGHTS[2] * pair,
            _ => {}
        }
    }
    let kronrod = kronrod * half;
    let gauss = gauss * half;
    Ok((kronrod, (kronrod - gauss).abs(), 15))
}

pub fn adaptive_gauss_kronrod_15<F>(
    function: F,
    a: f64,
    b: f64,
    absolute_tolerance: f64,
    max_evaluations: u32,
) -> Result<QuadratureResult, QuadratureError>
where
    F: Fn(f64) -> f64,
{
    if !a.is_finite()
        || !b.is_finite()
        || b <= a
        || !absolute_tolerance.is_finite()
        || absolute_tolerance <= 0.0
        || max_evaluations < 15
    {
        return Err(QuadratureError::InvalidDomain);
    }

    let mut stack = [IntervalPanel::default(); MAX_ADAPTIVE_PANELS];
    stack[0] = IntervalPanel {
        a,
        b,
        tolerance: absolute_tolerance,
    };
    let mut stack_len = 1;
    let mut evaluations = 0;
    let mut intervals = 0;
    let mut value = 0.0;
    let mut error = 0.0;

    while stack_len > 0 {
        stack_len -= 1;
        let panel = stack[stack_len];
        if evaluations + 15 > max_evaluations {
            return Err(QuadratureError::EvaluationBudgetExceeded);
        }
        let (estimate, local_error, used) = gauss_kronrod_15_panel(&function, panel.a, panel.b)?;
        evaluations += used;
        if local_error <= panel.tolerance {
            value += estimate;
            error += local_error;
            intervals += 1;
            continue;
        }
        if stack_len + 2 > stack.len() {
            return Err(QuadratureError::WorkspaceExceeded);
        }
        let midpoint = 0.5 * (panel.a + panel.b);
        let tolerance = panel.tolerance * 0.5;
        stack[stack_len] = IntervalPanel {
            a: midpoint,
            b: panel.b,
            tolerance,
        };
        stack[stack_len + 1] = IntervalPanel {
            a: panel.a,
            b: midpoint,
            tolerance,
        };
        stack_len += 2;
    }

    Ok(QuadratureResult {
        value,
        absolute_error: error,
        evaluations,
        intervals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_simpson_meets_tolerance_and_reports_budget_failure() {
        let result =
            adaptive_simpson(|x| x.sin(), 0.0, core::f64::consts::PI, 1e-12, 10_000).unwrap();
        assert!((result.value - 2.0).abs() < 1e-12);
        assert!(result.absolute_error < 1e-12);
        assert_eq!(
            adaptive_simpson(|x| x.sin(), 0.0, 1.0, 1e-14, 3),
            Err(QuadratureError::EvaluationBudgetExceeded)
        );
    }

    #[test]
    fn gauss_kronrod_integrates_polynomial_and_oscillation() {
        let polynomial =
            adaptive_gauss_kronrod_15(|x| x.powi(12), 0.0, 1.0, 1e-13, 10_000).unwrap();
        assert!((polynomial.value - 1.0 / 13.0).abs() < 1e-13);

        let oscillatory =
            adaptive_gauss_kronrod_15(|x| (50.0 * x).sin(), 0.0, 1.0, 1e-10, 50_000).unwrap();
        let expected = (1.0 - 50.0_f64.cos()) / 50.0;
        assert!((oscillatory.value - expected).abs() < 1e-10);
    }

    #[test]
    fn quadrature_rejects_reversed_and_non_finite_domains() {
        assert_eq!(
            adaptive_simpson(|x| x, 1.0, 0.0, 1e-6, 100),
            Err(QuadratureError::InvalidDomain)
        );
        assert!(matches!(
            adaptive_simpson(|x| if x > 0.4 { f64::NAN } else { x }, 0.0, 1.0, 1e-6, 100),
            Err(QuadratureError::NonFiniteIntegrand { .. })
        ));
    }
}
