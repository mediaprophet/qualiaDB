//! Zero-heap mathematical easing curves and parametric interpolators.
//!
//! Provides standard CSS / Penner easings, cubic Bézier parametric solvers,
//! and relativistic time-warp mappings without heap allocation.

use std::f64::consts::PI;

/// Standard animation curve types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EasingCurve {
    Linear,
    QuadIn,
    QuadOut,
    QuadInOut,
    CubicIn,
    CubicOut,
    CubicInOut,
    QuartIn,
    QuartOut,
    QuartInOut,
    QuintIn,
    QuintOut,
    QuintInOut,
    SineIn,
    SineOut,
    SineInOut,
    ExpoIn,
    ExpoOut,
    ExpoInOut,
    CircIn,
    CircOut,
    CircInOut,
    ElasticIn,
    ElasticOut,
    ElasticInOut,
    BackIn,
    BackOut,
    BackInOut,
    BounceIn,
    BounceOut,
    BounceInOut,
}

impl EasingCurve {
    /// Parse an easing curve from its canonical name (e.g., "ease-in-out", "cubic-in").
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().replace('_', "-").as_str() {
            "linear" => Some(Self::Linear),
            "quad-in" | "ease-in-quad" => Some(Self::QuadIn),
            "quad-out" | "ease-out-quad" => Some(Self::QuadOut),
            "quad-in-out" | "ease-in-out-quad" => Some(Self::QuadInOut),
            "cubic-in" | "ease-in" => Some(Self::CubicIn),
            "cubic-out" | "ease-out" => Some(Self::CubicOut),
            "cubic-in-out" | "ease-in-out" => Some(Self::CubicInOut),
            "quart-in" | "ease-in-quart" => Some(Self::QuartIn),
            "quart-out" | "ease-out-quart" => Some(Self::QuartOut),
            "quart-in-out" | "ease-in-out-quart" => Some(Self::QuartInOut),
            "quint-in" | "ease-in-quint" => Some(Self::QuintIn),
            "quint-out" | "ease-out-quint" => Some(Self::QuintOut),
            "quint-in-out" | "ease-in-out-quint" => Some(Self::QuintInOut),
            "sine-in" | "ease-in-sine" => Some(Self::SineIn),
            "sine-out" | "ease-out-sine" => Some(Self::SineOut),
            "sine-in-out" | "ease-in-out-sine" => Some(Self::SineInOut),
            "expo-in" | "ease-in-expo" => Some(Self::ExpoIn),
            "expo-out" | "ease-out-expo" => Some(Self::ExpoOut),
            "expo-in-out" | "ease-in-out-expo" => Some(Self::ExpoInOut),
            "circ-in" | "ease-in-circ" => Some(Self::CircIn),
            "circ-out" | "ease-out-circ" => Some(Self::CircOut),
            "circ-in-out" | "ease-in-out-circ" => Some(Self::CircInOut),
            "elastic-in" | "ease-in-elastic" => Some(Self::ElasticIn),
            "elastic-out" | "ease-out-elastic" => Some(Self::ElasticOut),
            "elastic-in-out" | "ease-in-out-elastic" => Some(Self::ElasticInOut),
            "back-in" | "ease-in-back" => Some(Self::BackIn),
            "back-out" | "ease-out-back" => Some(Self::BackOut),
            "back-in-out" | "ease-in-out-back" => Some(Self::BackInOut),
            "bounce-in" | "ease-in-bounce" => Some(Self::BounceIn),
            "bounce-out" | "ease-out-bounce" => Some(Self::BounceOut),
            "bounce-in-out" | "ease-in-out-bounce" => Some(Self::BounceInOut),
            _ => None,
        }
    }

    /// Evaluate the curve at normalized progress `t` in [0.0, 1.0].
    #[inline]
    pub fn eval(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::QuadIn => t * t,
            Self::QuadOut => t * (2.0 - t),
            Self::QuadInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Self::CubicIn => t * t * t,
            Self::CubicOut => {
                let f = t - 1.0;
                f * f * f + 1.0
            }
            Self::CubicInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let f = 2.0 * t - 2.0;
                    0.5 * f * f * f + 1.0
                }
            }
            Self::QuartIn => t * t * t * t,
            Self::QuartOut => {
                let f = t - 1.0;
                1.0 - f * f * f * f
            }
            Self::QuartInOut => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    let f = t - 1.0;
                    1.0 - 8.0 * f * f * f * f
                }
            }
            Self::QuintIn => t * t * t * t * t,
            Self::QuintOut => {
                let f = t - 1.0;
                f * f * f * f * f + 1.0
            }
            Self::QuintInOut => {
                if t < 0.5 {
                    16.0 * t * t * t * t * t
                } else {
                    let f = 2.0 * t - 2.0;
                    0.5 * f * f * f * f * f + 1.0
                }
            }
            Self::SineIn => 1.0 - (t * PI * 0.5).cos(),
            Self::SineOut => (t * PI * 0.5).sin(),
            Self::SineInOut => 0.5 * (1.0 - (PI * t).cos()),
            Self::ExpoIn => {
                if t == 0.0 {
                    0.0
                } else {
                    (2.0f64).powf(10.0 * (t - 1.0))
                }
            }
            Self::ExpoOut => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - (2.0f64).powf(-10.0 * t)
                }
            }
            Self::ExpoInOut => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    0.5 * (2.0f64).powf(20.0 * t - 10.0)
                } else {
                    1.0 - 0.5 * (2.0f64).powf(-20.0 * t + 10.0)
                }
            }
            Self::CircIn => 1.0 - (1.0 - t * t).sqrt(),
            Self::CircOut => (1.0 - (t - 1.0).powi(2)).sqrt(),
            Self::CircInOut => {
                if t < 0.5 {
                    0.5 * (1.0 - (1.0 - 4.0 * t * t).sqrt())
                } else {
                    0.5 * ((1.0 - (2.0 * t - 2.0).powi(2)).sqrt() + 1.0)
                }
            }
            Self::ElasticIn => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    -(2.0f64).powf(10.0 * (t - 1.0)) * ((t - 1.1) * 5.0 * PI).sin()
                }
            }
            Self::ElasticOut => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    (2.0f64).powf(-10.0 * t) * ((t - 0.1) * 5.0 * PI).sin() + 1.0
                }
            }
            Self::ElasticInOut => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    -0.5 * (2.0f64).powf(20.0 * t - 10.0) * ((2.0 * t - 1.1) * 5.0 * PI).sin()
                } else {
                    0.5 * (2.0f64).powf(-20.0 * t + 10.0) * ((2.0 * t - 1.1) * 5.0 * PI).sin() + 1.0
                }
            }
            Self::BackIn => {
                let s = 1.70158;
                t * t * ((s + 1.0) * t - s)
            }
            Self::BackOut => {
                let s = 1.70158;
                let f = t - 1.0;
                f * f * ((s + 1.0) * f + s) + 1.0
            }
            Self::BackInOut => {
                let s = 1.70158 * 1.525;
                if t < 0.5 {
                    0.5 * (4.0 * t * t * ((s + 1.0) * 2.0 * t - s))
                } else {
                    let f = 2.0 * t - 2.0;
                    0.5 * (f * f * ((s + 1.0) * f + s) + 2.0)
                }
            }
            Self::BounceIn => 1.0 - Self::BounceOut.eval(1.0 - t),
            Self::BounceOut => {
                let n1 = 7.5625;
                let d1 = 2.75;
                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    let t = t - 1.5 / d1;
                    n1 * t * t + 0.75
                } else if t < 2.5 / d1 {
                    let t = t - 2.25 / d1;
                    n1 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / d1;
                    n1 * t * t + 0.984375
                }
            }
            Self::BounceInOut => {
                if t < 0.5 {
                    0.5 * Self::BounceIn.eval(2.0 * t)
                } else {
                    0.5 * Self::BounceOut.eval(2.0 * t - 1.0) + 0.5
                }
            }
        }
    }
}

