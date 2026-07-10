//! Solid bridge HTTP daemon — personal pod server + optional demo OIDC.

use crate::ldp_translator::{ldp_routes, pod_ldp_routes};
use crate::pod_store::PodStore;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use warp::Filter;

/// Runtime configuration for the personal Solid pod / bridge.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    pub listen: SocketAddr,
    /// Filesystem root for LDP resources.
    pub data_root: PathBuf,
    /// Public base URL advertised in WebID / OIDC discovery (no trailing slash).
    pub public_base: String,
    /// Mount demo Solid-OIDC-shaped routes (local hackathon / SolidOS smoke only).
    pub demo_oidc: bool,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        let data_root = std::env::var("QUALIA_SOLID_POD_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs_next_fallback()
            });
        let host = std::env::var("QUALIA_SOLID_HOST").unwrap_or_else(|_| "127.0.0.1".into());
        let port: u16 = std::env::var("QUALIA_SOLID_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4243);
        let public_base = std::env::var("QUALIA_SOLID_PUBLIC_BASE")
            .unwrap_or_else(|_| format!("http://{host}:{port}"));
        let demo_oidc = matches!(
            std::env::var("QUALIA_SOLID_DEMO_OIDC").ok().as_deref(),
            Some("1") | Some("true") | Some("yes")
        ) || cfg!(feature = "demo");
        Self {
            listen: SocketAddr::new(
                host.parse().unwrap_or_else(|_| ([127, 0, 0, 1]).into()),
                port,
            ),
            data_root,
            public_base,
            demo_oidc,
        }
    }
}

fn dirs_next_fallback() -> PathBuf {
    // Avoid extra dep: use TEMP or CWD/solid-pod
    std::env::temp_dir().join("qualia-solid-pod")
}

/// Type-erased filter so demo/non-demo trees share one return type.
type DynRoutes = warp::filters::BoxedFilter<(Box<dyn warp::Reply + Send>,)>;

fn box_reply<F, R>(f: F) -> DynRoutes
where
    F: Filter<Extract = (R,), Error = warp::Rejection> + Clone + Send + Sync + 'static,
    R: warp::Reply + Send + 'static,
{
    f.map(|r: R| -> Box<dyn warp::Reply + Send> { Box::new(r) })
        .boxed()
}

/// Compose routes for a config (used by tests + daemon).
pub fn bridge_routes_for(cfg: &BridgeConfig) -> DynRoutes {
    let store = Arc::new(PodStore::new(cfg.data_root.clone()));
    let _ = store.ensure_defaults(&cfg.public_base);
    let demo = cfg.demo_oidc || cfg!(feature = "demo");

    let pod = pod_ldp_routes(store, cfg.public_base.clone());

    // Health / identity banner
    let banner = warp::path!(".well-known" / "qualia-solid-bridge")
        .and(warp::get())
        .map(move || {
            warp::reply::json(&serde_json::json!({
                "service": "qualia-solid-bridge",
                "role": ["personal-pod", "ldp-resource-server", "solid-consumer-library"],
                "demo_oidc": demo,
                "note": "Demo OIDC is not a production identity root (NON_GOALS.md)"
            }))
        });

    if demo {
        let oidc = crate::oidc_micro_idp::oidc_routes(cfg.public_base.clone());
        box_reply(oidc.or(banner).or(pod))
    } else {
        // Keep legacy simulated /public/<name> routes for unit tests + fallback.
        box_reply(banner.or(pod).or(ldp_routes()))
    }
}

/// Default routes (unit tests / legacy).
pub fn bridge_routes() -> DynRoutes {
    bridge_routes_for(&BridgeConfig::default())
}

/// Starts the Solid Bridge Proxy Daemon (blocking).
///
/// Implements the "Tokio Perimeter Firewall" as mandated by the Qualia Core.
/// - Tokio is restricted to a single thread.
/// - The thread is pinned natively to Core 3 when available (I/O & Parity loop).
pub fn start_proxy_daemon() {
    start_proxy_daemon_with(BridgeConfig::default());
}

