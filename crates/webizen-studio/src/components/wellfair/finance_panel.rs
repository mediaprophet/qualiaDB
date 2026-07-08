//! Personal finance — ledger entries and derived per-currency balance (Phase 5 / FIN).

use super::host_client::{
    add_ledger_entry, fetch_health_records, fetch_ledger_balance, BalanceReportDto,
};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

#[derive(Clone, Debug)]
struct FinanceUi {
    status: String,
    description: String,
    amount: String,
    currency: String,
    direction: String,
    category: String,
    balance: BalanceReportDto,
    records: Vec<HealthRecordDto>,
}

impl Default for FinanceUi {
    fn default() -> Self {
        Self {
            status: String::new(),
            description: String::new(),
            amount: String::new(),
            currency: "AUD".into(),
            direction: "out".into(),
            category: String::new(),
            balance: BalanceReportDto::default(),
            records: Vec::new(),
        }
    }
}

/// Parse a decimal money string like "42.50" into signed minor units (cents).
fn to_cents(amount: &str, outgoing: bool) -> Option<i64> {
    let parsed: f64 = amount.trim().parse().ok()?;
    if parsed < 0.0 {
        return None; // magnitude only; direction sets the sign
    }
    let cents = (parsed * 100.0).round() as i64;
    Some(if outgoing { -cents } else { cents })
}

fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

#[component]
pub fn WellfairFinancePanel() -> Element {
    let mut ui = use_signal(FinanceUi::default);

    let reload = move || {
        spawn(async move {
            if let Ok(list) = fetch_health_records(64).await {
                ui.write().records = list
                    .into_iter()
                    .filter(|r| r.kind == "ledger_entry")
                    .collect();
            }
            if let Ok(balance) = fetch_ledger_balance(128).await {
                ui.write().balance = balance;
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair personal finance",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Personal finance ledger" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Self-reported ledger. Balances are derived from unique signed entries — a duplicate or replayed sync can never move money."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }

            div {
                style: "display:grid;grid-template-columns:2fr 1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                input {
                    r#type: "text",
                    placeholder: "Description (e.g. Groceries)",
                    value: "{ui().description}",
                    oninput: move |e| ui.write().description = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Amount (42.50)",
                    value: "{ui().amount}",
                    oninput: move |e| ui.write().amount = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Currency",
                    value: "{ui().currency}",
                    oninput: move |e| ui.write().currency = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            div {
                style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Direction"
                    select {
                        value: "{ui().direction}",
                        onchange: move |e| ui.write().direction = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                        option { value: "out", "Money out (expense)" }
                        option { value: "in", "Money in (income)" }
                    }
                }
                input {
                    r#type: "text",
                    placeholder: "Category (optional)",
                    value: "{ui().category}",
                    oninput: move |e| ui.write().category = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            button {
                style: "margin-bottom:0.85rem;padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let description = ui().description.trim().to_string();
                    let outgoing = ui().direction == "out";
                    let currency = ui().currency.trim().to_uppercase();
                    let category = ui().category.trim().to_string();
                    if description.is_empty() {
                        ui.write().status = "Description required.".into();
                        return;
                    }
                    let Some(cents) = to_cents(&ui().amount, outgoing) else {
                        ui.write().status = "Enter a positive amount like 42.50.".into();
                        return;
                    };
                    if currency.is_empty() {
                        ui.write().status = "Currency required (e.g. AUD).".into();
                        return;
                    }
                    spawn(async move {
                        ui.write().status = "Saving ledger entry…".into();
                        let cat = if category.is_empty() { None } else { Some(category.as_str()) };
                        match add_ledger_entry(&description, cents, &currency, cat).await {
                            Ok(_) => {
                                ui.write().status = "Ledger entry saved.".into();
                                ui.write().description = String::new();
                                ui.write().amount = String::new();
                                ui.write().category = String::new();
                                reload();
                            }
                            Err(e) => ui.write().status = format!("Failed: {e}"),
                        }
                    });
                },
                "Add ledger entry"
            }

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Balance" }
            if ui().balance.by_currency.is_empty() {
                p {
                    style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#888);",
                    "No entries yet."
                }
            } else {
                ul {
                    style: "margin:0 0 0.85rem;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.3rem;",
                    for bal in ui().balance.by_currency.clone() {
                        li {
                            key: "{bal.currency}",
                            style: "display:flex;justify-content:space-between;padding:0.4rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.78rem;",
                            strong { "{bal.currency}" }
                            span {
                                style: if bal.net_cents < 0 { "color:#b5341f;" } else { "color:#2a7a3f;" },
                                "{format_cents(bal.net_cents)} ({bal.entry_count} entries)"
                            }
                        }
                    }
                }
            }

            if !ui().records.is_empty() {
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Recent entries ({ui().records.len()})" }
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.35rem;",
                    for r in ui().records.clone() {
                        li {
                            key: "{r.id}",
                            style: "padding:0.4rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.74rem;",
                            span { style: "color:var(--qualia-text-muted,#888);",
                                "{r.summary.as_deref().unwrap_or(\"—\")}"
                            }
                        }
                    }
                }
            }
        }
    }
}
