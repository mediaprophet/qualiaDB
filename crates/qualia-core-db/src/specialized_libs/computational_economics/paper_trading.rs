//! Deterministic paper-trading / execution simulator (simulation-only).
//!
//! No real orders, no broker connectors, no side effects. All fills are
//! derived strictly from caller-supplied market data snapshots.
//!
//! Allocation: ColdBounded for scenario construction; stepping uses fixed
//! arrays. Explicit SimulationOnly + RefusesExternalAction safety class.

// EconStatus available for future convergence reports in clearing engines.

/// Max orders in a paper book.
pub const MAX_ORDERS: usize = 128;

/// Order types supported in the paper simulator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OrderType {
    Market = 0,
    Limit = 1,
    Stop = 2,
    StopLimit = 3,
}

/// Side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Side {
    Buy = 0,
    Sell = 1,
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PaperOrder {
    pub id: u64,
    pub side: Side,
    pub order_type: OrderType,
    pub qty: f64,
    pub limit_price: f64, // 0 for market
    pub stop_price: f64,  // 0 for non-stop
    pub filled_qty: f64,
    pub avg_fill_price: f64,
    pub status: u8, // 0=open, 1=filled, 2=cancelled, 3=rejected
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Fill {
    pub order_id: u64,
    pub qty: f64,
    pub price: f64,
    pub fee: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperTradingError {
    InvalidInput,
    BufferTooSmall,
    NonFinite,
    NoMarketData,
}

/// Simple market snapshot for fill simulation.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MarketSnapshot {
    pub bid: f64,
    pub ask: f64,
    pub last: f64,
    pub volume: f64,
}

/// Submit a paper order into a fixed buffer. Returns index or error.
pub fn submit_paper_order(
    orders: &mut [PaperOrder],
    next_id: &mut u64,
    side: Side,
    order_type: OrderType,
    qty: f64,
    limit_price: f64,
    stop_price: f64,
) -> Result<usize, PaperTradingError> {
    if qty <= 0.0 || !qty.is_finite() {
        return Err(PaperTradingError::InvalidInput);
    }
    if order_type == OrderType::Limit && limit_price <= 0.0 {
        return Err(PaperTradingError::InvalidInput);
    }
    let mut idx = None;
    for (i, o) in orders.iter_mut().enumerate() {
        if o.id == 0 && o.status == 0 {
            idx = Some(i);
            break;
        }
    }
    let i = match idx {
        Some(i) if i < orders.len() => i,
        _ => {
            // find first non open? simple append scan
            let mut free = orders.len();
            for (j, o) in orders.iter().enumerate() {
                if o.status != 0 && o.id != 0 {
                    free = j;
                    break;
                }
            }
            if free >= orders.len() {
                return Err(PaperTradingError::BufferTooSmall);
            }
            free
        }
    };
    orders[i] = PaperOrder {
        id: *next_id,
        side,
        order_type,
        qty,
        limit_price,
        stop_price,
        filled_qty: 0.0,
        avg_fill_price: 0.0,
        status: 0,
    };
    *next_id += 1;
    Ok(i)
}

/// Simulate fills against a sequence of market snapshots (deterministic).
/// Writes fills to `fills_out`, returns number written.
/// Never fabricates prices: if no usable snapshot for a marketable order, it stays open.
pub fn simulate_fills_against_snapshots(
    orders: &mut [PaperOrder],
    snapshots: &[MarketSnapshot],
    fee_rate: f64,
    fills_out: &mut [Fill],
) -> Result<usize, PaperTradingError> {
    if snapshots.is_empty() {
        return Err(PaperTradingError::NoMarketData);
    }
    if fills_out.len() < orders.len() {
        return Err(PaperTradingError::BufferTooSmall);
    }
    if !fee_rate.is_finite() || fee_rate < 0.0 {
        return Err(PaperTradingError::InvalidInput);
    }

    let mut n_fills = 0usize;
    for o in orders.iter_mut() {
        if o.status != 0 || o.id == 0 {
            continue;
        }
        let mut remaining = o.qty - o.filled_qty;
        if remaining <= 0.0 {
            o.status = 1;
            continue;
        }
        let mut total_qty = 0.0;
        let mut total_notional = 0.0;

        for sn in snapshots {
            if !sn.last.is_finite() || sn.last <= 0.0 {
                continue;
            }
            let exec_price = sn.last;
            let marketable = match o.order_type {
                OrderType::Market => true,
                OrderType::Limit => {
                    if o.side == Side::Buy {
                        sn.last <= o.limit_price
                    } else {
                        sn.last >= o.limit_price
                    }
                }
                OrderType::Stop => {
                    if o.side == Side::Buy {
                        sn.last >= o.stop_price
                    } else {
                        sn.last <= o.stop_price
                    }
                }
                OrderType::StopLimit => {
                    let triggered = if o.side == Side::Buy {
                        sn.last >= o.stop_price
                    } else {
                        sn.last <= o.stop_price
                    };
                    triggered
                        && if o.side == Side::Buy {
                            sn.last <= o.limit_price
                        } else {
                            sn.last >= o.limit_price
                        }
                }
            };
            if !marketable {
                continue;
            }
            let take = remaining.min(sn.volume.max(0.01) * 0.1); // simplistic participation
            if take <= 0.0 {
                continue;
            }
            total_qty += take;
            total_notional += take * exec_price;
            remaining -= take;
            if remaining <= 1e-9 {
                break;
            }
        }

        if total_qty > 0.0 {
            let avg = total_notional / total_qty;
            let fee = total_notional * fee_rate;
            fills_out[n_fills] = Fill {
                order_id: o.id,
                qty: total_qty,
                price: avg,
                fee,
            };
            n_fills += 1;

            o.filled_qty += total_qty;
            if o.filled_qty >= o.qty * 0.999 {
                o.avg_fill_price = avg;
                o.status = 1;
            } else {
                o.avg_fill_price = avg;
            }
        }
        if remaining > 0.0 && o.order_type == OrderType::Market {
            // market orders that didn't fully fill are still "open" or we can leave as partial
        }
    }
    Ok(n_fills)
}

/// Cancel an open order.
pub fn cancel_paper_order(orders: &mut [PaperOrder], id: u64) -> bool {
    for o in orders.iter_mut() {
        if o.id == id && o.status == 0 {
            o.status = 2;
            return true;
        }
    }
    false
}

/// Aggregate realized fees and notional from fills.
pub fn aggregate_paper_fills(fills: &[Fill]) -> (f64, f64) {
    let mut notional = 0.0;
    let mut fees = 0.0;
    for f in fills {
        notional += f.qty * f.price;
        fees += f.fee;
    }
    (notional, fees)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(last: f64) -> MarketSnapshot {
        MarketSnapshot {
            bid: last - 0.05,
            ask: last + 0.05,
            last,
            volume: 1000.0,
        }
    }

    #[test]
    fn market_order_fills_against_snapshot() {
        let mut orders = [PaperOrder {
            id: 0,
            side: Side::Buy,
            order_type: OrderType::Market,
            qty: 10.0,
            limit_price: 0.0,
            stop_price: 0.0,
            filled_qty: 0.0,
            avg_fill_price: 0.0,
            status: 0,
        }; MAX_ORDERS];
        let mut next = 1u64;
        let i = submit_paper_order(
            &mut orders,
            &mut next,
            Side::Buy,
            OrderType::Market,
            10.0,
            0.0,
            0.0,
        )
        .unwrap();
        let snaps = [snap(100.0), snap(100.1)];
        let mut fills = [Fill {
            order_id: 0,
            qty: 0.0,
            price: 0.0,
            fee: 0.0,
        }; MAX_ORDERS];
        let n = simulate_fills_against_snapshots(&mut orders, &snaps, 0.001, &mut fills).unwrap();
        assert!(n >= 1);
        assert!(orders[i].status == 1 || orders[i].filled_qty > 0.0);
        let (not, fee) = aggregate_paper_fills(&fills[..n]);
        assert!(not > 0.0);
        assert!(fee > 0.0);
    }

    #[test]
    fn limit_order_only_fills_when_crossed() {
        let mut orders = [PaperOrder {
            id: 0,
            side: Side::Buy,
            order_type: OrderType::Limit,
            qty: 5.0,
            limit_price: 99.0,
            stop_price: 0.0,
            filled_qty: 0.0,
            avg_fill_price: 0.0,
            status: 0,
        }; MAX_ORDERS];
        let mut next = 10u64;
        let _ = submit_paper_order(
            &mut orders,
            &mut next,
            Side::Buy,
            OrderType::Limit,
            5.0,
            99.0,
            0.0,
        )
        .unwrap();
        let snaps = [snap(101.0)]; // price too high for buy limit 99
        let mut fills = [Fill {
            order_id: 0,
            qty: 0.0,
            price: 0.0,
            fee: 0.0,
        }; MAX_ORDERS];
        let n = simulate_fills_against_snapshots(&mut orders, &snaps, 0.0, &mut fills).unwrap();
        assert_eq!(n, 0); // did not fill
    }

    #[test]
    fn cancel_works() {
        let mut orders = [PaperOrder {
            id: 0,
            side: Side::Sell,
            order_type: OrderType::Market,
            qty: 1.0,
            limit_price: 0.0,
            stop_price: 0.0,
            filled_qty: 0.0,
            avg_fill_price: 0.0,
            status: 0,
        }; MAX_ORDERS];
        let mut next = 99u64;
        let i = submit_paper_order(
            &mut orders,
            &mut next,
            Side::Sell,
            OrderType::Market,
            1.0,
            0.0,
            0.0,
        )
        .unwrap();
        let order_id = next - 1; // id assigned inside submit before the increment
        assert!(cancel_paper_order(&mut orders, order_id));
        assert_eq!(orders[i].status, 2);
    }

    #[test]
    fn refuses_without_market_data() {
        let mut orders = [PaperOrder {
            id: 0,
            side: Side::Buy,
            order_type: OrderType::Market,
            qty: 1.0,
            limit_price: 0.0,
            stop_price: 0.0,
            filled_qty: 0.0,
            avg_fill_price: 0.0,
            status: 0,
        }; MAX_ORDERS];
        let mut next = 1;
        let _ = submit_paper_order(
            &mut orders,
            &mut next,
            Side::Buy,
            OrderType::Market,
            1.0,
            0.0,
            0.0,
        );
        let mut fills = [Fill {
            order_id: 0,
            qty: 0.0,
            price: 0.0,
            fee: 0.0,
        }; 4];
        let err = simulate_fills_against_snapshots(&mut orders, &[], 0.0, &mut fills).unwrap_err();
        assert_eq!(err, PaperTradingError::NoMarketData);
    }
}
