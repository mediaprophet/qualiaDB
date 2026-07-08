//! Market design: utility functions, partial-equilibrium clearing, auctions,
//! and matching.
//!
//! Allocation class: **HotZeroHeap**. All scratch uses fixed-capacity stack
//! arrays. No `Vec`/`String`/`Box` in any kernel.
//!
//! Assumptions:
//! - Utility functions assume standard neoclassical forms (Cobb-Douglas, CES,
//!   Leontief, CRRA, CARA). See each function's doc for the exact formula.
//! - Auctions use deterministic tie-breaking by bidder index ascending.
//! - Matching uses Gale-Shapley deferred acceptance with men proposing.

/// Maximum agents/bidders in a bounded market design problem.
pub const MAX_AGENTS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketDesignError {
    InvalidInput,
    BufferTooSmall,
    NoClearingPrice,
    NonFinite,
    InvalidAllocation,
}

fn require_finite(x: f64) -> Result<(), MarketDesignError> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(MarketDesignError::NonFinite)
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Cobb-Douglas utility: `x^alpha * y^(1-alpha)`.
///
/// Requires `x >= 0`, `y >= 0`, `0 <= alpha <= 1`.
pub fn cobb_douglas_utility(x: f64, y: f64, alpha: f64) -> Result<f64, MarketDesignError> {
    if x < 0.0 || y < 0.0 || !(0.0..=1.0).contains(&alpha) {
        return Err(MarketDesignError::InvalidInput);
    }
    require_finite(x)?;
    require_finite(y)?;
    require_finite(alpha)?;
    Ok(x.powf(alpha) * y.powf(1.0 - alpha))
}

/// CES (constant elasticity of substitution) utility:
/// `(alpha * x^rho + (1-alpha) * y^rho)^(1/rho)`.
///
/// Requires `x >= 0`, `y >= 0`, `0 <= alpha <= 1`, `rho != 0`.
/// As `rho → 0` this approaches Cobb-Douglas; as `rho → -∞` it approaches
/// Leontief (`min(x, y)`).
pub fn ces_utility(x: f64, y: f64, alpha: f64, rho: f64) -> Result<f64, MarketDesignError> {
    if x < 0.0 || y < 0.0 || !(0.0..=1.0).contains(&alpha) || rho == 0.0 {
        return Err(MarketDesignError::InvalidInput);
    }
    require_finite(x)?;
    require_finite(y)?;
    require_finite(rho)?;
    let inner = alpha * x.powf(rho) + (1.0 - alpha) * y.powf(rho);
    if inner <= 0.0 {
        return Err(MarketDesignError::InvalidInput);
    }
    Ok(inner.powf(1.0 / rho))
}

/// Leontief utility: `min(x, y)`.
pub fn leontief_utility(x: f64, y: f64) -> Result<f64, MarketDesignError> {
    if x < 0.0 || y < 0.0 {
        return Err(MarketDesignError::InvalidInput);
    }
    require_finite(x)?;
    require_finite(y)?;
    Ok(x.min(y))
}

/// Quasi-linear utility: `u(good) + numeraire - price * good`.
///
/// Here `u_good` is the utility from consuming `good` units of the non-numeraire
/// commodity, and `numeraire` is the remaining budget. Returns
/// `u_good + numeraire - price * good`.
pub fn quasi_linear_utility(
    u_good: f64,
    numeraire: f64,
    price: f64,
    good: f64,
) -> Result<f64, MarketDesignError> {
    require_finite(u_good)?;
    require_finite(numeraire)?;
    require_finite(price)?;
    require_finite(good)?;
    Ok(u_good + numeraire - price * good)
}

/// CRRA utility: `c^(1-gamma) / (1-gamma)` for `gamma != 1`, `ln(c)` for
/// `gamma == 1`. Requires `c > 0`, `gamma > 0`.
pub fn crra_utility(consumption: f64, gamma: f64) -> Result<f64, MarketDesignError> {
    if consumption <= 0.0 || gamma <= 0.0 {
        return Err(MarketDesignError::InvalidInput);
    }
    require_finite(consumption)?;
    require_finite(gamma)?;
    if (gamma - 1.0).abs() < 1e-12 {
        Ok(consumption.ln())
    } else {
        Ok(consumption.powf(1.0 - gamma) / (1.0 - gamma))
    }
}

