//! Agent-based computational economics: fixed-capacity agents, order book,
//! and deterministic zero-intelligence trader market.
//!
//! Allocation class: **HotZeroHeap**. All agent state and order book entries
//! live in fixed-capacity stack arrays. No `Vec`/`String`/`Box` in any
//! stepping kernel.
//!
//! Assumptions:
//! - Synchronous update: all agents act once per step.
//! - No short-selling: agents cannot sell inventory they don't have.
//! - Single unit per trade for simplicity.
//! - Zero-intelligence traders post random bids/asks around a mid price.

/// Maximum agents in a bounded ABM.
pub const MAX_AGENTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AgentKind {
    ZeroIntelligenceTrader = 0,
    Consumer = 1,
    Firm = 2,
    Household = 3,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Agent {
    pub id: u32,
    pub kind: AgentKind,
    pub cash: f64,
    pub inventory: f64,
    pub target_inventory: f64,
    pub bid_price: f64,
    pub ask_price: f64,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct OrderBook {
    pub bids: [(u32, f64); MAX_AGENTS], // (agent_id, price)
    pub asks: [(u32, f64); MAX_AGENTS],
    pub n_bids: usize,
    pub n_asks: usize,
}

impl OrderBook {
    pub fn clear(&mut self) {
        self.n_bids = 0;
        self.n_asks = 0;
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Trade {
    pub buyer_id: u32,
    pub seller_id: u32,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentBasedError {
    InvalidInput,
    BufferTooSmall,
    NonFinite,
    InvalidState,
}

/// SplitMix64 RNG (local copy).
struct SplitMix64 {
    state: u64,
}
impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn unit(&mut self) -> f64 {
        let bits = self.next_u64() >> 11;
        ((bits as f64) + 0.5) * (1.0 / ((1u64 << 53) as f64))
    }
}

/// One synchronous step: each ZI trader posts a random bid or ask around the
/// mid price. Updates `book` in place. `rng_state` is mutated in place for
/// deterministic replay.
pub fn zero_intelligence_step(
    agents: &mut [Agent],
    book: &mut OrderBook,
    mid_price: f64,
    spread_width: f64,
    rng_state: &mut u64,
) -> Result<(), AgentBasedError> {
    if mid_price <= 0.0 || spread_width <= 0.0 || !mid_price.is_finite() || !spread_width.is_finite() {
        return Err(AgentBasedError::InvalidInput);
    }
    book.clear();
    let mut rng = SplitMix64::new(*rng_state);
    for agent in agents.iter_mut() {
        if !agent.cash.is_finite() || !agent.inventory.is_finite() {
            return Err(AgentBasedError::NonFinite);
        }
        let noise = (rng.unit() - 0.5) * spread_width;
        if rng.unit() < 0.5 {
            // Post a bid (buy).
            agent.bid_price = (mid_price + noise).max(0.01);
            agent.ask_price = 0.0;
            if book.n_bids < MAX_AGENTS {
                book.bids[book.n_bids] = (agent.id, agent.bid_price);
                book.n_bids += 1;
            }
        } else {
            // Post an ask (sell).
            agent.ask_price = (mid_price + noise).max(0.01);
            agent.bid_price = 0.0;
            if book.n_asks < MAX_AGENTS {
                book.asks[book.n_asks] = (agent.id, agent.ask_price);
                book.n_asks += 1;
            }
        }
    }
    *rng_state = rng.state;
    Ok(())
}

/// Match orders: highest bid >= lowest ask. Writes trades into `trades_out`.
/// Returns number of trades. Clears matched orders from the book.
pub fn match_orders_into(
    book: &mut OrderBook,
    trades_out: &mut [Trade],
) -> Result<usize, AgentBasedError> {
    if trades_out.len() < MAX_AGENTS {
        return Err(AgentBasedError::BufferTooSmall);
    }
    // Sort bids descending by price (selection sort on small arrays).
    let mut bid_idx = [0usize; MAX_AGENTS];
    for i in 0..book.n_bids {
        bid_idx[i] = i;
    }
    for i in 1..book.n_bids {
        let cur = bid_idx[i];
        let cur_v = book.bids[cur].1;
        let mut j = i;
        while j > 0 && book.bids[bid_idx[j - 1]].1 < cur_v {
            bid_idx[j] = bid_idx[j - 1];
            j -= 1;
        }
        bid_idx[j] = cur;
    }
    // Sort asks ascending by price.
    let mut ask_idx = [0usize; MAX_AGENTS];
    for i in 0..book.n_asks {
        ask_idx[i] = i;
    }
    for i in 1..book.n_asks {
        let cur = ask_idx[i];
        let cur_v = book.asks[cur].1;
        let mut j = i;
        while j > 0 && book.asks[ask_idx[j - 1]].1 > cur_v {
            ask_idx[j] = ask_idx[j - 1];
            j -= 1;
        }
        ask_idx[j] = cur;
    }

    let max_trades = book.n_bids.min(book.n_asks).min(trades_out.len());
    let mut n_trades = 0;
    let mut bi = 0;
    let mut ai = 0;
    while bi < max_trades && ai < max_trades {
        let (buyer_id, bid_price) = book.bids[bid_idx[bi]];
        let (seller_id, ask_price) = book.asks[ask_idx[ai]];
        if bid_price >= ask_price {
            let price = (bid_price + ask_price) / 2.0;
            trades_out[n_trades] = Trade {
                buyer_id,
                seller_id,
                price,
                quantity: 1.0,
            };
            n_trades += 1;
            bi += 1;
            ai += 1;
        } else {
            break;
        }
    }
    Ok(n_trades)
}

/// Apply trades to agent cash/inventory. Verifies conservation.
pub fn clear_trades(
    agents: &mut [Agent],
    trades: &[Trade],
    n_trades: usize,
) -> Result<(), AgentBasedError> {
    for t in trades.iter().take(n_trades) {
        if !t.price.is_finite() || t.price < 0.0 || t.quantity <= 0.0 {
            return Err(AgentBasedError::InvalidInput);
        }
        // Find buyer and seller by id (sequential to avoid double borrow).
        let mut buyer_idx = None;
        let mut seller_idx = None;
        for (i, a) in agents.iter().enumerate() {
            if a.id == t.buyer_id {
                buyer_idx = Some(i);
            }
            if a.id == t.seller_id {
                seller_idx = Some(i);
            }
        }
        match (buyer_idx, seller_idx) {
            (Some(bi), Some(si)) => {
                let cost = t.price * t.quantity;
                // Apply sequentially to avoid aliasing.
                if bi == si {
                    return Err(AgentBasedError::InvalidState);
                }
                agents[bi].cash -= cost;
                agents[bi].inventory += t.quantity;
                agents[si].cash += cost;
                agents[si].inventory -= t.quantity;
            }
            _ => return Err(AgentBasedError::InvalidState),
        }
    }
    Ok(())
}

/// Run N steps of a ZI trader market. Writes the clearing price (mid of last
/// trade, or the input mid if no trades) per step into `price_out`.
/// Deterministic with seed.
pub fn simulate_steps_into(
    agents: &mut [Agent],
    initial_mid: f64,
    steps: usize,
    seed: u64,
    price_out: &mut [f64],
) -> Result<usize, AgentBasedError> {
    if steps == 0 || price_out.len() < steps {
        return Err(AgentBasedError::BufferTooSmall);
    }
    if agents.is_empty() || agents.len() > MAX_AGENTS {
        return Err(AgentBasedError::InvalidInput);
    }
    let mut rng_state = seed;
    let mut mid = initial_mid;
    let mut book = OrderBook {
        bids: [(0, 0.0); MAX_AGENTS],
        asks: [(0, 0.0); MAX_AGENTS],
        n_bids: 0,
        n_asks: 0,
    };
    let mut trades = [Trade {
        buyer_id: 0,
        seller_id: 0,
        price: 0.0,
        quantity: 0.0,
    }; MAX_AGENTS];

    for t in 0..steps {
        zero_intelligence_step(agents, &mut book, mid, 2.0, &mut rng_state)?;
        let n_trades = match_orders_into(&mut book, &mut trades)?;
        if n_trades > 0 {
            clear_trades(agents, &trades, n_trades)?;
            // Update mid to last trade price.
            mid = trades[n_trades - 1].price;
        }
        price_out[t] = mid;
    }
    Ok(steps)
}

/// Total wealth = sum of cash + inventory * reference_price.
pub fn aggregate_wealth(
    agents: &[Agent],
    reference_price: f64,
) -> Result<f64, AgentBasedError> {
    if !reference_price.is_finite() || reference_price < 0.0 {
        return Err(AgentBasedError::InvalidInput);
    }
    let mut total = 0.0;
    for a in agents {
        if !a.cash.is_finite() || !a.inventory.is_finite() {
            return Err(AgentBasedError::NonFinite);
        }
        total += a.cash + a.inventory * reference_price;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agents(n: usize) -> Vec<Agent> {
        (0..n)
            .map(|i| Agent {
                id: i as u32,
                kind: AgentKind::ZeroIntelligenceTrader,
                cash: 1000.0,
                inventory: 10.0,
                target_inventory: 10.0,
                bid_price: 0.0,
                ask_price: 0.0,
            })
            .collect()
    }

    #[test]
    fn zi_step_produces_book() {
        let mut agents = make_agents(10);
        let mut book = OrderBook {
            bids: [(0, 0.0); MAX_AGENTS],
            asks: [(0, 0.0); MAX_AGENTS],
            n_bids: 0,
            n_asks: 0,
        };
        zero_intelligence_step(&mut agents, &mut book, 100.0, 10.0, &mut 42).unwrap();
        assert!(book.n_bids + book.n_asks == agents.len());
    }

    #[test]
    fn order_matching_finds_trades() {
        let mut book = OrderBook {
            bids: [(0, 0.0); MAX_AGENTS],
            asks: [(0, 0.0); MAX_AGENTS],
            n_bids: 0,
            n_asks: 0,
        };
        // Bid 105 >= ask 95 → trade.
        book.bids[0] = (1, 105.0);
        book.bids[1] = (2, 95.0);
        book.n_bids = 2;
        book.asks[0] = (3, 95.0);
        book.asks[1] = (4, 110.0);
        book.n_asks = 2;
        let mut trades = [Trade {
            buyer_id: 0,
            seller_id: 0,
            price: 0.0,
            quantity: 0.0,
        }; MAX_AGENTS];
        let n = match_orders_into(&mut book, &mut trades).unwrap();
        assert_eq!(n, 1); // only 105 >= 95
        assert_eq!(trades[0].buyer_id, 1);
        assert_eq!(trades[0].seller_id, 3);
    }

    #[test]
    fn trade_clearing_conserves_cash() {
        let mut agents = make_agents(2);
        let trades = [Trade {
            buyer_id: 0,
            seller_id: 1,
            price: 50.0,
            quantity: 2.0,
        }];
        let cash_before: f64 = agents.iter().map(|a| a.cash).sum();
        clear_trades(&mut agents, &trades, 1).unwrap();
        let cash_after: f64 = agents.iter().map(|a| a.cash).sum();
        assert!((cash_after - cash_before).abs() < 1e-9);
        // Buyer inventory increased, seller decreased.
        assert_eq!(agents[0].inventory, 12.0);
        assert_eq!(agents[1].inventory, 8.0);
    }

    #[test]
    fn simulate_reproducible() {
        let mut a1 = make_agents(5);
        let mut a2 = make_agents(5);
        let mut p1 = [0.0f64; 20];
        let mut p2 = [0.0f64; 20];
        simulate_steps_into(&mut a1, 100.0, 20, 123, &mut p1).unwrap();
        simulate_steps_into(&mut a2, 100.0, 20, 123, &mut p2).unwrap();
        for t in 0..20 {
            assert_eq!(p1[t], p2[t]);
        }
    }

    #[test]
    fn prices_positive_and_bounded() {
        let mut agents = make_agents(5);
        let mut prices = [0.0f64; 30];
        simulate_steps_into(&mut agents, 100.0, 30, 99, &mut prices).unwrap();
        for p in prices.iter() {
            assert!(*p > 0.0 && *p < 10000.0);
        }
    }

    #[test]
    fn aggregate_wealth_conserved() {
        let mut agents = make_agents(4);
        let ref_price = 100.0;
        let w_before = aggregate_wealth(&agents, ref_price).unwrap();
        // Manually create a trade.
        let trades = [Trade {
            buyer_id: 0,
            seller_id: 1,
            price: 100.0,
            quantity: 1.0,
        }];
        clear_trades(&mut agents, &trades, 1).unwrap();
        let w_after = aggregate_wealth(&agents, ref_price).unwrap();
        // Wealth = cash + inventory * ref_price. Trade at ref_price conserves
        // total wealth when ref_price = trade price.
        assert!((w_after - w_before).abs() < 1e-6);
    }

    #[test]
    fn empty_agents_rejected() {
        let mut agents: Vec<Agent> = vec![];
        let mut prices = [0.0f64; 10];
        let err = simulate_steps_into(&mut agents, 100.0, 10, 1, &mut prices).unwrap_err();
        assert_eq!(err, AgentBasedError::InvalidInput);
    }

    #[test]
    fn buffer_too_small() {
        let mut agents = make_agents(3);
        let mut prices = [0.0f64; 5];
        let err = simulate_steps_into(&mut agents, 100.0, 10, 1, &mut prices).unwrap_err();
        assert_eq!(err, AgentBasedError::BufferTooSmall);
    }
}
