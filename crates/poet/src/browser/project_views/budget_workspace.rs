//! Auditable project economics workspace.

use base64::Engine;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement};

use crate::browser::cop_records::{build_family_panel, CopField};
use crate::browser::native_daemon::{
    daemon_records_query, is_daemon_connected, NativeRecordQueryRequest,
};

use super::budget_model::{format_amount, summarize_payloads, BudgetSummary};

const FAMILIES: [(&str, &str); 5] = [
    ("project_budget", "Plan"),
    ("project_actual", "Actuals"),
    ("project_funding", "Funding"),
    ("project_royalty", "Royalties"),
    ("project_tax", "Tax"),
];

const BUDGET_FIELDS: &[CopField] = &[
    CopField {
        key: "amount",
        placeholder: "Amount (up to 6 decimals)",
    },
    CopField {
        key: "currency",
        placeholder: "Currency/unit code (AUD, USD, Q42…)",
    },
    CopField {
        key: "category",
        placeholder: "Budget category",
    },
    CopField {
        key: "lifecycle",
        placeholder: "draft | approved | committed | cancelled",
    },
    CopField {
        key: "effective_date",
        placeholder: "Effective date (YYYY-MM-DD)",
    },
    CopField {
        key: "actor",
        placeholder: "Responsible actor DID",
    },
    CopField {
        key: "provenance",
        placeholder: "Approval / source reference",
    },
    CopField {
        key: "sensitivity",
        placeholder: "Sensitivity (public | restricted | classified)",
    },
];
const ACTUAL_FIELDS: &[CopField] = &[
    CopField {
        key: "amount",
        placeholder: "Amount (up to 6 decimals)",
    },
    CopField {
        key: "currency",
        placeholder: "Currency/unit code",
    },
    CopField {
        key: "category",
        placeholder: "Cost category",
    },
    CopField {
        key: "lifecycle",
        placeholder: "observed | verified | settled",
    },
    CopField {
        key: "effective_date",
        placeholder: "Transaction date (YYYY-MM-DD)",
    },
    CopField {
        key: "actor",
        placeholder: "Verifier / responsible actor DID",
    },
    CopField {
        key: "provenance",
        placeholder: "Invoice / receipt / evidence reference",
    },
    CopField {
        key: "sensitivity",
        placeholder: "Sensitivity (public | restricted | classified)",
    },
];
const FUNDING_FIELDS: &[CopField] = &[
    CopField {
        key: "amount",
        placeholder: "Amount (up to 6 decimals)",
    },
    CopField {
        key: "currency",
        placeholder: "Currency/unit code",
    },
    CopField {
        key: "source",
        placeholder: "Funding source",
    },
    CopField {
        key: "lifecycle",
        placeholder: "pledged | received | restricted | returned",
    },
    CopField {
        key: "effective_date",
        placeholder: "Effective date (YYYY-MM-DD)",
    },
    CopField {
        key: "actor",
        placeholder: "Responsible actor DID",
    },
    CopField {
        key: "provenance",
        placeholder: "Grant / transfer / source reference",
    },
    CopField {
        key: "sensitivity",
        placeholder: "Sensitivity (public | restricted | classified)",
    },
];
const ROYALTY_FIELDS: &[CopField] = &[
    CopField {
        key: "amount",
        placeholder: "Royalty amount due",
    },
    CopField {
        key: "currency",
        placeholder: "Currency/unit code",
    },
    CopField {
        key: "beneficiary",
        placeholder: "Beneficiary DID",
    },
    CopField {
        key: "rate",
        placeholder: "Rate and basis (informational)",
    },
    CopField {
        key: "lifecycle",
        placeholder: "calculated | approved | settled",
    },
    CopField {
        key: "effective_date",
        placeholder: "Effective date (YYYY-MM-DD)",
    },
    CopField {
        key: "actor",
        placeholder: "Approver / responsible actor DID",
    },
    CopField {
        key: "provenance",
        placeholder: "Agreement / calculation reference",
    },
    CopField {
        key: "sensitivity",
        placeholder: "Sensitivity (public | restricted | classified)",
    },
];
const TAX_FIELDS: &[CopField] = &[
    CopField {
        key: "amount",
        placeholder: "Tax amount due",
    },
    CopField {
        key: "currency",
        placeholder: "Currency/unit code",
    },
    CopField {
        key: "jurisdiction",
        placeholder: "Jurisdiction",
    },
    CopField {
        key: "rate",
        placeholder: "Rate and basis (informational)",
    },
    CopField {
        key: "lifecycle",
        placeholder: "estimated | filed | settled",
    },
    CopField {
        key: "effective_date",
        placeholder: "Effective date (YYYY-MM-DD)",
    },
    CopField {
        key: "actor",
        placeholder: "Preparer / responsible actor DID",
    },
    CopField {
        key: "provenance",
        placeholder: "Return / assessment / source reference",
    },
    CopField {
        key: "sensitivity",
        placeholder: "Sensitivity (public | restricted | classified)",
    },
];

