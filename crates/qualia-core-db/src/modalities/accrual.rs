//! Accrual, slow variables, and irreversible loss modality (§GAP-11, accrual.md).
//!
//! Models standing stocks with multi-dimensional time-depth (biomass, habitat structure,
//! hydrological function, genetic distinctness, custodial relation), temporal asymmetry
//! (formation time vs destruction time), and irreversibility classes (`AC-AX-1` to `AC-AX-6`).
//! Zero-heap hot path.

use crate::q_hash;

/// Maturity class representing temporal depth of an entity.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum MaturityClass {
    #[default]
    Juvenile = 0,
    Mature = 1,
    OldGrowth = 2,
    Centennial = 3,
    DeepTime = 4,
}

impl MaturityClass {
    pub const fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Juvenile,
            1 => Self::Mature,
            2 => Self::OldGrowth,
            3 => Self::Centennial,
            4 => Self::DeepTime,
            _ => Self::Juvenile,
        }
    }
}

/// Irreversibility classification of loss or degradation.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum IrreversibilityClass {
    /// Readily reversible within seasonal or short horizons (months to few years).
    #[default]
    Reversible = 0,
    /// Slow recovery within standard planning horizons (decades; e.g. biomass regrowth).
    SlowReversible = 1,
    /// Recovery horizon exceeds human planning timescale (>100 years; e.g. tree hollows, mature soil profile).
    HorizonExceeding = 2,
    /// Irreversible loss (extinction, erased language/custodial lineage).
    Absolute = 3,
}

impl IrreversibilityClass {
    pub const fn from_u8(val: u8) -> Self {
        match val {
            0 => Self::Reversible,
            1 => Self::SlowReversible,
            2 => Self::HorizonExceeding,
            3 => Self::Absolute,
            _ => Self::Reversible,
        }
    }
}

/// Multi-dimensional accrual profile representing non-collapsible slow variables.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AccrualProfile {
    /// Biomass stock magnitude (normalised or physical kg).
    pub biomass: f32,
    /// Structural habitat complexity (e.g. tree hollows, complex deadwood).
    pub structural_complexity: f32,
    /// Hydrological retention / water-cycle buffering function.
    pub hydrological_function: f32,
    /// Genetic and phylogenetic distinctness.
    pub genetic_distinctness: f32,
    /// Custodial and cultural relationship depth.
    pub custodial_depth: f32,
}

impl AccrualProfile {
    /// Scale profile by a factor in `[0.0, 1.0]`.
    pub fn scale(&self, factor: f32) -> Self {
        let f = factor.clamp(0.0, 1.0);
        Self {
            biomass: self.biomass * f,
            structural_complexity: self.structural_complexity * f,
            hydrological_function: self.hydrological_function * f,
            genetic_distinctness: self.genetic_distinctness * f,
            custodial_depth: self.custodial_depth * f,
        }
    }

    /// Subtract lost profile from base, saturating at 0.0.
    pub fn subtract(&self, lost: &Self) -> Self {
        Self {
            biomass: (self.biomass - lost.biomass).max(0.0),
            structural_complexity: (self.structural_complexity - lost.structural_complexity)
                .max(0.0),
            hydrological_function: (self.hydrological_function - lost.hydrological_function)
                .max(0.0),
            genetic_distinctness: (self.genetic_distinctness - lost.genetic_distinctness).max(0.0),
            custodial_depth: (self.custodial_depth - lost.custodial_depth).max(0.0),
        }
    }
}

/// A standing ecological or social stock entity (`AC-AX-1`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StockEntity {
    pub entity_id: u64,
    pub maturity: MaturityClass,
    pub accrual_age_years: f32,
    /// Declared formation timescale in seconds (e.g. 30 years).
    pub formation_time_s: u64,
    /// Destruction vulnerability timescale in seconds (e.g. 2 nights).
    pub destruction_time_s: u64,
    pub profile: AccrualProfile,
    pub irreversibility: IrreversibilityClass,
}

/// Compute the temporal asymmetry ratio (`AC-AX-3`): `formation_time / destruction_time`.
/// Higher values denote greater fragility and slow-variable vulnerability.
pub fn asymmetry_ratio(formation_time_s: u64, destruction_time_s: u64) -> f64 {
    if destruction_time_s == 0 {
        f64::INFINITY
    } else {
        formation_time_s as f64 / destruction_time_s as f64
    }
}

/// An acute loss or destruction event (`AC-AX-6`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LossEvent {
    pub event_id: u64,
    pub stock_id: u64,
    pub cause_hash: u64,
    pub timestamp_epoch: u32,
    /// Extent of destruction in percentage `[0.0, 100.0]`.
    pub extent_loss_pct: f32,
    pub irreversibility_override: Option<IrreversibilityClass>,
}

