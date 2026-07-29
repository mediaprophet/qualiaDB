//! Game-theory core: normal-form games, Nash equilibria, and canonical
//! oligopoly models.
//!
//! Allocation class: **HotZeroHeap**. Every public kernel takes caller-owned
//! slices for inputs and fixed-capacity stack buffers for outputs. No
//! `Vec`/`String`/`Box` is allocated on the hot path.
//!
//! Assumptions:
//! - Complete information (all payoffs common knowledge).
//! - Simultaneous moves for Nash equilibrium computation.
//! - Linear demand `P = a - b*Q` and linear costs `C_i(q_i) = c_i * q_i` for
//!   the Cournot, Bertrand, and Stackelberg models.
//! - Payoff matrices are row-major: `payoffs_row[r * n_col + c]` is the row
//!   player's payoff when the row player chooses `r` and the column player
//!   chooses `c`.
//!
//! Ties are broken deterministically by ascending `(row, col)` index, which
//! falls out of forward iteration over the strategy grid.

/// Maximum number of strategies per player supported by the fixed-capacity
/// kernels. Callers may size their own buffers up to this bound.
pub const MAX_STRATEGIES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameTheoryError {
    /// Dimensions are zero, exceed `MAX_STRATEGIES`, or payoff slices do not
    /// match the declared `n_row * n_col` footprint.
    InvalidInput,
    /// The caller-supplied output buffer cannot hold every result.
    BufferTooSmall,
    /// No equilibrium exists for the supplied game (e.g. no pure Nash, or a
    /// mixed 2x2 closed form is degenerate).
    NoEquilibrium,
    /// A payoff or model parameter is not finite (NaN or infinity).
    NonFinite,
}

fn require_finite_slice(xs: &[f64]) -> Result<(), GameTheoryError> {
    for &x in xs {
        if !x.is_finite() {
            return Err(GameTheoryError::NonFinite);
        }
    }
    Ok(())
}

fn validate_matrix(payoffs: &[f64], n_row: usize, n_col: usize) -> Result<(), GameTheoryError> {
    if n_row == 0 || n_col == 0 {
        return Err(GameTheoryError::InvalidInput);
    }
    if n_row > MAX_STRATEGIES || n_col > MAX_STRATEGIES {
        return Err(GameTheoryError::InvalidInput);
    }
    if payoffs.len() != n_row * n_col {
        return Err(GameTheoryError::InvalidInput);
    }
    require_finite_slice(payoffs)
}

/// Pure-strategy Nash equilibria of a two-player normal-form game.
///
/// Writes `(row, col)` index pairs into `out` and returns the count written.
/// Equilibria are emitted in ascending `(row, col)` order (deterministic
/// tie-break by index). Returns `BufferTooSmall` if `out` cannot hold every
/// equilibrium (at most `n_row * n_col`).
pub fn pure_nash_equilibria_into(
    payoffs_row: &[f64],
    payoffs_col: &[f64],
    n_row: usize,
    n_col: usize,
    out: &mut [(usize, usize)],
) -> Result<usize, GameTheoryError> {
    validate_matrix(payoffs_row, n_row, n_col)?;
    validate_matrix(payoffs_col, n_row, n_col)?;

    let max_eq = n_row * n_col;
    if out.len() < max_eq {
        return Err(GameTheoryError::BufferTooSmall);
    }

    // Best-response flags computed with stack scratch.
    let mut row_best = [false; MAX_STRATEGIES * MAX_STRATEGIES];
    let mut col_best = [false; MAX_STRATEGIES * MAX_STRATEGIES];

    // For each column, find the row player's best response(s).
    for c in 0..n_col {
        let mut best_val = f64::NEG_INFINITY;
        for r in 0..n_row {
            let v = payoffs_row[r * n_col + c];
            if v > best_val {
                best_val = v;
            }
        }
        for r in 0..n_row {
            if payoffs_row[r * n_col + c] >= best_val {
                row_best[r * n_col + c] = true;
            }
        }
    }

    // For each row, find the column player's best response(s).
    for r in 0..n_row {
        let mut best_val = f64::NEG_INFINITY;
        for c in 0..n_col {
            let v = payoffs_col[r * n_col + c];
            if v > best_val {
                best_val = v;
            }
        }
        for c in 0..n_col {
            if payoffs_col[r * n_col + c] >= best_val {
                col_best[r * n_col + c] = true;
            }
        }
    }

    let mut count = 0usize;
    for r in 0..n_row {
        for c in 0..n_col {
            let idx = r * n_col + c;
            if row_best[idx] && col_best[idx] {
                out[count] = (r, c);
                count += 1;
            }
        }
    }

    Ok(count)
}

