//! Vitals metric selector and accessible data view (HLT-02).
//!
//! Provides interactive metric selection (BP, Heart Rate, Glucose, Lab Analytes),
//! SVG visual trend projection, and an accessible HTML table alternative.
//! Strictly partitions points by unit to prevent silent unit mixing.
//! Does not apply unlicensed clinical interpretations or diagnostic ranges.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Element, KeyboardEvent, MouseEvent};

use super::model::{
    available_metric_kinds, extract_metric_series, HealthRecord, MetricKind, MetricSeries,
};

/// Render or update the vitals panel inside the container.
///
/// State (selected metric and view mode) is preserved in data-attributes on `container`.
pub fn render_vitals_panel(container: &Element, records: &[HealthRecord]) {
    let document = match container.owner_document() {
        Some(doc) => doc,
        None => return,
    };

    let selected_id = container
        .get_attribute("data-selected-metric")
        .unwrap_or_else(|| "bp".into());
    let view_mode = container
        .get_attribute("data-view-mode")
        .unwrap_or_else(|| "chart".into());

    let all_kinds = available_metric_kinds(records);
    let selected_kind = all_kinds
        .iter()
        .find(|k| k.id() == selected_id)
        .cloned()
        .unwrap_or_else(|| all_kinds.first().cloned().unwrap_or(MetricKind::BloodPressure));

    // Clear previous panel content
    while let Some(child) = container.first_element_child() {
        child.remove();
    }

    // 1. Controls bar: metric tabs and view mode toggle
    let controls = document.create_element("div").unwrap();
    controls.set_class_name("vitals-controls");

    let tablist = document.create_element("div").unwrap();
    tablist.set_class_name("vitals-metric-nav");
    tablist.set_attribute("role", "tablist").ok();
    tablist
        .set_attribute("aria-label", "Select health metric to inspect")
        .ok();

    for kind in &all_kinds {
        let is_selected = kind.id() == selected_kind.id();
        let tab = document.create_element("button").unwrap();
        tab.set_class_name("vitals-metric-tab");
        tab.set_attribute("role", "tab").ok();
        tab.set_attribute("type", "button").ok();
        tab.set_attribute("data-metric-id", &kind.id()).ok();
        tab.set_attribute("aria-selected", if is_selected { "true" } else { "false" })
            .ok();
        tab.set_attribute(
            "tabindex",
            if is_selected { "0" } else { "-1" },
        )
        .ok();
        tab.set_text_content(Some(&kind.label()));

        // Click handler to select metric
        let c_clone = container.clone();
        let recs_clone = records.to_vec();
        let m_id = kind.id();
        let click_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            c_clone.set_attribute("data-selected-metric", &m_id).ok();
            render_vitals_panel(&c_clone, &recs_clone);
        }) as Box<dyn FnMut(_)>);
        tab.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
            .ok();
        click_closure.forget();

        // Keyboard navigation (ArrowLeft / ArrowRight)
        let c_clone_k = container.clone();
        let recs_clone_k = records.to_vec();
        let all_ids = all_kinds.iter().map(|k| k.id()).collect::<Vec<_>>();
        let cur_id = kind.id();
        let key_closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
            let key = e.key();
            let mut target_id: Option<String> = None;
            if let Some(pos) = all_ids.iter().position(|id| id == &cur_id) {
                if key == "ArrowRight" || key == "ArrowDown" {
                    let next = (pos + 1) % all_ids.len();
                    target_id = Some(all_ids[next].clone());
                } else if key == "ArrowLeft" || key == "ArrowUp" {
                    let prev = if pos == 0 { all_ids.len() - 1 } else { pos - 1 };
                    target_id = Some(all_ids[prev].clone());
                } else if key == "Home" {
                    target_id = all_ids.first().cloned();
                } else if key == "End" {
                    target_id = all_ids.last().cloned();
                }
            }
            if let Some(new_id) = target_id {
                e.prevent_default();
                c_clone_k.set_attribute("data-selected-metric", &new_id).ok();
                render_vitals_panel(&c_clone_k, &recs_clone_k);
                // Move focus to the newly selected tab
                if let Ok(Some(new_tab)) = c_clone_k.query_selector(&format!("[data-metric-id=\"{new_id}\"]")) {
                    if let Ok(html_el) = new_tab.dyn_into::<web_sys::HtmlElement>() {
                        html_el.focus().ok();
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);
        tab.add_event_listener_with_callback("keydown", key_closure.as_ref().unchecked_ref())
            .ok();
        key_closure.forget();

        tablist.append_child(&tab).unwrap();
    }
    controls.append_child(&tablist).unwrap();

    // View toggle button: Chart vs Table
    let toggle_btn = document.create_element("button").unwrap();
    toggle_btn.set_class_name("vitals-view-toggle");
    toggle_btn.set_attribute("type", "button").ok();
    toggle_btn.set_attribute("aria-label", "Toggle between visual chart and accessible data table").ok();
    let is_table = view_mode == "table";
    toggle_btn.set_text_content(Some(if is_table {
        "📈 Switch to visual chart"
    } else {
        "📋 Switch to accessible table"
    }));

    let c_toggle = container.clone();
    let recs_toggle = records.to_vec();
    let toggle_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let new_mode = if is_table { "chart" } else { "table" };
        c_toggle.set_attribute("data-view-mode", new_mode).ok();
        render_vitals_panel(&c_toggle, &recs_toggle);
    }) as Box<dyn FnMut(_)>);
    toggle_btn.add_event_listener_with_callback("click", toggle_closure.as_ref().unchecked_ref())
        .ok();
    toggle_closure.forget();

    controls.append_child(&toggle_btn).unwrap();
    container.append_child(&controls).unwrap();

    // 2. Body: Content area
    let content_area = document.create_element("div").unwrap();
    content_area.set_class_name("vitals-display-area");

    let series_list = extract_metric_series(records, &selected_kind);

    if series_list.is_empty() || series_list.iter().all(|s| s.points.is_empty()) {
        let empty = document.create_element("div").unwrap();
        empty.set_class_name("health-empty-state");
        empty.set_attribute("role", "status").ok();
        let icon = document.create_element("span").unwrap();
        icon.set_text_content(Some("⌁"));
        let strong = document.create_element("strong").unwrap();
        strong.set_text_content(Some(&format!("No {} measurements yet", selected_kind.label())));
        let small = document.create_element("small").unwrap();
        small.set_text_content(Some("Save a reading to view historical pattern and records."));
        empty.append_child(&icon).unwrap();
        empty.append_child(&strong).unwrap();
        empty.append_child(&small).unwrap();
        content_area.append_child(&empty).unwrap();
    } else if is_table {
        render_table_view(&document, &content_area, &selected_kind, &series_list);
    } else {
        render_chart_view(&document, &content_area, &selected_kind, &series_list);
    }

    container.append_child(&content_area).unwrap();
}

