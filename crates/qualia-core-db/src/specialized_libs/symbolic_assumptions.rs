//! **Simplification under assumptions** (Gap analysis §3.3) — CAS simplifications that are
//! only *valid* when the simplifier knows a variable's sign / nonzero-ness.
//!
//! Plain [`simplify`](super::symbolic_algebra::simplify) must stay sound for *all* real
//! inputs, so it cannot turn `√(x²)` into `x` (that is `|x|`), or `ln(a·b)` into
//! `ln a + ln b` (the log laws need positivity). This module takes an explicit
//! [`Assumptions`] set (`x > 0`, `n ≠ 0`, …) and applies exactly those rewrites the
//! assumptions license — and **no others**. Every rewrite is gated on a *proof* of the
//! needed sign from the assumptions (see [`Assumptions::is_positive`] etc.); when the sign
//! cannot be established the node is left untouched (fail-closed: never an unsound rewrite).

use super::symbolic_algebra::{add, c, ln, mul, neg, pow, simplify, sqrt, Expr};
use std::collections::HashMap;

/// A sign / domain assumption about a single variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sign {
    /// `x > 0`.
    Positive,
    /// `x ≥ 0`.
    NonNegative,
    /// `x < 0`.
    Negative,
    /// `x ≤ 0`.
    NonPositive,
    /// `x ≠ 0` (sign unknown).
    Nonzero,
}

/// A set of per-variable sign assumptions used to license otherwise-unsound rewrites.
#[derive(Debug, Clone, Default)]
pub struct Assumptions {
    signs: HashMap<String, Sign>,
}

impl Assumptions {
    pub fn new() -> Self {
        Self {
            signs: HashMap::new(),
        }
    }

    /// Assert a sign for `var`. Builder-style (chainable).
    pub fn assume(mut self, var: &str, sign: Sign) -> Self {
        self.signs.insert(var.to_string(), sign);
        self
    }

    fn var_sign(&self, name: &str) -> Option<Sign> {
        self.signs.get(name).copied()
    }

    /// Provable `expr ≥ 0` under these assumptions (a *sufficient* test — `None`-of-proof
    /// means "unknown", never "negative").
    pub fn is_nonnegative(&self, e: &Expr) -> bool {
        match e {
            Expr::Const(k) => *k >= 0.0,
            Expr::Var(name) => matches!(
                self.var_sign(name),
                Some(Sign::Positive) | Some(Sign::NonNegative)
            ),
            // Even integer powers are ≥ 0 for any real base; odd powers inherit the base.
            Expr::Pow(a, n) => *n % 2 == 0 || self.is_nonnegative(a),
            Expr::Sqrt(_) | Expr::Exp(_) => true, // real sqrt ≥ 0, exp > 0
            Expr::Mul(a, b) => {
                (self.is_nonnegative(a) && self.is_nonnegative(b))
                    || (self.is_nonpositive(a) && self.is_nonpositive(b))
            }
            Expr::Add(a, b) => self.is_nonnegative(a) && self.is_nonnegative(b),
            Expr::Neg(a) => self.is_nonpositive(a),
            _ => false,
        }
    }

    /// Provable `expr > 0` under these assumptions.
    pub fn is_positive(&self, e: &Expr) -> bool {
        match e {
            Expr::Const(k) => *k > 0.0,
            Expr::Var(name) => matches!(self.var_sign(name), Some(Sign::Positive)),
            Expr::Exp(_) => true,
            Expr::Sqrt(a) => self.is_positive(a),
            Expr::Pow(a, n) => *n % 2 == 0 && self.is_nonzero(a) || self.is_positive(a),
            Expr::Mul(a, b) => {
                (self.is_positive(a) && self.is_positive(b))
                    || (self.is_negative(a) && self.is_negative(b))
            }
            Expr::Add(a, b) => {
                self.is_positive(a) && self.is_nonnegative(b)
                    || self.is_nonnegative(a) && self.is_positive(b)
            }
            Expr::Neg(a) => self.is_negative(a),
            _ => false,
        }
    }

    /// Provable `expr ≤ 0`.
    pub fn is_nonpositive(&self, e: &Expr) -> bool {
        match e {
            Expr::Const(k) => *k <= 0.0,
            Expr::Var(name) => matches!(
                self.var_sign(name),
                Some(Sign::Negative) | Some(Sign::NonPositive)
            ),
            Expr::Neg(a) => self.is_nonnegative(a),
            _ => false,
        }
    }

    /// Provable `expr < 0`.
    pub fn is_negative(&self, e: &Expr) -> bool {
        match e {
            Expr::Const(k) => *k < 0.0,
            Expr::Var(name) => matches!(self.var_sign(name), Some(Sign::Negative)),
            Expr::Neg(a) => self.is_positive(a),
            _ => false,
        }
    }

    /// Provable `expr ≠ 0`.
    pub fn is_nonzero(&self, e: &Expr) -> bool {
        match e {
            Expr::Const(k) => *k != 0.0,
            Expr::Var(name) => matches!(
                self.var_sign(name),
                Some(Sign::Positive) | Some(Sign::Negative) | Some(Sign::Nonzero)
            ),
            Expr::Exp(_) => true,
            Expr::Pow(a, _) => self.is_nonzero(a),
            Expr::Sqrt(a) => self.is_positive(a),
            Expr::Mul(a, b) => self.is_nonzero(a) && self.is_nonzero(b),
            Expr::Neg(a) => self.is_nonzero(a),
            _ => false,
        }
    }
}