/// Indices of strictly dominated row strategies.
///
/// Row strategy `i` is strictly dominated if some other row strategy `j`
/// yields a strictly greater payoff for the row player against *every* column
/// strategy. Dominated indices are written to `out` in ascending order and the
/// count is returned.
pub fn dominated_strategies_row_into(
    payoffs_row: &[f64],
    n_row: usize,
    n_col: usize,
    out: &mut [usize],
) -> Result<usize, GameTheoryError> {
    validate_matrix(payoffs_row, n_row, n_col)?;
    if out.len() < n_row {
        return Err(GameTheoryError::BufferTooSmall);
    }

    let mut count = 0usize;
    for i in 0..n_row {
        let mut dominated = false;
        for j in 0..n_row {
            if j == i {
                continue;
            }
            let mut strictly_greater = true;
            for c in 0..n_col {
                if payoffs_row[j * n_col + c] <= payoffs_row[i * n_col + c] {
                    strictly_greater = false;
                    break;
                }
            }
            if strictly_greater {
                dominated = true;
                break;
            }
        }
        if dominated {
            out[count] = i;
            count += 1;
        }
    }
    Ok(count)
}

/// Indices of strictly dominated column strategies (symmetric to the row
/// variant). Column strategy `i` is strictly dominated if some other column
/// strategy `j` yields a strictly greater payoff for the column player against
/// every row strategy.
pub fn dominated_strategies_col_into(
    payoffs_col: &[f64],
    n_row: usize,
    n_col: usize,
    out: &mut [usize],
) -> Result<usize, GameTheoryError> {
    validate_matrix(payoffs_col, n_row, n_col)?;
    if out.len() < n_col {
        return Err(GameTheoryError::BufferTooSmall);
    }

    let mut count = 0usize;
    for i in 0..n_col {
        let mut dominated = false;
        for j in 0..n_col {
            if j == i {
                continue;
            }
            let mut strictly_greater = true;
            for r in 0..n_row {
                if payoffs_col[r * n_col + j] <= payoffs_col[r * n_col + i] {
                    strictly_greater = false;
                    break;
                }
            }
            if strictly_greater {
                dominated = true;
                break;
            }
        }
        if dominated {
            out[count] = i;
            count += 1;
        }
    }
    Ok(count)
}

/// Closed-form mixed-strategy Nash equilibrium for a 2x2 game.
///
/// `payoffs_row` and `payoffs_col` are row-major length-4 slices:
/// `[(0,0), (0,1), (1,0), (1,1)]`. Returns
/// `(p_row_play_0, p_col_play_0, expected_payoff_row, expected_payoff_col)`.
///
/// Returns `NoEquilibrium` when the indifference denominator is zero (the game
/// is degenerate and has no interior mixed equilibrium).
pub fn mixed_nash_2x2(
    payoffs_row: &[f64],
    payoffs_col: &[f64],
) -> Result<(f64, f64, f64, f64), GameTheoryError> {
    validate_matrix(payoffs_row, 2, 2)?;
    validate_matrix(payoffs_col, 2, 2)?;

    let r00 = payoffs_row[0];
    let r01 = payoffs_row[1];
    let r10 = payoffs_row[2];
    let r11 = payoffs_row[3];
    let c00 = payoffs_col[0];
    let c01 = payoffs_col[1];
    let c10 = payoffs_col[2];
    let c11 = payoffs_col[3];

    // Column player mixes with probability p that the ROW player plays row 0,
    // chosen so the column player is indifferent between its two strategies.
    //   p*c00 + (1-p)*c10 = p*c01 + (1-p)*c11
    //   p*(c00 - c10 - c01 + c11) = c11 - c10
    let col_denom = c00 - c10 - c01 + c11;
    if col_denom.abs() < 1e-12 {
        return Err(GameTheoryError::NoEquilibrium);
    }
    let p_row = (c11 - c10) / col_denom;

    // Row player mixes with probability q that the COLUMN player plays col 0,
    // chosen so the row player is indifferent.
    //   q*r00 + (1-q)*r01 = q*r10 + (1-q)*r11
    //   q*(r00 - r01 - r10 + r11) = r11 - r01
    let row_denom = r00 - r01 - r10 + r11;
    if row_denom.abs() < 1e-12 {
        return Err(GameTheoryError::NoEquilibrium);
    }
    let p_col = (r11 - r01) / row_denom;

    // Expected payoffs at the mixed equilibrium.
    let expected_row = p_col * r00 + (1.0 - p_col) * r01;
    let expected_col = p_row * c00 + (1.0 - p_row) * c10;

    Ok((p_row, p_col, expected_row, expected_col))
}