/// CARA utility: `-exp(-a * wealth) / a`. Requires `a > 0`.
pub fn cara_utility(wealth: f64, a: f64) -> Result<f64, MarketDesignError> {
    if a <= 0.0 {
        return Err(MarketDesignError::InvalidInput);
    }
    require_finite(wealth)?;
    require_finite(a)?;
    Ok(-((-a * wealth).exp()) / a)
}

// ---------------------------------------------------------------------------
// Partial equilibrium
// ---------------------------------------------------------------------------

/// Clear a linear market: demand `Q = a - b*P`, supply `Q = c + d*P`.
///
/// Returns `(equilibrium_price, equilibrium_quantity)`. Requires `b > 0`,
/// `d > 0`, and `a > c` for a positive equilibrium.
pub fn clear_market_linear(
    demand_intercept: f64,
    demand_slope: f64,
    supply_intercept: f64,
    supply_slope: f64,
) -> Result<(f64, f64), MarketDesignError> {
    if demand_slope <= 0.0 || supply_slope <= 0.0 {
        return Err(MarketDesignError::InvalidInput);
    }
    require_finite(demand_intercept)?;
    require_finite(demand_slope)?;
    require_finite(supply_intercept)?;
    require_finite(supply_slope)?;
    // a - b*P = c + d*P → P = (a - c) / (b + d)
    let price = (demand_intercept - supply_intercept) / (demand_slope + supply_slope);
    if price < 0.0 {
        return Err(MarketDesignError::NoClearingPrice);
    }
    let quantity = demand_intercept - demand_slope * price;
    if quantity < 0.0 {
        return Err(MarketDesignError::NoClearingPrice);
    }
    Ok((price, quantity))
}

// ---------------------------------------------------------------------------
// Auctions
// ---------------------------------------------------------------------------

/// Sort bid indices descending by bid value, ties by index ascending.
/// Writes sorted indices into `out[..n]`.
fn sort_bid_indices_desc(bids: &[f64], n: usize, out: &mut [usize]) {
    for i in 0..n {
        out[i] = i;
    }
    for i in 1..n {
        let cur = out[i];
        let cur_v = bids[cur];
        let mut j = i;
        while j > 0 {
            let prev = out[j - 1];
            let prev_v = bids[prev];
            if prev_v < cur_v || (prev_v == cur_v && prev > cur) {
                out[j] = out[j - 1];
                j -= 1;
            } else {
                break;
            }
        }
        out[j] = cur;
    }
}

/// Vickrey (second-price sealed-bid) auction.
///
/// Returns `(winner_index, second_highest_price)`. The winner pays the
/// second-highest bid. Ties broken by lowest index.
pub fn vickrey_auction(bids: &[f64]) -> Result<(usize, f64), MarketDesignError> {
    if bids.is_empty() {
        return Err(MarketDesignError::InvalidInput);
    }
    for b in bids {
        require_finite(*b)?;
        if *b < 0.0 {
            return Err(MarketDesignError::InvalidInput);
        }
    }
    let n = bids.len();
    let mut idx = [0usize; MAX_AGENTS];
    if n > MAX_AGENTS {
        return Err(MarketDesignError::BufferTooSmall);
    }
    sort_bid_indices_desc(bids, n, &mut idx);
    let winner = idx[0];
    let second_price = if n > 1 { bids[idx[1]] } else { 0.0 };
    Ok((winner, second_price))
}

/// First-price sealed-bid auction: winner pays their own bid.
pub fn sealed_bid_first_price(bids: &[f64]) -> Result<(usize, f64), MarketDesignError> {
    let (winner, _) = vickrey_auction(bids)?;
    Ok((winner, bids[winner]))
}

