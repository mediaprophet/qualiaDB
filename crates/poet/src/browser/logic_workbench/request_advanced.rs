//! Request construction for the eight advanced-logic workbench panels.

use super::helpers::field_value;
use super::request_parse::{optional_u64, required_assignment, required_f64, required_f64_list};
use web_sys::Document;

pub(super) fn advanced_request(
    document: &Document,
    mode: &str,
) -> Result<(&'static str, serde_json::Value), String> {
    let (field, arguments) = match mode {
        "abductive" => {
            let source = field_value(document, "abductive-editor");
            let hypotheses = bracket_items(&source, "hypotheses")?
                .into_iter()
                .map(|item| {
                    let parts = item.split(':').map(str::trim).collect::<Vec<_>>();
                    if parts.len() != 3 || parts[0].is_empty() {
                        return Err("Each hypothesis must be `id:prior:likelihood`.".to_string());
                    }
                    let prior = finite(parts[1], "hypothesis prior")?;
                    let likelihood = finite(parts[2], "hypothesis likelihood")?;
                    Ok(serde_json::json!({
                        "id": parts[0], "prior": prior, "likelihood": likelihood
                    }))
                })
                .collect::<Result<Vec<_>, String>>()?;
            (
                "abductive-editor",
                serde_json::json!({ "mode": mode, "hypotheses": hypotheses }),
            )
        }
        "fuzzy" => {
            let source = field_value(document, "fuzzy-editor");
            (
                "fuzzy-editor",
                serde_json::json!({
                    "mode": mode,
                    "operation": required_assignment(&source, "operation")?,
                    "a": required_f64(&source, "a")?,
                    "b": required_f64(&source, "b")?
                }),
            )
        }
        "probabilistic" => {
            let source = field_value(document, "probabilistic-editor");
            (
                "probabilistic-editor",
                serde_json::json!({
                    "mode": mode,
                    "prior": required_f64(&source, "prior")?,
                    "likelihood_true": required_f64(&source, "likelihood_true")?,
                    "likelihood_false": required_f64(&source, "likelihood_false")?,
                    "threshold": required_f64(&source, "threshold")?
                }),
            )
        }
        "graph" => {
            let source = field_value(document, "graph-theory-editor");
            let edges = named_pairs(&source, "edges")?;
            (
                "graph-theory-editor",
                serde_json::json!({ "mode": mode, "edges": edges }),
            )
        }
        "interval" => {
            let source = field_value(document, "interval-editor");
            let a = required_f64_list(&source, "a")?;
            let b = required_f64_list(&source, "b")?;
            if a.len() != 2 || b.len() != 2 || a.iter().chain(&b).any(|value| value.fract() != 0.0)
            {
                return Err("Intervals `a` and `b` must each contain two integer bounds.".into());
            }
            (
                "interval-editor",
                serde_json::json!({ "mode": mode, "a": a, "b": b }),
            )
        }
        "manifold-10d" => {
            let source = field_value(document, "manifold-10d-editor");
            let parameters = required_f64_list(&source, "parameters")?;
            if parameters.len() != 10 {
                return Err("`parameters` must contain exactly 10 finite numbers.".into());
            }
            (
                "manifold-10d-editor",
                serde_json::json!({ "mode": "manifold_10d", "parameters": parameters }),
            )
        }
        "epistemic-boundaries" => {
            let source = field_value(document, "epistemic-boundaries-editor");
            let severity = optional_u64(&source, "severity")?.unwrap_or(0);
            if severity > 255 {
                return Err("`severity` must be in 0..=255.".into());
            }
            (
                "epistemic-boundaries-editor",
                serde_json::json!({
                    "mode": "epistemic_boundaries",
                    "subject": required_assignment(&source, "subject")?,
                    "predicate": required_assignment(&source, "predicate")?,
                    "severity": severity
                }),
            )
        }
        "modal" => {
            let source = field_value(document, "modal-editor");
            (
                "modal-editor",
                serde_json::json!({
                    "mode": mode,
                    "system": required_assignment(&source, "system")?,
                    "operator": required_assignment(&source, "operator")?,
                    "world": required_assignment(&source, "world")?,
                    "proposition": required_assignment(&source, "proposition")?,
                    "worlds": bracket_items(&source, "worlds")?,
                    "accesses": named_pairs(&source, "accesses")?,
                    "holds_in": bracket_items(&source, "holds_in")?
                }),
            )
        }
        _ => return Err(format!("Unknown advanced-logic panel `{mode}`.")),
    };
    let _ = field;
    Ok(("AdvancedLogic.compute", arguments))
}

fn finite(source: &str, label: &str) -> Result<f64, String> {
    source
        .parse::<f64>()
        .map_err(|_| format!("{label} must be numeric."))
        .and_then(|value| {
            value
                .is_finite()
                .then_some(value)
                .ok_or_else(|| format!("{label} must be finite."))
        })
}

fn bracket_items(source: &str, key: &str) -> Result<Vec<String>, String> {
    let marker = format!("{key}=[");
    let start = source
        .find(&marker)
        .map(|index| index + marker.len())
        .ok_or_else(|| format!("Enter `{key}=[...]` in the panel input."))?;
    let rest = &source[start..];
    let end = rest
        .find(']')
        .ok_or_else(|| format!("`{key}` needs a closing `]`."))?;
    let values = rest[..end]
        .split('|')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if values.is_empty() {
        Err(format!("`{key}` must not be empty."))
    } else {
        Ok(values)
    }
}

pub(super) fn named_pairs(source: &str, key: &str) -> Result<Vec<Vec<String>>, String> {
    bracket_items(source, key)?
        .into_iter()
        .map(|value| {
            let mut parts = value.split(':').map(str::trim);
            let from = parts
                .next()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| format!("`{key}` contains an empty source."))?;
            let to = parts
                .next()
                .filter(|v| !v.is_empty())
                .ok_or_else(|| format!("`{key}` entries must be `from:to`."))?;
            if parts.next().is_some() {
                return Err(format!("`{key}` entries must contain one colon."));
            }
            Ok(vec![from.to_string(), to.to_string()])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{bracket_items, named_pairs};

    #[test]
    fn parses_bounded_workbench_lists() {
        assert_eq!(
            bracket_items("worlds=[w0|w1]", "worlds").unwrap(),
            ["w0", "w1"]
        );
        assert_eq!(
            named_pairs("edges=[a:b|b:c]", "edges").unwrap()[1],
            ["b", "c"]
        );
        assert!(named_pairs("edges=[a:b:c]", "edges").is_err());
    }
}
