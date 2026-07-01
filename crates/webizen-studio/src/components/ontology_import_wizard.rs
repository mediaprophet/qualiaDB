//! Ontology import + domain-matched layout presets for the studio canvas.

use dioxus::prelude::*;
use serde::Deserialize;

use crate::canvas_model::{PanePlacement, PresentationMode};

#[derive(Clone, Debug)]
pub struct OntologyLayoutSuggestion {
    pub label: String,
    pub domain: String,
    pub description: String,
    pub panes: Vec<PanePlacement>,
    pub presentation: PresentationMode,
    pub ontology_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CatalogOntology {
    id: String,
    name: String,
    domain: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
struct CatalogResponse {
    ontologies: Vec<CatalogOntology>,
}

#[derive(Clone, Debug, Deserialize)]
struct EnqueueJobResponse {
    id: String,
}

#[derive(Clone, Debug, Deserialize)]
struct LocalJob {
    id: String,
    status: JobStatus,
    progress: f64,
    message: String,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

async fn poll_interval_ms(ms: u32) {
    #[cfg(target_arch = "wasm32")]
    {
        gloo_timers::future::TimeoutFuture::new(ms).await;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{Duration, Instant};
        let deadline = Instant::now() + Duration::from_millis(ms as u64);
        while Instant::now() < deadline {
            std::thread::yield_now();
        }
    }
}

async fn poll_job_until_done(job_id: &str) -> Result<LocalJob, String> {
    let client = reqwest::Client::new();
    let url = crate::endpoints::job_url(job_id);
    for _ in 0..120 {
        let res = client
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("job poll failed ({})", res.status()));
        }
        let job = res.json::<LocalJob>().await.map_err(|e| e.to_string())?;
        match job.status {
            JobStatus::Completed => return Ok(job),
            JobStatus::Failed => {
                return Err(format!(
                    "job {} failed: {}",
                    job.id,
                    job.error.unwrap_or_else(|| job.message.clone())
                ));
            }
            JobStatus::Cancelled => return Err("job cancelled".to_string()),
            JobStatus::Queued | JobStatus::Running => {
                poll_interval_ms(500).await;
            }
        }
    }
    Err(format!("job {job_id} poll timed out"))
}

fn pane(id: &str, x: u16, y: u16, w: u16, h: u16) -> PanePlacement {
    PanePlacement {
        component_id: id.to_string(),
        x,
        y,
        w,
        h,
        data_bindings: Vec::new(),
        binds_rpc: None,
        requires_capability: Vec::new(),
        ui_mode: None,
        layer: Default::default(),
        anchor: None,
        min_w_points: 0,
        min_h_points: 0,
        supported_presentations: Vec::new(),
        theme: Default::default(),
    }
}

pub fn builtin_layout_suggestions(catalog: &[CatalogOntology]) -> Vec<OntologyLayoutSuggestion> {
    let specs = vec![
        (
            "Legal & guardianship",
            "legal",
            "N3 rules, SHACL shapes, and contextual workspace for care contracts.",
            PresentationMode::GridBound,
            vec![
                pane("contextual-workspace", 0, 0, 56, 62),
                pane("n3-logic-studio", 58, 0, 36, 30),
                pane("sparql-explorer", 58, 32, 36, 30),
            ],
            &["shacl", "legal", "guardianship"] as &[&str],
        ),
        (
            "Health & clinical",
            "health",
            "Vitals monitor, ontology browser, and inference harness for FHIR-aligned graphs.",
            PresentationMode::NodeRelational,
            vec![
                pane("health-monitor", 0, 0, 48, 40),
                pane("personal-ontology-builder", 50, 0, 44, 40),
                pane("llm-harness", 0, 42, 94, 20),
            ],
            &["health", "fhir", "clinical", "medical"] as &[&str],
        ),
        (
            "Commons & spatial",
            "commons",
            "10D manifold portal with nexus dashboard for bilateral micro-commons intake.",
            PresentationMode::Spatial,
            vec![
                pane("nexus", 0, 0, 40, 36),
                pane("render-preview", 42, 0, 52, 62),
                pane("wal-inspector", 0, 38, 40, 24),
            ],
            &["commons", "governance", "rights"] as &[&str],
        ),
        (
            "Research & semantics",
            "semantics",
            "WordNet demo, SPARQL explorer, and diffusion visualizer for lexical grounding.",
            PresentationMode::GridBound,
            vec![
                pane("wordnet-demo", 0, 0, 46, 30),
                pane("sparql-explorer", 48, 0, 46, 30),
                pane("diffusion-visualizer", 0, 32, 94, 30),
            ],
            &["wordnet", "linguistics", "lexicon", "semantics"] as &[&str],
        ),
    ];

    specs
        .into_iter()
        .map(|(label, domain, description, presentation, panes, keywords)| {
            let ontology_ids = resolve_ontology_ids(domain, keywords, catalog);
            OntologyLayoutSuggestion {
                label: label.to_string(),
                domain: domain.to_string(),
                description: description.to_string(),
                panes,
                presentation,
                ontology_ids,
            }
        })
        .collect()
}

fn resolve_ontology_ids(domain: &str, keywords: &[&str], catalog: &[CatalogOntology]) -> Vec<String> {
    let mut ids = Vec::new();
    for ont in catalog {
        let domain_hit = ont
            .domain
            .as_deref()
            .map(|d| d.to_ascii_lowercase().contains(domain))
            .unwrap_or(false);
        let tag_hit = ont.tags.as_ref().map(|tags| {
            tags.iter().any(|t| {
                let tl = t.to_ascii_lowercase();
                keywords.iter().any(|k| tl.contains(k))
            })
        }).unwrap_or(false);
        let id_hit = keywords.iter().any(|k| ont.id.to_ascii_lowercase().contains(k));
        let name_hit = keywords
            .iter()
            .any(|k| ont.name.to_ascii_lowercase().contains(k));
        if domain_hit || tag_hit || id_hit || name_hit {
            ids.push(ont.id.clone());
        }
    }
    if ids.is_empty() && (domain == "legal" || domain == "commons") {
        ids.push("shacl".to_string());
    }
    ids.sort();
    ids.dedup();
    ids.truncate(2);
    ids
}

async fn fetch_catalog() -> Vec<CatalogOntology> {
    if !crate::endpoints::is_native_host() {
        return Vec::new();
    }
    match reqwest::get(crate::endpoints::assets_catalog_url()).await {
        Ok(res) if res.status().is_success() => res
            .json::<CatalogResponse>()
            .await
            .map(|c| c.ontologies)
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

async fn enqueue_ontology_import(ontology_id: &str, bundled: bool) -> Result<String, String> {
    let body = if bundled {
        serde_json::json!({
            "kind": "bundled_ontology_seed",
            "ontology_id": ontology_id
        })
    } else {
        serde_json::json!({
            "kind": "ontology_catalog_import",
            "ontology_id": ontology_id
        })
    };
    let client = reqwest::Client::new();
    let res = client
        .post(crate::endpoints::assets_enqueue_url())
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("enqueue failed ({})", res.status()));
    }
    let job = res
        .json::<EnqueueJobResponse>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(job.id)
}

#[component]
pub fn OntologyImportWizard(
    on_apply: EventHandler<OntologyLayoutSuggestion>,
) -> Element {
    let mut catalog = use_signal(Vec::<CatalogOntology>::new);
    let import_status = use_signal(|| String::new());

    use_effect(move || {
        spawn(async move {
            let rows = fetch_catalog().await;
            catalog.set(rows);
        });
    });

    let suggestions = builtin_layout_suggestions(&catalog.read());

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 0.65rem;",
            h3 {
                style: "font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.1em; color: var(--qualia-text-muted, #888); margin: 0;",
                "Ontology import"
            }
            p {
                style: "font-size: 0.72rem; color: var(--qualia-text-muted, #888); margin: 0; line-height: 1.45;",
                "Import domain ontologies via the local job queue, then apply a matched pane layout."
            }
            if !import_status.read().is_empty() {
                p {
                    style: "font-size: 0.68rem; color: var(--qualia-accent); margin: 0;",
                    "{import_status.read()}"
                }
            }
            for suggestion in suggestions {
                div {
                    key: "{suggestion.domain}",
                    style: "padding: 0.65rem 0.75rem; border-radius: 8px; border: 1px solid var(--qualia-border, #333); background: var(--qualia-surface-elevated, #1a1a1a);",
                    div {
                        style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.25rem;",
                        "{suggestion.label}"
                    }
                    div {
                        style: "font-size: 0.68rem; color: var(--qualia-text-muted, #888); margin-bottom: 0.5rem; line-height: 1.4;",
                        "{suggestion.description}"
                    }
                    if !suggestion.ontology_ids.is_empty() {
                        div {
                            style: "font-size: 0.6rem; color: var(--qualia-text-muted); margin-bottom: 0.45rem;",
                            "Ontologies: {suggestion.ontology_ids.join(\", \")}"
                        }
                    }
                    div {
                        style: "display: flex; justify-content: flex-end; align-items: center; gap: 0.35rem; flex-wrap: wrap;",
                        span {
                            style: "font-size: 0.62rem; color: var(--qualia-accent, #f59e0b); text-transform: uppercase; letter-spacing: 0.06em; margin-right: auto;",
                            "{suggestion.domain} · {suggestion.panes.len()} panes"
                        }
                        if crate::endpoints::is_native_host() && !suggestion.ontology_ids.is_empty() {
                            button {
                                style: "padding: 0.25rem 0.55rem; font-size: 0.65rem; border-radius: 6px; border: 1px solid var(--qualia-border); background: transparent; color: var(--qualia-text); cursor: pointer;",
                                onclick: {
                                    let ids = suggestion.ontology_ids.clone();
                                    let mut import_status = import_status.clone();
                                    move |_| {
                                        let ids = ids.clone();
                                        import_status.set("Queuing ontology import…".to_string());
                                        spawn(async move {
                                            let mut messages = Vec::new();
                                            for id in ids {
                                                let bundled = id == "shacl";
                                                match enqueue_ontology_import(&id, bundled).await {
                                                    Ok(job_id) => {
                                                        import_status.set(format!(
                                                            "{id}: queued ({job_id})…"
                                                        ));
                                                        match poll_job_until_done(&job_id).await {
                                                            Ok(job) => messages.push(format!(
                                                                "{id}: done ({:.0}%) — {}",
                                                                job.progress * 100.0,
                                                                job.message
                                                            )),
                                                            Err(err) => messages.push(format!(
                                                                "{id}: {err}"
                                                            )),
                                                        }
                                                        import_status.set(messages.join(" · "));
                                                    }
                                                    Err(err) => messages.push(format!("{id}: {err}")),
                                                }
                                            }
                                            import_status.set(messages.join(" · "));
                                        });
                                    }
                                },
                                "Import"
                            }
                        }
                        button {
                            style: "padding: 0.25rem 0.55rem; font-size: 0.65rem; border-radius: 6px; border: 1px solid var(--qualia-accent); background: rgba(245,158,11,0.1); color: var(--qualia-text); cursor: pointer;",
                            onclick: {
                                let s = suggestion.clone();
                                move |_| on_apply.call(s.clone())
                            },
                            "Apply layout"
                        }
                    }
                }
            }
        }
    }
}