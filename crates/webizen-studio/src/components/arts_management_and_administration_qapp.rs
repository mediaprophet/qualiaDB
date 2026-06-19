use dioxus::prelude::*;

#[component]
pub fn ArtsManagementAndAdministrationQapp() -> Element {
    let mut organisation_type = use_signal(|| "Museum".to_string());
    let mut management_domain = use_signal(|| "Programming".to_string());
    let mut funding_model = use_signal(|| "Mixed".to_string());
    let mut attendance_target = use_signal(|| 50000u32);
    let mut revenue_budget_usd = use_signal(|| 500000u32);
    let mut earned_revenue_pct = use_signal(|| 40u32);
    let mut notes = use_signal(|| String::new());

    let org_types = [
        "Museum",
        "Gallery",
        "Performing Arts Venue",
        "Festival",
        "Artist Residency",
        "Cultural NGO",
        "Public Arts Agency",
        "Creative Industry",
    ];
    let domains = [
        "Programming",
        "Finance",
        "Marketing",
        "Development",
        "HR",
        "Digital Strategy",
        "Community Engagement",
        "Grant Writing",
    ];
    let funding_models = [
        "Public",
        "Private",
        "Mixed",
        "Crowdfunded",
        "Endowment",
        "Earned Revenue",
        "Hybrid",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Arts Management & Administration" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Organisation Type" }
                    select {
                        value: "{organisation_type}",
                        onchange: move |e| organisation_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in org_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Management Domain" }
                    select {
                        value: "{management_domain}",
                        onchange: move |e| management_domain.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in domains { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Funding Model" }
                    select {
                        value: "{funding_model}",
                        onchange: move |e| funding_model.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in funding_models { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Attendance Target" }
                    input {
                        r#type: "number",
                        value: "{attendance_target}",
                        oninput: move |e| attendance_target.set(e.value().parse().unwrap_or(50000)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Revenue Budget (USD)" }
                    input {
                        r#type: "number",
                        value: "{revenue_budget_usd}",
                        oninput: move |e| revenue_budget_usd.set(e.value().parse().unwrap_or(500000)),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Earned Revenue % (0–100): {earned_revenue_pct}%" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    value: "{earned_revenue_pct}",
                    oninput: move |e| earned_revenue_pct.set(e.value().parse().unwrap_or(40)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{organisation_type} | {management_domain} | {funding_model} | Earned: {earned_revenue_pct}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → cultural sector graph | funding model sieve | audience analytics engine" }
            }
        }
    }
}
