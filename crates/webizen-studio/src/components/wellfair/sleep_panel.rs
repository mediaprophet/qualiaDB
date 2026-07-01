//! Sleep dashboard — journal projections with duration, efficiency, and weekly summary.

use super::host_client::fetch_health_records;
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

#[derive(Debug, Clone, Default)]
struct SleepMetrics {
    duration_min: Option<f64>,
    efficiency: Option<f64>,
    deep_min: Option<f64>,
    rem_min: Option<f64>,
}

fn parse_sleep_summary(summary: &str) -> SleepMetrics {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(summary) else {
        return SleepMetrics::default();
    };
    SleepMetrics {
        duration_min: v.get("duration_min").and_then(|x| x.as_f64()),
        efficiency: v.get("efficiency").and_then(|x| x.as_f64()),
        deep_min: v.get("deep_min").and_then(|x| x.as_f64()),
        rem_min: v.get("rem_min").and_then(|x| x.as_f64()),
    }
}

fn format_hours(minutes: f64) -> String {
    format!("{:.1}h", minutes / 60.0)
}

fn format_pct(value: f64) -> String {
    format!("{:.0}%", value)
}

fn format_minutes(value: f64) -> String {
    format!("{:.0}m", value)
}

fn cell_duration(summary: Option<&String>) -> String {
    summary
        .and_then(|s| parse_sleep_summary(s).duration_min)
        .map(format_hours)
        .unwrap_or_else(|| "—".into())
}

fn cell_efficiency(summary: Option<&String>) -> String {
    summary
        .and_then(|s| parse_sleep_summary(s).efficiency)
        .map(format_pct)
        .unwrap_or_else(|| "—".into())
}

fn cell_deep(summary: Option<&String>) -> String {
    summary
        .and_then(|s| parse_sleep_summary(s).deep_min)
        .map(format_minutes)
        .unwrap_or_else(|| "—".into())
}

fn cell_rem(summary: Option<&String>) -> String {
    summary
        .and_then(|s| parse_sleep_summary(s).rem_min)
        .map(format_minutes)
        .unwrap_or_else(|| "—".into())
}

#[component]
pub fn WellfairSleepPanel() -> Element {
    let mut sleep_rows = use_signal(Vec::<HealthRecordDto>::new);
    let mut status = use_signal(|| "Loading sleep records…".to_string());
    let mut avg_duration = use_signal(|| None::<f64>);
    let mut avg_efficiency = use_signal(|| None::<f64>);

    let reload = move || {
        spawn(async move {
            status.set("Loading…".into());
            match fetch_health_records(128).await {
                Ok(list) => {
                    let sleep: Vec<_> = list.into_iter().filter(|r| r.kind == "sleep").collect();
                    let n = sleep.len();
                    if n == 0 {
                        status.set(
                            "No sleep records yet. Sync Samsung Health sleep CSV from your phone via Tools."
                                .into(),
                        );
                        avg_duration.set(None);
                        avg_efficiency.set(None);
                    } else {
                        let mut dur_sum = 0.0;
                        let mut dur_count = 0u32;
                        let mut eff_sum = 0.0;
                        let mut eff_count = 0u32;
                        for row in &sleep {
                            if let Some(ref s) = row.summary {
                                let m = parse_sleep_summary(s);
                                if let Some(d) = m.duration_min {
                                    dur_sum += d;
                                    dur_count += 1;
                                }
                                if let Some(e) = m.efficiency {
                                    eff_sum += e;
                                    eff_count += 1;
                                }
                            }
                        }
                        avg_duration.set(if dur_count > 0 {
                            Some(dur_sum / dur_count as f64)
                        } else {
                            None
                        });
                        avg_efficiency.set(if eff_count > 0 {
                            Some(eff_sum / eff_count as f64)
                        } else {
                            None
                        });
                        status.set(format!(
                            "{n} sleep night(s). Non-diagnostic summary — not a clinical assessment."
                        ));
                    }
                    sleep_rows.set(sleep);
                }
                Err(e) => status.set(format!("Could not load sleep data: {e}")),
            }
        });
    };

    use_effect(move || {
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair sleep dashboard",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.75rem;",
            div {
                style: "display:flex;align-items:center;justify-content:space-between;margin-bottom:0.5rem;",
                h2 { style: "margin:0;font-size:1rem;", "Sleep — trends" }
                button {
                    style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                    onclick: move |_| reload(),
                    "Refresh"
                }
            }
            p {
                style: "margin:0 0 0.75rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                "{status()}"
            }

            if avg_duration().is_some() || avg_efficiency().is_some() {
                div {
                    style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(120px,1fr));gap:0.5rem;margin-bottom:0.75rem;",
                    if let Some(d) = avg_duration() {
                        div {
                            style: "padding:0.55rem;border-radius:8px;background:#457b9d12;text-align:center;",
                            div { style: "font-size:0.7rem;color:var(--qualia-text-muted,#666);", "Avg duration" }
                            strong { style: "font-size:1.05rem;", "{format_hours(d)}" }
                        }
                    }
                    if let Some(e) = avg_efficiency() {
                        div {
                            style: "padding:0.55rem;border-radius:8px;background:#2a9d8f12;text-align:center;",
                            div { style: "font-size:0.7rem;color:var(--qualia-text-muted,#666);", "Avg efficiency" }
                            strong { style: "font-size:1.05rem;", "{format_pct(e)}" }
                        }
                    }
                    div {
                        style: "padding:0.55rem;border-radius:8px;background:#e9c46a12;text-align:center;",
                        div { style: "font-size:0.7rem;color:var(--qualia-text-muted,#666);", "Nights logged" }
                        strong { style: "font-size:1.05rem;", "{sleep_rows.read().len()}" }
                    }
                }
            }

            if !sleep_rows.read().is_empty() {
                div {
                    style: "overflow-x:auto;",
                    table {
                        style: "width:100%;border-collapse:collapse;font-size:0.76rem;",
                        thead {
                            tr {
                                style: "text-align:left;border-bottom:1px solid var(--qualia-border,#ddd);",
                                th { style: "padding:0.35rem 0.5rem;", "Night (unix)" }
                                th { style: "padding:0.35rem 0.5rem;", "Duration" }
                                th { style: "padding:0.35rem 0.5rem;", "Efficiency" }
                                th { style: "padding:0.35rem 0.5rem;", "Deep" }
                                th { style: "padding:0.35rem 0.5rem;", "REM" }
                                th { style: "padding:0.35rem 0.5rem;", "Source" }
                            }
                        }
                        tbody {
                            for row in sleep_rows.read().clone() {
                                tr {
                                    key: "{row.id}",
                                    style: "border-bottom:1px solid var(--qualia-border,#eee);",
                                    td { style: "padding:0.35rem 0.5rem;", "{row.asserted_time_unix}" }
                                    td { style: "padding:0.35rem 0.5rem;", "{cell_duration(row.summary.as_ref())}" }
                                    td { style: "padding:0.35rem 0.5rem;", "{cell_efficiency(row.summary.as_ref())}" }
                                    td { style: "padding:0.35rem 0.5rem;", "{cell_deep(row.summary.as_ref())}" }
                                    td { style: "padding:0.35rem 0.5rem;", "{cell_rem(row.summary.as_ref())}" }
                                    td { style: "padding:0.35rem 0.5rem;", "{row.source}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}