/// Defines modular tax and jurisdictional rulesets to be loaded into the Webizen VM.
/// These rules are applied to the immutable Quins to compute dynamic, mutable
/// tax liabilities without polluting the underlying graph.

/// Integer division with banker's rounding (round half to even) (`AGENTS.md` §0-A).
#[inline]
pub fn div_round_bankers(numerator: i128, denominator: i128) -> i128 {
    assert!(denominator > 0, "denominator must be positive");
    let sign = if numerator < 0 { -1i128 } else { 1i128 };
    let abs_num = numerator.abs();
    let q = abs_num / denominator;
    let r = abs_num % denominator;
    let twice_r = r * 2;
    let rounded = if twice_r > denominator || (twice_r == denominator && (q & 1) == 1) {
        q + 1
    } else {
        q
    };
    sign * rounded
}

/// Zero-heap integer minor-unit (`i64`) & basis-point (`u32`) tax withholding calculation
/// using banker's rounding (`AGENTS.md` §0-A).
#[inline]
pub fn compute_withholding_cents_bankers(gross_cents: i64, rate_bps: u32) -> i64 {
    if gross_cents <= 0 || rate_bps == 0 {
        return 0;
    }
    let capped_bps = rate_bps.min(10_000) as i128;
    div_round_bankers((gross_cents as i128) * capped_bps, 10_000) as i64
}

/// Split `gross_cents` into `(net_cents, withheld_tax_cents)` conserving `net + tax == gross`.
#[inline]
pub fn split_gross_net_tax_cents_bankers(gross_cents: i64, rate_bps: u32) -> (i64, i64) {
    let withheld = compute_withholding_cents_bankers(gross_cents, rate_bps);
    (gross_cents.saturating_sub(withheld), withheld)
}

pub struct TaxRuleSchema {
    pub jurisdiction_id: String,
    pub description: String,
    pub rules: Vec<TaxRule>,
}

pub struct TaxRule {
    pub match_category: String,
    pub calculation_fn: fn(f64) -> f64,
}

impl TaxRuleSchema {
    /// Mock AU GST schema (10% GST on income, 10% credit on expenses)
    pub fn new_au_gst() -> Self {
        TaxRuleSchema {
            jurisdiction_id: "AU_GST_2026".to_string(),
            description: "Australian Goods and Services Tax (10%)".to_string(),
            rules: vec![
                TaxRule {
                    match_category: "Income".to_string(),
                    calculation_fn: |amount| amount * 0.10, // 10% GST Owed
                },
                TaxRule {
                    match_category: "Expense".to_string(),
                    calculation_fn: |amount| amount * -0.10, // 10% GST Credit
                },
            ],
        }
    }

    /// EU VAT (standard 20%): 20% owed on income, 20% creditable on expenses.
    pub fn new_eu_vat() -> Self {
        TaxRuleSchema {
            jurisdiction_id: "EU_VAT_2026".to_string(),
            description: "EU Value Added Tax (standard 20%)".to_string(),
            rules: vec![
                TaxRule {
                    match_category: "Income".to_string(),
                    calculation_fn: |amount| amount * 0.20,
                },
                TaxRule {
                    match_category: "Expense".to_string(),
                    calculation_fn: |amount| amount * -0.20,
                },
            ],
        }
    }

    /// US combined sales tax (illustrative ~7% on sales; expenses are not creditable).
    pub fn new_us_sales_tax() -> Self {
        TaxRuleSchema {
            jurisdiction_id: "US_SALES_2026".to_string(),
            description: "US combined sales tax (~7%)".to_string(),
            rules: vec![TaxRule {
                match_category: "Income".to_string(),
                calculation_fn: |amount| amount * 0.07,
            }],
        }
    }

    /// Zero-rated / exempt jurisdiction — no liability on any category.
    pub fn new_zero_rated() -> Self {
        TaxRuleSchema {
            jurisdiction_id: "ZERO_RATED".to_string(),
            description: "Zero-rated / exempt".to_string(),
            rules: Vec::new(),
        }
    }

    /// Evaluates a given amount and category against the active ruleset
    pub fn evaluate(&self, category: &str, amount: f64) -> f64 {
        for rule in &self.rules {
            if rule.match_category == category {
                return (rule.calculation_fn)(amount);
            }
        }
        0.0
    }
}

/// A single transaction line to clear — typically projected from a transaction Quin
/// (`jurisdiction`, `category`, `amount`), so clearing happens "at the nquin level"
/// without mutating the immutable graph.
pub struct TaxLineItem<'a> {
    pub jurisdiction_id: &'a str,
    pub category: &'a str,
    pub amount: f64,
}

/// Net cleared liability for one jurisdiction.
pub struct JurisdictionLiability {
    pub jurisdiction_id: String,
    pub liability: f64,
}

/// The result of clearing a batch across jurisdictions: per-jurisdiction net
/// liabilities plus the grand net.
pub struct ClearingResult {
    pub per_jurisdiction: Vec<JurisdictionLiability>,
    pub net_liability: f64,
}

