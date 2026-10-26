//! Typed parsers for `input.parse_quantity`, `input.parse_currency`, `input.parse_uncertainty`,
//! `input.parse_coordinate`, `input.parse_instant`, and `input.parse_duration`.

use std::collections::BTreeMap;

use crate::cosmic::usri::Usri;
use crate::error::{DiagCode, Diagnostic};
use crate::quantity::{lookup_unit, Unit};
use crate::span::Span;
use crate::value::{Duration, Instant, Quantity, TimeScale, Value};

use super::{extract_f64, extract_i64, extract_opt_str, extract_str};

pub(crate) fn resolve_unit_iri(unit_str: &str) -> Option<Unit> {
    if let Some(u) = lookup_unit(unit_str) {
        return Some(u);
    }
    let tail = unit_str
        .strip_prefix("http://qudt.org/vocab/unit/")
        .or_else(|| unit_str.strip_prefix("https://qudt.org/vocab/unit/"))
        .or_else(|| unit_str.strip_prefix("qudt:"))
        .unwrap_or(unit_str);
    let mapped = match tail {
        "M" | "Meter" | "Metre" => "m",
        "KiloM" | "KiloMeter" | "KiloMetre" => "km",
        "CentiM" | "CentiMeter" => "cm",
        "MilliM" | "MilliMeter" => "mm",
        "NanoM" | "NanoMeter" => "nm",
        "SEC" | "Second" => "s",
        "MilliSEC" | "MilliSecond" => "ms",
        "KiloGM" | "KiloGram" => "kg",
        "GM" | "Gram" => "g",
        "A" | "Ampere" => "A",
        "K" | "Kelvin" => "K",
        "DEG_C" | "DegreeCelsius" | "Celsius" => "°C",
        "DEG_F" | "DegreeFahrenheit" | "Fahrenheit" => "°F",
        "N" | "Newton" => "N",
        "KiloN" | "KiloNewton" => "kN",
        "PA" | "Pascal" => "Pa",
        "KiloPA" | "KiloPascal" => "kPa",
        "MegaPA" | "MegaPascal" => "MPa",
        "J" | "Joule" => "J",
        "KiloJ" | "KiloJoule" => "kJ",
        "W" | "Watt" => "W",
        "KiloW" | "KiloWatt" => "kW",
        "HZ" | "Hertz" => "Hz",
        "KiloHZ" | "KiloHertz" => "kHz",
        "MegaHZ" | "MegaHertz" => "MHz",
        "GigaHZ" | "GigaHertz" => "GHz",
        "DEG" | "Degree" => "deg",
        "RAD" | "Radian" => "rad",
        "PERCENT" | "Percent" => "%",
        "M-PER-SEC" | "MeterPerSecond" => "m/s",
        other => other,
    };
    lookup_unit(mapped)
}

/// `input.parse_quantity(value, unit_iri, target_unit_iri_opt)`
pub fn eval_parse_quantity(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let value = extract_f64(args, 0, "input.parse_quantity", span)?;
    let unit_iri = extract_str(args, 1, "input.parse_quantity", span)?;
    if let Some(target_iri) = extract_opt_str(args, 2) {
        if target_iri != unit_iri {
            let src = resolve_unit_iri(&unit_iri).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, format!("input.parse_quantity: unknown source unit '{unit_iri}'"))
            })?;
            let dst = resolve_unit_iri(&target_iri).ok_or_else(|| {
                Diagnostic::new(DiagCode::E100, span, format!("input.parse_quantity: unknown target unit '{target_iri}'"))
            })?;
            let converted = src.convert(value, &dst).map_err(|msg| Diagnostic::new(DiagCode::E100, span, msg))?;
            return Ok(Value::Quantity(Quantity { value: converted, unit: target_iri }));
        }
    }
    Ok(Value::Quantity(Quantity { value, unit: unit_iri }))
}

