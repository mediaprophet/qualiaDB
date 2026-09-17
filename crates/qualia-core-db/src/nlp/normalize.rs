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
    if !is_valid_gregorian_date(y, m, d) {
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
        // Space-separated: longest KNOWN_UNITS prefix (so "m/s" wins over "m")
        if rest.starts_with(' ') {
            if let Some(u) = longest_known_unit(&rest[1..]) {
                unit = Some(u);
                end = j + 1 + u.len();
            }
        }
        // Direct attachment (no space) for degree units; `°` is 2 UTF-8 bytes.
        if unit.is_none() {
            for &u in &["°C", "°F"] {
                if rest.starts_with(u) {
                    unit = Some(u);
                    end = j + u.len();
                    break;
                }
            }
        }
    }
    Some(Normalized::Number {
        span: DocSpan::new(i as u32, end as u32),
        value,
        unit,
    })
}

/// Longest `KNOWN_UNITS` prefix of `after` that ends on a non-alphanumeric boundary.
fn longest_known_unit(after: &str) -> Option<&'static str> {
    let mut best: Option<&'static str> = None;
    for &u in KNOWN_UNITS {
        if !after.starts_with(u) {
            continue;
        }
        let rest = &after[u.len()..];
        if !rest.is_empty() && rest.as_bytes()[0].is_ascii_alphanumeric() {
            continue;
        }
        if best.map_or(true, |b| u.len() > b.len()) {
            best = Some(u);
        }
    }
    best
}

fn is_leap_year(y: u32) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn days_in_month(y: u32, m: u32) -> Option<u32> {
    Some(match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        _ => return None,
    })
}

fn is_valid_gregorian_date(y: u32, m: u32, d: u32) -> bool {
    if y < 1000 {
        return false;
    }
    days_in_month(y, m).is_some_and(|max| (1..=max).contains(&d))
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

    fn unit_of(src: &str) -> Option<&'static str> {
        normalize_dates_and_numbers(src)
            .into_iter()
            .find_map(|x| match x {
                Normalized::Number { unit, .. } => unit,
                _ => None,
            })
    }

    #[test]
    fn attached_degree_celsius_span() {
        let src = "Recorded 25°C today";
        let n = normalize_dates_and_numbers(src);
        let found = n
            .iter()
            .find(|x| {
                matches!(
                    x,
                    Normalized::Number {
                        unit: Some("°C"),
                        ..
                    }
                )
            })
            .expect("attached °C");
        match found {
            Normalized::Number { span, value, unit } => {
                assert!((value - 25.0).abs() < 1e-9);
                assert_eq!(*unit, Some("°C"));
                assert_eq!(&src[span.as_range()], "25°C");
            }
            _ => panic!("expected number"),
        }
        // Space-separated degree units still match via KNOWN_UNITS.
        assert_eq!(unit_of("25 °C"), Some("°C"));
        assert_eq!(unit_of("25 °F"), Some("°F"));
        let f_src = "boils at 212°F";
        let f = normalize_dates_and_numbers(f_src);
        let f_found = f
            .iter()
            .find(|x| {
                matches!(
                    x,
                    Normalized::Number {
                        unit: Some("°F"),
                        ..
                    }
                )
            })
            .expect("attached °F");
        if let Normalized::Number { span, .. } = f_found {
            assert_eq!(&f_src[span.as_range()], "212°F");
        }
    }

    #[test]
    fn compound_unit_longest_match() {
        assert_eq!(unit_of("10 m/s"), Some("m/s"));
        assert_eq!(unit_of("10 km/h"), Some("km/h"));
        assert_eq!(unit_of("10 ms"), Some("ms"));
        assert_eq!(unit_of("10 m"), Some("m"));
    }

    fn date_labels(src: &str) -> Vec<String> {
        normalize_dates_and_numbers(src)
            .into_iter()
            .filter_map(|x| match x {
                Normalized::DateIso { yyyy_mm_dd, .. } => {
                    Some(String::from_utf8_lossy(&yyyy_mm_dd).into_owned())
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn rejects_invalid_calendar_dates() {
        let src = "bad 2026-02-31 and 2023-02-29 and 2026-04-31 ok 2024-02-29 and 2026-08-15";
        let dates = date_labels(src);
        assert!(
            !dates.iter().any(|d| d == "2026-02-31"),
            "Feb 31 is not a calendar date: {dates:?}"
        );
        assert!(
            !dates.iter().any(|d| d == "2023-02-29"),
            "2023 is not a leap year: {dates:?}"
        );
        assert!(
            !dates.iter().any(|d| d == "2026-04-31"),
            "April has 30 days: {dates:?}"
        );
        assert!(
            dates.iter().any(|d| d == "2024-02-29"),
            "2024-02-29 is a leap day: {dates:?}"
        );
        assert!(
            dates.iter().any(|d| d == "2026-08-15"),
            "existing valid date must still parse: {dates:?}"
        );
    }
}