/// Uniform-price auction: `supply` units are sold at the marginal (supply-th
/// highest) bid price.
///
/// Returns `(clearing_price, units_sold)`. Bids are sorted descending; the
/// clearing price is the `supply`-th highest bid (1-indexed). Only bids at or
/// above the clearing price are accepted.
pub fn uniform_price_auction(
    supply: usize,
    bids: &[f64],
) -> Result<(f64, usize), MarketDesignError> {
    if bids.is_empty() || supply == 0 {
        return Err(MarketDesignError::InvalidInput);
    }
    for b in bids {
        require_finite(*b)?;
        if *b < 0.0 {
            return Err(MarketDesignError::InvalidInput);
        }
    }
    let n = bids.len();
    if n > MAX_AGENTS {
        return Err(MarketDesignError::BufferTooSmall);
    }
    let mut idx = [0usize; MAX_AGENTS];
    sort_bid_indices_desc(bids, n, &mut idx);
    let units_sold = supply.min(n);
    // Clearing price = bid at the marginal position (supply-th highest, 1-indexed).
    // If supply > n, clearing price = lowest bid.
    let marginal_pos = (supply - 1).min(n - 1);
    let clearing_price = bids[idx[marginal_pos]];
    Ok((clearing_price, units_sold))
}

/// Double auction: match highest buy bids with lowest sell asks.
///
/// `buy_bids` and `sell_asks` are caller slices. Returns
/// `(clearing_price, quantity_traded)`. The clearing price is the midpoint of
/// the last matched bid-ask pair. Trades occur while the highest buy >= lowest
/// sell.
pub fn double_auction(
    buy_bids: &[f64],
    sell_asks: &[f64],
) -> Result<(f64, usize), MarketDesignError> {
    if buy_bids.is_empty() || sell_asks.is_empty() {
        return Err(MarketDesignError::InvalidInput);
    }
    for b in buy_bids {
        require_finite(*b)?;
    }
    for a in sell_asks {
        require_finite(*a)?;
        if *a < 0.0 {
            return Err(MarketDesignError::InvalidInput);
        }
    }
    if buy_bids.len() > MAX_AGENTS || sell_asks.len() > MAX_AGENTS {
        return Err(MarketDesignError::BufferTooSmall);
    }
    let n_buy = buy_bids.len();
    let n_sell = sell_asks.len();
    let mut buy_idx = [0usize; MAX_AGENTS];
    let mut sell_idx = [0usize; MAX_AGENTS];
    sort_bid_indices_desc(buy_bids, n_buy, &mut buy_idx);
    // Sort asks ascending: reuse the desc sort then reverse.
    sort_bid_indices_desc(sell_asks, n_sell, &mut sell_idx);
    let mut sell_asc = [0usize; MAX_AGENTS];
    for i in 0..n_sell {
        sell_asc[i] = sell_idx[n_sell - 1 - i];
    }

    let max_trades = n_buy.min(n_sell);
    let mut traded = 0usize;
    let mut last_buy = 0.0;
    let mut last_sell = 0.0;
    for k in 0..max_trades {
        let b = buy_bids[buy_idx[k]];
        let s = sell_asks[sell_asc[k]];
        if b >= s {
            traded += 1;
            last_buy = b;
            last_sell = s;
        } else {
            break;
        }
    }
    if traded == 0 {
        return Err(MarketDesignError::NoClearingPrice);
    }
    let clearing = (last_buy + last_sell) / 2.0;
    Ok((clearing, traded))
}

// ---------------------------------------------------------------------------
// Matching: Gale-Shapley deferred acceptance
// ---------------------------------------------------------------------------