/// Simplify `expr` using assumption-gated rewrites on top of the plain (always-sound)
/// [`simplify`]. Applied to a bounded fixpoint. Rewrites performed (each only when the
/// assumptions *prove* the side condition):
///
/// - `√(x²) → x`           when `x ≥ 0`  (and `→ −x` when `x ≤ 0`)
/// - `(√x)² → x`           when `x ≥ 0`
/// - `ln(a·b) → ln a + ln b`   when `a, b > 0`
/// - `ln(aⁿ) → n·ln a`         when `a > 0`
pub fn simplify_with_assumptions(expr: &Expr, asm: &Assumptions) -> Expr {
    let mut cur = simplify(expr);
    for _ in 0..16 {
        let next = simplify(&rewrite(&cur, asm));
        if next == cur {
            break;
        }
        cur = next;
    }
    cur
}

fn rewrite(e: &Expr, asm: &Assumptions) -> Expr {
    // Rewrite children first (bottom-up).
    let e = match e {
        Expr::Add(a, b) => add(rewrite(a, asm), rewrite(b, asm)),
        Expr::Sub(a, b) => Expr::Sub(Box::new(rewrite(a, asm)), Box::new(rewrite(b, asm))),
        Expr::Mul(a, b) => mul(rewrite(a, asm), rewrite(b, asm)),
        Expr::Div(a, b) => Expr::Div(Box::new(rewrite(a, asm)), Box::new(rewrite(b, asm))),
        Expr::Pow(a, n) => pow(rewrite(a, asm), *n),
        Expr::Neg(a) => neg(rewrite(a, asm)),
        Expr::Sqrt(a) => sqrt(rewrite(a, asm)),
        Expr::Exp(a) => Expr::Exp(Box::new(rewrite(a, asm))),
        Expr::Ln(a) => ln(rewrite(a, asm)),
        Expr::Sin(a) => Expr::Sin(Box::new(rewrite(a, asm))),
        Expr::Cos(a) => Expr::Cos(Box::new(rewrite(a, asm))),
        Expr::Tan(a) => Expr::Tan(Box::new(rewrite(a, asm))),
        Expr::Const(_) | Expr::Var(_) => e.clone(),
    };

    match &e {
        // √(x²) → x (x≥0) / −x (x≤0).
        Expr::Sqrt(inner) => {
            if let Expr::Pow(base, 2) = &**inner {
                if asm.is_nonnegative(base) {
                    return (**base).clone();
                }
                if asm.is_nonpositive(base) {
                    return neg((**base).clone());
                }
            }
            e
        }
        // (√x)² → x  when x ≥ 0.
        Expr::Pow(base, 2) => {
            if let Expr::Sqrt(under) = &**base {
                if asm.is_nonnegative(under) {
                    return (**under).clone();
                }
            }
            e
        }
        // ln laws under positivity.
        Expr::Ln(arg) => match &**arg {
            Expr::Mul(a, b) if asm.is_positive(a) && asm.is_positive(b) => {
                add(ln((**a).clone()), ln((**b).clone()))
            }
            Expr::Pow(a, n) if asm.is_positive(a) => mul(c(*n as f64), ln((**a).clone())),
            _ => e,
        },
        _ => e,
    }
}

#[cfg(test)]
mod tests {
    use super::super::symbolic_algebra::{c, exp, mul, pow, sqrt, var};
    use super::*;

    #[test]
    fn sqrt_of_square_uses_sign() {
        // √(x²) → x when x ≥ 0.
        let pos = Assumptions::new().assume("x", Sign::Positive);
        assert_eq!(
            simplify_with_assumptions(&sqrt(pow(var("x"), 2)), &pos),
            var("x")
        );
        // → −x when x ≤ 0.
        let neg_x = Assumptions::new().assume("x", Sign::Negative);
        assert_eq!(
            simplify_with_assumptions(&sqrt(pow(var("x"), 2)), &neg_x),
            neg(var("x"))
        );
        // Unknown sign → left as √(x²) (no unsound rewrite).
        let unknown = Assumptions::new();
        let s = simplify_with_assumptions(&sqrt(pow(var("x"), 2)), &unknown);
        assert_eq!(s, sqrt(pow(var("x"), 2)));
    }

    #[test]
    fn sqrt_square_inverse() {
        // (√x)² → x when x ≥ 0.
        let asm = Assumptions::new().assume("x", Sign::NonNegative);
        assert_eq!(
            simplify_with_assumptions(&pow(sqrt(var("x")), 2), &asm),
            var("x")
        );
    }

    #[test]
    fn log_laws_need_positivity() {
        let asm = Assumptions::new()
            .assume("a", Sign::Positive)
            .assume("b", Sign::Positive);
        // ln(a·b) → ln a + ln b ; numerically equal at a sample point.
        let got = simplify_with_assumptions(&ln(mul(var("a"), var("b"))), &asm);
        let mut env = HashMap::new();
        env.insert("a".to_string(), 3.0);
        env.insert("b".to_string(), 5.0);
        assert!((got.eval(&env).unwrap() - (15.0_f64).ln()).abs() < 1e-9);
        assert_eq!(got, add(ln(var("a")), ln(var("b"))));
        // ln(a³) → 3·ln a.
        let got2 = simplify_with_assumptions(&ln(pow(var("a"), 3)), &asm);
        assert_eq!(got2, mul(c(3.0), ln(var("a"))));
        // Without the positivity assumption, no rewrite.
        let none = Assumptions::new();
        assert_eq!(
            simplify_with_assumptions(&ln(mul(var("a"), var("b"))), &none),
            ln(mul(var("a"), var("b")))
        );
    }

    #[test]
    fn sign_inference_basics() {
        let asm = Assumptions::new().assume("x", Sign::Positive);
        assert!(asm.is_positive(&exp(var("x"))));
        assert!(asm.is_nonnegative(&pow(var("y"), 2))); // even power, any base
        assert!(asm.is_nonzero(&var("x")));
        assert!(!asm.is_positive(&var("y"))); // unknown
    }
}
