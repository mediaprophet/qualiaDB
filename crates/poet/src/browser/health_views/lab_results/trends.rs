//! Lab Results — trends tab: time-series sparkline graphs per test (§3.1).
//!
//! Renders inline SVG sparklines with reference-range bands, out-of-range
//! point highlighting, and directional indicators. Each test with historical
//! data gets its own row with a sparkline + latest value + trend arrow.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Historical data point: (date, value).
type DataPoint = (&'static str, f64);

/// Test history record with reference range and time-series data.
struct TestHistory {
    test_name: &'static str,
    unit: &'static str,
    ref_low: f64,
    ref_high: f64,
    critical_low: Option<f64>,
    critical_high: Option<f64>,
    loinc: &'static str,
    points: &'static [DataPoint],
}

const HISTORIES: &[TestHistory] = &[
    TestHistory {
        test_name: "Ferritin",
        unit: "\u{00B5}g/L",
        ref_low: 30.0,
        ref_high: 400.0,
        critical_low: Some(10.0),
        critical_high: None,
        loinc: "LOINC 2276-4",
        points: &[
            ("2024-06-10", 45.0),
            ("2025-01-15", 32.0),
            ("2025-06-20", 22.0),
            ("2025-11-20", 18.0),
            ("2026-03-10", 15.0),
            ("2026-08-15", 12.0),
        ],
    },
    TestHistory {
        test_name: "Haemoglobin",
        unit: "g/L",
        ref_low: 120.0,
        ref_high: 160.0,
        critical_low: Some(80.0),
        critical_high: Some(200.0),
        loinc: "LOINC 718-7",
        points: &[
            ("2024-06-10", 138.0),
            ("2025-01-15", 140.0),
            ("2025-06-20", 145.0),
            ("2026-01-15", 148.0),
            ("2026-06-01", 144.0),
            ("2026-08-15", 142.0),
        ],
    },
    TestHistory {
        test_name: "TSH",
        unit: "mIU/L",
        ref_low: 0.4,
        ref_high: 4.0,
        critical_low: None,
        critical_high: Some(10.0),
        loinc: "LOINC 3019-0",
        points: &[
            ("2024-03-15", 5.8),
            ("2024-06-10", 3.2),
            ("2025-01-15", 2.5),
            ("2025-06-20", 2.0),
            ("2026-01-15", 2.3),
            ("2026-06-01", 2.1),
        ],
    },
    TestHistory {
        test_name: "25-OH Vitamin D",
        unit: "nmol/L",
        ref_low: 75.0,
        ref_high: 250.0,
        critical_low: Some(25.0),
        critical_high: None,
        loinc: "LOINC 62292-8",
        points: &[
            ("2024-06-10", 55.0),
            ("2025-01-15", 38.0),
            ("2025-06-20", 62.0),
            ("2025-11-20", 28.0),
            ("2026-03-10", 35.0),
            ("2026-08-15", 28.0),
        ],
    },
    TestHistory {
        test_name: "HbA1c",
        unit: "%",
        ref_low: 0.0,
        ref_high: 5.6,
        critical_low: None,
        critical_high: Some(6.5),
        loinc: "LOINC 4548-4",
        points: &[
            ("2024-06-10", 5.4),
            ("2025-01-15", 5.3),
            ("2025-06-20", 5.5),
            ("2026-01-15", 5.1),
            ("2026-03-10", 5.2),
        ],
    },
    TestHistory {
        test_name: "Total Cholesterol",
        unit: "mmol/L",
        ref_low: 0.0,
        ref_high: 5.2,
        critical_low: None,
        critical_high: Some(6.5),
        loinc: "LOINC 2093-3",
        points: &[
            ("2024-06-10", 5.5),
            ("2025-01-15", 5.3),
            ("2025-06-20", 5.1),
            ("2026-01-15", 5.0),
            ("2026-08-15", 4.9),
        ],
    },
    TestHistory {
        test_name: "ALT",
        unit: "U/L",
        ref_low: 10.0,
        ref_high: 45.0,
        critical_low: None,
        critical_high: Some(200.0),
        loinc: "LOINC 1742-6",
        points: &[
            ("2024-06-10", 28.0),
            ("2025-01-15", 26.0),
            ("2025-06-20", 30.0),
            ("2026-01-15", 22.0),
            ("2026-08-15", 24.0),
        ],
    },
    TestHistory {
        test_name: "Iron",
        unit: "\u{00B5}mol/L",
        ref_low: 10.0,
        ref_high: 30.0,
        critical_low: Some(5.0),
        critical_high: None,
        loinc: "LOINC 2502-3",
        points: &[
            ("2024-06-10", 18.0),
            ("2025-01-15", 14.0),
            ("2025-06-20", 12.0),
            ("2026-01-15", 10.0),
            ("2026-08-15", 8.0),
        ],
    },
];

/// SVG sparkline dimensions.
const SVG_W: f64 = 200.0;
const SVG_H: f64 = 50.0;
const PAD_L: f64 = 4.0;
const PAD_R: f64 = 4.0;
const PAD_T: f64 = 6.0;
const PAD_B: f64 = 6.0;

pub fn build_trends_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-lab-panel", "trends").unwrap();

    let info = document.create_element("div").unwrap();
    info.set_text_content(Some(
        "Time-series trends for tests with historical data. \
         Green band = reference range. Red band = critical threshold. \
         Orange points = outside reference range. Red points = critical.",
    ));
    let i_el: HtmlElement = info.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 6px; \
         background: var(--surface-panel); border-radius: 4px;",
    );
    panel.append_child(&info).unwrap();

    let table = make_table(
        document,
        &[
            "Test",
            "Trend",
            "Latest",
            "Unit",
            "Ref Range",
            "Change",
            "Code",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();

    for h in HISTORIES {
        let tr = document.create_element("tr").unwrap();

        // Test name
        let td_name = document.create_element("td").unwrap();
        td_name.set_text_content(Some(h.test_name));
        let dn_el: HtmlElement = td_name.clone().dyn_into().unwrap();
        dn_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-primary); font-size: 9px; font-weight: 600; \
             font-family: var(--font-mono); white-space: nowrap;",
        );
        tr.append_child(&td_name).unwrap();

        // Sparkline SVG
        let td_svg = document.create_element("td").unwrap();
        let svg = build_sparkline_svg(document, h);
        td_svg.append_child(&svg).unwrap();
        let ds_el: HtmlElement = td_svg.clone().dyn_into().unwrap();
        ds_el
            .style()
            .set_css_text("padding: 2px 6px; border-bottom: 1px solid var(--border-subtle);");
        tr.append_child(&td_svg).unwrap();

        // Latest value
        let (latest_date, latest_val) = h.points[h.points.len() - 1];
        let flag = classify_value(
            latest_val,
            h.ref_low,
            h.ref_high,
            h.critical_low,
            h.critical_high,
        );

        let td_latest = document.create_element("td").unwrap();
        td_latest.set_text_content(Some(&format!("{:.1}", latest_val)));
        let dl_el: HtmlElement = td_latest.clone().dyn_into().unwrap();
        let latest_color = match flag {
            Flag::Normal => "var(--text-primary)",
            Flag::Low | Flag::High => "rgba(255, 165, 0, 0.9)",
            Flag::CriticalLow | Flag::CriticalHigh => "rgba(255, 0, 0, 0.9)",
        };
        dl_el.style().set_css_text(&format!(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: {}; font-size: 10px; font-weight: 700; \
             font-family: var(--font-mono);",
            latest_color,
        ));
        tr.append_child(&td_latest).unwrap();

        // Unit
        let td_unit = document.create_element("td").unwrap();
        td_unit.set_text_content(Some(h.unit));
        let du_el: HtmlElement = td_unit.clone().dyn_into().unwrap();
        du_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
        );
        tr.append_child(&td_unit).unwrap();

        // Ref range
        let td_ref = document.create_element("td").unwrap();
        td_ref.set_text_content(Some(&format!("{}-{}", h.ref_low, h.ref_high)));
        let dr_el: HtmlElement = td_ref.clone().dyn_into().unwrap();
        dr_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
        );
        tr.append_child(&td_ref).unwrap();

        // Change (delta from previous)
        let td_change = document.create_element("td").unwrap();
        let (change_text, change_color) = if h.points.len() >= 2 {
            let prev = h.points[h.points.len() - 2].1;
            let delta = latest_val - prev;
            if delta.abs() < 0.01 {
                ("\u{2014}".to_string(), "var(--text-muted)")
            } else if delta > 0.0 {
                (
                    format!("+{:.1} \u{2191}", delta),
                    "rgba(100, 200, 255, 0.8)",
                )
            } else {
                (format!("{:.1} \u{2193}", delta), "rgba(255, 165, 0, 0.8)")
            }
        } else {
            ("\u{2014}".to_string(), "var(--text-muted)")
        };
        td_change.set_text_content(Some(&change_text));
        let dc_el: HtmlElement = td_change.clone().dyn_into().unwrap();
        dc_el.style().set_css_text(&format!(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: {}; font-size: 8px; font-family: var(--font-mono);",
            change_color,
        ));
        tr.append_child(&td_change).unwrap();

        // LOINC code
        let td_code = document.create_element("td").unwrap();
        td_code.set_text_content(Some(h.loinc));
        let dco_el: HtmlElement = td_code.clone().dyn_into().unwrap();
        dco_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-muted); font-size: 7px; font-family: var(--font-mono);",
        );
        tr.append_child(&td_code).unwrap();

        tbody.append_child(&tr).unwrap();
    }

    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    // Add detail sparkline cards for tests with out-of-range latest values
    let detail_header = document.create_element("div").unwrap();
    detail_header.set_text_content(Some("Out-of-Range Detail"));
    let dh_el: HtmlElement = detail_header.clone().dyn_into().unwrap();
    dh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px; \
         padding: 0 2px;",
    );
    panel.append_child(&detail_header).unwrap();

    for h in HISTORIES {
        let (_, latest_val) = h.points[h.points.len() - 1];
        let flag = classify_value(
            latest_val,
            h.ref_low,
            h.ref_high,
            h.critical_low,
            h.critical_high,
        );
        if flag == Flag::Normal {
            continue;
        }

        let card = build_detail_card(document, h, flag);
        panel.append_child(&card).unwrap();
    }

    panel
}