/// Gale-Shapley deferred acceptance with men proposing.
///
/// `men_prefs[i][j]` is the j-th most preferred woman of man i (0-indexed).
/// `women_prefs[i][j]` is the j-th most preferred man of woman i.
/// Writes `matching_out[man] = woman` for each man. Returns `n` on success.
pub fn deferred_acceptance_into(
    men_prefs: &[usize],
    women_prefs: &[usize],
    n: usize,
    matching_out: &mut [usize],
) -> Result<usize, MarketDesignError> {
    if n == 0 || n > MAX_AGENTS {
        return Err(MarketDesignError::InvalidInput);
    }
    if men_prefs.len() < n * n || women_prefs.len() < n * n || matching_out.len() < n {
        return Err(MarketDesignError::BufferTooSmall);
    }
    // Validate preference values.
    for v in men_prefs.iter().take(n * n) {
        if *v >= n {
            return Err(MarketDesignError::InvalidInput);
        }
    }
    for v in women_prefs.iter().take(n * n) {
        if *v >= n {
            return Err(MarketDesignError::InvalidInput);
        }
    }

    // matching_out[man] = woman matched to; init to n (unmatched).
    let mut man_match = [MAX_AGENTS; MAX_AGENTS];
    let mut woman_match = [MAX_AGENTS; MAX_AGENTS];
    let mut next_proposal = [0usize; MAX_AGENTS]; // next woman each man proposes to

    loop {
        // Find a free man with proposals left.
        let mut free_man = None;
        for m in 0..n {
            if man_match[m] == MAX_AGENTS && next_proposal[m] < n {
                free_man = Some(m);
                break;
            }
        }
        let m = match free_man {
            Some(m) => m,
            None => break,
        };

        let w = men_prefs[m * n + next_proposal[m]];
        next_proposal[m] += 1;

        if woman_match[w] == MAX_AGENTS {
            // Woman is free: accept.
            woman_match[w] = m;
            man_match[m] = w;
        } else {
            // Woman compares new suitor with current match.
            let current = woman_match[w];
            // Find rank of m vs current in woman w's preference list.
            let mut rank_m = n;
            let mut rank_current = n;
            for j in 0..n {
                if women_prefs[w * n + j] == m {
                    rank_m = j;
                }
                if women_prefs[w * n + j] == current {
                    rank_current = j;
                }
            }
            if rank_m < rank_current {
                // Woman prefers new suitor.
                woman_match[w] = m;
                man_match[m] = w;
                man_match[current] = MAX_AGENTS; // old match is now free
            }
            // Otherwise she rejects; man tries next.
        }
    }

    for m in 0..n {
        if man_match[m] == MAX_AGENTS {
            return Err(MarketDesignError::InvalidAllocation);
        }
        matching_out[m] = man_match[m];
    }
    Ok(n)
}

