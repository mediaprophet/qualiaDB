//! Portfolio Analyzer — local weights + host Monte Carlo VaR.
//!
//! Host: `calculate_monte_carlo_var` → economics::run_monte_carlo_var (10k paths).
//! Honesty: **Partial** — VaR is real host Monte Carlo; expected shortfall from host is
//! a simple 1.25× heuristic (not a full ES model); return/vol cards are client-side weighted sums.

use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
use crate::components::qapp_engine::invoke_json;
use dioxus::prelude::*;
use serde::Deserialize;

#[derive(Clone, PartialEq)]
struct Asset {
    id: usize,
    ticker: String,
    weight: f64,
    expected_return: f64,
    volatility: f64,
}

#[derive(Deserialize, Default, Clone, PartialEq)]
struct RiskProps {
    monte_carlo_var: f64,
    expected_shortfall: f64,
}

#[derive(Clone, PartialEq)]
enum McPhase {
    Idle,
    Loading,
    Ready(RiskProps),
    Error(String),
}

#[component]
pub fn PortfolioAnalyzer() -> Element {
    let mut assets = use_signal(|| {
        vec![
            Asset {
                id: 1,
                ticker: "AAPL".to_string(),
                weight: 0.4,
                expected_return: 0.12,
                volatility: 0.20,
            },
            Asset {
                id: 2,
                ticker: "GOOGL".to_string(),
                weight: 0.3,
                expected_return: 0.10,
                volatility: 0.25,
            },
            Asset {
                id: 3,
                ticker: "TSLA".to_string(),
                weight: 0.3,
                expected_return: 0.15,
                volatility: 0.40,
            },
        ]
    });

    let mut portfolio_value = use_signal(|| 1_000_000.0_f64);
    let mut time_horizon_days = use_signal(|| 10.0_f64);
    let mut mc_phase = use_signal(|| McPhase::Idle);

    let portfolio_return = use_memo(move || {
        assets
            .read()
            .iter()
            .map(|a| a.weight * a.expected_return)
            .sum::<f64>()
    });

    let portfolio_volatility = use_memo(move || {
        let var: f64 = assets
            .read()
            .iter()
            .map(|a| (a.weight * a.volatility).powi(2))
            .sum();
        var.sqrt()
    });

    let sharpe_ratio = use_memo(move || {
        let r = portfolio_return();
        let v = portfolio_volatility();
        if v == 0.0 {
            0.0
        } else {
            (r - 0.02) / v
        }
    });

    let run_monte_carlo = move |_| {
        let value = portfolio_value();
        let vol = portfolio_volatility();
        let horizon = time_horizon_days();
        mc_phase.set(McPhase::Loading);
        spawn(async move {
            let args = serde_json::json!({
                "portfolio_value": value,
                "volatility": vol,
                "time_horizon": horizon,
            });
            match invoke_json("calculate_monte_carlo_var", args).await {
                Ok(res) => match serde_json::from_value::<RiskProps>(res) {
                    Ok(props) => mc_phase.set(McPhase::Ready(props)),
                    Err(e) => mc_phase.set(McPhase::Error(format!("Bad host payload: {e}"))),
                },
                Err(e) => mc_phase.set(McPhase::Error(e)),
            }
        });
    };

    let add_asset = move |_| {
        let mut list = assets.write();
        let new_id = list.iter().map(|a| a.id).max().unwrap_or(0) + 1;
        list.push(Asset {
            id: new_id,
            ticker: "NEW".to_string(),
            weight: 0.0,
            expected_return: 0.0,
            volatility: 0.0,
        });
    };

    let mut update_ticker = move |id: usize, val: String| {
        if let Some(asset) = assets.write().iter_mut().find(|a| a.id == id) {
            asset.ticker = val;
        }
    };

    let mut update_weight = move |id: usize, val: f64| {
        if let Some(asset) = assets.write().iter_mut().find(|a| a.id == id) {
            asset.weight = val;
        }
    };

    let mut update_return = move |id: usize, val: f64| {
        if let Some(asset) = assets.write().iter_mut().find(|a| a.id == id) {
            asset.expected_return = val;
        }
    };

    let mut update_volatility = move |id: usize, val: f64| {
        if let Some(asset) = assets.write().iter_mut().find(|a| a.id == id) {
            asset.volatility = val;
        }
    };

    let mut remove_asset = move |id: usize| {
        assets.write().retain(|a| a.id != id);
    };

    let mc = mc_phase.read().clone();
    let mc_loading = matches!(mc, McPhase::Loading);

    rsx! {
        div {
            style: "flex: 1; padding: 2.5rem; background: linear-gradient(135deg, #0f172a, #1e293b); border-radius: 16px; color: #f8fafc; font-family: 'Inter', system-ui, sans-serif; box-shadow: 0 20px 40px rgba(0,0,0,0.4); display: flex; flex-direction: column; gap: 1.5rem; overflow-y: auto;",

            div {
                style: "display: flex; justify-content: space-between; align-items: flex-start; gap: 1rem; flex-wrap: wrap; border-bottom: 1px solid rgba(255,255,255,0.1); padding-bottom: 1rem;",
                div {
                    h2 {
                        style: "margin: 0; font-size: 2rem; font-weight: 800; background: linear-gradient(to right, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent;",
                        "Portfolio Analyzer"
                    }
                    p { style: "margin: 0.4rem 0 0 0; color: #94a3b8; font-size: 0.9rem;",
                        "Weighted return/vol on device · Monte Carlo VaR via host economics engine."
                    }
                }
                HonestyChip {
                    level: HonestyLevel::Partial,
                    detail: "MC VaR host · ES≈1.25× heuristic".to_string(),
                }
            }

            div {
                style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 1rem;",

                div {
                    style: "background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.05); border-radius: 16px; padding: 1.25rem; text-align: center;",
                    span { style: "color: #94a3b8; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em;", "Expected Return (local)" }
                    div { style: "font-size: 2rem; font-weight: 700; color: #34d399; margin-top: 0.35rem;", "{portfolio_return() * 100.0:.2}%" }
                }

                div {
                    style: "background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.05); border-radius: 16px; padding: 1.25rem; text-align: center;",
                    span { style: "color: #94a3b8; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em;", "Portfolio σ (local)" }
                    div { style: "font-size: 2rem; font-weight: 700; color: #f472b6; margin-top: 0.35rem;", "{portfolio_volatility() * 100.0:.2}%" }
                }

                div {
                    style: "background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.05); border-radius: 16px; padding: 1.25rem; text-align: center;",
                    span { style: "color: #94a3b8; font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em;", "Sharpe (local, rf=2%)" }
                    div { style: "font-size: 2rem; font-weight: 700; color: #60a5fa; margin-top: 0.35rem;", "{sharpe_ratio():.2}" }
                }
            }

            // Host Monte Carlo panel
            div {
                style: "background: rgba(0,0,0,0.25); border: 1px solid rgba(56,189,248,0.2); border-radius: 16px; padding: 1.25rem; display: flex; flex-direction: column; gap: 1rem;",
                div {
                    style: "display: flex; justify-content: space-between; align-items: center; flex-wrap: wrap; gap: 0.75rem;",
                    h3 { style: "margin: 0; font-size: 1rem; color: #7dd3fc;", "Host Monte Carlo VaR" }
                    button {
                        disabled: mc_loading,
                        style: if mc_loading {
                            "background: linear-gradient(135deg, #0ea5e9, #6366f1); border: none; color: white; padding: 0.6rem 1.1rem; border-radius: 8px; font-weight: 600; cursor: pointer; opacity: 0.7;"
                        } else {
                            "background: linear-gradient(135deg, #0ea5e9, #6366f1); border: none; color: white; padding: 0.6rem 1.1rem; border-radius: 8px; font-weight: 600; cursor: pointer; opacity: 1;"
                        },
                        onclick: run_monte_carlo,
                        if mc_loading { "Running 10k paths…" } else { "Run Monte Carlo VaR" }
                    }
                }
                div {
                    style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(160px, 1fr)); gap: 0.75rem;",
                    div {
                        label { style: "display: block; color: #94a3b8; font-size: 0.75rem; margin-bottom: 0.3rem;", "Portfolio value ($)" }
                        input {
                            r#type: "number",
                            value: "{portfolio_value()}",
                            oninput: move |e| portfolio_value.set(e.value().parse().unwrap_or(0.0)),
                            style: "width: 100%; background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; box-sizing: border-box;"
                        }
                    }
                    div {
                        label { style: "display: block; color: #94a3b8; font-size: 0.75rem; margin-bottom: 0.3rem;", "Horizon (days)" }
                        input {
                            r#type: "number",
                            value: "{time_horizon_days()}",
                            oninput: move |e| time_horizon_days.set(e.value().parse().unwrap_or(10.0)),
                            style: "width: 100%; background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; box-sizing: border-box;"
                        }
                    }
                    div {
                        label { style: "display: block; color: #94a3b8; font-size: 0.75rem; margin-bottom: 0.3rem;", "σ fed to host" }
                        div { style: "padding: 0.5rem; color: #f472b6; font-weight: 600;", "{portfolio_volatility() * 100.0:.2}%" }
                    }
                }

                match &mc {
                    McPhase::Idle => {
                        rsx! {
                            p { style: "margin: 0; color: #64748b; font-size: 0.85rem;",
                                "Press Run to call calculate_monte_carlo_var (not a mock VM payload)."
                            }
                        }
                    }
                    McPhase::Loading => {
                        rsx! {
                            p { style: "margin: 0; color: #38bdf8; font-size: 0.85rem;", "Host Monte Carlo in flight…" }
                        }
                    }
                    McPhase::Error(msg) => {
                        rsx! {
                            div {
                                style: "padding: 0.85rem; border-radius: 8px; background: rgba(127,29,29,0.35); border: 1px solid rgba(248,113,113,0.4); color: #fecaca;",
                                div { style: "font-weight: 700; margin-bottom: 0.25rem;", "Monte Carlo invoke failed" }
                                pre { style: "margin: 0; white-space: pre-wrap; font-size: 0.8rem;", "{msg}" }
                            }
                        }
                    }
                    McPhase::Ready(props) => {
                        // Rust format strings reject Python-style `:,.2` (U5-B pane; minimal compile fix for studio check).
                        let var_s = format!("${:.2}", props.monte_carlo_var);
                        let es_s = format!("${:.2}", props.expected_shortfall);
                        rsx! {
                            div {
                                style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 0.75rem;",
                                div {
                                    style: "padding: 1rem; border-radius: 12px; background: rgba(15,23,42,0.8); border: 1px solid rgba(248,113,113,0.25);",
                                    div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase;", "95% VaR (host)" }
                                    div { style: "font-size: 1.6rem; font-weight: 700; color: #f87171; margin-top: 0.25rem;",
                                        "{var_s}"
                                    }
                                    div { style: "font-size: 0.7rem; color: #64748b; margin-top: 0.25rem;", "run_monte_carlo_var · 10k paths" }
                                }
                                div {
                                    style: "padding: 1rem; border-radius: 12px; background: rgba(15,23,42,0.8); border: 1px solid rgba(251,191,36,0.25);",
                                    div { style: "font-size: 0.7rem; color: #94a3b8; text-transform: uppercase;", "ES (host heuristic)" }
                                    div { style: "font-size: 1.6rem; font-weight: 700; color: #fbbf24; margin-top: 0.25rem;",
                                        "{es_s}"
                                    }
                                    div { style: "font-size: 0.7rem; color: #64748b; margin-top: 0.25rem;", "currently VaR × 1.25 — not full ES" }
                                }
                            }
                        }
                    }
                }
            }

            div {
                style: "background: rgba(0,0,0,0.2); border-radius: 16px; border: 1px solid rgba(255,255,255,0.05); overflow: hidden;",
                table {
                    style: "width: 100%; border-collapse: collapse; text-align: left;",
                    thead {
                        style: "background: rgba(255,255,255,0.05);",
                        tr {
                            th { style: "padding: 1rem; color: #cbd5e1; font-weight: 600;", "Asset" }
                            th { style: "padding: 1rem; color: #cbd5e1; font-weight: 600;", "Weight (%)" }
                            th { style: "padding: 1rem; color: #cbd5e1; font-weight: 600;", "Exp. Return (%)" }
                            th { style: "padding: 1rem; color: #cbd5e1; font-weight: 600;", "Volatility (%)" }
                            th { style: "padding: 1rem; color: #cbd5e1; font-weight: 600; text-align: center;", "Actions" }
                        }
                    }
                    tbody {
                        for asset in assets.read().clone() {
                            tr {
                                style: "border-top: 1px solid rgba(255,255,255,0.05);",
                                td {
                                    style: "padding: 1rem;",
                                    input {
                                        r#type: "text",
                                        value: "{asset.ticker}",
                                        oninput: move |e| update_ticker(asset.id, e.value()),
                                        style: "background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; width: 100%; font-weight: 600; outline: none; box-sizing: border-box;"
                                    }
                                }
                                td {
                                    style: "padding: 1rem;",
                                    input {
                                        r#type: "number",
                                        step: "1",
                                        value: "{asset.weight * 100.0}",
                                        oninput: move |e| update_weight(asset.id, e.value().parse::<f64>().unwrap_or(0.0) / 100.0),
                                        style: "background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; width: 100%; outline: none; box-sizing: border-box;"
                                    }
                                }
                                td {
                                    style: "padding: 1rem;",
                                    input {
                                        r#type: "number",
                                        step: "1",
                                        value: "{asset.expected_return * 100.0}",
                                        oninput: move |e| update_return(asset.id, e.value().parse::<f64>().unwrap_or(0.0) / 100.0),
                                        style: "background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; width: 100%; outline: none; box-sizing: border-box;"
                                    }
                                }
                                td {
                                    style: "padding: 1rem;",
                                    input {
                                        r#type: "number",
                                        step: "1",
                                        value: "{asset.volatility * 100.0}",
                                        oninput: move |e| update_volatility(asset.id, e.value().parse::<f64>().unwrap_or(0.0) / 100.0),
                                        style: "background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; width: 100%; outline: none; box-sizing: border-box;"
                                    }
                                }
                                td {
                                    style: "padding: 1rem; text-align: center;",
                                    button {
                                        onclick: move |_| remove_asset(asset.id),
                                        style: "background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.4); color: #f87171; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer;",
                                        "Remove"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            button {
                onclick: add_asset,
                style: "align-self: flex-start; background: linear-gradient(135deg, #6366f1, #8b5cf6); border: none; color: white; padding: 0.75rem 1.5rem; border-radius: 8px; font-weight: 600; cursor: pointer; box-shadow: 0 4px 15px rgba(99, 102, 241, 0.3);",
                "+ Add Asset"
            }
        }
    }
}