fn build_detail_card(document: &Document, h: &TestHistory, flag: Flag) -> Element {
    let card = document.create_element("div").unwrap();
    let c_el: HtmlElement = card.clone().dyn_into().unwrap();
    let border = match flag {
        Flag::CriticalLow | Flag::CriticalHigh => "rgba(255, 0, 0, 0.3)",
        _ => "rgba(255, 165, 0, 0.3)",
    };
    c_el.style().set_css_text(&format!(
        "border: 1px solid {}; border-radius: 6px; padding: 8px; \
         margin-bottom: 6px; background: var(--surface-panel);",
        border,
    ));

    let hdr = document.create_element("div").unwrap();
    let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "display: flex; justify-content: space-between; align-items: center; \
         margin-bottom: 4px;",
    );

    let name_div = document.create_element("div").unwrap();
    name_div.set_text_content(Some(&format!("{} ({})", h.test_name, h.loinc)));
    let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono);",
    );
    hdr.append_child(&name_div).unwrap();

    let (flag_text, flag_color) = match flag {
        Flag::Low => ("LOW \u{2193}".to_string(), "rgba(255, 165, 0, 0.8)"),
        Flag::High => ("HIGH \u{2191}".to_string(), "rgba(255, 165, 0, 0.8)"),
        Flag::CriticalLow => ("CRITICAL LOW \u{2193}".to_string(), "rgba(255, 0, 0, 0.9)"),
        Flag::CriticalHigh => ("CRITICAL HIGH \u{2191}".to_string(), "rgba(255, 0, 0, 0.9)"),
        _ => ("NORMAL".to_string(), "rgba(100, 200, 100, 0.8)"),
    };
    let flag_badge = document.create_element("span").unwrap();
    flag_badge.set_text_content(Some(&flag_text));
    let fb_el: HtmlElement = flag_badge.clone().dyn_into().unwrap();
    fb_el.style().set_css_text(&format!(
        "font-size: 8px; color: {}; font-family: var(--font-mono); \
         font-weight: 700; text-transform: uppercase;",
        flag_color,
    ));
    hdr.append_child(&flag_badge).unwrap();
    card.append_child(&hdr).unwrap();

    // Large sparkline
    let large_svg = build_large_sparkline_svg(document, h);
    card.append_child(&large_svg).unwrap();

    // Data points table
    let pts_info = document.create_element("div").unwrap();
    let pts_text: String = h
        .points
        .iter()
        .map(|(d, v)| format!("{}: {:.1}", d, v))
        .collect::<Vec<_>>()
        .join("  |  ");
    pts_info.set_text_content(Some(&pts_text));
    let pi_el: HtmlElement = pts_info.clone().dyn_into().unwrap();
    pi_el.style().set_css_text(
        "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
         margin-top: 4px; line-height: 1.4;",
    );
    card.append_child(&pts_info).unwrap();

    card
}