fn render_table_view(
    document: &web_sys::Document,
    parent: &Element,
    kind: &MetricKind,
    series_list: &[MetricSeries],
) {
    let wrap = document.create_element("div").unwrap();
    wrap.set_class_name("vitals-table-wrap");
    wrap.set_attribute("role", "region").ok();
    wrap.set_attribute("tabindex", "0").ok();
    wrap.set_attribute("aria-label", &format!("Table view of {}", kind.label())).ok();

    for series in series_list {
        if series.points.is_empty() {
            continue;
        }
        let table = document.create_element("table").unwrap();
        table.set_class_name("vitals-table");

        let caption = document.create_element("caption").unwrap();
        caption.set_text_content(Some(&format!(
            "Historical measurements for {} (Unit: {})",
            kind.label(),
            series.unit
        )));
        table.append_child(&caption).unwrap();

        let thead = document.create_element("thead").unwrap();
        let head_row = document.create_element("tr").unwrap();

        for col_name in &["Date & Time", "Reading", "Unit", "Sensitivity", "Record ID"] {
            let th = document.create_element("th").unwrap();
            th.set_attribute("scope", "col").ok();
            th.set_text_content(Some(col_name));
            head_row.append_child(&th).unwrap();
        }
        thead.append_child(&head_row).unwrap();
        table.append_child(&thead).unwrap();

        let tbody = document.create_element("tbody").unwrap();
        // Display newest first in tabular view for easy reading
        for pt in series.points.iter().rev() {
            let row = document.create_element("tr").unwrap();

            let td_time = document.create_element("td").unwrap();
            td_time.set_text_content(Some(&pt.timestamp_label));
            row.append_child(&td_time).unwrap();

            let td_val = document.create_element("td").unwrap();
            let val_text = if let Some(sec) = pt.secondary {
                format!("{:.0}/{:.0}", pt.primary, sec)
            } else if pt.primary.fract() == 0.0 {
                format!("{:.0}", pt.primary)
            } else {
                format!("{:.1}", pt.primary)
            };
            td_val.set_text_content(Some(&val_text));
            row.append_child(&td_val).unwrap();

            let td_unit = document.create_element("td").unwrap();
            td_unit.set_text_content(Some(&pt.unit));
            row.append_child(&td_unit).unwrap();

            let td_sens = document.create_element("td").unwrap();
            td_sens.set_text_content(Some(&pt.sensitivity));
            row.append_child(&td_sens).unwrap();

            let td_id = document.create_element("td").unwrap();
            let code = document.create_element("code").unwrap();
            code.set_text_content(Some(&pt.record_id));
            td_id.append_child(&code).unwrap();
            row.append_child(&td_id).unwrap();

            tbody.append_child(&row).unwrap();
        }
        table.append_child(&tbody).unwrap();
        wrap.append_child(&table).unwrap();
    }
    parent.append_child(&wrap).unwrap();
}

