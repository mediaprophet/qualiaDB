//! Narrow date / number normalizers. Extended with QUDT unit vocabulary.

use super::span::DocSpan;

#[derive(Debug, Clone, PartialEq)]
pub enum Normalized {
    DateIso {
        span: DocSpan,
        yyyy_mm_dd: [u8; 10],
    },
    Number {
        span: DocSpan,
        value: f64,
        unit: Option<&'static str>,
    },
}

/// QUDT-compatible unit symbols recognized by the normalizer.
const KNOWN_UNITS: &[&str] = &[
    "mm", "cm", "m", "km", "in", "ft", "yd", "mi", "mg", "g", "kg", "t", "lb", "oz", "ms", "s",
    "min", "h", "d", "ml", "l", "gal", "Pa", "kPa", "MPa", "bar", "psi", "K", "°C", "°F", "Hz",
    "kHz", "MHz", "GHz", "V", "A", "W", "kW", "MW", "J", "kJ", "cal", "kcal", "mol", "ppm", "ppb",
    "rad", "deg", "m/s", "km/h", "mph",
];

pub fn normalize_dates_and_numbers(source: &str) -> Vec<Normalized> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(norm) = try_iso_date(source, i) {
            i = norm_end(&norm) as usize;
            out.push(norm);
            continue;
        }
        if let Some(norm) = try_number(source, i) {
            i = norm_end(&norm) as usize;
            out.push(norm);
            continue;
        }
        i += source[i..]
            .chars()
            .next()
            .map(|c| c.len_utf8())
            .unwrap_or(1);
    }
    out
}

fn try_iso_date(source: &str, i: usize) -> Option<Normalized> {
    let rest = &source[i..];
    if rest.len() < 10 {
        return None;
    }
    let cand = &rest.as_bytes()[..10];
    if cand[4] != b'-' || cand[7] != b'-' {
        return None;
    }
    if !cand[0..4].iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if !cand[5..7].iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if !cand[8..10].iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let y: u32 = std::str::from_utf8(&cand[0..4]).ok()?.parse().ok()?;
    let m: u32 = std::str::from_utf8(&cand[5..7]).ok()?.parse().ok()?;
    let d: u32 = std::str::from_utf8(&cand[8..10]).ok()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || y < 1000 {
        return None;
    }
    if i > 0 && source.as_bytes()[i - 1].is_ascii_digit() {
        return None;
    }
    if i + 10 < source.len() && source.as_bytes()[i + 10].is_ascii_digit() {
        return None;
    }
    let mut yyyy_mm_dd = [0u8; 10];
    yyyy_mm_dd.copy_from_slice(cand);
    Some(Normalized::DateIso {
        span: DocSpan::new(i as u32, (i + 10) as u32),
        yyyy_mm_dd,
    })
}

fn try_number(source: &str, i: usize) -> Option<Normalized> {
    let bytes = source.as_bytes();
    if !bytes[i].is_ascii_digit() {
        return None;
    }
    if i > 0 && bytes[i - 1].is_ascii_alphanumeric() {
        return None;
    }
    let mut j = i;
    while j < bytes.len() && bytes[j].is_ascii_digit() {
        j += 1;
    }
    if j < bytes.len() && bytes[j] == b'.' {
        let k = j + 1;
        if k < bytes.len() && bytes[k].is_ascii_digit() {
            j = k;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
        }
    }
    let value: f64 = source[i..j].parse().ok()?;
    let mut unit = None;
    let mut end = j;
    // Try to match a known unit after the number (preceded by space or directly)
    if j < source.len() {
        let rest = &source[j..];
        // Try space-separated units first
        if rest.starts_with(' ') {
            let after_space = &rest[1..];
            for &u in KNOWN_UNITS {
                if after_space.starts_with(u) {
                    // Check word boundary after unit
                    let after_unit = &after_space[u.len()..];
                    if after_unit.is_empty() || !after_unit.as_bytes()[0].is_ascii_alphanumeric() {
                        unit = Some(u);
                        end = j + 1 + u.len();
                        break;
                    }
                }
            }
        }
        // Try direct attachment (no space) for degree units
        if unit.is_none() && rest.starts_with("°C") {
            unit = Some("°C");
            end = j + 2;
        } else if unit.is_none() && rest.starts_with("°F") {
            unit = Some("°F");
            end = j + 2;
        }
    }
    Some(Normalized::Number {
        span: DocSpan::new(i as u32, end as u32),
        value,
        unit,
    })
}

fn norm_end(n: &Normalized) -> u32 {
    match n {
        Normalized::DateIso { span, .. } | Normalized::Number { span, .. } => span.end_utf8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso_date_and_mm() {
        let src = "12.5 mm on 2026-08-15";
        let n = normalize_dates_and_numbers(src);
        assert!(n.iter().any(|x| matches!(x, Normalized::Number { value, unit: Some("mm"), .. } if (*value - 12.5).abs() < 1e-9)));
        assert!(n.iter().any(|x| matches!(x, Normalized::DateIso { .. })));
    }

    #[test]
    fn multi_unit_normalization() {
        let src = "100 kg and 25 °C and 50 Hz";
        let n = normalize_dates_and_numbers(src);
        assert!(n.iter().any(|x| matches!(x, Normalized::Number { value, unit: Some("kg"), .. } if (*value - 100.0).abs() < 1e-9)));
        assert!(n.iter().any(|x| matches!(x, Normalized::Number { value, unit: Some("°C"), .. } if (*value - 25.0).abs() < 1e-9)));
        assert!(n.iter().any(|x| matches!(x, Normalized::Number { value, unit: Some("Hz"), .. } if (*value - 50.0).abs() < 1e-9)));
    }

    #[test]
    fn no_unit_for_bare_numbers() {
        let src = "There are 42 items.";
        let n = normalize_dates_and_numbers(src);
        assert!(n.iter().any(|x| matches!(x, Normalized::Number { unit: None, value, .. } if (*value - 42.0).abs() < 1e-9)));
    }
}