/// Multi-jurisdiction "Information Banking" tax clearing house.
pub struct TaxClearingHouse {
    schemas: Vec<TaxRuleSchema>,
}

impl TaxClearingHouse {
    /// Creates a default `TaxClearingHouse`.
    pub fn new() -> Self {
        Self {
            schemas: Vec::new(),
        }
    }

    /// Register a jurisdiction schema (builder style).
    pub fn with_schema(mut self, schema: TaxRuleSchema) -> Self {
        self.schemas.push(schema);
        self
    }

    /// A clearing house pre-loaded with the standard AU-GST / EU-VAT / US-sales /
    /// zero-rated schemas.
    pub fn with_standard_schemas() -> Self {
        Self::new()
            .with_schema(TaxRuleSchema::new_au_gst())
            .with_schema(TaxRuleSchema::new_eu_vat())
            .with_schema(TaxRuleSchema::new_us_sales_tax())
            .with_schema(TaxRuleSchema::new_zero_rated())
    }

    fn schema_for(&self, jurisdiction_id: &str) -> Option<&TaxRuleSchema> {
        self.schemas
            .iter()
            .find(|s| s.jurisdiction_id == jurisdiction_id)
    }

    /// Clear one line item: apply the matching jurisdiction's schema (0 if the
    /// jurisdiction is unknown — fail-safe, never guesses a foreign rate).
    pub fn clear_item(&self, jurisdiction_id: &str, category: &str, amount: f64) -> f64 {
        self.schema_for(jurisdiction_id)
            .map_or(0.0, |s| s.evaluate(category, amount))
    }

    /// Clear a batch of line items into per-jurisdiction net liabilities + grand net.
    pub fn clear_batch(&self, items: &[TaxLineItem]) -> ClearingResult {
        let mut per: Vec<JurisdictionLiability> = Vec::new();
        let mut net = 0.0;
        for item in items {
            let liability = self.clear_item(item.jurisdiction_id, item.category, item.amount);
            net += liability;
            if let Some(j) = per
                .iter_mut()
                .find(|j| j.jurisdiction_id == item.jurisdiction_id)
            {
                j.liability += liability;
            } else {
                per.push(JurisdictionLiability {
                    jurisdiction_id: item.jurisdiction_id.to_string(),
                    liability,
                });
            }
        }
        ClearingResult {
            per_jurisdiction: per,
            net_liability: net,
        }
    }
}

impl Default for TaxClearingHouse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_jurisdiction_rates_apply() {
        let house = TaxClearingHouse::with_standard_schemas();
        assert!((house.clear_item("AU_GST_2026", "Income", 1000.0) - 100.0).abs() < 1e-9);
        assert!((house.clear_item("EU_VAT_2026", "Income", 1000.0) - 200.0).abs() < 1e-9);
        assert!((house.clear_item("US_SALES_2026", "Income", 1000.0) - 70.0).abs() < 1e-9);
        assert_eq!(house.clear_item("ZERO_RATED", "Income", 1000.0), 0.0);
        assert_eq!(house.clear_item("XX_UNKNOWN", "Income", 1000.0), 0.0);
    }

    #[test]
    fn batch_clears_net_per_jurisdiction() {
        let house = TaxClearingHouse::with_standard_schemas();
        let items = [
            TaxLineItem {
                jurisdiction_id: "AU_GST_2026",
                category: "Income",
                amount: 1000.0,
            },
            TaxLineItem {
                jurisdiction_id: "AU_GST_2026",
                category: "Expense",
                amount: 400.0,
            },
            TaxLineItem {
                jurisdiction_id: "EU_VAT_2026",
                category: "Income",
                amount: 500.0,
            },
        ];
        let result = house.clear_batch(&items);
        assert!((result.net_liability - 160.0).abs() < 1e-9);
        assert_eq!(result.per_jurisdiction.len(), 2);
        let au = result
            .per_jurisdiction
            .iter()
            .find(|j| j.jurisdiction_id == "AU_GST_2026")
            .unwrap();
        assert!((au.liability - 60.0).abs() < 1e-9);
        let eu = result
            .per_jurisdiction
            .iter()
            .find(|j| j.jurisdiction_id == "EU_VAT_2026")
            .unwrap();
        assert!((eu.liability - 100.0).abs() < 1e-9);
    }

    #[test]
    fn withholding_cents_bankers_rounds_half_to_even() {
        // 50 bps = 0.5%: 100 cents -> 0.5 cents -> rounds to even 0
        assert_eq!(compute_withholding_cents_bankers(100, 50), 0);
        // 50 bps = 0.5%: 300 cents -> 1.5 cents -> rounds to even 2
        assert_eq!(compute_withholding_cents_bankers(300, 50), 2);
        let (net, tax) = split_gross_net_tax_cents_bankers(10_000, 1_000);
        assert_eq!(net + tax, 10_000);
        assert_eq!(tax, 1_000);
    }
}