fn render_chart_view(
    document: &web_sys::Document,
    parent: &Element,
    kind: &MetricKind,
    series_list: &[MetricSeries],
) {
    let wrap = document.create_element("div").unwrap();
    wrap.set_class_name("vitals-chart-wrap");

    for series in series_list {
        if series.points.is_empty() {
            continue;
        }

        let series_card = document.create_element("div").unwrap();
        series_card.set_class_name("vitals-series-chart");

        let visible = &series.points[series.points.len().saturating_sub(16)..];
        let is_bp = matches!(kind, MetricKind::BloodPressure);

        // Find min and max for scaling
        let mut min_val = f64::MAX;
        let mut max_val = f64::MIN;
        for p in visible {
            min_val = min_val.min(p.primary);
            max_val = max_val.max(p.primary);
            if let Some(sec) = p.secondary {
                min_val = min_val.min(sec);
                max_val = max_val.max(sec);
            }
        }
        if min_val >= max_val {
            min_val -= 10.0;
            max_val += 10.0;
        } else {
            let padding = (max_val - min_val) * 0.15;
            min_val -= padding;
            max_val += padding;
        }

        let val_range = (max_val - min_val).max(1.0);
        let n_points = visible.len().saturating_sub(1).max(1) as f64;

        let scale_x = |idx: usize| -> f64 { 44.0 + (idx as f64) * (560.0 / n_points) };
        let scale_y = |val: f64| -> f64 { 180.0 - ((val - min_val) / val_range * 140.0) };

        // Construct SVG paths
        let mut primary_pts = Vec::new();
        let mut secondary_pts = Vec::new();
        let mut dots_svg = String::new();

        for (i, p) in visible.iter().enumerate() {
            let x = scale_x(i);
            let y1 = scale_y(p.primary);
            primary_pts.push(format!("{x:.1},{y1:.1}"));
            let tooltip_val1 = if is_bp {
                format!("Systolic: {:.0} {}", p.primary, p.unit)
            } else {
                format!("Value: {:.1} {}", p.primary, p.unit)
            };
            dots_svg.push_str(&format!(
                "<circle class=\"health-dot-sys\" cx=\"{x:.1}\" cy=\"{y1:.1}\" r=\"4\"><title>{}: {}</title></circle>",
                p.timestamp_label, tooltip_val1
            ));

            if let Some(sec) = p.secondary {
                let y2 = scale_y(sec);
                secondary_pts.push(format!("{x:.1},{y2:.1}"));
                dots_svg.push_str(&format!(
                    "<circle class=\"health-dot-dia\" cx=\"{x:.1}\" cy=\"{y2:.1}\" r=\"4\"><title>{}: Diastolic: {:.0} {}</title></circle>",
                    p.timestamp_label, sec, p.unit
                ));
            }
        }

        let primary_polyline = primary_pts.join(" ");
        let secondary_polyline = secondary_pts.join(" ");

        let latest = visible.last().unwrap();
        let latest_label = if let Some(sec) = latest.secondary {
            format!("{:.0}/{:.0} {}", latest.primary, sec, latest.unit)
        } else if latest.primary.fract() == 0.0 {
            format!("{:.0} {}", latest.primary, latest.unit)
        } else {
            format!("{:.1} {}", latest.primary, latest.unit)
        };

        let svg_html = format!(
            r#"<div class="vitals-chart-header">
                <span class="vitals-chart-unit-badge">Unit: <strong>{}</strong></span>
                <span class="vitals-chart-latest">Latest: <strong>{}</strong></span>
              </div>
              <svg viewBox="0 0 640 210" class="vitals-svg" aria-label="Trend chart for {} in {}" role="img">
                <g class="health-chart-grid">
                  <line x1="44" y1="40" x2="604" y2="40" />
                  <line x1="44" y1="110" x2="604" y2="110" />
                  <line x1="44" y1="180" x2="604" y2="180" />
                </g>
                <text x="38" y="44" text-anchor="end" class="chart-axis-label">{:.0}</text>
                <text x="38" y="114" text-anchor="end" class="chart-axis-label">{:.0}</text>
                <text x="38" y="184" text-anchor="end" class="chart-axis-label">{:.0}</text>
                <polyline class="{}" points="{}" fill="none" />
                {}
                {}
              </svg>
              <div class="vitals-chart-footer">
                <small class="vitals-chart-range">Timeline: {} &rarr; {}</small>
                <span class="vitals-no-diagnosis-notice">Source record projection &middot; No clinical range applied</span>
              </div>"#,
            series.unit,
            latest_label,
            kind.label(),
            series.unit,
            max_val,
            (min_val + max_val) / 2.0,
            min_val,
            if is_bp { "health-line-sys" } else { "health-line-hr" },
            primary_polyline,
            if is_bp && !secondary_polyline.is_empty() {
                format!("<polyline class=\"health-line-dia\" points=\"{}\" fill=\"none\" />", secondary_polyline)
            } else {
                String::new()
            },
            dots_svg,
            visible.first().map(|p| p.timestamp_label.as_str()).unwrap_or(""),
            visible.last().map(|p| p.timestamp_label.as_str()).unwrap_or("")
        );

        series_card.set_inner_html(&svg_html);
        wrap.append_child(&series_card).unwrap();
    }

    parent.append_child(&wrap).unwrap();
}
