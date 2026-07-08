//! Deterministic market-data primitives.
//!
//! This module intentionally works only from supplied bars and supplied
//! corporate actions. It never fabricates missing prices, live data, or
//! adjustment evidence.

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct MarketBar {
    pub timestamp: u64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorporateActionKind {
    Split,
    CashDividend,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct CorporateAction {
    pub effective_timestamp: u64,
    pub kind: CorporateActionKind,
    /// Split ratio for `Split`; cash amount per share for `CashDividend`.
    pub value: f64,
    /// Hash of the source/vendor/evidence record. Zero means absent evidence.
    pub source_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketDataError {
    InvalidInput,
    MissingProvenance,
    MissingPreEventPrice,
    OutputBufferTooSmall,
}

fn valid_price(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

fn valid_nonnegative(x: f64) -> bool {
    x.is_finite() && x >= 0.0
}

fn validate_bars(bars: &[MarketBar]) -> Result<(), MarketDataError> {
    if bars.is_empty() {
        return Err(MarketDataError::InvalidInput);
    }

    let mut prev_timestamp = 0u64;
    for (idx, bar) in bars.iter().enumerate() {
        if idx > 0 && bar.timestamp <= prev_timestamp {
            return Err(MarketDataError::InvalidInput);
        }
        if !valid_price(bar.open)
            || !valid_price(bar.high)
            || !valid_price(bar.low)
            || !valid_price(bar.close)
            || !valid_nonnegative(bar.volume)
            || bar.high < bar.low
        {
            return Err(MarketDataError::InvalidInput);
        }
        prev_timestamp = bar.timestamp;
    }
    Ok(())
}

fn previous_close(bars: &[MarketBar], effective_timestamp: u64) -> Option<f64> {
    let mut close = None;
    for bar in bars {
        if bar.timestamp >= effective_timestamp {
            break;
        }
        close = Some(bar.close);
    }
    close
}

/// Backward adjustment factors for supplied bars.
///
/// A 2-for-1 split applies factor `0.5` to bars before the split timestamp.
/// A cash dividend applies the standard total-return factor
/// `(previous_close - dividend) / previous_close` to bars before the ex-date.
pub fn adjustment_factors_into(
    bars: &[MarketBar],
    actions: &[CorporateAction],
    out: &mut [f64],
) -> Result<usize, MarketDataError> {
    validate_bars(bars)?;
    if out.len() < bars.len() {
        return Err(MarketDataError::OutputBufferTooSmall);
    }

    for factor in out.iter_mut().take(bars.len()) {
        *factor = 1.0;
    }

    for action in actions {
        if action.source_hash == 0 {
            return Err(MarketDataError::MissingProvenance);
        }
        if !valid_price(action.value) {
            return Err(MarketDataError::InvalidInput);
        }

        let factor = match action.kind {
            CorporateActionKind::Split => 1.0 / action.value,
            CorporateActionKind::CashDividend => {
                let prior = previous_close(bars, action.effective_timestamp)
                    .ok_or(MarketDataError::MissingPreEventPrice)?;
                if action.value >= prior {
                    return Err(MarketDataError::InvalidInput);
                }
                (prior - action.value) / prior
            }
        };

        for (idx, bar) in bars.iter().enumerate() {
            if bar.timestamp < action.effective_timestamp {
                out[idx] *= factor;
            }
        }
    }

    Ok(bars.len())
}

/// Adjusted close series from supplied bars and corporate actions.
pub fn adjusted_close_into(
    bars: &[MarketBar],
    actions: &[CorporateAction],
    out: &mut [f64],
) -> Result<usize, MarketDataError> {
    let n = adjustment_factors_into(bars, actions, out)?;
    for idx in 0..n {
        out[idx] *= bars[idx].close;
    }
    Ok(n)
}

/// Simple returns `p_t / p_(t-1) - 1` into caller-owned output.
pub fn simple_returns_into(prices: &[f64], out: &mut [f64]) -> Result<usize, MarketDataError> {
    if prices.len() < 2 {
        return Err(MarketDataError::InvalidInput);
    }
    if out.len() + 1 < prices.len() {
        return Err(MarketDataError::OutputBufferTooSmall);
    }

    for (idx, pair) in prices.windows(2).enumerate() {
        if !valid_price(pair[0]) || !valid_price(pair[1]) {
            return Err(MarketDataError::InvalidInput);
        }
        out[idx] = pair[1] / pair[0] - 1.0;
    }
    Ok(prices.len() - 1)
}

/// Log returns `ln(p_t / p_(t-1))` into caller-owned output.
pub fn log_returns_into(prices: &[f64], out: &mut [f64]) -> Result<usize, MarketDataError> {
    if prices.len() < 2 {
        return Err(MarketDataError::InvalidInput);
    }
    if out.len() + 1 < prices.len() {
        return Err(MarketDataError::OutputBufferTooSmall);
    }

    for (idx, pair) in prices.windows(2).enumerate() {
        if !valid_price(pair[0]) || !valid_price(pair[1]) {
            return Err(MarketDataError::InvalidInput);
        }
        out[idx] = (pair[1] / pair[0]).ln();
    }
    Ok(prices.len() - 1)
}

/// Volume-weighted average close price over supplied bars.
pub fn close_vwap(bars: &[MarketBar]) -> Result<f64, MarketDataError> {
    validate_bars(bars)?;
    let mut weighted = 0.0;
    let mut total_volume = 0.0;
    for bar in bars {
        weighted += bar.close * bar.volume;
        total_volume += bar.volume;
    }
    if total_volume <= 0.0 {
        return Err(MarketDataError::InvalidInput);
    }
    Ok(weighted / total_volume)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(timestamp: u64, close: f64, volume: f64) -> MarketBar {
        MarketBar {
            timestamp,
            open: close,
            high: close,
            low: close,
            close,
            volume,
        }
    }

    #[test]
    fn split_adjusts_prior_closes_only() {
        let bars = [bar(1, 100.0, 10.0), bar(2, 110.0, 10.0), bar(3, 60.0, 10.0)];
        let actions = [CorporateAction {
            effective_timestamp: 3,
            kind: CorporateActionKind::Split,
            value: 2.0,
            source_hash: 42,
        }];
        let mut out = [0.0; 3];
        let n = adjusted_close_into(&bars, &actions, &mut out).unwrap();
        assert_eq!(n, 3);
        assert_eq!(out, [50.0, 55.0, 60.0]);
    }

    #[test]
    fn cash_dividend_applies_total_return_factor() {
        let bars = [bar(1, 100.0, 10.0), bar(2, 98.0, 10.0)];
        let actions = [CorporateAction {
            effective_timestamp: 2,
            kind: CorporateActionKind::CashDividend,
            value: 2.0,
            source_hash: 99,
        }];
        let mut out = [0.0; 2];
        adjusted_close_into(&bars, &actions, &mut out).unwrap();
        assert!((out[0] - 98.0).abs() < 1e-12);
        assert!((out[1] - 98.0).abs() < 1e-12);
    }

    #[test]
    fn returns_are_computed_from_adjusted_prices() {
        let prices = [50.0, 55.0, 60.5];
        let mut out = [0.0; 2];
        let n = simple_returns_into(&prices, &mut out).unwrap();
        assert_eq!(n, 2);
        assert!((out[0] - 0.1).abs() < 1e-12);
        assert!((out[1] - 0.1).abs() < 1e-12);
    }

    #[test]
    fn action_without_provenance_is_rejected() {
        let bars = [bar(1, 100.0, 10.0), bar(2, 50.0, 10.0)];
        let actions = [CorporateAction {
            effective_timestamp: 2,
            kind: CorporateActionKind::Split,
            value: 2.0,
            source_hash: 0,
        }];
        let mut out = [0.0; 2];
        assert_eq!(
            adjusted_close_into(&bars, &actions, &mut out),
            Err(MarketDataError::MissingProvenance)
        );
    }

    #[test]
    fn unsorted_bars_are_rejected() {
        let bars = [bar(2, 100.0, 10.0), bar(1, 101.0, 10.0)];
        assert_eq!(close_vwap(&bars), Err(MarketDataError::InvalidInput));
    }

    #[test]
    fn close_vwap_matches_hand_calculation() {
        let bars = [bar(1, 10.0, 2.0), bar(2, 20.0, 3.0)];
        let vwap = close_vwap(&bars).unwrap();
        assert!((vwap - 16.0).abs() < 1e-12);
    }

    #[test]
    fn log_returns_match_ratio_log() {
        let prices = [100.0, 110.0];
        let mut out = [0.0; 1];
        log_returns_into(&prices, &mut out).unwrap();
        assert!((out[0] - 1.1f64.ln()).abs() < 1e-12);
    }
}
