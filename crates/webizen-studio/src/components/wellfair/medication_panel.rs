//! Medication & Nutrition — catalogue, administrations, and diet (Phase 2 Q6).

use super::host_client::{
    add_diet_entry, add_medication, fetch_due_med_reminders, fetch_health_records,
    fetch_med_reminder_prefs, grant_med_reminder_permission, record_administration,
    set_med_reminders_enabled, DueMedReminderDto, MedReminderPrefsDto,
};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

#[component]
pub fn WellfairMedicationPanel() -> Element {
    let mut med_name = use_signal(String::new);
    let mut med_dose = use_signal(|| "1 tablet".to_string());
    let mut med_route = use_signal(|| "oral".to_string());
    let mut med_schedule = use_signal(|| "08:00".to_string());
    let mut diet_desc = use_signal(String::new);
    let mut diet_meal = use_signal(|| "breakfast".to_string());
    let mut diet_cal = use_signal(String::new);
    let mut rows = use_signal(Vec::<HealthRecordDto>::new);
    let mut status = use_signal(|| "Load medication and diet records from your vault.".to_string());
    let mut reminder_prefs = use_signal(MedReminderPrefsDto::default);
    let mut due_reminders = use_signal(Vec::<DueMedReminderDto>::new);

    let reload = move || {
        spawn(async move {
            match fetch_health_records(128).await {
                Ok(list) => {
                    let med: Vec<_> = list
                        .into_iter()
                        .filter(|r| {
                            matches!(
                                r.kind.as_str(),
                                "medication" | "med_administration" | "diet"
                            )
                        })
                        .collect();
                    let n = med.len();
                    status.set(format!(
                        "{n} medication/nutrition record(s). Self-reported — not a prescribing system."
                    ));
                    rows.set(med);
                }
                Err(e) => status.set(format!("Could not load records: {e}")),
            }
        });
    };

    let reload_reminders = move || {
        spawn(async move {
            if let Ok(p) = fetch_med_reminder_prefs().await {
                reminder_prefs.set(p);
            }
            if let Ok(due) = fetch_due_med_reminders(45).await {
                due_reminders.set(due);
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        reload();
        reload_reminders();
    });

    let on_add_med = move |_| {
        let name = med_name.read().clone();
        if name.trim().is_empty() {
            status.set("Enter a medication name.".into());
            return;
        }
        let dose = med_dose.read().clone();
        let route = med_route.read().clone();
        let sched = med_schedule.read().clone();
        spawn(async move {
            match add_medication(&name, &dose, &route, &sched).await {
                Ok(_) => {
                    status.set(format!("Added medication “{name}”."));
                    reload();
                }
                Err(e) => status.set(format!("Add failed: {e}")),
            }
        });
    };

    let on_log_diet = move |_| {
        let desc = diet_desc.read().clone();
        if desc.trim().is_empty() {
            status.set("Enter a food description.".into());
            return;
        }
        let meal = diet_meal.read().clone();
        let cal = diet_cal.read().clone();
        let cal_opt = cal.trim().parse::<u32>().ok();
        spawn(async move {
            match add_diet_entry(&desc, &meal, cal_opt).await {
                Ok(_) => {
                    status.set("Diet entry saved.".into());
                    reload();
                }
                Err(e) => status.set(format!("Diet save failed: {e}")),
            }
        });
    };

    rsx! {
        section {
            aria_label: "WellFair medication and nutrition",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.75rem;",
            super::shared::DomainChrome { domain: "Care", chip: "Body · meds & nutrition", show_memory: true }
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Medication & nutrition" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                "{status()}"
            }

            div {
                style: "margin-bottom:1rem;padding:0.65rem;border:1px solid var(--qualia-border,#eee);border-radius:8px;",
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Local reminders" }
                p {
                    style: "margin:0 0 0.5rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                    "Schedule-based reminders stay on-device. Grant permission once, then enable."
                }
                if !reminder_prefs.read().permission_granted {
                    button {
                        style: "padding:0.35rem 0.65rem;border-radius:6px;border:none;background:#e76f51;color:#fff;font-size:0.78rem;cursor:pointer;margin-right:0.5rem;",
                        onclick: move |_| {
                            spawn(async move {
                                match grant_med_reminder_permission().await {
                                    Ok(p) => {
                                        reminder_prefs.set(p);
                                        status.set("Reminder permission granted.".into());
                                    }
                                    Err(e) => status.set(format!("Permission failed: {e}")),
                                }
                            });
                        },
                        "Grant reminder permission"
                    }
                } else {
                    label {
                        style: "display:flex;align-items:center;gap:0.4rem;font-size:0.78rem;",
                        input {
                            r#type: "checkbox",
                            checked: reminder_prefs.read().enabled,
                            onchange: move |e| {
                                let on = e.checked();
                                spawn(async move {
                                    match set_med_reminders_enabled(on).await {
                                        Ok(p) => reminder_prefs.set(p),
                                        Err(err) => status.set(format!("{err}")),
                                    }
                                });
                            },
                        }
                        "Enable medication reminders"
                    }
                }
                if !due_reminders.read().is_empty() {
                    ul {
                        style: "margin:0.5rem 0 0;padding:0;list-style:none;font-size:0.76rem;",
                        for r in due_reminders.read().clone() {
                            li {
                                key: "{r.medication_id}-{r.schedule_slot}",
                                style: "padding:0.25rem 0;",
                                strong { "{r.medication_name}" }
                                span { " — due {r.schedule_slot}" }
                                if r.minutes_until_due < 0 {
                                    span { style: "color:#c1121f;margin-left:0.35rem;", "(overdue)" }
                                }
                            }
                        }
                    }
                }
            }

            div {
                style: "display:grid;gap:0.65rem;margin-bottom:1rem;padding:0.65rem;border:1px solid var(--qualia-border,#eee);border-radius:8px;",
                h3 { style: "margin:0;font-size:0.88rem;", "Add medication" }
                label { style: "font-size:0.78rem;", "Name" }
                input {
                    r#type: "text",
                    value: "{med_name}",
                    oninput: move |e| med_name.set(e.value()),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                }
                label { style: "font-size:0.78rem;", "Dose" }
                input {
                    r#type: "text",
                    value: "{med_dose}",
                    oninput: move |e| med_dose.set(e.value()),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                }
                label { style: "font-size:0.78rem;", "Route" }
                input {
                    r#type: "text",
                    value: "{med_route}",
                    oninput: move |e| med_route.set(e.value()),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                }
                label { style: "font-size:0.78rem;", "Schedule (HH:MM, comma-separated)" }
                input {
                    r#type: "text",
                    value: "{med_schedule}",
                    oninput: move |e| med_schedule.set(e.value()),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:6px;border:none;background:#457b9d;color:#fff;cursor:pointer;font-size:0.8rem;",
                    onclick: on_add_med,
                    "Save medication"
                }
            }

            div {
                style: "display:grid;gap:0.65rem;margin-bottom:1rem;padding:0.65rem;border:1px solid var(--qualia-border,#eee);border-radius:8px;",
                h3 { style: "margin:0;font-size:0.88rem;", "Log diet" }
                label { style: "font-size:0.78rem;", "Description" }
                input {
                    r#type: "text",
                    value: "{diet_desc}",
                    oninput: move |e| diet_desc.set(e.value()),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                }
                label { style: "font-size:0.78rem;", "Meal" }
                select {
                    value: "{diet_meal}",
                    onchange: move |e| diet_meal.set(e.value()),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    option { value: "breakfast", "Breakfast" }
                    option { value: "lunch", "Lunch" }
                    option { value: "dinner", "Dinner" }
                    option { value: "snack", "Snack" }
                }
                label { style: "font-size:0.78rem;", "Calories (optional)" }
                input {
                    r#type: "number",
                    value: "{diet_cal}",
                    oninput: move |e| diet_cal.set(e.value()),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:6px;border:none;background:#2a9d8f;color:#fff;cursor:pointer;font-size:0.8rem;",
                    onclick: on_log_diet,
                    "Save diet entry"
                }
            }

            if !rows.read().is_empty() {
                div {
                    style: "overflow-x:auto;",
                    table {
                        style: "width:100%;border-collapse:collapse;font-size:0.76rem;",
                        thead {
                            tr {
                                style: "text-align:left;border-bottom:1px solid var(--qualia-border,#ddd);",
                                th { style: "padding:0.35rem;", "Kind" }
                                th { style: "padding:0.35rem;", "When" }
                                th { style: "padding:0.35rem;", "Summary" }
                                th { style: "padding:0.35rem;", "Action" }
                            }
                        }
                        tbody {
                            for row in rows.read().clone() {
                                tr {
                                    key: "{row.id}",
                                    style: "border-bottom:1px solid var(--qualia-border,#eee);",
                                    td { style: "padding:0.35rem;", "{row.kind}" }
                                    td { style: "padding:0.35rem;", "{row.asserted_time_unix}" }
                                    td {
                                        style: "padding:0.35rem;max-width:200px;overflow:hidden;text-overflow:ellipsis;",
                                        "{row.summary.clone().unwrap_or_default()}"
                                    }
                                    td {
                                        style: "padding:0.35rem;",
                                        if row.kind == "medication" {
                                            button {
                                                style: "font-size:0.72rem;padding:0.2rem 0.45rem;border-radius:4px;border:1px solid #2a9d8f;background:transparent;cursor:pointer;",
                                                onclick: {
                                                    let id = row.id.clone();
                                                    let summary = row.summary.clone();
                                                    move |_| {
                                                        let med_name = summary
                                                            .as_ref()
                                                            .and_then(|s| {
                                                                serde_json::from_str::<serde_json::Value>(s).ok()
                                                            })
                                                            .and_then(|v| {
                                                                v.get("name")
                                                                    .and_then(|n| n.as_str())
                                                                    .map(|s| s.to_string())
                                                            })
                                                            .unwrap_or_else(|| "medication".into());
                                                        let mid = id.clone();
                                                        spawn(async move {
                                                            match record_administration(&mid, &med_name, "taken", None).await {
                                                                Ok(_) => status.set("Marked taken.".into()),
                                                                Err(e) => status.set(format!("{e}")),
                                                            }
                                                        });
                                                    }
                                                },
                                                "Taken"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            button {
                style: "margin-top:0.5rem;padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                onclick: move |_| reload(),
                "Refresh"
            }
        }
    }
}