/// Outcome report of an accrual loss event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccrualLossReport {
    pub stock_id: u64,
    pub effective_irreversibility: IrreversibilityClass,
    pub pre_loss_profile: AccrualProfile,
    pub lost_profile: AccrualProfile,
    pub residual_profile: AccrualProfile,
    pub is_critical_stock_breached: bool,
}

/// Evaluates the impact of a loss event on a standing stock entity.
/// Zero-heap hot path.
pub fn eval_accrual_loss(stock: &StockEntity, event: &LossEvent) -> AccrualLossReport {
    let loss_fraction = (event.extent_loss_pct / 100.0).clamp(0.0, 1.0);
    let lost_profile = stock.profile.scale(loss_fraction);
    let residual_profile = stock.profile.subtract(&lost_profile);

    let effective_irreversibility = event
        .irreversibility_override
        .unwrap_or(stock.irreversibility);

    // Critical stock breached if loss is HorizonExceeding or Absolute and exceeds 20% extent,
    // or if structural hollows / genetic distinctness are depleted past 50%.
    let is_critical_stock_breached = matches!(
        effective_irreversibility,
        IrreversibilityClass::HorizonExceeding | IrreversibilityClass::Absolute
    ) && (loss_fraction >= 0.20
        || lost_profile.structural_complexity >= stock.profile.structural_complexity * 0.50
        || lost_profile.genetic_distinctness >= stock.profile.genetic_distinctness * 0.50);

    AccrualLossReport {
        stock_id: stock.entity_id,
        effective_irreversibility,
        pre_loss_profile: stock.profile,
        lost_profile,
        residual_profile,
        is_critical_stock_breached,
    }
}

/// Predicate URI for loss event in graph assertions (`accrual:LossEvent`).
#[inline]
pub fn loss_event_predicate() -> u64 {
    q_hash("qualia:accrual:LossEvent")
}

/// Predicate URI for irreversibility class (`accrual:irreversibilityClass`).
#[inline]
pub fn irreversibility_predicate() -> u64 {
    q_hash("qualia:accrual:irreversibilityClass")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asymmetry_ratio() {
        // 30 years growth (~946,080,000s) vs 2 nights burn (~172,800s)
        let thirty_years = 30 * 365 * 86400;
        let two_nights = 2 * 86400;
        let ratio = asymmetry_ratio(thirty_years, two_nights);
        assert!((ratio - 5475.0).abs() < 1.0);
    }

    #[test]
    fn test_eval_accrual_loss_critical_breach() {
        let stock = StockEntity {
            entity_id: 0x111,
            maturity: MaturityClass::OldGrowth,
            accrual_age_years: 120.0,
            formation_time_s: 120 * 365 * 86400,
            destruction_time_s: 2 * 86400,
            profile: AccrualProfile {
                biomass: 5000.0,
                structural_complexity: 85.0,
                hydrological_function: 90.0,
                genetic_distinctness: 75.0,
                custodial_depth: 95.0,
            },
            irreversibility: IrreversibilityClass::HorizonExceeding,
        };

        // 80% destruction fire event
        let fire_event = LossEvent {
            event_id: 0x999,
            stock_id: 0x111,
            cause_hash: q_hash("event:bushfire"),
            timestamp_epoch: 1700000000,
            extent_loss_pct: 80.0,
            irreversibility_override: None,
        };

        let report = eval_accrual_loss(&stock, &fire_event);
        assert_eq!(
            report.effective_irreversibility,
            IrreversibilityClass::HorizonExceeding
        );
        assert!(report.is_critical_stock_breached);
        assert_eq!(report.lost_profile.structural_complexity, 68.0);
        assert_eq!(report.residual_profile.structural_complexity, 17.0);
    }

    #[test]
    fn test_eval_accrual_loss_reversible() {
        let stock = StockEntity {
            entity_id: 0x222,
            maturity: MaturityClass::Juvenile,
            accrual_age_years: 1.0,
            formation_time_s: 365 * 86400,
            destruction_time_s: 7 * 86400,
            profile: AccrualProfile {
                biomass: 100.0,
                structural_complexity: 5.0,
                hydrological_function: 10.0,
                genetic_distinctness: 10.0,
                custodial_depth: 5.0,
            },
            irreversibility: IrreversibilityClass::Reversible,
        };

        let event = LossEvent {
            event_id: 0x888,
            stock_id: 0x222,
            cause_hash: q_hash("event:seasonal_grazing"),
            timestamp_epoch: 1700000000,
            extent_loss_pct: 30.0,
            irreversibility_override: None,
        };

        let report = eval_accrual_loss(&stock, &event);
        assert!(!report.is_critical_stock_breached);
        assert_eq!(
            report.effective_irreversibility,
            IrreversibilityClass::Reversible
        );
    }
}
