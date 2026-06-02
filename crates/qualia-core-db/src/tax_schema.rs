/// Defines modular tax and jurisdictional rulesets to be loaded into the Sentinel VM.
/// These rules are applied to the immutable Quins to compute dynamic, mutable 
/// tax liabilities without polluting the underlying graph.

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
                }
            ],
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
