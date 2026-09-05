//! Save-mode dialog construction and persistence dispatch.

use super::*;

/// Open the Save Mode dialog — lets the user choose a save mode,
/// provide a label, and see the actor identity that will be recorded.
///
/// See `SAVE_ARCHITECTURE.md` for the full specification.
pub(super) fn open_save_mode_dialog(document: &Document) {
    let return_focus = document.active_element();
    // Remove any existing dialog
    if let Some(existing) = document.get_element_by_id("save-mode-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("save-mode-dialog");
    overlay.set_attribute("data-beat", "entrance").ok();
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; \
         display: flex; align-items: center; justify-content: center;",
    );

    let panel = document.create_element("div").unwrap();
    panel.set_attribute("role", "dialog").unwrap();
    panel.set_attribute("aria-modal", "true").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 420px; background: var(--surface-glass-heavy); \
         backdrop-filter: blur(20px); border: 1px solid var(--border-medium); \
         border-radius: var(--radius-sm); box-shadow: var(--shadow-lg); \
         padding: 20px; display: flex; flex-direction: column; gap: 16px; \
         font-family: var(--font-mono); color: var(--text-primary);",
    );

    // Title
    let title = document.create_element("div").unwrap();
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-size: 14px; font-weight: 700; color: var(--text-primary);");
    title.set_text_content(Some("\u{1F4BE} Save \u{2014} Checkpoint Mode"));
    panel.append_child(&title).unwrap();

    // Actor info
    let actor_info = document.create_element("div").unwrap();
    let actor_el: HtmlElement = actor_info.clone().dyn_into().unwrap();
    actor_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    actor_info.set_text_content(Some(&format!(
        "Actor: {} — saves are attributed to the bound observer.",
        super::super::current_observer_did()
    )));
    panel.append_child(&actor_info).unwrap();

    // Mode selector
    let mode_label = document.create_element("div").unwrap();
    let ml_el: HtmlElement = mode_label.clone().dyn_into().unwrap();
    ml_el
        .style()
        .set_css_text("font-size: 11px; color: var(--text-secondary);");
    mode_label.set_text_content(Some("Save mode:"));
    panel.append_child(&mode_label).unwrap();

    let mode_group = document.create_element("div").unwrap();
    let mg_el: HtmlElement = mode_group.clone().dyn_into().unwrap();
    mg_el.style().set_css_text("display: flex; gap: 6px;");

    let modes = [
        ("Auto", "auto", "Frequency-based\nrolling buffer"),
        ("Checkpoint", "checkpoint", "Named save\nwith label"),
        ("Snapshot", "snapshot", "Immutable seed set\n(exportable)"),
        ("Pruned", "pruned", "Tombstones pruned\n(distribution)"),
    ];

    for (idx, (label, mode_id, desc)) in modes.iter().enumerate() {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("save-mode-btn");
        btn.set_attribute("data-save-mode", mode_id).unwrap();
        if *mode_id == "pruned" {
            btn.set_attribute("disabled", "").ok();
            btn.set_attribute("aria-disabled", "true").ok();
            btn.set_attribute(
                "title",
                "Unavailable: the checkpoint store does not yet retain an operation/tombstone DAG to prune.",
            )
            .ok();
        }
        let btn_el: HtmlElement = btn.clone().dyn_into().unwrap();
        btn_el.style().set_css_text(if idx == 1 {
            "flex: 1; padding: 10px 8px; border: 1px solid var(--accent-cyan); \
             border-radius: var(--radius-xs); background: var(--surface-panel-elevated); \
             color: var(--text-primary); font-family: var(--font-mono); font-size: 10px; \
             cursor: pointer; display: flex; flex-direction: column; gap: 4px; \
             align-items: center; text-align: center;"
        } else {
            "flex: 1; padding: 10px 8px; border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); background: var(--surface-panel); \
             color: var(--text-secondary); font-family: var(--font-mono); font-size: 10px; \
             cursor: pointer; display: flex; flex-direction: column; gap: 4px; \
             align-items: center; text-align: center; transition: var(--trans-fast);"
        });
        if idx == 1 {
            btn.class_list().add_1("selected").unwrap();
        }

        let name_el = document.create_element("div").unwrap();
        name_el.set_text_content(Some(label));
        name_el
            .set_attribute("style", "font-weight: 700; font-size: 11px;")
            .unwrap();
        btn.append_child(&name_el).unwrap();

        let desc_el = document.create_element("div").unwrap();
        desc_el.set_text_content(Some(desc));
        desc_el
            .set_attribute(
                "style",
                "font-size: 9px; color: var(--text-muted); white-space: pre-line;",
            )
            .unwrap();
        btn.append_child(&desc_el).unwrap();

        mode_group.append_child(&btn).unwrap();
    }
    panel.append_child(&mode_group).unwrap();

    // Label input
    let label_div = document.create_element("div").unwrap();
    label_div
        .set_attribute("style", "display: flex; flex-direction: column; gap: 4px;")
        .unwrap();

    let label_text = document.create_element("div").unwrap();
    let lt_el: HtmlElement = label_text.clone().dyn_into().unwrap();
    lt_el
        .style()
        .set_css_text("font-size: 11px; color: var(--text-secondary);");
    label_text.set_text_content(Some("Checkpoint label:"));
    label_div.append_child(&label_text).unwrap();

    let label_input = document.create_element("input").unwrap();
    let li_el: HtmlInputElement = label_input.clone().dyn_into().unwrap();
    li_el.set_placeholder("e.g. v0.3 draft, before NLP extraction\u{2026}");
    label_input.set_id("save-mode-label-input");
    label_input.set_attribute("style",
        "padding: 8px 10px; background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
         border-radius: var(--radius-xs); color: var(--text-primary); font-family: var(--font-mono); \
         font-size: 12px; outline: none;"
    ).unwrap();
    label_div.append_child(&label_input).unwrap();
    panel.append_child(&label_div).unwrap();

    // Honesty note
    let honesty = document.create_element("div").unwrap();
    let h_el: HtmlElement = honesty.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan);",
    );
    honesty.set_text_content(Some(
        "Auto / Checkpoint / Snapshot write the UI seed to browser storage. They are not a .q42 volume. Durable sanctuary save uses GraphDatabase.volume_commit when a daemon is connected and a path is set. Pruned stays disabled until an operation DAG exists. wasm without a daemon never pretends a volume was saved.",
    ));
    panel.append_child(&honesty).unwrap();

    let vol_state = document.create_element("div").unwrap();
    vol_state.set_id("save-volume-state");
    vol_state.set_class_name("volume-state-chip");
    let initial_state = if super::super::native_daemon::is_daemon_connected() {
        "closed"
    } else {
        "denied"
    };
    vol_state
        .set_attribute("data-volume-state", initial_state)
        .ok();
    vol_state.set_attribute("data-beat", "entrance").ok();
    vol_state.set_text_content(Some(if initial_state == "denied" {
        "denied · no daemon"
    } else {
        "closed"
    }));
    panel.append_child(&vol_state).unwrap();

    let vol_div = document.create_element("div").unwrap();
    vol_div
        .set_attribute("style", "display: flex; flex-direction: column; gap: 4px;")
        .unwrap();
    let vol_label = document.create_element("div").unwrap();
    vol_label.set_text_content(Some("Sanctuary .q42 path (optional, daemon):"));
    vol_label
        .set_attribute("style", "font-size: 11px; color: var(--text-secondary);")
        .unwrap();
    vol_div.append_child(&vol_label).unwrap();
    let vol_input = document.create_element("input").unwrap();
    vol_input.set_id("save-volume-path");
    vol_input
        .set_attribute(
            "style",
            "padding: 8px 10px; background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); color: var(--text-primary); font-family: var(--font-mono); \
             font-size: 12px; outline: none;",
        )
        .unwrap();
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(path)) = storage.get_item("qualia-ui:sanctuary-volume-path") {
                let input: HtmlInputElement = vol_input.clone().dyn_into().unwrap();
                input.set_value(&path);
            }
        }
    }
    vol_div.append_child(&vol_input).unwrap();
    panel.append_child(&vol_div).unwrap();

    // Buttons
    let btn_row = document.create_element("div").unwrap();
    let br_el: HtmlElement = btn_row.clone().dyn_into().unwrap();
    br_el
        .style()
        .set_css_text("display: flex; gap: 8px; justify-content: flex-end;");

    let cancel_btn = document.create_element("button").unwrap();
    cancel_btn.set_text_content(Some("Cancel"));
    let cb_el: HtmlElement = cancel_btn.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "padding: 8px 16px; border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         background: var(--surface-panel); color: var(--text-secondary); font-family: var(--font-mono); \
         font-size: 11px; cursor: pointer;"
    );
    btn_row.append_child(&cancel_btn).unwrap();

    let save_btn = document.create_element("button").unwrap();
    save_btn.set_id("save-mode-confirm-btn");
    save_btn.set_text_content(Some("\u{1F4BE} Save"));
    let sb_el: HtmlElement = save_btn.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "padding: 8px 16px; border: 1px solid var(--accent-cyan); border-radius: var(--radius-xs); \
         background: var(--accent-cyan); color: var(--bg-deep); font-family: var(--font-mono); \
         font-size: 11px; font-weight: 700; cursor: pointer;",
    );
    btn_row.append_child(&save_btn).unwrap();

    let open_vol_btn = document.create_element("button").unwrap();
    open_vol_btn.set_id("save-volume-open-btn");
    open_vol_btn.set_text_content(Some("Open .q42"));
    let ov_el: HtmlElement = open_vol_btn.clone().dyn_into().unwrap();
    ov_el.style().set_css_text(
        "padding: 8px 16px; border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); \
         background: var(--surface-panel); color: var(--text-secondary); font-family: var(--font-mono); \
         font-size: 11px; cursor: pointer;"
    );
    btn_row.append_child(&open_vol_btn).unwrap();
    panel.append_child(&btn_row).unwrap();

    overlay.append_child(&panel).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }

    // Wire mode button selection
    let mode_btns = document.query_selector_all(".save-mode-btn").unwrap();
    for i in 0..mode_btns.length() {
        let btn = mode_btns.get(i).unwrap();
        let btn_el: Element = btn.dyn_into().unwrap();
        let btn_el_for_listener = btn_el.clone();

        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            // Deselect all
            let all = doc.query_selector_all(".save-mode-btn").unwrap();
            for j in 0..all.length() {
                let b = all.get(j).unwrap();
                let be: Element = b.dyn_into().unwrap();
                let be_html: HtmlElement = be.clone().dyn_into().unwrap();
                be.class_list().remove_1("selected").unwrap();
                be_html
                    .style()
                    .set_property("border", "1px solid var(--border-subtle)")
                    .unwrap();
                be_html
                    .style()
                    .set_property("background", "var(--surface-panel)")
                    .unwrap();
                be_html
                    .style()
                    .set_property("color", "var(--text-secondary)")
                    .unwrap();
            }
            // Select this
            btn_el.class_list().add_1("selected").unwrap();
            let btn_html: HtmlElement = btn_el.clone().dyn_into().unwrap();
            btn_html
                .style()
                .set_property("border", "1px solid var(--accent-cyan)")
                .unwrap();
            btn_html
                .style()
                .set_property("background", "var(--surface-panel-elevated)")
                .unwrap();
            btn_html
                .style()
                .set_property("color", "var(--text-primary)")
                .unwrap();
        }) as Box<dyn FnMut(web_sys::Event)>);

        btn_el_for_listener
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire cancel button
    let overlay_for_cancel = overlay.clone();
    let cancel_closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        overlay_for_cancel.remove();
    }) as Box<dyn FnMut(web_sys::Event)>);
    cancel_btn
        .add_event_listener_with_callback("click", cancel_closure.as_ref().unchecked_ref())
        .unwrap();
    cancel_closure.forget();

    // Wire save button
    let overlay_for_save = overlay.clone();
    let save_closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let doc = web_sys::window().unwrap().document().unwrap();

        // Get selected mode
        let selected = doc.query_selector(".save-mode-btn.selected").unwrap();
        let mode_id = selected
            .as_ref()
            .and_then(|el| el.get_attribute("data-save-mode"))
            .unwrap_or_else(|| "checkpoint".to_string());

        // Get label
        let label = doc
            .get_element_by_id("save-mode-label-input")
            .map(|el| {
                let input: HtmlInputElement = el.dyn_into().unwrap();
                input.value()
            })
            .unwrap_or_default();

        // Map mode ID to SaveMode
        let mode = match mode_id.as_str() {
            "auto" => super::super::manifest::SaveMode::Auto,
            "checkpoint" => super::super::manifest::SaveMode::Checkpoint,
            "snapshot" => super::super::manifest::SaveMode::Snapshot,
            "pruned" => super::super::manifest::SaveMode::Pruned,
            _ => super::super::manifest::SaveMode::Checkpoint,
        };

        let volume_path = doc
            .get_element_by_id("save-volume-path")
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|input| input.value())
            .unwrap_or_default();
        let volume_path = volume_path.trim().to_string();
        if !volume_path.is_empty() {
            if let Some(window) = web_sys::window() {
                if let Ok(Some(storage)) = window.local_storage() {
                    let _ = storage.set_item("qualia-ui:sanctuary-volume-path", &volume_path);
                }
            }
        }

        // Local UI seed persistence — not a .q42 volume.
        let result = super::super::manifest::save_checkpoint(&label, mode.clone());

        // Close dialog
        overlay_for_save.remove();

        // Show result notification
        match result {
            Ok(meta) => {
                let mode_name = match mode {
                    super::super::manifest::SaveMode::Auto => "Auto",
                    super::super::manifest::SaveMode::Checkpoint => "Checkpoint",
                    super::super::manifest::SaveMode::Snapshot => "Snapshot",
                    super::super::manifest::SaveMode::Pruned => "Pruned",
                };
                let label_part = if meta.label.is_empty() {
                    String::new()
                } else {
                    format!(" \u{2014} \"{}\"", meta.label)
                };
                show_menu_notification(
                    &doc,
                    &format!(
                        "{} saved{} \u{2014} actor: {}, ts: {}",
                        mode_name, label_part, meta.actor, meta.timestamp
                    ),
                );
            }
            Err(e) => {
                show_menu_notification(&doc, &format!("Save failed: {}", e));
            }
        }

        if !volume_path.is_empty() {
            if super::super::native_daemon::is_daemon_connected() {
                set_volume_state(&doc, "open", "open");
                let notify_doc = doc.clone();
                let path = volume_path.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let args = serde_json::json!({ "path": path, "sanctuary": true });
                    match super::super::native_daemon::daemon_invoke(
                        "GraphDatabase.volume_commit",
                        args,
                    )
                    .await
                    {
                        Ok(_) => {
                            set_volume_state(&notify_doc, "committed", "committed");
                            show_menu_notification(
                                &notify_doc,
                                "Sanctuary volume committed via GraphDatabase.volume_commit.",
                            );
                        }
                        Err(err) => {
                            set_volume_state(&notify_doc, "fault", "fault");
                            show_menu_notification(
                                &notify_doc,
                                &format!(
                                    "Volume commit failed ({err}). Browser checkpoint is local only — not a durable .q42."
                                ),
                            );
                        }
                    }
                });
            } else {
                set_volume_state(&doc, "denied", "denied · no daemon");
                show_menu_notification(
                    &doc,
                    "Unavailable: start the local QualiaDB daemon to run GraphDatabase.volume_commit. Browser checkpoint is local only.",
                );
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    save_btn
        .add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref())
        .unwrap();
    save_closure.forget();

    let open_vol_closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        let volume_path = doc
            .get_element_by_id("save-volume-path")
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|input| input.value())
            .unwrap_or_default();
        let volume_path = volume_path.trim().to_string();
        if volume_path.is_empty() {
            show_menu_notification(&doc, "Set a sanctuary .q42 path before opening.");
            return;
        }
        if !super::super::native_daemon::is_daemon_connected() {
            set_volume_state(&doc, "denied", "denied · no daemon");
            show_menu_notification(
                &doc,
                "Unavailable: start the local QualiaDB daemon to run GraphDatabase.volume_open.",
            );
            return;
        }
        set_volume_state(&doc, "open", "open");
        let notify_doc = doc.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let args = serde_json::json!({ "path": volume_path, "load": true });
            match super::super::native_daemon::daemon_invoke("GraphDatabase.volume_open", args)
                .await
            {
                Ok(_) => {
                    set_volume_state(&notify_doc, "open", "open");
                    show_menu_notification(
                        &notify_doc,
                        "Sanctuary volume opened via GraphDatabase.volume_open.",
                    );
                }
                Err(err) => {
                    set_volume_state(&notify_doc, "fault", "fault");
                    show_menu_notification(
                        &notify_doc,
                        &format!("Volume open failed ({err}). No fake graph was loaded."),
                    );
                }
            }
        });
    }) as Box<dyn FnMut(web_sys::Event)>);
    open_vol_btn
        .add_event_listener_with_callback("click", open_vol_closure.as_ref().unchecked_ref())
        .unwrap();
    open_vol_closure.forget();

    let initial_focus = document.get_element_by_id("save-mode-label-input");
    super::super::accessibility::wire_modal_accessibility(
        document,
        &overlay,
        &panel,
        return_focus,
        initial_focus,
    );
}

fn set_volume_state(document: &Document, state: &str, label: &str) {
    if let Some(chip) = document.get_element_by_id("save-volume-state") {
        chip.set_attribute("data-volume-state", state).ok();
        chip.set_attribute(
            "data-beat",
            if state == "committed" {
                "exit"
            } else if state == "open" {
                "dwell"
            } else {
                "entrance"
            },
        )
        .ok();
        chip.set_text_content(Some(label));
    }
    if let Some(chip) = document.get_element_by_id("statusbar-volume-state") {
        chip.set_attribute("data-volume-state", state).ok();
        chip.set_text_content(Some(label));
    }
    if let Some(bar) = document.query_selector(".bottom-statusbar").ok().flatten() {
        bar.set_attribute("data-volume-state", state).ok();
    }
}
