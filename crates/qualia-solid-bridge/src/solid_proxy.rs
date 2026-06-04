use warp::Filter;

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
        // Define the WARP routes
        let oidc_routes = crate::oidc_micro_idp::oidc_routes();
        let ldp_routes = crate::ldp_translator::ldp_routes();
        
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["Authorization", "Content-Type", "Accept"])
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]);

        let api = oidc_routes.or(ldp_routes).with(cors);

        println!("🟢 WebID-Webizen Bridge listening at http://127.0.0.1:4243");
        warp::serve(api).run(([127, 0, 0, 1], 4243)).await;
    });
}