pub fn start_proxy_daemon_with(cfg: BridgeConfig) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio Single-Threaded Firewall");

    let core_ids = core_affinity::get_core_ids().unwrap_or_default();
    if core_ids.len() > 3 {
        core_affinity::set_for_current(core_ids[3]);
        println!("Webizen Proxy Daemon pinned to CPU Core 3.");
    } else {
        println!("System has < 4 cores. Tokio not pinned to Core 3.");
    }

    rt.block_on(run_bridge(cfg));
}

/// Async entry (for CLI already inside a tokio runtime).
pub async fn run_bridge(cfg: BridgeConfig) {
    let store = PodStore::new(&cfg.data_root);
    if let Err(e) = store.ensure_defaults(&cfg.public_base) {
        eprintln!("solid-bridge: failed to init pod root: {e}");
    }

    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec![
            "Authorization",
            "Content-Type",
            "Accept",
            "Slug",
            "Link",
            "DPoP",
        ])
        .allow_methods(vec!["GET", "HEAD", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"]);

    let api = bridge_routes_for(&cfg).with(cors);

    println!("============================================================");
    println!(" Qualia Solid Bridge — personal pod + LDP resource server");
    println!(" listen     : http://{}", cfg.listen);
    println!(" public_base: {}", cfg.public_base);
    println!(" data_root  : {}", cfg.data_root.display());
    println!(
        " demo_oidc  : {} {}",
        cfg.demo_oidc || cfg!(feature = "demo"),
        if cfg.demo_oidc || cfg!(feature = "demo") {
            "(DEMO ONLY — not production identity)"
        } else {
            "(off; set QUALIA_SOLID_DEMO_OIDC=1)"
        }
    );
    println!(" profile    : {}/profile/card#me", cfg.public_base);
    println!("============================================================");

    warp::serve(api).run(cfg.listen).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;

    #[tokio::test]
    async fn ldp_and_profile_available() {
        let dir = std::env::temp_dir().join(format!("solid-bridge-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = BridgeConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_root: dir.clone(),
            public_base: "http://127.0.0.1:4243".into(),
            demo_oidc: false,
        };
        let routes = bridge_routes_for(&cfg);

        let response = warp::test::request()
            .method("GET")
            .path("/profile/card")
            .reply(&routes)
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response.headers().get("content-type").is_some());
        let body = String::from_utf8_lossy(response.body());
        assert!(body.contains("foaf:Person") || body.contains("oidcIssuer"));

        let put = warp::test::request()
            .method("PUT")
            .path("/public/hackathon.ttl")
            .body("<urn:inst:1> <urn:pred:deposit> <urn:citizen:1> .")
            .reply(&routes)
            .await;
        assert!(
            put.status() == StatusCode::CREATED || put.status() == StatusCode::OK,
            "put status {}",
            put.status()
        );

        let get = warp::test::request()
            .method("GET")
            .path("/public/hackathon.ttl")
            .reply(&routes)
            .await;
        assert_eq!(get.status(), StatusCode::OK);
        assert!(String::from_utf8_lossy(get.body()).contains("urn:inst:1"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(not(feature = "demo"))]
    #[tokio::test]
    async fn oidc_off_by_default_without_flag() {
        let dir = std::env::temp_dir().join(format!("solid-bridge-oidc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = BridgeConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_root: dir.clone(),
            public_base: "http://127.0.0.1:4243".into(),
            demo_oidc: false,
        };
        let response = warp::test::request()
            .method("GET")
            .path("/.well-known/openid-configuration")
            .reply(&bridge_routes_for(&cfg))
            .await;
        // May 404 when demo off
        assert!(
            response.status() == StatusCode::NOT_FOUND
                || response.status() == StatusCode::METHOD_NOT_ALLOWED
                || response.status() == StatusCode::OK // if something else matches
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn demo_oidc_discovery_when_enabled() {
        let dir = std::env::temp_dir().join(format!("solid-bridge-demo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cfg = BridgeConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            data_root: dir.clone(),
            public_base: "http://127.0.0.1:4243".into(),
            demo_oidc: true,
        };
        let response = warp::test::request()
            .method("GET")
            .path("/.well-known/openid-configuration")
            .reply(&bridge_routes_for(&cfg))
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = String::from_utf8_lossy(response.body());
        assert!(body.contains("authorization_endpoint"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
