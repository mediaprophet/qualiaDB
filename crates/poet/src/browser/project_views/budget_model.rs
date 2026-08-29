//! Exact, unit-aware aggregation for the Project Budget workspace.
//!
//! Amounts are parsed into millionths of a currency unit. Different currency
//! codes are never combined and draft/unverified rows never inflate totals.

use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CurrencySummary {
    pub currency: String,
    pub approved_plan: i64,
    pub verified_actual: i64,
    pub received_funding: i64,
    pub royalties_due: i64,
    pub tax_due: i64,
}

impl CurrencySummary {
    pub fn variance(&self) -> i64 {
        self.approved_plan.saturating_sub(self.verified_actual)
    }

    pub fn funding_position(&self) -> i64 {
        self.received_funding.saturating_sub(self.verified_actual)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BudgetSummary {
    pub currencies: Vec<CurrencySummary>,
    pub included_rows: usize,
    pub pending_rows: usize,
    pub invalid_rows: usize,
}

pub fn summarize_payloads(payloads: &[(&str, &Value)]) -> BudgetSummary {
    let mut currencies = BTreeMap::<String, CurrencySummary>::new();
    let mut summary = BudgetSummary::default();

    for (family, payload) in payloads {
        let Some(records) = payload.get("records").and_then(Value::as_array) else {
            continue;
        };
        for record in records {
            let Some(fields) = record.get("fields").and_then(Value::as_object) else {
                summary.invalid_rows += 1;
                continue;
            };
            let Some(currency) = fields
                .get("currency")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_ascii_uppercase)
            else {
                summary.invalid_rows += 1;
                continue;
            };
            let Some(amount) = fields.get("amount").and_then(parse_amount_micros) else {
                summary.invalid_rows += 1;
                continue;
            };
            let lifecycle = fields
                .get("lifecycle")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();

            let row = currencies
                .entry(currency.clone())
                .or_insert_with(|| CurrencySummary {
                    currency,
                    ..CurrencySummary::default()
                });
            let included = match *family {
                "project_budget" if matches!(lifecycle.as_str(), "approved" | "committed") => {
                    row.approved_plan = row.approved_plan.saturating_add(amount);
                    true
                }
                "project_actual" if matches!(lifecycle.as_str(), "verified" | "settled") => {
                    row.verified_actual = row.verified_actual.saturating_add(amount);
                    true
                }
                "project_funding" if matches!(lifecycle.as_str(), "received" | "restricted") => {
                    row.received_funding = row.received_funding.saturating_add(amount);
                    true
                }
                "project_royalty" if matches!(lifecycle.as_str(), "calculated" | "approved") => {
                    row.royalties_due = row.royalties_due.saturating_add(amount);
                    true
                }
                "project_tax" if matches!(lifecycle.as_str(), "estimated" | "filed") => {
                    row.tax_due = row.tax_due.saturating_add(amount);
                    true
                }
                "project_budget" | "project_actual" | "project_funding" | "project_royalty"
                | "project_tax" => false,
                _ => {
                    summary.invalid_rows += 1;
                    continue;
                }
            };
            if included {
                summary.included_rows += 1;
            } else {
                summary.pending_rows += 1;
            }
        }
    }

    summary.currencies = currencies.into_values().collect();
    summary
}

pub fn parse_amount_micros(value: &Value) -> Option<i64> {
    let owned;
    let text = match value {
        Value::String(text) => text.trim(),
        Value::Number(number) => {
            owned = number.to_string();
            owned.as_str()
        }
        _ => return None,
    };
    if text.is_empty() || text.starts_with('-') || text.contains(['e', 'E']) {
        return None;
    }
    let mut parts = text.split('.');
    let whole = parts.next()?;
    let fraction = parts.next().unwrap_or("");
    if parts.next().is_some()
        || whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 6
    {
        return None;
    }
    let whole = whole.parse::<i64>().ok()?;
    let fraction_value = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i64>().ok()? * 10_i64.pow((6 - fraction.len()) as u32)
    };
    whole.checked_mul(1_000_000)?.checked_add(fraction_value)
}

pub fn format_amount(micros: i64) -> String {
    let negative = micros < 0;
    let absolute = micros.unsigned_abs();
    let whole = absolute / 1_000_000;
    let fraction = absolute % 1_000_000;
    let sign = if negative { "-" } else { "" };
    if fraction == 0 {
        format!("{sign}{whole}")
    } else {
        let trimmed = format!("{fraction:06}").trim_end_matches('0').to_string();
        format!("{sign}{whole}.{trimmed}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_amount_parser_rejects_ambiguous_values() {
        assert_eq!(
            parse_amount_micros(&serde_json::json!("12.345678")),
            Some(12_345_678)
        );
        assert_eq!(parse_amount_micros(&serde_json::json!("12.3456789")), None);
        assert_eq!(parse_amount_micros(&serde_json::json!("-1")), None);
        assert_eq!(parse_amount_micros(&serde_json::json!("1e3")), None);
    }

    #[test]
    fn currencies_and_lifecycle_states_are_not_conflated() {
        let plan = serde_json::json!({"records": [
            {"fields": {"amount": "100", "currency": "AUD", "lifecycle": "approved"}},
            {"fields": {"amount": "500", "currency": "AUD", "lifecycle": "draft"}},
            {"fields": {"amount": "50", "currency": "USD", "lifecycle": "committed"}}
        ]});
        let actual = serde_json::json!({"records": [
            {"fields": {"amount": "35.25", "currency": "AUD", "lifecycle": "verified"}},
            {"fields": {"amount": "900", "currency": "AUD", "lifecycle": "observed"}}
        ]});
        let summary = summarize_payloads(&[("project_budget", &plan), ("project_actual", &actual)]);
        assert_eq!(summary.currencies.len(), 2);
        assert_eq!(summary.included_rows, 3);
        assert_eq!(summary.pending_rows, 2);
        let aud = summary
            .currencies
            .iter()
            .find(|row| row.currency == "AUD")
            .unwrap();
        assert_eq!(aud.approved_plan, 100_000_000);
        assert_eq!(aud.verified_actual, 35_250_000);
        assert_eq!(format_amount(aud.variance()), "64.75");
    }
}
