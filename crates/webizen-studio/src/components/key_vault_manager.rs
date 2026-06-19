use dioxus::prelude::*;

#[component]
pub fn KeyVaultManager() -> Element {
    rsx! {
        div {
            style: "flex: 1; padding: 2rem; background: rgba(10, 15, 20, 0.95); border-radius: 16px; border: 1px solid rgba(0, 255, 170, 0.15); color: #FFF;",

            h2 {
                style: "font-family: 'Space Grotesk', sans-serif; font-size: 2rem; color: #00FFAA; margin-bottom: 1.5rem; display: flex; align-items: center; gap: 0.5rem;",
                "🔐 Key Vault Manager"
            }

            div {
                style: "background: rgba(0,255,170,0.03); border: 1px solid rgba(0,255,170,0.1); border-radius: 12px; padding: 2rem;",

                div {
                    style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 2rem; padding-bottom: 1rem; border-bottom: 1px solid rgba(255,255,255,0.05);",
                    div {
                        div { style: "font-size: 1.2rem; font-weight: bold;", "Master Ed25519 Root" }
                        div { style: "color: #888; font-family: monospace; font-size: 0.9rem; margin-top: 0.2rem;", "did:q42:8f7a9...3b2c" }
                    }
                    button {
                        style: "padding: 0.6rem 1.2rem; background: rgba(255,255,255,0.1); border: 1px solid rgba(255,255,255,0.2); border-radius: 6px; color: #FFF; cursor: pointer;",
                        "Export Backup"
                    }
                }

                h3 { style: "color: #CCC; margin-bottom: 1rem;", "Derived Lane Keys" }

                table {
                    style: "width: 100%; border-collapse: collapse;",
                    thead {
                        style: "text-align: left; color: #888; font-size: 0.9rem;",
                        tr {
                            th { style: "padding: 0.8rem; border-bottom: 1px solid rgba(255,255,255,0.1);", "Lane" }
                            th { style: "padding: 0.8rem; border-bottom: 1px solid rgba(255,255,255,0.1);", "Derivation Path" }
                            th { style: "padding: 0.8rem; border-bottom: 1px solid rgba(255,255,255,0.1);", "Status" }
                        }
                    }
                    tbody {
                        tr {
                            td { style: "padding: 0.8rem; border-bottom: 1px solid rgba(255,255,255,0.05);", "Passthrough" }
                            td { style: "padding: 0.8rem; border-bottom: 1px solid rgba(255,255,255,0.05); font-family: monospace; color: #00FFAA;", "m/44'/42'/0'" }
                            td { style: "padding: 0.8rem; border-bottom: 1px solid rgba(255,255,255,0.05);", span { style: "padding: 0.2rem 0.6rem; background: rgba(0,255,170,0.1); color: #00FFAA; border-radius: 12px; font-size: 0.8rem;", "Active" } }
                        }
                        tr {
                            td { style: "padding: 0.8rem;", "Sanctuary Mode" }
                            td { style: "padding: 0.8rem; font-family: monospace; color: #FF3366;", "m/44'/42'/1'" }
                            td { style: "padding: 0.8rem;", span { style: "padding: 0.2rem 0.6rem; background: rgba(255,51,102,0.1); color: #FF3366; border-radius: 12px; font-size: 0.8rem;", "Locked" } }
                        }
                    }
                }
            }
        }
    }
}