/// Parametric Cubic Bézier curve defined by control points `(x1, y1)` and `(x2, y2)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl CubicBezier {
    /// Create a cubic Bézier easing curve.
    pub const fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Standard CSS ease: cubic-bezier(0.25, 0.1, 0.25, 1.0)
    pub const fn ease() -> Self {
        Self::new(0.25, 0.1, 0.25, 1.0)
    }

    /// Standard CSS ease-in: cubic-bezier(0.42, 0.0, 1.0, 1.0)
    pub const fn ease_in() -> Self {
        Self::new(0.42, 0.0, 1.0, 1.0)
    }

    /// Standard CSS ease-out: cubic-bezier(0.0, 0.0, 0.58, 1.0)
    pub const fn ease_out() -> Self {
        Self::new(0.0, 0.0, 0.58, 1.0)
    }

    /// Standard CSS ease-in-out: cubic-bezier(0.42, 0.0, 0.58, 1.0)
    pub const fn ease_in_out() -> Self {
        Self::new(0.42, 0.0, 0.58, 1.0)
    }

    #[inline]
    fn sample_curve_x(&self, t: f64) -> f64 {
        // 3*(1-t)^2*t*x1 + 3*(1-t)*t^2*x2 + t^3
        let c_x = 3.0 * self.x1;
        let b_x = 3.0 * (self.x2 - self.x1) - c_x;
        let a_x = 1.0 - c_x - b_x;
        ((a_x * t + b_x) * t + c_x) * t
    }

    #[inline]
    fn sample_curve_y(&self, t: f64) -> f64 {
        let c_y = 3.0 * self.y1;
        let b_y = 3.0 * (self.y2 - self.y1) - c_y;
        let a_y = 1.0 - c_y - b_y;
        ((a_y * t + b_y) * t + c_y) * t
    }

    #[inline]
    fn sample_curve_derivative_x(&self, t: f64) -> f64 {
        let c_x = 3.0 * self.x1;
        let b_x = 3.0 * (self.x2 - self.x1) - c_x;
        let a_x = 1.0 - c_x - b_x;
        (3.0 * a_x * t + 2.0 * b_x) * t + c_x
    }

    /// Solve for parametric parameter `t` given progress `x` using Newton-Raphson.
    pub fn solve_t(&self, x: f64) -> f64 {
        let x = x.clamp(0.0, 1.0);
        let mut t = x;
        // Newton-Raphson iteration (max 8 steps for sub-microsecond zero-heap convergence)
        for _ in 0..8 {
            let x2 = self.sample_curve_x(t) - x;
            if x2.abs() < 1e-7 {
                return t;
            }
            let d2 = self.sample_curve_derivative_x(t);
            if d2.abs() < 1e-7 {
                break;
            }
            t -= x2 / d2;
        }

        // Fallback binary subdivision if Newton diverges
        let mut t0 = 0.0;
        let mut t1 = 1.0;
        let mut t = x;
        for _ in 0..12 {
            let x2 = self.sample_curve_x(t);
            if (x2 - x).abs() < 1e-7 {
                return t;
            }
            if x > x2 {
                t0 = t;
            } else {
                t1 = t;
            }
            t = (t1 + t0) * 0.5;
        }
        t
    }

    /// Evaluate the cubic Bézier curve at `x` in [0.0, 1.0].
    #[inline]
    pub fn eval(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        let t = self.solve_t(x);
        self.sample_curve_y(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_eval_endpoints() {
        assert_eq!(EasingCurve::Linear.eval(0.0), 0.0);
        assert_eq!(EasingCurve::Linear.eval(0.5), 0.5);
        assert_eq!(EasingCurve::Linear.eval(1.0), 1.0);
    }

    #[test]
    fn all_curves_stay_in_bounds_at_endpoints() {
        let curves = [
            EasingCurve::Linear,
            EasingCurve::QuadIn,
            EasingCurve::QuadOut,
            EasingCurve::QuadInOut,
            EasingCurve::CubicIn,
            EasingCurve::CubicOut,
            EasingCurve::CubicInOut,
            EasingCurve::SineIn,
            EasingCurve::SineOut,
            EasingCurve::SineInOut,
            EasingCurve::ExpoIn,
            EasingCurve::ExpoOut,
            EasingCurve::CircIn,
            EasingCurve::CircOut,
            EasingCurve::ElasticIn,
            EasingCurve::ElasticOut,
            EasingCurve::BackIn,
            EasingCurve::BackOut,
            EasingCurve::BounceIn,
            EasingCurve::BounceOut,
        ];
        for c in &curves {
            let v0 = c.eval(0.0);
            let v1 = c.eval(1.0);
            assert!(v0.abs() < 1e-6, "curve {c:?} at 0.0 was {v0}");
            assert!((v1 - 1.0).abs() < 1e-6, "curve {c:?} at 1.0 was {v1}");
        }
    }

    #[test]
    fn cubic_bezier_ease_in_out() {
        let bez = CubicBezier::ease_in_out();
        assert_eq!(bez.eval(0.0), 0.0);
        assert_eq!(bez.eval(1.0), 1.0);
        let mid = bez.eval(0.5);
        assert!((mid - 0.5).abs() < 1e-3);
    }
}
