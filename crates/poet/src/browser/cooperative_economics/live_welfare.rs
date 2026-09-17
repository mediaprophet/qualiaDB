//! Live `Econ.gini` panel for cooperative economics (user-supplied incomes only).

use serde_json::json;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlTextAreaElement};

use super::super::native_daemon::{daemon_invoke, is_daemon_connected};
use super::super::tool_dual_path;

fn parse_incomes(source: &str) -> Vec<f64> {
    source
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | '|'))
        .filter_map(|token| token.trim().parse::<f64>().ok())
        .filter(|n| n.is_finite() && *n >= 0.0)
        .take(4096)
        .collect()
}

fn local_gini(incomes: &[f64]) -> Option<f64> {
    if incomes.len() < 2 {
        return None;
    }
    let mut sorted = incomes.to_vec();
    sorted.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let n = sorted.len() as f64;
    let sum: f64 = sorted.iter().sum();
    if sum <= 0.0 {
        return Some(0.0);
    }
    let mut weighted = 0.0;
    for (i, x) in sorted.iter().enumerate() {
        weighted += (2.0 * (i as f64 + 1.0) - n - 1.0) * x;
    }
    Some(weighted / (n * sum))
}

const CARD_CSS: &str = "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
     border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 6px;";

/// Panel: enter incomes → local sketch offline / `Econ.gini` when daemon is up.
pub(super) fn build_live_welfare_panel(document: &Document) -> Element {
    let card = document.create_element("div").unwrap();
    let card_el: HtmlElement = card.clone().dyn_into().unwrap();
    card_el.style().set_css_text(CARD_CSS);
    let _ = card.set_attribute("data-econ-live-panel", "gini");
    let _ = card.set_attribute("data-live-capability", "Econ.gini");

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("Commons inequality (Live Econ.gini)"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    card.append_child(&title).unwrap();

    let hint = document.create_element("div").unwrap();
    hint.set_text_content(Some(
        "Enter income numbers you supply (comma or space separated). Nothing is invented. Offline = local sketch; daemon = Econ.gini.",
    ));
    let hint_el: HtmlElement = hint.clone().dyn_into().unwrap();
    hint_el
        .style()
        .set_css_text("font-size: 10px; color: #94a3b8;");
    card.append_child(&hint).unwrap();

    let textarea = document.create_element("textarea").unwrap();
    let _ = textarea.set_attribute("id", "coop-econ-incomes");
    let _ = textarea.set_attribute("rows", "3");
    let _ = textarea.set_attribute(
        "placeholder",
        "e.g. 12000 18000 22000 45000  (your figures only)",
    );
    let ta_el: HtmlElement = textarea.clone().dyn_into().unwrap();
    ta_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; background: rgba(0,0,0,0.4); \
         color: #cbd5e1; border: 1px solid rgba(255,255,255,0.1); border-radius: 4px; padding: 6px;",
    );
    card.append_child(&textarea).unwrap();

    let row = document.create_element("div").unwrap();
    let row_el: HtmlElement = row.clone().dyn_into().unwrap();
    row_el
        .style()
        .set_css_text("display: flex; gap: 6px; align-items: center;");

    let btn = document.create_element("button").unwrap();
    btn.set_class_name("vibe-run-btn");
    btn.set_text_content(Some("Compute Gini"));
    let _ = btn.set_attribute("type", "button");
    let _ = btn.set_attribute("data-live-capability", "Econ.gini");
    let btn_el: HtmlElement = btn.clone().dyn_into().unwrap();
    btn_el.style().set_css_text(
        "background: var(--accent-emerald, #00f2a9); color: #020617; font-weight: 700; \
         font-size: 10px; padding: 3px 8px; border-radius: 4px; border: none; cursor: pointer;",
    );
    row.append_child(&btn).unwrap();
    card.append_child(&row).unwrap();

    let status = document.create_element("div").unwrap();
    let _ = status.set_attribute("role", "status");
    let _ = status.set_attribute("data-honesty", "idle");
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; color: #94a3b8; \
         background: rgba(0,0,0,0.3); padding: 4px 6px; border-radius: 4px;",
    );
    status.set_text_content(Some(
        if is_daemon_connected() {
            "Ready — will invoke Econ.gini on the daemon."
        } else {
            "Daemon offline — will show a local Gini sketch only."
        },
    ));
    card.append_child(&status).unwrap();

    let ta_click = textarea.clone();
    let status_click = status.clone();
    let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
        let Ok(area) = ta_click.clone().dyn_into::<HtmlTextAreaElement>() else {
            return;
        };
        let incomes = parse_incomes(&area.value());
        let Some(g) = local_gini(&incomes) else {
            let _ = status_click.set_attribute("data-honesty", "error");
            status_click.set_text_content(Some(
                "Enter at least two non-negative income numbers you supply.",
            ));
            return;
        };
        if !is_daemon_connected() {
            let report = tool_dual_path::local_sketch(
                "Econ.gini",
                &format!("Gini sketch over {} incomes: {g:.4}", incomes.len()),
            );
            let _ = status_click.set_attribute("data-honesty", "local");
            status_click.set_text_content(Some(&report.message));
            return;
        }
        let _ = status_click.set_attribute("data-honesty", "running");
        status_click.set_text_content(Some("Running Econ.gini…"));
        let status_async = status_click.clone();
        let args = json!({ "incomes": incomes });
        wasm_bindgen_futures::spawn_local(async move {
            match daemon_invoke("Econ.gini", args).await {
                Ok(response) if response.ok => {
                    let report = tool_dual_path::live_ok("Econ.gini", &response.value);
                    let _ = status_async.set_attribute("data-honesty", "live");
                    status_async.set_text_content(Some(&report.message));
                }
                Ok(response) => {
                    let report = tool_dual_path::live_denied(
                        "Econ.gini",
                        response
                            .diagnostic
                            .as_deref()
                            .unwrap_or("Econ.gini failed."),
                    );
                    let _ = status_async.set_attribute("data-honesty", "denied");
                    status_async.set_text_content(Some(&report.message));
                }
                Err(error) => {
                    let report = tool_dual_path::live_denied("Econ.gini", &error);
                    let _ = status_async.set_attribute("data-honesty", "denied");
                    status_async.set_text_content(Some(&report.message));
                }
            }
        });
    }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    card
}

#[cfg(test)]
mod tests {
    use super::{local_gini, parse_incomes};

    #[test]
    fn parse_incomes_splits_common_separators() {
        assert_eq!(
            parse_incomes("10, 20;30|40"),
            vec![10.0, 20.0, 30.0, 40.0]
        );
    }

    #[test]
    fn local_gini_needs_two_values() {
        assert!(local_gini(&[1.0]).is_none());
        assert!(local_gini(&[10.0, 10.0]).unwrap().abs() < 1e-9);
    }
}
