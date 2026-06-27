use warp::Filter;

/// Routes exposed by the Solid bridge in production/default builds.
///
/// The mock OIDC provider is intentionally excluded unless the non-default
/// `demo` feature is enabled. Qualia acts as a Solid resource bridge by default;
/// production WebID-OIDC provider support must be a separate audited identity
/// subsystem (see `NON_GOALS.md`).
pub fn bridge_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let ldp_routes = crate::ldp_translator::ldp_routes();

    #[cfg(feature = "demo")]
    {
        crate::oidc_micro_idp::oidc_routes().or(ldp_routes)
    }

    #[cfg(not(feature = "demo"))]
    {
        ldp_routes
    }
}

/// Starts the Solid Bridge Proxy Daemon
///
/// Implements the "Tokio Perimeter Firewall" as mandated by the Qualia Core.
/// - Tokio is restricted to a single thread.
/// - The thread is pinned natively to Core 3 (I/O & Parity loop).
pub fn start_proxy_daemon() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to initialize Tokio Single-Threaded Firewall");

    // Pin this runtime's underlying thread to Core 3
    let core_ids = core_affinity::get_core_ids().unwrap();
    if core_ids.len() > 3 {
        core_affinity::set_for_current(core_ids[3]);
        println!("🚀 Webizen Proxy Daemon successfully pinned to CPU Core 3.");
    } else {
        println!("⚠️ System has < 4 cores. Tokio is not physically pinned to Core 3.");
    }

    rt.block_on(async {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["Authorization", "Content-Type", "Accept"])
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]);

        let api = bridge_routes().with(cors);

        println!("🟢 WebID-Webizen Bridge listening at http://127.0.0.1:4243");
        warp::serve(api).run(([127, 0, 0, 1], 4243)).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use warp::http::StatusCode;

    #[tokio::test]
    async fn ldp_routes_remain_available_without_provider() {
        let response = warp::test::request()
            .method("GET")
            .path("/public/card")
            .reply(&bridge_routes())
            .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "text/turtle"
        );
    }

    #[cfg(not(feature = "demo"))]
    #[tokio::test]
    async fn oidc_provider_routes_are_unreachable_without_demo() {
        let response = warp::test::request()
            .method("GET")
            .path("/.well-known/openid-configuration")
            .reply(&bridge_routes())
            .await;

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[cfg(feature = "demo")]
    #[tokio::test]
    async fn oidc_provider_routes_are_demo_only() {
        let response = warp::test::request()
            .method("GET")
            .path("/.well-known/openid-configuration")
            .reply(&bridge_routes())
            .await;

        assert_eq!(response.status(), StatusCode::OK);
    }
}