#[derive(Clone, Copy, PartialEq)]
enum Flag {
    Normal,
    Low,
    High,
    CriticalLow,
    CriticalHigh,
}

fn classify_value(
    val: f64,
    ref_low: f64,
    ref_high: f64,
    crit_low: Option<f64>,
    crit_high: Option<f64>,
) -> Flag {
    if let Some(cl) = crit_low {
        if val <= cl {
            return Flag::CriticalLow;
        }
    }
    if let Some(ch) = crit_high {
        if val >= ch {
            return Flag::CriticalHigh;
        }
    }
    if val < ref_low {
        Flag::Low
    } else if val > ref_high {
        Flag::High
    } else {
        Flag::Normal
    }
}

fn build_sparkline_svg(document: &Document, h: &TestHistory) -> Element {
    build_svg(document, h, SVG_W, SVG_H, false)
}

fn build_large_sparkline_svg(document: &Document, h: &TestHistory) -> Element {
    build_svg(document, h, 400.0, 100.0, true)
}

fn build_svg(document: &Document, h: &TestHistory, w: f64, ht: f64, show_labels: bool) -> Element {
    let svg_ns = "http://www.w3.org/2000/svg";
    let svg = document.create_element_ns(Some(svg_ns), "svg").unwrap();
    svg.set_attribute("width", &format!("{}", w as u32))
        .unwrap();
    svg.set_attribute("height", &format!("{}", ht as u32))
        .unwrap();
    svg.set_attribute("viewBox", &format!("0 0 {} {}", w, ht))
        .unwrap();

    let plot_w = w - PAD_L - PAD_R;
    let plot_h = ht - PAD_T - PAD_B;

    // Compute Y range: include all data points + ref range + critical bounds
    let mut y_min = h
        .ref_low
        .min(h.points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min));
    let mut y_max = h.ref_high.max(
        h.points
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max),
    );
    if let Some(cl) = h.critical_low {
        y_min = y_min.min(cl);
    }
    if let Some(ch) = h.critical_high {
        y_max = y_max.max(ch);
    }
    let y_range = y_max - y_min;
    if y_range < 0.01 {
        y_min -= 1.0;
        y_max += 1.0;
    } else {
        let pad = y_range * 0.1;
        y_min -= pad;
        y_max += pad;
    }
    let y_range_padded = y_max - y_min;

    let n = h.points.len() as f64;
    let x_step = if n > 1.0 { plot_w / (n - 1.0) } else { 0.0 };

    let scale_y = |val: f64| -> f64 { PAD_T + plot_h * (1.0 - (val - y_min) / y_range_padded) };
    let scale_x = |i: usize| -> f64 { PAD_L + i as f64 * x_step };

    // Reference range band (green shaded area)
    let ref_band = document.create_element_ns(Some(svg_ns), "rect").unwrap();
    ref_band.set_attribute("x", &format!("{}", PAD_L)).unwrap();
    ref_band
        .set_attribute("y", &format!("{}", scale_y(h.ref_high)))
        .unwrap();
    ref_band
        .set_attribute("width", &format!("{}", plot_w))
        .unwrap();
    ref_band
        .set_attribute(
            "height",
            &format!("{}", scale_y(h.ref_low) - scale_y(h.ref_high)),
        )
        .unwrap();
    ref_band
        .set_attribute("fill", "rgba(100, 200, 100, 0.12)")
        .unwrap();
    ref_band.set_attribute("stroke", "none").unwrap();
    svg.append_child(&ref_band).unwrap();

    // Reference range lines
    for (val, color, dash) in &[
        (h.ref_low, "rgba(100, 200, 100, 0.4)", "2,2"),
        (h.ref_high, "rgba(100, 200, 100, 0.4)", "2,2"),
    ] {
        let line = document.create_element_ns(Some(svg_ns), "line").unwrap();
        line.set_attribute("x1", &format!("{}", PAD_L)).unwrap();
        line.set_attribute("x2", &format!("{}", PAD_L + plot_w))
            .unwrap();
        line.set_attribute("y1", &format!("{}", scale_y(*val)))
            .unwrap();
        line.set_attribute("y2", &format!("{}", scale_y(*val)))
            .unwrap();
        line.set_attribute("stroke", color).unwrap();
        line.set_attribute("stroke-width", "0.5").unwrap();
        line.set_attribute("stroke-dasharray", dash).unwrap();
        svg.append_child(&line).unwrap();
    }

    // Critical threshold lines
    if let Some(cl) = h.critical_low {
        let line = document.create_element_ns(Some(svg_ns), "line").unwrap();
        line.set_attribute("x1", &format!("{}", PAD_L)).unwrap();
        line.set_attribute("x2", &format!("{}", PAD_L + plot_w))
            .unwrap();
        line.set_attribute("y1", &format!("{}", scale_y(cl)))
            .unwrap();
        line.set_attribute("y2", &format!("{}", scale_y(cl)))
            .unwrap();
        line.set_attribute("stroke", "rgba(255, 0, 0, 0.4)")
            .unwrap();
        line.set_attribute("stroke-width", "0.5").unwrap();
        line.set_attribute("stroke-dasharray", "3,1").unwrap();
        svg.append_child(&line).unwrap();
    }
    if let Some(ch) = h.critical_high {
        let line = document.create_element_ns(Some(svg_ns), "line").unwrap();
        line.set_attribute("x1", &format!("{}", PAD_L)).unwrap();
        line.set_attribute("x2", &format!("{}", PAD_L + plot_w))
            .unwrap();
        line.set_attribute("y1", &format!("{}", scale_y(ch)))
            .unwrap();
        line.set_attribute("y2", &format!("{}", scale_y(ch)))
            .unwrap();
        line.set_attribute("stroke", "rgba(255, 0, 0, 0.4)")
            .unwrap();
        line.set_attribute("stroke-width", "0.5").unwrap();
        line.set_attribute("stroke-dasharray", "3,1").unwrap();
        svg.append_child(&line).unwrap();
    }

    // Data line (polyline connecting all points)
    let points_str: String = h
        .points
        .iter()
        .enumerate()
        .map(|(i, (_, v))| format!("{:.1},{:.1}", scale_x(i), scale_y(*v)))
        .collect::<Vec<_>>()
        .join(" ");
    let polyline = document
        .create_element_ns(Some(svg_ns), "polyline")
        .unwrap();
    polyline.set_attribute("points", &points_str).unwrap();
    polyline.set_attribute("fill", "none").unwrap();
    polyline
        .set_attribute("stroke", "var(--accent-cyan)")
        .unwrap();
    polyline.set_attribute("stroke-width", "1.2").unwrap();
    polyline.set_attribute("stroke-linejoin", "round").unwrap();
    polyline.set_attribute("stroke-linecap", "round").unwrap();
    svg.append_child(&polyline).unwrap();

    // Data points (circles colored by flag)
    for (i, (_, val)) in h.points.iter().enumerate() {
        let flag = classify_value(*val, h.ref_low, h.ref_high, h.critical_low, h.critical_high);
        let (fill, r) = match flag {
            Flag::Normal => ("rgba(100, 200, 100, 0.7)", "1.5"),
            Flag::Low | Flag::High => ("rgba(255, 165, 0, 0.9)", "2.0"),
            Flag::CriticalLow | Flag::CriticalHigh => ("rgba(255, 0, 0, 0.9)", "2.5"),
        };
        let circle = document.create_element_ns(Some(svg_ns), "circle").unwrap();
        circle
            .set_attribute("cx", &format!("{:.1}", scale_x(i)))
            .unwrap();
        circle
            .set_attribute("cy", &format!("{:.1}", scale_y(*val)))
            .unwrap();
        circle.set_attribute("r", r).unwrap();
        circle.set_attribute("fill", fill).unwrap();
        svg.append_child(&circle).unwrap();
    }

    // Labels for large sparkline
    if show_labels {
        // Y-axis labels
        for (val, label) in &[
            (h.ref_high, format!("Ref high: {:.1}", h.ref_high)),
            (h.ref_low, format!("Ref low: {:.1}", h.ref_low)),
        ] {
            let text = document.create_element_ns(Some(svg_ns), "text").unwrap();
            text.set_attribute("x", &format!("{}", PAD_L + 2.0))
                .unwrap();
            text.set_attribute("y", &format!("{}", scale_y(*val) - 2.0))
                .unwrap();
            text.set_attribute("fill", "rgba(100, 200, 100, 0.6)")
                .unwrap();
            text.set_attribute("font-size", "7").unwrap();
            text.set_attribute("font-family", "monospace").unwrap();
            text.set_text_content(Some(label));
            svg.append_child(&text).unwrap();
        }

        // X-axis labels (first and last date)
        if let Some((first_date, _)) = h.points.first() {
            let text = document.create_element_ns(Some(svg_ns), "text").unwrap();
            text.set_attribute("x", &format!("{}", PAD_L)).unwrap();
            text.set_attribute("y", &format!("{}", ht - 1.0)).unwrap();
            text.set_attribute("fill", "var(--text-muted)").unwrap();
            text.set_attribute("font-size", "7").unwrap();
            text.set_attribute("font-family", "monospace").unwrap();
            text.set_text_content(Some(first_date));
            svg.append_child(&text).unwrap();
        }
        if let Some((last_date, _)) = h.points.last() {
            let text = document.create_element_ns(Some(svg_ns), "text").unwrap();
            text.set_attribute("x", &format!("{}", PAD_L + plot_w - 50.0))
                .unwrap();
            text.set_attribute("y", &format!("{}", ht - 1.0)).unwrap();
            text.set_attribute("fill", "var(--text-muted)").unwrap();
            text.set_attribute("font-size", "7").unwrap();
            text.set_attribute("font-family", "monospace").unwrap();
            text.set_text_content(Some(last_date));
            svg.append_child(&text).unwrap();
        }
    }

    svg
}

fn make_table(document: &Document, headers: &[&str]) -> Element {
    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 9px;");
    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in headers {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 4px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
