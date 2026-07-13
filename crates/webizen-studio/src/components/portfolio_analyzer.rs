use crate::components::qapp_engine::invoke_json;
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
struct Asset {
    id: usize,
    ticker: String,
    weight: f64,
    expected_return: f64,
    volatility: f64,
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
    
    let mut is_loading = use_signal(|| false);
    let mut backend_status = use_signal(|| "QualiaDB Mock Active".to_string());

    let run_backend_analysis = move |_| {
        is_loading.set(true);
        backend_status.set("Analyzing...".to_string());
        spawn(async move {
            let args = serde_json::json!({
                "payload": [5, 6, 7], // mock payload for SLG VM
            });

            if let Ok(res) = invoke_json("execute_computational_vm", args).await {
                if let Ok(text) = serde_json::from_value::<String>(res) {
                    backend_status.set(format!("VM: {}", text));
                }
            } else {
                backend_status.set("Error".to_string());
            }
            is_loading.set(false);
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

    let portfolio_return = use_memo(move || {
        assets
            .read()
            .clone()
            .into_iter()
            .map(|a| a.weight * a.expected_return)
            .sum::<f64>()
    });

    let portfolio_volatility = use_memo(move || {
        let var: f64 = assets
            .read()
            .clone()
            .into_iter()
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
        } // 2% risk-free rate assumption
    });

    rsx! {
        div {
            style: "flex: 1; padding: 2.5rem; background: linear-gradient(135deg, #0f172a, #1e293b); border-radius: 16px; color: #f8fafc; font-family: 'Inter', system-ui, sans-serif; box-shadow: 0 20px 40px rgba(0,0,0,0.4); display: flex; flex-direction: column; gap: 2rem; overflow-y: auto;",

            div {
                style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 1px solid rgba(255,255,255,0.1); padding-bottom: 1rem;",
                h2 {
                    style: "margin: 0; font-size: 2.5rem; font-weight: 800; background: linear-gradient(to right, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent;",
                    "Portfolio Analyzer"
                }
                    button {
                        style: "background: rgba(56, 189, 248, 0.1); border: 1px solid rgba(56, 189, 248, 0.3); padding: 0.5rem 1rem; border-radius: 9999px; font-size: 0.875rem; color: #38bdf8; display: flex; align-items: center; gap: 0.5rem; cursor: pointer;",
                        onclick: run_backend_analysis,
                        div { style: "width: 8px; height: 8px; border-radius: 50%; background: #38bdf8; box-shadow: 0 0 10px #38bdf8;" }
                        "{backend_status}"
                    }
            }

            div {
                style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1.5rem;",

                div {
                    style: "background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.05); border-radius: 16px; padding: 1.5rem; display: flex; flex-direction: column; align-items: center; justify-content: center; backdrop-filter: blur(10px); transition: transform 0.2s; cursor: default;",
                    span { style: "color: #94a3b8; font-size: 0.875rem; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.5rem;", "Expected Return" }
                    div { style: "font-size: 3rem; font-weight: 700; color: #34d399;", "{portfolio_return() * 100.0:.2}%" }
                }

                div {
                    style: "background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.05); border-radius: 16px; padding: 1.5rem; display: flex; flex-direction: column; align-items: center; justify-content: center; backdrop-filter: blur(10px);",
                    span { style: "color: #94a3b8; font-size: 0.875rem; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.5rem;", "Expected Volatility" }
                    div { style: "font-size: 3rem; font-weight: 700; color: #f472b6;", "{portfolio_volatility() * 100.0:.2}%" }
                }

                div {
                    style: "background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.05); border-radius: 16px; padding: 1.5rem; display: flex; flex-direction: column; align-items: center; justify-content: center; backdrop-filter: blur(10px);",
                    span { style: "color: #94a3b8; font-size: 0.875rem; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 0.5rem;", "Sharpe Ratio" }
                    div { style: "font-size: 3rem; font-weight: 700; color: #60a5fa;", "{sharpe_ratio():.2}" }
                }
            }

            div {
                style: "background: rgba(0,0,0,0.2); border-radius: 16px; border: 1px solid rgba(255,255,255,0.05); overflow: hidden; backdrop-filter: blur(5px);",
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
                                style: "border-top: 1px solid rgba(255,255,255,0.05); transition: background 0.2s;",
                                td {
                                    style: "padding: 1rem;",
                                    input {
                                        type: "text",
                                        value: "{asset.ticker}",
                                        oninput: move |e| update_ticker(asset.id, e.value()),
                                        style: "background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; width: 100%; font-weight: 600; outline: none; transition: border-color 0.2s;"
                                    }
                                }
                                td {
                                    style: "padding: 1rem;",
                                    input {
                                        type: "number", step: "1",
                                        value: "{asset.weight * 100.0}",
                                        oninput: move |e| update_weight(asset.id, e.value().parse::<f64>().unwrap_or(0.0) / 100.0),
                                        style: "background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; width: 100%; outline: none;"
                                    }
                                }
                                td {
                                    style: "padding: 1rem;",
                                    input {
                                        type: "number", step: "1",
                                        value: "{asset.expected_return * 100.0}",
                                        oninput: move |e| update_return(asset.id, e.value().parse::<f64>().unwrap_or(0.0) / 100.0),
                                        style: "background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; width: 100%; outline: none;"
                                    }
                                }
                                td {
                                    style: "padding: 1rem;",
                                    input {
                                        type: "number", step: "1",
                                        value: "{asset.volatility * 100.0}",
                                        oninput: move |e| update_volatility(asset.id, e.value().parse::<f64>().unwrap_or(0.0) / 100.0),
                                        style: "background: rgba(255,255,255,0.05); border: 1px solid rgba(255,255,255,0.1); color: white; padding: 0.5rem; border-radius: 6px; width: 100%; outline: none;"
                                    }
                                }
                                td {
                                    style: "padding: 1rem; text-align: center;",
                                    button {
                                        onclick: move |_| remove_asset(asset.id),
                                        style: "background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.4); color: #f87171; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; transition: all 0.2s;",
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
                style: "align-self: flex-start; background: linear-gradient(135deg, #6366f1, #8b5cf6); border: none; color: white; padding: 0.75rem 1.5rem; border-radius: 8px; font-weight: 600; cursor: pointer; transition: transform 0.2s, box-shadow 0.2s; box-shadow: 0 4px 15px rgba(99, 102, 241, 0.3);",
                "+ Add Asset"
            }
        }
    }
}