pub fn build_budget_view(document: &Document) -> Element {
    let root = document.create_element("section").unwrap();
    root.set_attribute("data-project-budget-workspace", "live")
        .ok();
    root.set_attribute("data-honesty", "running").ok();

    let intro = document.create_element("div").unwrap();
    intro.set_inner_html(
        "<h3 style=\"margin:0 0 4px\">Project economics</h3>\
         <p style=\"margin:0;color:var(--text-muted);font-size:11px\">\
         Auditable plan, actual, funding, royalty and tax ledgers. Only approved/verified states \
         enter the summary; settlement is never inferred.</p>",
    );
    root.append_child(&intro).unwrap();

    let controls = document.create_element("div").unwrap();
    style(
        &controls,
        "display:flex;gap:6px;align-items:center;margin:10px 0;",
    );
    let refresh = button(document, "Recalculate from ledgers");
    let export = button(document, "Export audit JSON");
    controls.append_child(&refresh).unwrap();
    controls.append_child(&export).unwrap();
    root.append_child(&controls).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    status.set_attribute("data-budget-status", "").ok();
    style(
        &status,
        "font:10px var(--font-mono);color:var(--text-muted);margin-bottom:8px;",
    );
    status.set_text_content(Some("Loading economic ledgers…"));
    root.append_child(&status).unwrap();

    let summary = document.create_element("div").unwrap();
    summary.set_attribute("data-budget-summary", "").ok();
    root.append_child(&summary).unwrap();

    let specs = [
        (
            "project_budget",
            "Plan — approvals and commitments",
            BUDGET_FIELDS,
        ),
        (
            "project_actual",
            "Actuals — observed, verified and settled costs",
            ACTUAL_FIELDS,
        ),
        (
            "project_funding",
            "Funding — pledges, receipts, restrictions and returns",
            FUNDING_FIELDS,
        ),
        (
            "project_royalty",
            "Royalties — calculated obligations and settlement",
            ROYALTY_FIELDS,
        ),
        (
            "project_tax",
            "Tax — estimates, filings and settlement",
            TAX_FIELDS,
        ),
    ];
    for (family, label, fields) in specs {
        let details = document.create_element("details").unwrap();
        style(
            &details,
            "border-top:1px solid var(--border-medium);padding:8px 0;",
        );
        let heading = document.create_element("summary").unwrap();
        heading.set_text_content(Some(label));
        style(&heading, "cursor:pointer;font-size:11px;font-weight:600;");
        details.append_child(&heading).unwrap();
        details
            .append_child(&build_family_panel(document, family, label, fields))
            .unwrap();
        root.append_child(&details).unwrap();
    }

    let refresh_root = root.clone();
    let refresh_status = status.clone();
    let refresh_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_summary(&refresh_root, &refresh_status);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", refresh_closure.as_ref().unchecked_ref())
        .unwrap();
    refresh_closure.forget();

    let export_doc = document.clone();
    let export_status = status.clone();
    let export_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let document = export_doc.clone();
        let status = export_status.clone();
        wasm_bindgen_futures::spawn_local(async move {
            status.set_text_content(Some("Building audit export from live ledgers…"));
            match query_ledgers().await {
                Ok(payloads) => {
                    let object = payloads
                        .into_iter()
                        .map(|(family, value)| (family.to_string(), value))
                        .collect::<serde_json::Map<_, _>>();
                    let bytes = serde_json::to_vec_pretty(&serde_json::json!({
                        "contract": "poet-project-economics-audit-v1",
                        "warning": "Lifecycle states are evidence; this export does not assert payment or legal settlement.",
                        "ledgers": object
                    }))
                    .unwrap_or_default();
                    download_json(&document, &bytes);
                    status.set_text_content(Some(
                        "Audit JSON exported from the current daemon state.",
                    ));
                }
                Err(error) => status.set_text_content(Some(&error)),
            }
        });
    }) as Box<dyn FnMut(_)>);
    export
        .add_event_listener_with_callback("click", export_closure.as_ref().unchecked_ref())
        .unwrap();
    export_closure.forget();

    refresh_summary(&root, &status);
    root
}