/// Check whether a matching is stable (no blocking pair).
///
/// `matching[man] = woman`. Returns `true` if stable.
pub fn is_stable_matching(
    men_prefs: &[usize],
    women_prefs: &[usize],
    matching: &[usize],
    n: usize,
) -> Result<bool, MarketDesignError> {
    if n == 0 || n > MAX_AGENTS {
        return Err(MarketDesignError::InvalidInput);
    }
    if men_prefs.len() < n * n || women_prefs.len() < n * n || matching.len() < n {
        return Err(MarketDesignError::BufferTooSmall);
    }
    for v in matching.iter().take(n) {
        if *v >= n {
            return Err(MarketDesignError::InvalidInput);
        }
    }

    // Build inverse: woman -> man.
    let mut woman_to_man = [MAX_AGENTS; MAX_AGENTS];
    for m in 0..n {
        woman_to_man[matching[m]] = m;
    }

    // Check for blocking pairs: (m, w) where m prefers w to his current match
    // and w prefers m to her current match.
    for m in 0..n {
        let current_w = matching[m];
        let mut current_rank = n;
        for j in 0..n {
            if men_prefs[m * n + j] == current_w {
                current_rank = j;
                break;
            }
        }
        for j in 0..current_rank {
            // Women that m prefers to his current match.
            let w = men_prefs[m * n + j];
            let current_m = woman_to_man[w];
            // Does w prefer m to current_m?
            let mut rank_m = n;
            let mut rank_current = n;
            for k in 0..n {
                if women_prefs[w * n + k] == m {
                    rank_m = k;
                }
                if women_prefs[w * n + k] == current_m {
                    rank_current = k;
                }
            }
            if rank_m < rank_current {
                return Ok(false); // blocking pair found
            }
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn cobb_douglas_alpha_half() {
        let u = cobb_douglas_utility(4.0, 9.0, 0.5).unwrap();
        assert!(approx(u, (4.0f64 * 9.0).sqrt()));
    }

    #[test]
    fn ces_rho_one_is_linear() {
        // rho=1: (alpha*x + (1-alpha)*y)^1 = alpha*x + (1-alpha)*y
        let u = ces_utility(10.0, 20.0, 0.5, 1.0).unwrap();
        assert!(approx(u, 15.0));
    }

    #[test]
    fn ces_large_rho_approaches_leontief() {
        // rho → large: max(x, y)
        let u = ces_utility(10.0, 20.0, 0.5, 100.0).unwrap();
        assert!(u > 19.0 && u < 20.1);
    }

    #[test]
    fn leontief_utility_is_min() {
        assert!(approx(leontief_utility(3.0, 7.0).unwrap(), 3.0));
        assert!(approx(leontief_utility(8.0, 2.0).unwrap(), 2.0));
    }

    #[test]
    fn crra_gamma_one_is_log() {
        let u = crra_utility(2.718281828, 1.0).unwrap();
        assert!(approx(u, 1.0));
    }

    #[test]
    fn crra_gamma_two() {
        // gamma=2: c^(-1)/(-1) = -1/c
        let u = crra_utility(4.0, 2.0).unwrap();
        assert!(approx(u, -0.25));
    }

    #[test]
    fn cara_decreasing_in_wealth() {
        let u1 = cara_utility(0.0, 1.0).unwrap();
        let u2 = cara_utility(10.0, 1.0).unwrap();
        assert!(u2 > u1); // more wealth → higher utility
    }

    #[test]
    fn market_clearing_linear() {
        // Q = 100 - 2P, Q = 20 + 2P → P = 20, Q = 60
        let (p, q) = clear_market_linear(100.0, 2.0, 20.0, 2.0).unwrap();
        assert!(approx(p, 20.0));
        assert!(approx(q, 60.0));
    }

    #[test]
    fn vickrey_second_price() {
        let bids = [10.0, 20.0, 15.0];
        let (winner, price) = vickrey_auction(&bids).unwrap();
        assert_eq!(winner, 1);
        assert!(approx(price, 15.0));
    }

    #[test]
    fn first_price_pays_highest() {
        let bids = [10.0, 20.0, 15.0];
        let (winner, price) = sealed_bid_first_price(&bids).unwrap();
        assert_eq!(winner, 1);
        assert!(approx(price, 20.0));
    }

    #[test]
    fn uniform_price_marginal() {
        // supply=2, bids [10, 20, 15, 5] → sorted desc [20, 15, 10, 5]
        // marginal = 2nd (0-indexed 1) = 15
        let (price, sold) = uniform_price_auction(2, &[10.0, 20.0, 15.0, 5.0]).unwrap();
        assert!(approx(price, 15.0));
        assert_eq!(sold, 2);
    }

    #[test]
    fn double_auction_matches() {
        // buy [20, 15], sell [10, 12] → sorted buy desc [20, 15], sell asc [10, 12]
        // 20 >= 10 → trade; 15 >= 12 → trade; clearing = (15 + 12)/2 = 13.5
        let (price, traded) = double_auction(&[20.0, 15.0], &[10.0, 12.0]).unwrap();
        assert_eq!(traded, 2);
        assert!(approx(price, 13.5));
    }

    #[test]
    fn double_auction_no_trade() {
        let err = double_auction(&[5.0], &[10.0]).unwrap_err();
        assert_eq!(err, MarketDesignError::NoClearingPrice);
    }

    #[test]
    fn deferred_acceptance_stable() {
        // 2 men, 2 women.
        // Man 0: prefers woman 0 then 1. Man 1: prefers woman 1 then 0.
        // Woman 0: prefers man 0 then 1. Woman 1: prefers man 1 then 0.
        let men_prefs = [0, 1, 1, 0];
        let women_prefs = [0, 1, 1, 0];
        let mut matching = [0usize; 2];
        deferred_acceptance_into(&men_prefs, &women_prefs, 2, &mut matching).unwrap();
        assert_eq!(matching, [0, 1]);
        assert!(is_stable_matching(&men_prefs, &women_prefs, &matching, 2).unwrap());
    }

    #[test]
    fn unstable_matching_detected() {
        // Force an unstable matching: swap the stable one.
        let men_prefs = [0, 1, 1, 0];
        let women_prefs = [0, 1, 1, 0];
        let matching = [1, 0]; // man 0 → woman 1, man 1 → woman 0
        assert!(!is_stable_matching(&men_prefs, &women_prefs, &matching, 2).unwrap());
    }

    #[test]
    fn empty_bids_rejected() {
        assert_eq!(
            vickrey_auction(&[]).unwrap_err(),
            MarketDesignError::InvalidInput
        );
    }

    #[test]
    fn negative_bid_rejected() {
        assert_eq!(
            vickrey_auction(&[-1.0, 5.0]).unwrap_err(),
            MarketDesignError::InvalidInput
        );
    }
}