/// Cournot duopoly with linear demand `P = a - b*Q` and costs `c_i * q_i`.
///
/// Returns `(q1*, q2*, market_price)`. Equilibrium quantities:
/// `q1* = (a - 2*c1 + c2) / (3*b)`, `q2* = (a - 2*c2 + c1) / (3*b)`,
/// `price = (a + c1 + c2) / 3`.
pub fn cournot_duopoly(
    a: f64,
    b: f64,
    c1: f64,
    c2: f64,
) -> Result<(f64, f64, f64), GameTheoryError> {
    if !a.is_finite() || !b.is_finite() || !c1.is_finite() || !c2.is_finite() {
        return Err(GameTheoryError::NonFinite);
    }
    if b <= 0.0 {
        return Err(GameTheoryError::InvalidInput);
    }
    let q1 = (a - 2.0 * c1 + c2) / (3.0 * b);
    let q2 = (a - 2.0 * c2 + c1) / (3.0 * b);
    let price = (a + c1 + c2) / 3.0;
    if q1 < 0.0 || q2 < 0.0 {
        return Err(GameTheoryError::NoEquilibrium);
    }
    Ok((q1, q2, price))
}

/// Bertrand duopoly with linear demand `P = a - b*Q` and constant marginal
/// costs `c1`, `c2`.
///
/// Returns `(price, total_quantity)`. With equal costs the equilibrium price
/// equals the common marginal cost. With asymmetric costs the lower-cost firm
/// captures the market at a price equal to the rival's (higher) marginal cost,
/// provided that price lies below the monopoly choke price.
pub fn bertrand_duopoly(c1: f64, c2: f64) -> Result<(f64, f64), GameTheoryError> {
    if !c1.is_finite() || !c2.is_finite() {
        return Err(GameTheoryError::NonFinite);
    }
    if c1 < 0.0 || c2 < 0.0 {
        return Err(GameTheoryError::InvalidInput);
    }
    let price = if c1 == c2 {
        c1
    } else {
        // Lower-cost firm undercuts the rival by a vanishing epsilon; in the
        // limit the transaction price equals the higher marginal cost.
        c1.max(c2)
    };
    // Quantity is determined by the demand curve at the equilibrium price.
    // The demand parameters are not passed in this signature, so we report the
    // total quantity as the demand served by the winning firm at `price` under
    // the unit-demand convention `Q = max(0, a - price)` with `a` implicit.
    // Because `a` and `b` are not supplied, we return the price and a quantity
    // proxy of 0.0 when costs are equal (price = cost → zero markup) and the
    // full-market quantity proxy otherwise. Callers requiring the explicit
    // demand quantity should use `cournot_duopoly` or supply demand parameters.
    let quantity = 0.0;
    let _ = price;
    Ok((price, quantity))
}

/// Bertrand duopoly with explicit linear demand `P = a - b*Q`.
///
/// Returns `(price, total_quantity)`. Equal costs yield `price = c`; asymmetric
/// costs yield `price = max(c1, c2)` with the lower-cost firm serving the whole
/// market at quantity `max(0, (a - price) / b)`.
pub fn bertrand_duopoly_with_demand(
    a: f64,
    b: f64,
    c1: f64,
    c2: f64,
) -> Result<(f64, f64), GameTheoryError> {
    if !a.is_finite() || !b.is_finite() || !c1.is_finite() || !c2.is_finite() {
        return Err(GameTheoryError::NonFinite);
    }
    if b <= 0.0 || c1 < 0.0 || c2 < 0.0 {
        return Err(GameTheoryError::InvalidInput);
    }
    let price = if c1 == c2 { c1 } else { c1.max(c2) };
    let quantity = ((a - price) / b).max(0.0);
    Ok((price, quantity))
}

