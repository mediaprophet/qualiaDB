use dioxus::prelude::*;

#[component]
pub fn DeonticLogicEditor() -> Element {
    rsx! {
        div {
            style: "flex: 1; padding: 2rem; background: rgba(40, 20, 20, 0.8); backdrop-filter: blur(15px); border-radius: 16px; border: 1px solid rgba(255, 100, 100, 0.2); color: #FFF;",

            h2 {
                style: "font-family: 'Outfit', sans-serif; font-size: 2.2rem; color: #FF6B6B; margin-bottom: 0.5rem;",
                "Deontic Logic Editor"
            }
            p { style: "color: #CCC; margin-bottom: 2rem;", "Author OBLIGATE, PERMIT, and FORBID operators over the 42MB Sentinel engine." }

            div {
                style: "display: flex; flex-direction: column; gap: 1rem;",

                div {
                    style: "display: flex; gap: 1rem; align-items: stretch;",
                    div {
                        style: "background: #FF6B6B; color: #000; padding: 1rem; border-radius: 8px; font-weight: bold; display: flex; align-items: center;",
                        "OBLIGATE"
                    }
                    div {
                        style: "flex: 1; background: rgba(0,0,0,0.4); padding: 1rem; border-radius: 8px; border: 1px solid rgba(255,255,255,0.1); display: flex; align-items: center;",
                        span { style: "color: #00FF88; font-family: monospace; margin-right: 0.5rem;", "did:q42:tenant" },
                        "must pay",
                        span { style: "color: #00B8FF; font-family: monospace; margin-left: 0.5rem;", "500 USDC" },
                        " monthly."
                    }
                    button { style: "background: rgba(255,255,255,0.1); border: none; border-radius: 8px; width: 40px; color: #FFF; cursor: pointer;", "⋮" }
                }

                div {
                    style: "display: flex; gap: 1rem; align-items: stretch;",
                    div {
                        style: "background: #00B8FF; color: #000; padding: 1rem; border-radius: 8px; font-weight: bold; display: flex; align-items: center;",
                        "PERMIT"
                    }
                    div {
                        style: "flex: 1; background: rgba(0,0,0,0.4); padding: 1rem; border-radius: 8px; border: 1px solid rgba(255,255,255,0.1); display: flex; align-items: center;",
                        span { style: "color: #00FF88; font-family: monospace; margin-right: 0.5rem;", "did:q42:tenant" },
                        "may use",
                        span { style: "color: #FF00FF; font-family: monospace; margin-left: 0.5rem;", "Common Area" },
                        "."
                    }
                    button { style: "background: rgba(255,255,255,0.1); border: none; border-radius: 8px; width: 40px; color: #FFF; cursor: pointer;", "⋮" }
                }

                button {
                    style: "margin-top: 1rem; padding: 1rem; background: transparent; border: 1px dashed rgba(255, 107, 107, 0.5); border-radius: 8px; color: #FF6B6B; font-weight: bold; cursor: pointer; transition: background 0.2s;",
                    "+ Add Deontic Norm"
                }
            }
        }
    }
}
