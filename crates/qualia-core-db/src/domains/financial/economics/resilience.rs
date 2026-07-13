//! Resilience economics kernels.

/// Survival-first internal resource pricing for an autonomous resilience basecamp.
pub fn resilience_resource_pricing(
    stock: &[f64],
    survival_demand: &[f64],
    production_cost: &[f64],
    survival_premium: f64,
    n: usize,
    price_out: &mut [f64],
    tradeable_surplus_out: &mut [f64],
) -> usize {
    let count = n
        .min(stock.len())
        .min(survival_demand.len())
        .min(production_cost.len())
        .min(price_out.len())
        .min(tradeable_surplus_out.len());
    for i in 0..count {
        let demand = survival_demand[i];
        let cost = production_cost[i];
        if demand <= 0.0 {
            price_out[i] = cost;
            tradeable_surplus_out[i] = stock[i].max(0.0);
            continue;
        }
        let coverage = stock[i] / demand;
        if coverage < 1.0 {
            let c = coverage.clamp(0.0, 1.0);
            price_out[i] = cost * (1.0 + (survival_premium - 1.0).max(0.0) * (1.0 - c));
            tradeable_surplus_out[i] = 0.0;
        } else {
            price_out[i] = cost;
            tradeable_surplus_out[i] = stock[i] - demand;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resilience_pricing_prioritizes_survival() {
        let stock = [5.0, 20.0];
        let demand = [10.0, 10.0];
        let cost = [2.0, 2.0];
        let mut price = [0.0f64; 2];
        let mut surplus = [0.0f64; 2];
        let n =
            resilience_resource_pricing(&stock, &demand, &cost, 3.0, 2, &mut price, &mut surplus);
        assert_eq!(n, 2);
        assert!((price[0] - 4.0).abs() < 1e-9);
        assert_eq!(surplus[0], 0.0);
        assert!((price[1] - 2.0).abs() < 1e-9);
        assert!((surplus[1] - 10.0).abs() < 1e-9);
    }
}
