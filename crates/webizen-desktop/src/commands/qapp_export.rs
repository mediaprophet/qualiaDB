//! Standalone QApp Export (WASM + QR + LAN)

#![allow(non_snake_case)]

use super::*;
use super::qapp_telemetry::{qapp_slug, QappAnalysisRequest};
use super::qapp_host::qapp_analyze;
use tauri::command;

// â”€â”€ Standalone QApp Export (WASM + QR + LAN server) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

/// Export a QApp as a self-contained WASM app package (single package using webizen-web).
/// Generates .q42 from the QApp using full QualiaDB, bundles with the web WASM runtime,
/// a minimal loader HTML (with DOM generation support), starts a LAN-accessible static server,
/// and returns the URL (frontend can display QR code using existing shoelace qr component).
/// This enables the off-grid "create on desktop, QR on LAN, load on mobile/other device" flow.
#[command]
pub async fn export_qapp_as_wasm_package(qapp_name: String) -> Result<QappWasmExport, String> {
    let slug = qapp_slug(&qapp_name);
    let export_base = std::env::temp_dir().join("webizen-exported-qapps");
    let export_dir = export_base.join(&slug);
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;

    let qapp_result = qapp_analyze(QappAnalysisRequest {
        discipline: qapp_name.clone(),
        fields: vec![("export_mode".into(), "standalone_wasm".into())],
        notes: "LAN standalone export bundle".into(),
    })?;

    #[derive(serde::Serialize)]
    struct ExportTriple {
        s: String,
        p: String,
        o: String,
    }

    let subject = format!("qapp:{slug}");
    let mut triples = vec![
        ExportTriple {
            s: subject.clone(),
            p: "rdfs:label".into(),
            o: qapp_name.clone(),
        },
        ExportTriple {
            s: subject.clone(),
            p: "q42:summary".into(),
            o: qapp_result.summary.clone(),
        },
        ExportTriple {
            s: subject.clone(),
            p: "q42:provenance".into(),
            o: qapp_result.provenance_hash.clone(),
        },
    ];
    for (i, assertion) in qapp_result.assertions.iter().enumerate() {
        triples.push(ExportTriple {
            s: subject.clone(),
            p: format!("q42:assertion/{i}"),
            o: assertion.clone(),
        });
    }

    let scene_json = serde_json::to_string(&triples).map_err(|e| e.to_string())?;
    std::fs::write(export_dir.join("qapp_scene.json"), scene_json.as_bytes())
        .map_err(|e| e.to_string())?;

    let web_pkg_src = resolve_web_pkg_src();
    let pkg_dst = export_dir.join("pkg");
    std::fs::create_dir_all(&pkg_dst).map_err(|e| e.to_string())?;
    if !web_pkg_src.exists() {
        return Err(format!(
            "webizen-web/pkg not found at {}. Run wasm-pack build --target web --out-dir pkg in webizen-web.",
            web_pkg_src.display()
        ));
    }
    for entry in std::fs::read_dir(&web_pkg_src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_file() {
            std::fs::copy(entry.path(), pkg_dst.join(entry.file_name()))
                .map_err(|e| e.to_string())?;
        }
    }

    let loader_html = format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>{qapp} Â· Webizen QApp</title>
<style>
body {{ margin:0; background:#0a0a0a; color:#e8eaed; font-family:Inter,system-ui,sans-serif; }}
.qapp-root {{ padding:1rem 1.25rem; max-width:960px; margin:0 auto; }}
.qapp-chrome h1 {{ margin:0 0 0.25rem; font-size:1.35rem; }}
.qapp-meta {{ opacity:0.75; font-size:0.9rem; }}
.qapp-panel {{ margin-top:1rem; display:grid; gap:0.5rem; }}
.qapp-panel h2 {{ margin:0.35rem 0 0; font-size:1.1rem; color:#a5f3fc; }}
.qapp-panel p, .qapp-panel div, .qapp-panel li {{ margin:0; line-height:1.45; }}
#viewport {{ display:block; width:min(100%, 900px); margin:1.25rem auto; border:1px solid #333; border-radius:6px; }}
</style></head><body>
<div id="root"></div>
<canvas id="viewport" width="900" height="520"></canvas>
<script type="module">
import init, {{ WebEngine }} from './pkg/webizen_web.js';
await init();
const engine = new WebEngine();
const scene = await (await fetch('./qapp_scene.json')).text();
engine.load_json_scene(scene);
engine.mount_qapp('root');
engine.render_to_canvas(document.getElementById('viewport'));
</script>
</body></html>"#,
        qapp = qapp_name
    );
    std::fs::write(export_dir.join("index.html"), loader_html).map_err(|e| e.to_string())?;

    let port: u16 = 8081;
    ensure_lan_export_server(export_base.clone(), port);

    let lan_ip = guess_lan_ipv4().unwrap_or_else(|| "127.0.0.1".to_string());
    let path = format!("/{slug}/index.html");
    let url = format!("http://127.0.0.1:{port}{path}");
    let lan_url = format!("http://{lan_ip}:{port}{path}");

    let note = format!(
        "QApp '{qapp_name}' exported to {}. Scan QR with lan_url ({lan_url}). \
         Package = single WASM + qapp_scene.json. Works offline on LAN after first load.",
        export_dir.display()
    );

    Ok(QappWasmExport {
        url,
        lan_url,
        lan_ip,
        package_dir: export_dir.to_string_lossy().to_string(),
        note,
    })
}

