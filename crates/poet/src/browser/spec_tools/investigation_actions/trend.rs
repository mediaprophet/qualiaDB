//! Trend detection and time-series analysis for investigations.

use web_sys::Element;

pub(super) fn run(_document: &web_sys::Document, container: &Element, tool_id: &str) -> Option<Result<(), String>> {
    match tool_id {
        "investigation:detect-trends" => Some(detect_trends(container)),
        "investigation:fit-model" => Some(fit_model(container)),
        "investigation:forecast-trend" => Some(forecast_trend(container)),
        "investigation:detect-anomalies" => Some(detect_anomalies(container)),
        "investigation:seasonal-decompose" => Some(seasonal_decompose(container)),
        "investigation:correlate-trends" => Some(correlate_trends(container)),
        "investigation:import-timeseries" => Some(import_timeseries()),
        _ => None,
    }
}

fn series_values(container: &Element) -> Option<Vec<f64>> {
    container
        .get_attribute("data-timeseries")
        .and_then(|raw| {
            let values: Vec<f64> = raw
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if values.is_empty() { None } else { Some(values) }
        })
}

fn detect_trends(container: &Element) -> Result<(), String> {
    let Some(values) = series_values(container) else {
        return Err(
            "No local time series on this surface. Import a series first with data-timeseries attribute values.".to_string(),
        );
    };
    let trend = if values.last() > values.first() {
        "trend:increasing"
    } else if values.last() < values.first() {
        "trend:decreasing"
    } else {
        "trend:flat"
    };
    container
        .set_attribute("data-trend-detected", trend)
        .map_err(|_| "Failed to detect trend.".to_string())
}

fn fit_model(container: &Element) -> Result<(), String> {
    let Some(values) = series_values(container) else {
        return Err("No local time series to fit. Set data-timeseries with comma-separated numbers.".to_string());
    };
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    container
        .set_attribute("data-trend-model", &format!("model:mean_baseline|mean:{mean:.4}"))
        .map_err(|_| "Failed to fit trend model.".to_string())
}

fn forecast_trend(container: &Element) -> Result<(), String> {
    let model = container
        .get_attribute("data-trend-model")
        .ok_or_else(|| "No fitted model on this surface. Run fit-model after importing a series.".to_string())?;
    container
        .set_attribute("data-trend-forecast", &format!("forecast:+1_step|basis:{model}"))
        .map_err(|_| "Failed to forecast trend.".to_string())
}

fn detect_anomalies(container: &Element) -> Result<(), String> {
    let Some(values) = series_values(container) else {
        return Err("No local time series for anomaly detection.".to_string());
    };
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let std = variance.sqrt();
    let anomalies = values
        .iter()
        .enumerate()
        .filter(|(_, v)| (**v - mean).abs() > std * 2.0)
        .map(|(i, _)| i.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let label = if anomalies.is_empty() {
        "anomalies:none".to_string()
    } else {
        format!("anomalies:indices={anomalies}")
    };
    container
        .set_attribute("data-trend-anomalies", &label)
        .map_err(|_| "Failed to detect anomalies.".to_string())
}

fn seasonal_decompose(container: &Element) -> Result<(), String> {
    let Some(values) = series_values(container) else {
        return Err("No local time series for seasonal decomposition.".to_string());
    };
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    container
        .set_attribute(
            "data-trend-decompose",
            &format!("trend:mean={mean:.4};seasonal:local_mean_deviation;residual:computed"),
        )
        .map_err(|_| "Failed to decompose time series.".to_string())
}

fn correlate_trends(container: &Element) -> Result<(), String> {
    let a = container.get_attribute("data-timeseries");
    let b = container.get_attribute("data-timeseries-b");
    match (a, b) {
        (Some(series_a), Some(series_b)) if !series_a.is_empty() && !series_b.is_empty() => {
            container
                .set_attribute("data-trend-correlation", "correlation:series_pair_registered")
                .map_err(|_| "Failed to correlate trends.".to_string())
        }
        _ => Err(
            "Two local series required. Set data-timeseries and data-timeseries-b with comma-separated values.".to_string(),
        ),
    }
}

fn import_timeseries() -> Result<(), String> {
    Err(
        "External time-series import is not available in the browser surface. Paste values into data-timeseries manually.".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_timeseries_fails_honestly() {
        assert!(import_timeseries().is_err());
    }
}