/// `input.parse_currency(text, currency_code, scale_decimals_opt)`
pub fn eval_parse_currency(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let text = extract_str(args, 0, "input.parse_currency", span)?;
    let currency = extract_str(args, 1, "input.parse_currency", span)?;
    let scale_decimals = match args.get(2) {
        Some(Value::Null) | None => 2,
        Some(_) => extract_i64(args, 2, "input.parse_currency", span)?.clamp(0, 12),
    };

    let mut cleaned = text.trim().to_string();
    for sym in ["A$", "US$", "NZ$", "CA$", "HK$", "S$"] {
        cleaned = cleaned.replace(sym, "");
    }
    if !currency.is_empty() {
        cleaned = cleaned.replace(&currency, "");
    }

    let mut negative = false;
    let mut numeric = String::with_capacity(cleaned.len());
    for c in cleaned.chars() {
        match c {
            '-' => negative = true,
            '+' | '$' | '€' | '£' | '¥' | ',' | ' ' | '_' | '\u{00A0}' => {}
            _ => numeric.push(c),
        }
    }

    let parts: Vec<&str> = numeric.split('.').collect();
    let int_str = parts.first().copied().unwrap_or("");
    let frac_str = parts.get(1).copied().unwrap_or("");
    if parts.len() > 2 || numeric.is_empty() || !int_str.chars().all(|c| c.is_ascii_digit()) || !frac_str.chars().all(|c| c.is_ascii_digit()) {
        return Err(Diagnostic::new(DiagCode::E100, span, format!("input.parse_currency: invalid currency text '{text}'")));
    }

    let major: i64 = if int_str.is_empty() {
        0
    } else {
        int_str.parse::<i64>().map_err(|_| Diagnostic::new(DiagCode::E100, span, "input.parse_currency: integer overflow"))?
    };

    let scale_factor = 10_i64.pow(scale_decimals as u32);
    let frac_minor: i64 = if scale_decimals == 0 {
        0
    } else {
        let mut padded: String = frac_str.chars().take(scale_decimals as usize).collect();
        while padded.len() < scale_decimals as usize {
            padded.push('0');
        }
        padded.parse::<i64>().unwrap_or(0)
    };

    let abs_minor = major.saturating_mul(scale_factor).saturating_add(frac_minor);
    let minor_units = if negative { -abs_minor } else { abs_minor };

    let abs_major = (abs_minor / scale_factor).unsigned_abs();
    let abs_frac = (abs_minor % scale_factor).unsigned_abs();
    let mut formatted = String::new();
    if negative && minor_units != 0 {
        formatted.push('-');
    }
    formatted.push_str(&format_with_commas(abs_major));
    if scale_decimals > 0 {
        formatted.push('.');
        formatted.push_str(&format!("{:0width$}", abs_frac, width = scale_decimals as usize));
    }

    let mut rec = BTreeMap::new();
    rec.insert("minor_units".to_string(), Value::I64(minor_units));
    rec.insert("formatted".to_string(), Value::String(formatted));
    rec.insert("currency".to_string(), Value::String(currency));
    rec.insert("scale_decimals".to_string(), Value::I64(scale_decimals));
    Ok(Value::Record(rec))
}

fn format_with_commas(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(b as char);
    }
    out
}

/// `input.parse_uncertainty(nominal, plus_delta, minus_delta, sig_figs, unit_iri)`
pub fn eval_parse_uncertainty(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let nominal = extract_f64(args, 0, "input.parse_uncertainty", span)?;
    let plus_delta = extract_f64(args, 1, "input.parse_uncertainty", span)?;
    let minus_delta = extract_f64(args, 2, "input.parse_uncertainty", span)?;
    let sig_figs = extract_i64(args, 3, "input.parse_uncertainty", span)?;
    let unit_iri = extract_str(args, 4, "input.parse_uncertainty", span)?;

    let mut rec = BTreeMap::new();
    rec.insert("nominal".to_string(), Value::F64(nominal));
    rec.insert("min_bound".to_string(), Value::F64(nominal - minus_delta.abs()));
    rec.insert("max_bound".to_string(), Value::F64(nominal + plus_delta.abs()));
    rec.insert("plus_delta".to_string(), Value::F64(plus_delta));
    rec.insert("minus_delta".to_string(), Value::F64(minus_delta));
    rec.insert("sig_figs".to_string(), Value::I64(sig_figs));
    rec.insert("unit".to_string(), Value::String(unit_iri));
    Ok(Value::Record(rec))
}

