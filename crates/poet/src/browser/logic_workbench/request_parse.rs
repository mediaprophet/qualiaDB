//! Small, validated parsers for the Logic Workbench's text-oriented panels.

pub(super) fn call_arguments(source: &str, call: &str) -> Option<Vec<String>> {
    let marker = format!("{call}(");
    let start = source.find(&marker)? + marker.len();
    let rest = &source[start..];
    let end = rest.find(')')?;
    let values = rest[..end]
        .split(',')
        .map(|value| value.trim().trim_matches(['\'', '"']).to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    (!values.is_empty()).then_some(values)
}

pub(super) fn assignment<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!("{key}=");
    let start = source.match_indices(&marker).find_map(|(index, _)| {
        let boundary_ok = index == 0
            || source[..index]
                .chars()
                .next_back()
                .is_some_and(|character| !character.is_ascii_alphanumeric() && character != '_');
        boundary_ok.then_some(index + marker.len())
    })?;
    let value = &source[start..];
    let end = value.find([',', ')', '\n']).unwrap_or(value.len());
    let value = value[..end].trim().trim_matches(['\'', '"']);
    (!value.is_empty()).then_some(value)
}

pub(super) fn required_assignment(source: &str, key: &str) -> Result<String, String> {
    assignment(source, key)
        .map(str::to_string)
        .ok_or_else(|| format!("Enter `{key}=...` in the panel input."))
}

pub(super) fn required_f64(source: &str, key: &str) -> Result<f64, String> {
    let value = required_assignment(source, key)?;
    value
        .parse::<f64>()
        .map_err(|_| format!("`{key}` must be a finite number."))
        .and_then(|number| {
            number
                .is_finite()
                .then_some(number)
                .ok_or_else(|| format!("`{key}` must be finite."))
        })
}

pub(super) fn optional_f64(source: &str, key: &str) -> Result<Option<f64>, String> {
    assignment(source, key)
        .map(|_| required_f64(source, key))
        .transpose()
}

pub(super) fn optional_f64_aliases(source: &str, keys: &[&str]) -> Result<Option<f64>, String> {
    for key in keys {
        if assignment(source, key).is_some() {
            return required_f64(source, key).map(Some);
        }
    }
    Ok(None)
}

pub(super) fn optional_u64(source: &str, key: &str) -> Result<Option<u64>, String> {
    let Some(value) = assignment(source, key) else {
        return Ok(None);
    };
    value
        .parse::<u64>()
        .map(Some)
        .map_err(|_| format!("`{key}` must be a non-negative integer."))
}

pub(super) fn optional_f64_list(source: &str, key: &str) -> Result<Option<Vec<f64>>, String> {
    let marker = format!("{key}=[");
    let Some(start) = source.find(&marker).map(|index| index + marker.len()) else {
        return Ok(None);
    };
    let values = &source[start..];
    let end = values
        .find(']')
        .ok_or_else(|| format!("`{key}` list needs a closing `]`."))?;
    let parsed = values[..end]
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|_| format!("`{key}` contains a non-numeric value."))
                .and_then(|number| {
                    number
                        .is_finite()
                        .then_some(number)
                        .ok_or_else(|| format!("`{key}` values must be finite."))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    (!parsed.is_empty())
        .then_some(parsed)
        .ok_or_else(|| format!("`{key}` must contain at least one value."))
        .map(Some)
}

pub(super) fn required_u64(source: &str, key: &str) -> Result<u64, String> {
    optional_u64(source, key)?.ok_or_else(|| format!("Enter `{key}=...` in the panel input."))
}

pub(super) fn required_f64_list(source: &str, key: &str) -> Result<Vec<f64>, String> {
    optional_f64_list(source, key)?
        .ok_or_else(|| format!("Enter `{key}=[...]` in the panel input."))
}

pub(super) fn required_string_list(source: &str, key: &str) -> Result<Vec<String>, String> {
    let marker = format!("{key}=[");
    let start = source
        .find(&marker)
        .map(|index| index + marker.len())
        .ok_or_else(|| format!("Enter `{key}=[item|item]` in the panel input."))?;
    let values = &source[start..];
    let end = values
        .find(']')
        .ok_or_else(|| format!("`{key}` list needs a closing `]`."))?;
    let parsed = values[..end]
        .split('|')
        .map(|value| value.trim().trim_matches(['\'', '"']).to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if parsed.is_empty() {
        return Err(format!("`{key}` must contain at least one value."));
    }
    Ok(parsed)
}

pub(super) fn bool_assignment(source: &str, key: &str) -> Result<bool, String> {
    match required_assignment(source, key)?
        .to_ascii_lowercase()
        .as_str()
    {
        "true" | "yes" | "1" => Ok(true),
        "false" | "no" | "0" => Ok(false),
        _ => Err(format!("`{key}` must be true or false.")),
    }
}

pub(super) fn optional_bool(source: &str, key: &str) -> Result<Option<bool>, String> {
    assignment(source, key)
        .map(|_| bool_assignment(source, key))
        .transpose()
}

pub(super) fn optional_string_list(source: &str, key: &str) -> Result<Option<Vec<String>>, String> {
    let marker = format!("{key}=[");
    let Some(start) = source.find(&marker).map(|index| index + marker.len()) else {
        return Ok(None);
    };
    let values = &source[start..];
    let end = values
        .find(']')
        .ok_or_else(|| format!("`{key}` list needs a closing `]`."))?;
    let parsed = values[..end]
        .split('|')
        .map(|value| value.trim().trim_matches(['\'', '"']).to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    Ok(Some(parsed))
}

#[cfg(test)]
mod tests {
    use super::{assignment, required_f64};

    #[test]
    fn assignments_parse_typed_panel_source() {
        let source = "patient(age=65, sex=male, smoker=true, total_chol=5.7).";
        assert_eq!(assignment(source, "sex"), Some("male"));
        assert_eq!(required_f64(source, "age").unwrap(), 65.0);
        assert_eq!(required_f64(source, "total_chol").unwrap(), 5.7);
        assert_eq!(assignment("A=1e10, Ea=50000", "a"), None);
        assert_eq!(assignment("A=1e10, Ea=50000", "A"), Some("1e10"));
    }

    #[test]
    fn non_finite_values_fail_closed() {
        assert!(required_f64("patient(age=NaN).", "age").is_err());
    }
}