fn refresh_summary(root: &Element, status: &Element) {
    if !is_daemon_connected() {
        root.set_attribute("data-honesty", "unavailable").ok();
        status.set_text_content(Some(
            "Unavailable: start the local QualiaDB daemon; no totals are fabricated.",
        ));
        return;
    }
    root.set_attribute("data-honesty", "running").ok();
    status.set_text_content(Some("Recalculating from live economic ledgers…"));
    let root = root.clone();
    let status = status.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match query_ledgers().await {
            Ok(payloads) => {
                let borrowed = payloads
                    .iter()
                    .map(|(family, value)| (*family, value))
                    .collect::<Vec<_>>();
                render_summary(&root, &summarize_payloads(&borrowed));
                root.set_attribute("data-honesty", "live").ok();
                status.set_text_content(Some("Live ledger totals. Pending states are excluded; currency units remain separate."));
            }
            Err(error) => {
                root.set_attribute("data-honesty", "error").ok();
                status.set_text_content(Some(&error));
            }
        }
    });
}

async fn query_ledgers() -> Result<Vec<(&'static str, serde_json::Value)>, String> {
    let mut payloads = Vec::with_capacity(FAMILIES.len());
    for (family, _) in FAMILIES {
        let response = daemon_records_query(NativeRecordQueryRequest {
            family: family.to_string(),
            query: String::new(),
            kind: String::new(),
        })
        .await?;
        if !response.ok {
            return Err(response
                .diagnostic
                .unwrap_or_else(|| format!("{family} query failed")));
        }
        payloads.push((family, response.data));
    }
    Ok(payloads)
}

fn render_summary(root: &Element, data: &BudgetSummary) {
    let Some(container) = root.query_selector("[data-budget-summary]").ok().flatten() else {
        return;
    };
    container.set_inner_html("");
    if data.currencies.is_empty() {
        container.set_text_content(Some("No approved or verified economic rows yet."));
        return;
    }
    let document = root.owner_document().unwrap();
    for row in &data.currencies {
        let card = document.create_element("div").unwrap();
        style(
            &card,
            "border:1px solid var(--border-medium);border-radius:6px;padding:8px;margin:6px 0;background:var(--surface-panel);",
        );
        let values = [
            ("Approved plan", row.approved_plan),
            ("Verified actual", row.verified_actual),
            ("Variance", row.variance()),
            ("Funding received", row.received_funding),
            ("Funding position", row.funding_position()),
            ("Royalties due", row.royalties_due),
            ("Tax due", row.tax_due),
        ];
        let title = document.create_element("strong").unwrap();
        title.set_text_content(Some(&row.currency));
        card.append_child(&title).unwrap();
        let grid = document.create_element("div").unwrap();
        style(
            &grid,
            "display:grid;grid-template-columns:repeat(auto-fit,minmax(110px,1fr));gap:6px;margin-top:6px;",
        );
        for (label, amount) in values {
            let metric = document.create_element("div").unwrap();
            metric.set_inner_html(&format!("<small style=\"color:var(--text-muted)\">{label}</small><br><span style=\"font:600 12px var(--font-mono)\">{}</span>", format_amount(amount)));
            grid.append_child(&metric).unwrap();
        }
        card.append_child(&grid).unwrap();
        container.append_child(&card).unwrap();
    }
    let audit = document.create_element("p").unwrap();
    audit.set_text_content(Some(&format!("{} rows included · {} pending/settled/cancelled rows excluded · {} invalid legacy rows excluded", data.included_rows, data.pending_rows, data.invalid_rows)));
    style(&audit, "font:9px var(--font-mono);color:var(--text-muted);");
    container.append_child(&audit).unwrap();
}

fn button(document: &Document, label: &str) -> Element {
    let button = document.create_element("button").unwrap();
    button.set_attribute("type", "button").ok();
    button.set_text_content(Some(label));
    style(
        &button,
        "padding:5px 10px;border:1px solid var(--border-medium);background:transparent;color:var(--text-secondary);border-radius:3px;cursor:pointer;font-size:10px;",
    );
    button
}

fn style(element: &Element, css: &str) {
    element
        .clone()
        .dyn_into::<HtmlElement>()
        .unwrap()
        .style()
        .set_css_text(css);
}

fn download_json(document: &Document, bytes: &[u8]) {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let anchor = document.create_element("a").unwrap();
    anchor
        .set_attribute("href", &format!("data:application/json;base64,{encoded}"))
        .ok();
    anchor
        .set_attribute("download", "poet-project-economics-audit.json")
        .ok();
    style(&anchor, "display:none;");
    if let Some(body) = document.body() {
        body.append_child(&anchor).ok();
        anchor.clone().dyn_into::<HtmlElement>().unwrap().click();
        anchor.remove();
    }
}