/// `input.parse_coordinate(input)`
pub fn eval_parse_coordinate(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let input = extract_str(args, 0, "input.parse_coordinate", span)?;
    let trimmed = input.trim();

    if trimmed.starts_with("usri://") || trimmed.starts_with("urn:omni:") {
        if let Ok(usri) = Usri::parse(trimmed) {
            return Ok(usri.to_value());
        }
        if let Some(rest) = trimmed.strip_prefix("usri://") {
            let candidate = if rest.starts_with("urn:omni:") { rest.to_string() } else { format!("urn:omni:{rest}") };
            if let Ok(usri) = Usri::parse(&candidate).or_else(|_| Usri::parse(&format!("urn:omni:{}", rest.replace('/', ":")))) {
                return Ok(usri.to_value());
            }
        }
        return Err(Diagnostic::new(DiagCode::E100, span, format!("input.parse_coordinate: invalid USRI '{trimmed}'")));
    }

    let parts: Vec<&str> = trimmed.split(',').map(str::trim).collect();
    if !(2..=3).contains(&parts.len()) {
        return Err(Diagnostic::new(DiagCode::E100, span, "input.parse_coordinate: expected 'lat,lon[,alt]' or 'usri://...'"));
    }
    let lat_deg = parts[0].parse::<f64>().map_err(|_| Diagnostic::new(DiagCode::E100, span, "input.parse_coordinate: invalid latitude"))?;
    let lon_deg = parts[1].parse::<f64>().map_err(|_| Diagnostic::new(DiagCode::E100, span, "input.parse_coordinate: invalid longitude"))?;
    let alt_m = match parts.get(2) {
        Some(s) if !s.is_empty() => s.parse::<f64>().map_err(|_| Diagnostic::new(DiagCode::E100, span, "input.parse_coordinate: invalid altitude"))?,
        _ => 0.0,
    };
    if !(-90.0..=90.0).contains(&lat_deg) || !(-180.0..=180.0).contains(&lon_deg) {
        return Err(Diagnostic::new(DiagCode::E100, span, "input.parse_coordinate: latitude in [-90, 90] and longitude in [-180, 180] required"));
    }

    let mut rec = BTreeMap::new();
    rec.insert("lat_deg".to_string(), Value::F64(lat_deg));
    rec.insert("lon_deg".to_string(), Value::F64(lon_deg));
    rec.insert("alt_m".to_string(), Value::F64(alt_m));
    Ok(Value::Record(rec))
}

/// `input.parse_instant(secs, nanos_opt, scale_opt, frame_opt)`
pub fn eval_parse_instant(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let secs = extract_i64(args, 0, "input.parse_instant", span)?;
    let nanos = match args.get(1) {
        Some(Value::Null) | None => 0,
        Some(_) => extract_i64(args, 1, "input.parse_instant", span)?.clamp(0, 999_999_999),
    };
    let scale = match extract_opt_str(args, 2) {
        Some(s) => match s.to_ascii_lowercase().as_str() {
            "unix" | "utc" => TimeScale::Unix,
            "tai" => TimeScale::Tai,
            "gps" => TimeScale::Gps,
            "monotonic" => TimeScale::Monotonic,
            "proper" => TimeScale::Proper,
            _ => return Err(Diagnostic::new(DiagCode::E100, span, format!("input.parse_instant: unknown time scale '{s}'"))),
        },
        None => TimeScale::Unix,
    };
    Ok(Value::Instant(Instant {
        scale,
        secs,
        nanos: nanos as u32,
        frame: extract_opt_str(args, 3),
        seal: None,
    }))
}

/// `input.parse_duration(secs, nanos_opt)`
pub fn eval_parse_duration(args: &[Value], span: Span) -> Result<Value, Diagnostic> {
    let secs = extract_i64(args, 0, "input.parse_duration", span)?;
    let nanos = match args.get(1) {
        Some(Value::Null) | None => 0,
        Some(_) => extract_i64(args, 1, "input.parse_duration", span)?,
    };
    Ok(Value::Duration(Duration {
        secs: secs + nanos.div_euclid(1_000_000_000),
        nanos: nanos.rem_euclid(1_000_000_000) as u32,
    }))
}
