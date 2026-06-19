use dioxus::prelude::*;

#[component]
pub fn AgreementsRights() -> Element {
    rsx! {
        div {
            style: "flex: 1; padding: 2.5rem; background: linear-gradient(180deg, rgba(30,20,40,0.9) 0%, rgba(20,20,30,0.9) 100%); backdrop-filter: blur(20px); border-radius: 16px; border: 1px solid rgba(255,0,128,0.2); color: #FFF;",

            h2 {
                style: "font-family: 'Inter', sans-serif; font-size: 2.2rem; background: linear-gradient(90deg, #FF0080, #7928CA); -webkit-background-clip: text; -webkit-text-fill-color: transparent; margin-bottom: 0.5rem;",
                "Agreements & Rights"
            }
            p { style: "color: #A0A0B0; margin-bottom: 2rem;", "Manage Bilateral Micro-Commons and Guardianship Contracts." }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 2rem;",

                div {
                    style: "background: rgba(0,0,0,0.4); padding: 1.5rem; border-radius: 12px; border: 1px solid rgba(255,255,255,0.05);",
                    h3 { style: "color: #FF0080; margin-top: 0;", "Active Agreements" }
                    div {
                        style: "padding: 1rem; background: rgba(255,0,128,0.1); border-left: 3px solid #FF0080; border-radius: 0 8px 8px 0; margin-bottom: 1rem;",
                        div { style: "font-weight: bold; margin-bottom: 0.2rem;", "Data Sharing Contract" }
                        div { style: "font-size: 0.85rem; color: #CCC;", "Counterparty: did:q42:hospital_xyz" }
                        div { style: "font-size: 0.85rem; color: #00FF88; margin-top: 0.5rem;", "Status: Active • M-of-N Consented" }
                    }
                    button {
                        style: "width: 100%; padding: 0.8rem; background: transparent; border: 1px dashed rgba(255,255,255,0.3); border-radius: 8px; color: #FFF; cursor: pointer;",
                        "+ Draft New Agreement"
                    }
                }

                div {
                    style: "background: rgba(0,0,0,0.4); padding: 1.5rem; border-radius: 12px; border: 1px solid rgba(255,255,255,0.05);",
                    h3 { style: "color: #7928CA; margin-top: 0;", "Delegated Access (Suspended)" }
                    div {
                        style: "padding: 1rem; background: rgba(121,40,202,0.1); border-left: 3px solid #7928CA; border-radius: 0 8px 8px 0;",
                        div { style: "font-weight: bold; margin-bottom: 0.2rem;", "Guardianship Approval" }
                        div { style: "font-size: 0.85rem; color: #CCC;", "Waiting for 2-of-3 signatures" }
                        div {
                            style: "display: flex; gap: 0.5rem; margin-top: 1rem;",
                            button { style: "flex: 1; padding: 0.5rem; background: #00FF88; border: none; border-radius: 4px; color: #000; font-weight: bold; cursor: pointer;", "Sign" }
                            button { style: "flex: 1; padding: 0.5rem; background: #FF3366; border: none; border-radius: 4px; color: #FFF; font-weight: bold; cursor: pointer;", "Reject" }
                        }
                    }
                }
            }
        }
    }
}