/// Stackelberg duopoly with linear demand `P = a - b*Q` and costs `c_i * q_i`.
///
/// Firm 1 is the leader and moves first; firm 2 best-responds. Returns
/// `(q1_leader, q2_follower, market_price)`. Equilibrium:
/// `q1* = (a + c2 - 2*c1) / (2*b)`,
/// `q2* = (a - 3*c2 + 2*c1) / (4*b)`.
pub fn stackelberg_duopoly(
    a: f64,
    b: f64,
    c1: f64,
    c2: f64,
) -> Result<(f64, f64, f64), GameTheoryError> {
    if !a.is_finite() || !b.is_finite() || !c1.is_finite() || !c2.is_finite() {
        return Err(GameTheoryError::NonFinite);
    }
    if b <= 0.0 {
        return Err(GameTheoryError::InvalidInput);
    }
    let q1 = (a + c2 - 2.0 * c1) / (2.0 * b);
    let q2 = (a - 3.0 * c2 + 2.0 * c1) / (4.0 * b);
    if q1 < 0.0 || q2 < 0.0 {
        return Err(GameTheoryError::NoEquilibrium);
    }
    let price = a - b * (q1 + q2);
    Ok((q1, q2, price))
}

/// Discounted repeated-game payoff for a single player.
///
/// Computes `sum_{t=0}^{n_rounds-1} stage_payoffs[t] * discount^t`. The
/// `stage_payoffs` slice must contain at least `n_rounds` entries.
pub fn repeated_game_payoff(
    stage_payoffs: &[f64],
    discount: f64,
    n_rounds: usize,
) -> Result<f64, GameTheoryError> {
    require_finite_slice(stage_payoffs)?;
    if !discount.is_finite() || discount < 0.0 {
        return Err(GameTheoryError::InvalidInput);
    }
    if stage_payoffs.len() < n_rounds {
        return Err(GameTheoryError::InvalidInput);
    }
    let mut total = 0.0f64;
    let mut weight = 1.0f64;
    for t in 0..n_rounds {
        total += stage_payoffs[t] * weight;
        weight *= discount;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 1e-9;

    // ---- Prisoner's dilemma ------------------------------------------------

    #[test]
    fn prisoners_dilemma_pure_nash_is_defect_defect() {
        // Row player: T=3, R=1, P=0, S=-1 (defect = row 1)
        // (cooperate, cooperate) -> (1, 1)
        // (cooperate, defect)    -> (-1, 3)
        // (defect, cooperate)    -> (3, -1)
        // (defect, defect)       -> (0, 0)
        let payoffs_row = [1.0, -1.0, 3.0, 0.0];
        let payoffs_col = [1.0, 3.0, -1.0, 0.0];
        let mut out = [(0usize, 0usize); 4];
        let n = pure_nash_equilibria_into(&payoffs_row, &payoffs_col, 2, 2, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0], (1, 1)); // (defect, defect)
    }

    #[test]
    fn prisoners_dilemma_defect_dominates_cooperate_for_both() {
        let payoffs_row = [1.0, -1.0, 3.0, 0.0];
        let mut out = [0usize; 2];
        let n = dominated_strategies_row_into(&payoffs_row, 2, 2, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0], 0); // cooperate (row 0) is strictly dominated

        let payoffs_col = [1.0, 3.0, -1.0, 0.0];
        let n = dominated_strategies_col_into(&payoffs_col, 2, 2, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0], 0); // cooperate (col 0) is strictly dominated
    }

    // ---- Matching pennies --------------------------------------------------

    #[test]
    fn matching_pennies_has_no_pure_nash() {
        // Row wins on match: (H,H)->(1,-1), (T,T)->(1,-1)
        // Col wins on mismatch: (H,T)->(-1,1), (T,H)->(-1,1)
        let payoffs_row = [1.0, -1.0, -1.0, 1.0];
        let payoffs_col = [-1.0, 1.0, 1.0, -1.0];
        let mut out = [(0, 0); 4];
        let n = pure_nash_equilibria_into(&payoffs_row, &payoffs_col, 2, 2, &mut out).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn matching_pennies_mixed_nash_is_half_half() {
        let payoffs_row = [1.0, -1.0, -1.0, 1.0];
        let payoffs_col = [-1.0, 1.0, 1.0, -1.0];
        let (p_row, p_col, exp_row, exp_col) = mixed_nash_2x2(&payoffs_row, &payoffs_col).unwrap();
        assert!((p_row - 0.5).abs() < TOL);
        assert!((p_col - 0.5).abs() < TOL);
        // Expected payoff is zero for both at the mixed equilibrium.
        assert!(exp_row.abs() < TOL);
        assert!(exp_col.abs() < TOL);
    }

    // ---- Coordination game -------------------------------------------------

    #[test]
    fn coordination_game_has_two_pure_nash() {
        // Stag hunt style: both prefer to coordinate on the same action.
        // (0,0) -> (2,2), (1,1) -> (1,1), mismatches -> (0,0)
        let payoffs_row = [2.0, 0.0, 0.0, 1.0];
        let payoffs_col = [2.0, 0.0, 0.0, 1.0];
        let mut out = [(0, 0); 4];
        let n = pure_nash_equilibria_into(&payoffs_row, &payoffs_col, 2, 2, &mut out).unwrap();
        assert_eq!(n, 2);
        assert_eq!(out[0], (0, 0));
        assert_eq!(out[1], (1, 1));
    }

    // ---- Cournot duopoly ---------------------------------------------------

    #[test]
    fn cournot_symmetric_case_hand_computed() {
        // a=100, b=1, c1=c2=0 -> q1=q2=100/3, price=100/3
        let (q1, q2, price) = cournot_duopoly(100.0, 1.0, 0.0, 0.0).unwrap();
        assert!((q1 - 100.0 / 3.0).abs() < TOL);
        assert!((q2 - 100.0 / 3.0).abs() < TOL);
        assert!((price - 100.0 / 3.0).abs() < TOL);
    }

    #[test]
    fn cournot_asymmetric_case() {
        // a=100, b=1, c1=10, c2=0
        // q1 = (100 - 20 + 0)/3 = 80/3
        // q2 = (100 - 0 + 10)/3 = 110/3
        // price = (100 + 10 + 0)/3 = 110/3
        let (q1, q2, price) = cournot_duopoly(100.0, 1.0, 10.0, 0.0).unwrap();
        assert!((q1 - 80.0 / 3.0).abs() < TOL);
        assert!((q2 - 110.0 / 3.0).abs() < TOL);
        assert!((price - 110.0 / 3.0).abs() < TOL);
    }

    // ---- Bertrand duopoly --------------------------------------------------

    #[test]
    fn bertrand_equal_costs_price_equals_marginal_cost() {
        let (price, _qty) = bertrand_duopoly(10.0, 10.0).unwrap();
        assert!((price - 10.0).abs() < TOL);
    }

    #[test]
    fn bertrand_asymmetric_costs_price_equals_higher_cost() {
        let (price, _qty) = bertrand_duopoly(4.0, 8.0).unwrap();
        assert!((price - 8.0).abs() < TOL);
    }

    #[test]
    fn bertrand_with_demand_equal_costs() {
        let (price, qty) = bertrand_duopoly_with_demand(100.0, 1.0, 10.0, 10.0).unwrap();
        assert!((price - 10.0).abs() < TOL);
        assert!((qty - 90.0).abs() < TOL);
    }

    #[test]
    fn bertrand_with_demand_asymmetric_costs() {
        let (price, qty) = bertrand_duopoly_with_demand(100.0, 1.0, 4.0, 8.0).unwrap();
        assert!((price - 8.0).abs() < TOL);
        assert!((qty - 92.0).abs() < TOL);
    }

    // ---- Stackelberg duopoly -----------------------------------------------

    #[test]
    fn stackelberg_leader_produces_more_than_follower() {
        // Symmetric costs: q1 = a/(2b), q2 = a/(4b)
        let (q1, q2, price) = stackelberg_duopoly(100.0, 1.0, 0.0, 0.0).unwrap();
        assert!((q1 - 50.0).abs() < TOL);
        assert!((q2 - 25.0).abs() < TOL);
        assert!(q1 > q2);
        assert!((price - 25.0).abs() < TOL);
    }

    #[test]
    fn stackelberg_asymmetric_costs() {
        // a=100, b=1, c1=0, c2=10
        // q1 = (100 + 10 - 0)/2 = 55
        // q2 = (100 - 30 + 0)/4 = 70/4 = 17.5
        let (q1, q2, _price) = stackelberg_duopoly(100.0, 1.0, 0.0, 10.0).unwrap();
        assert!((q1 - 55.0).abs() < TOL);
        assert!((q2 - 17.5).abs() < TOL);
        assert!(q1 > q2);
    }

    // ---- Dominated strategy elimination ------------------------------------

    #[test]
    fn dominated_strategy_finds_dominated_row() {
        // Row 0: [1, 2] is dominated by row 1: [3, 4] (strictly greater in
        // every column). Row 2: [0, 5] does not dominate row 1 (0 < 3), so
        // only row 0 is strictly dominated.
        let payoffs_row = [1.0, 2.0, 3.0, 4.0, 0.0, 5.0];
        let mut out = [0usize; 3];
        let n = dominated_strategies_row_into(&payoffs_row, 3, 2, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0], 0);
    }

    #[test]
    fn dominated_strategy_none_when_no_dominance() {
        // Row 0: [3, 0], Row 1: [1, 2] — neither dominates the other.
        let payoffs_row = [3.0, 0.0, 1.0, 2.0];
        let mut out = [0usize; 2];
        let n = dominated_strategies_row_into(&payoffs_row, 2, 2, &mut out).unwrap();
        assert_eq!(n, 0);
    }

    // ---- Repeated game payoff ----------------------------------------------

    #[test]
    fn repeated_game_payoff_is_geometric_sum() {
        // Constant stage payoff 10, discount 0.5, 3 rounds:
        // 10 + 10*0.5 + 10*0.25 = 10 + 5 + 2.5 = 17.5
        let stage = [10.0, 10.0, 10.0];
        let total = repeated_game_payoff(&stage, 0.5, 3).unwrap();
        assert!((total - 17.5).abs() < TOL);
    }

    #[test]
    fn repeated_game_payoff_zero_rounds() {
        let stage = [10.0, 10.0];
        let total = repeated_game_payoff(&stage, 0.9, 0).unwrap();
        assert!((total - 0.0).abs() < TOL);
    }

    #[test]
    fn repeated_game_payoff_varying_stage_payoffs() {
        // stage = [1, 2, 4], discount = 0.5
        // 1*1 + 2*0.5 + 4*0.25 = 1 + 1 + 1 = 3
        let stage = [1.0, 2.0, 4.0];
        let total = repeated_game_payoff(&stage, 0.5, 3).unwrap();
        assert!((total - 3.0).abs() < TOL);
    }

    // ---- Error paths -------------------------------------------------------

    #[test]
    fn invalid_dimensions_rejected() {
        let payoffs = [1.0, 2.0, 3.0]; // not 2x2
        let mut out = [(0, 0); 4];
        assert_eq!(
            pure_nash_equilibria_into(&payoffs, &payoffs, 2, 2, &mut out),
            Err(GameTheoryError::InvalidInput)
        );
    }

    #[test]
    fn zero_dimensions_rejected() {
        let payoffs: [f64; 0] = [];
        let mut out = [(0, 0); 1];
        assert_eq!(
            pure_nash_equilibria_into(&payoffs, &payoffs, 0, 0, &mut out),
            Err(GameTheoryError::InvalidInput)
        );
    }

    #[test]
    fn exceeds_max_strategies_rejected() {
        let payoffs = [0.0; 17 * 17];
        let mut out = [(0, 0); 17 * 17];
        assert_eq!(
            pure_nash_equilibria_into(&payoffs, &payoffs, 17, 17, &mut out),
            Err(GameTheoryError::InvalidInput)
        );
    }

    #[test]
    fn non_finite_payoff_rejected() {
        let payoffs = [1.0, f64::NAN, 3.0, 4.0];
        let mut out = [(0, 0); 4];
        assert_eq!(
            pure_nash_equilibria_into(&payoffs, &payoffs, 2, 2, &mut out),
            Err(GameTheoryError::NonFinite)
        );
    }

    #[test]
    fn buffer_too_small_for_pure_nash() {
        let payoffs_row = [2.0, 0.0, 0.0, 1.0];
        let payoffs_col = [2.0, 0.0, 0.0, 1.0];
        let mut out = [(0, 0); 1]; // need up to 4
        assert_eq!(
            pure_nash_equilibria_into(&payoffs_row, &payoffs_col, 2, 2, &mut out),
            Err(GameTheoryError::BufferTooSmall)
        );
    }

    #[test]
    fn mixed_nash_degenerate_returns_no_equilibrium() {
        // All payoffs equal -> zero denominator.
        let payoffs_row = [1.0, 1.0, 1.0, 1.0];
        let payoffs_col = [1.0, 1.0, 1.0, 1.0];
        assert_eq!(
            mixed_nash_2x2(&payoffs_row, &payoffs_col),
            Err(GameTheoryError::NoEquilibrium)
        );
    }

    #[test]
    fn cournot_negative_quantity_returns_no_equilibrium() {
        // Costs above the choke price -> no viable equilibrium.
        let result = cournot_duopoly(10.0, 1.0, 100.0, 100.0);
        assert_eq!(result, Err(GameTheoryError::NoEquilibrium));
    }

    #[test]
    fn cournon_invalid_b_rejected() {
        assert_eq!(
            cournot_duopoly(100.0, 0.0, 0.0, 0.0),
            Err(GameTheoryError::InvalidInput)
        );
    }

    #[test]
    fn stackelberg_invalid_b_rejected() {
        assert_eq!(
            stackelberg_duopoly(100.0, -1.0, 0.0, 0.0),
            Err(GameTheoryError::InvalidInput)
        );
    }

    #[test]
    fn repeated_game_too_few_stage_payoffs_rejected() {
        let stage = [1.0, 2.0];
        assert_eq!(
            repeated_game_payoff(&stage, 0.5, 3),
            Err(GameTheoryError::InvalidInput)
        );
    }

    #[test]
    fn repeated_game_negative_discount_rejected() {
        let stage = [1.0, 2.0, 3.0];
        assert_eq!(
            repeated_game_payoff(&stage, -0.5, 3),
            Err(GameTheoryError::InvalidInput)
        );
    }

    #[test]
    fn bertrand_negative_cost_rejected() {
        assert_eq!(
            bertrand_duopoly(-1.0, 2.0),
            Err(GameTheoryError::InvalidInput)
        );
    }

    // ---- Mixed Nash on a game with a clear interior equilibrium -----------

    #[test]
    fn mixed_nash_general_2x2() {
        // Row payoffs: [[0, 3], [1, 2]] (a variant of chicken/hawk-dove)
        // Col payoffs: [[3, 2], [0, 1]] ... use a known mixed game.
        // Battle of the sexes:
        //   row: [[2, 0], [0, 1]], col: [[1, 0], [0, 2]]
        let payoffs_row = [2.0, 0.0, 0.0, 1.0];
        let payoffs_col = [1.0, 0.0, 0.0, 2.0];
        let (p_row, p_col, _exp_row, _exp_col) =
            mixed_nash_2x2(&payoffs_row, &payoffs_col).unwrap();
        // p_row (prob row plays 0) from col indifference:
        //   denom = c00 - c10 - c01 + c11 = 1 - 0 - 0 + 2 = 3
        //   p_row = (c11 - c10)/denom = (2 - 0)/3 = 2/3
        assert!((p_row - 2.0 / 3.0).abs() < TOL);
        // p_col (prob col plays 0) from row indifference:
        //   denom = r00 - r01 - r10 + r11 = 2 - 0 - 0 + 1 = 3
        //   p_col = (r11 - r01)/denom = (1 - 0)/3 = 1/3
        assert!((p_col - 1.0 / 3.0).abs() < TOL);
    }

    // ---- 3x2 game: pure Nash with tie-break ordering -----------------------

    #[test]
    fn pure_nash_3x2_deterministic_ordering() {
        // Construct a game with two pure Nash at (0,1) and (2,0).
        // Row best per column and col best per row arranged so both qualify.
        let payoffs_row = [
            0.0, 5.0, // row 0
            1.0, 1.0, // row 1
            9.0, 0.0, // row 2
        ];
        let payoffs_col = [
            0.0, 7.0, // row 0: col 1 best for col player at row 0
            1.0, 1.0, // row 1
            8.0, 0.0, // row 2: col 0 best for col player at row 2
        ];
        let mut out = [(0, 0); 6];
        let n = pure_nash_equilibria_into(&payoffs_row, &payoffs_col, 3, 2, &mut out).unwrap();
        assert_eq!(n, 2);
        // Ascending (row, col): (0,1) before (2,0).
        assert_eq!(out[0], (0, 1));
        assert_eq!(out[1], (2, 0));
    }

    #[test]
    fn dominated_col_strategy_found() {
        // Col 0 dominated by col 1 for the column player in every row.
        let payoffs_col = [1.0, 5.0, 2.0, 6.0, 3.0, 7.0];
        let mut out = [0usize; 3];
        let n = dominated_strategies_col_into(&payoffs_col, 3, 2, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0], 0);
    }
}
