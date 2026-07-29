use crate::Route;
use dioxus::prelude::*;

const DASHBOARD_CSS: &str = r#"
    .home-tab { 
        width: 100%; height: 100%; overflow-y: auto; 
        padding: 2.5rem; 
        background: radial-gradient(circle at top right, rgba(20, 30, 60, 0.4), transparent 50%),
                    radial-gradient(circle at bottom left, rgba(60, 20, 40, 0.2), transparent 50%);
    }
    .home-inner { width: min(1200px, 100%); margin: 0 auto; display: flex; flex-direction: column; gap: 2rem; }
    
    .home-header { text-align: center; margin-bottom: 1rem; }
    .home-header h1 { margin: 0 0 0.5rem; color: var(--qualia-text); font-size: 2.5rem; font-weight: 800; letter-spacing: -0.5px; background: linear-gradient(90deg, #fff, var(--qualia-text-muted)); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
    .home-header p { margin: 0 auto; max-width: 600px; color: var(--qualia-text-muted); font-size: 1.05rem; line-height: 1.6; }

    .widget-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: 1.5rem; }
    
    .widget-card { 
        padding: 1.5rem; 
        border: 1px solid rgba(255,255,255,0.08); 
        border-radius: 16px; 
        background: rgba(10, 15, 25, 0.6); 
        backdrop-filter: blur(20px);
        box-shadow: 0 8px 32px rgba(0,0,0,0.2);
        display: flex; flex-direction: column; gap: 1rem;
        transition: transform 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275), box-shadow 0.3s ease;
    }
    .widget-card:hover {
        transform: translateY(-4px);
        box-shadow: 0 12px 40px rgba(0,0,0,0.3);
        border-color: rgba(255,255,255,0.15);
    }
    
    .widget-header { display: flex; align-items: center; justify-content: space-between; }
    .widget-title { display: flex; align-items: center; gap: 0.6rem; color: var(--qualia-text); font-size: 1.1rem; font-weight: 700; }
    .widget-title sl-icon { color: var(--qualia-accent); font-size: 1.2rem; }
    
    .vital-row { display: flex; justify-content: space-between; align-items: center; padding: 0.75rem 0; border-bottom: 1px solid rgba(255,255,255,0.05); }
    .vital-row:last-child { border-bottom: none; }
    .vital-label { color: var(--qualia-text-muted); font-size: 0.9rem; }
    .vital-value { color: var(--qualia-text); font-size: 1rem; font-weight: 600; display: flex; align-items: center; gap: 0.5rem; }
    .status-dot { width: 8px; height: 8px; border-radius: 50%; background: #55c178; box-shadow: 0 0 8px #55c178; }
    .status-dot.warn { background: #e07a5f; box-shadow: 0 0 8px #e07a5f; }

    .social-feed { display: flex; flex-direction: column; gap: 0.8rem; }
    .social-item { display: grid; grid-template-columns: 40px 1fr; gap: 0.8rem; padding: 0.8rem; background: rgba(0,0,0,0.2); border-radius: 10px; }
    .social-avatar { width: 40px; height: 40px; border-radius: 50%; background: linear-gradient(135deg, var(--qualia-accent), #8a2387); }
    .social-content h4 { margin: 0 0 0.2rem; font-size: 0.9rem; color: var(--qualia-text); }
    .social-content p { margin: 0; font-size: 0.8rem; color: var(--qualia-text-muted); line-height: 1.4; }

    .quick-actions { display: grid; grid-template-columns: repeat(2, 1fr); gap: 0.8rem; }
    .btn-action { 
        display: flex; align-items: center; justify-content: center; gap: 0.5rem;
        padding: 0.8rem; border-radius: 10px;
        background: rgba(255,255,255,0.05); color: var(--qualia-text);
        text-decoration: none; font-size: 0.9rem; font-weight: 600;
        border: 1px solid transparent; transition: all 0.2s ease;
    }
    .btn-action:hover { background: var(--qualia-accent-glow); border-color: var(--qualia-accent); color: var(--qualia-text); }
"#;

#[derive(serde::Deserialize, Clone, PartialEq, Default)]
pub struct DashboardSnapshot {
    pub heart_rate: u32,
    pub sleep_quality: u32,
    pub stress_level: String,
    pub recent_social_events: Vec<SocialEvent>,
}

#[derive(serde::Deserialize, Clone, PartialEq, Default)]
pub struct SocialEvent {
    pub actor: String,
    pub message: String,
    pub timestamp: String,
}

use crate::components::qapp_engine::invoke_json;

#[component]
pub fn Dashboard() -> Element {
    let mut snapshot = use_signal(|| DashboardSnapshot::default());

    use_effect(move || {
        spawn(async move {
            if let Ok(res) = invoke_json("get_dashboard_snapshot", serde_json::json!({})).await {
                if let Ok(data) = serde_json::from_value::<DashboardSnapshot>(res) {
                    snapshot.set(data);
                }
            }
        });
    });

    let data = snapshot();

    rsx! {
        style { dangerous_inner_html: DASHBOARD_CSS }

        main { class: "home-tab",
            div { class: "home-inner",

                header { class: "home-header",
                    h1 { "Overview" }
                    p { "Webizen is local-first. Start with Talk (your agent), Keep (records & body), or Reach (browser)." }
                }

                div { class: "widget-grid",

                    article { class: "widget-card",
                        div { class: "widget-header",
                            div { class: "widget-title",
                                sl-icon { "name": "chat-dots" }
                                "Relations"
                            }
                            Link { to: Route::TalkRoute {}, sl-icon { "name": "box-arrow-up-right", style: "color: var(--qualia-text-muted); cursor: pointer;" } }
                        }
                        p { style: "margin:0; color: var(--qualia-text-muted); font-size: 0.9rem; line-height: 1.5;",
                            "Private conversation with your local agent. Load a model, then chat — streaming on-device. Instruments are not peers."
                        }
                        div { class: "quick-actions", style: "margin-top: 0.5rem;",
                            Link { to: Route::TalkRoute {}, class: "btn-action", sl-icon { "name": "chat-dots" } "Open Relations" }
                            Link { to: Route::LibraryRoute {}, class: "btn-action", sl-icon { "name": "archive" } "Open Memory" }
                        }
                    }

                    // Health Vault Widget
                    article { class: "widget-card",
                        div { class: "widget-header",
                            div { class: "widget-title",
                                sl-icon { "name": "heart-pulse" }
                                "Health Vault Vitals"
                            }
                            Link { to: Route::HealthRoute {}, sl-icon { "name": "box-arrow-up-right", style: "color: var(--qualia-text-muted); cursor: pointer;" } }
                        }
                        div {
                            div { class: "vital-row",
                                span { class: "vital-label", "Resting Heart Rate" }
                                span { class: "vital-value", div { class: "status-dot" } "{data.heart_rate} bpm" }
                            }
                            div { class: "vital-row",
                                span { class: "vital-label", "Sleep Quality (N3)" }
                                span { class: "vital-value", div { class: "status-dot warn" } "Fair ({data.sleep_quality}%)" }
                            }
                            div { class: "vital-row",
                                span { class: "vital-label", "Stress Inference" }
                                span { class: "vital-value", div { class: "status-dot" } "{data.stress_level}" }
                            }
                        }
                    }

                    // Social Nexus Widget
                    article { class: "widget-card",
                        div { class: "widget-header",
                            div { class: "widget-title",
                                sl-icon { "name": "people" }
                                "Decentralized Nexus"
                            }
                            Link { to: Route::NexusRoute {}, sl-icon { "name": "box-arrow-up-right", style: "color: var(--qualia-text-muted); cursor: pointer;" } }
                        }
                        div { class: "social-feed",
                            if data.recent_social_events.is_empty() {
                                div { class: "social-item",
                                    div { class: "social-avatar", style: "background: linear-gradient(135deg, #f2709c, #ff9472);" }
                                    div { class: "social-content",
                                        h4 { "Agent: Archivist" }
                                        p { "I've indexed 42 new semantic graphs from your latest reading." }
                                    }
                                }
                                div { class: "social-item",
                                    div { class: "social-avatar", style: "background: linear-gradient(135deg, #00c6ff, #0072ff);" }
                                    div { class: "social-content",
                                        h4 { "did:q42:alice" }
                                        p { "Accepted the smart contract for the shared knowledge vault." }
                                    }
                                }
                            } else {
                                for event in data.recent_social_events.iter() {
                                    div { class: "social-item",
                                        div { class: "social-avatar", style: "background: linear-gradient(135deg, var(--qualia-accent), #0072ff);" }
                                        div { class: "social-content",
                                            h4 { "{event.actor}" }
                                            p { "{event.message}" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // QApps & Spatial Ecosystem
                    article { class: "widget-card",
                        div { class: "widget-header",
                            div { class: "widget-title",
                                sl-icon { "name": "grid" }
                                "Spatial Ecosystem"
                            }
                        }
                        div { class: "quick-actions",
                            Link { to: Route::BrowserRoute {}, class: "btn-action", sl-icon { "name": "globe2" } "Reach" }
                            Link { to: Route::AnatomyRoute {}, class: "btn-action", sl-icon { "name": "person" } "Anatomy" }
                            Link { to: Route::LibraryRoute {}, class: "btn-action", sl-icon { "name": "collection" } "Library" }
                            Link { to: Route::SettingsRoute {}, class: "btn-action", sl-icon { "name": "gear" } "Settings" }
                        }
                    }

                }
            }
        }
    }
}
