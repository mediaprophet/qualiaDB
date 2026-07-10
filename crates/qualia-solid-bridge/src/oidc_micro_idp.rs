//! Demo-only mock Solid-OIDC-shaped provider for **local personal pod** testing.
//!
//! This is **not** a production identity root (see `NON_GOALS.md`).
//! Tokens are clearly labeled `demo-` / `mock-` and must never be treated as
//! real provenance. Enabled at **runtime** via `BridgeConfig.demo_oidc`
//! (and/or the historical `--features demo` compile flag for unit tests).

use serde_json::json;
use warp::Filter;

/// Demo OIDC + minimal WebID profile routes bound to `public_base`.
pub fn oidc_routes(
    public_base: String,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let base = public_base.trim_end_matches('/').to_string();
    let base_cfg = base.clone();
    let config = warp::path!(".well-known" / "openid-configuration")
        .and(warp::get())
        .map(move || {
            warp::reply::json(&json!({
                "issuer": base_cfg,
                "authorization_endpoint": format!("{base_cfg}/authorize"),
                "token_endpoint": format!("{base_cfg}/token"),
                "jwks_uri": format!("{base_cfg}/jwks"),
                "registration_endpoint": format!("{base_cfg}/register"),
                "response_types_supported": ["code", "id_token", "token"],
                "subject_types_supported": ["public"],
                "id_token_signing_alg_values_supported": ["EdDSA", "RS256"],
                "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post", "none"],
                "scopes_supported": ["openid", "profile", "webid", "offline_access"],
                "claims_supported": ["sub", "webid", "iss", "aud"],
                "code_challenge_methods_supported": ["S256", "plain"]
            }))
        });

    let jwks = warp::path!("jwks").and(warp::get()).map(|| {
        // Placeholder JWKS — demo only. Real keys require audited identity subsystem.
        warp::reply::json(&json!({
            "keys": [{
                "kty": "OKP",
                "crv": "Ed25519",
                "use": "sig",
                "kid": "qualia-demo-local-key",
                "x": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
                "alg": "EdDSA"
            }]
        }))
    });

    let base_tok = base.clone();
    let token = warp::path!("token")
        .and(warp::post())
        .and(warp::body::bytes())
        .map(move |_body: bytes::Bytes| {
            let webid = format!("{base_tok}/profile/card#me");
            // Opaque demo tokens — not real JWTs. Solid clients that only check
            // presence of Bearer will proceed; full DPoP/JWT validation needs R-band work.
            warp::reply::json(&json!({
                "access_token": format!("demo-access-{}", webid),
                "token_type": "Bearer",
                "expires_in": 3600,
                "id_token": format!("demo-id-token.webid={}", webid),
                "scope": "openid profile webid"
            }))
        });

    let base_auth = base.clone();
    let authorize = warp::path!("authorize")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .map(move |q: std::collections::HashMap<String, String>| {
            // Auto-approve demo: redirect to redirect_uri with code=demo-code
            let redirect = q
                .get("redirect_uri")
                .cloned()
                .unwrap_or_else(|| format!("{base_auth}/"));
            let state = q.get("state").cloned().unwrap_or_default();
            let sep = if redirect.contains('?') { "&" } else { "?" };
            let loc = format!(
                "{redirect}{sep}code=demo-auth-code&state={state}"
            );
            warp::http::Response::builder()
                .status(302)
                .header("location", loc)
                .header("content-type", "text/plain")
                .body("demo OIDC auto-approve redirect".to_string())
                .unwrap_or_else(|_| {
                    warp::http::Response::new("redirect failed".to_string())
                })
        });

    // Dynamic client registration stub (many Solid clients call this).
    let register = warp::path!("register")
        .and(warp::post())
        .and(warp::body::bytes())
        .map(move |_body: bytes::Bytes| {
            warp::reply::json(&json!({
                "client_id": "qualia-demo-client",
                "client_secret": "demo-not-secret",
                "redirect_uris": [],
                "client_name": "Qualia Demo Solid Client",
                "token_endpoint_auth_method": "client_secret_basic"
            }))
        });

    let base_prof = base.clone();
    let profile = warp::path!("webizen" / "profile" / "card")
        .and(warp::get())
        .map(move || {
            let turtle = format!(
                r#"@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix solid: <http://www.w3.org/ns/solid/terms#> .

<#me> a foaf:Person ;
    foaf:name "Local Webizen" ;
    solid:oidcIssuer <{base_prof}> .
"#
            );
            warp::reply::with_header(turtle, "content-type", "text/turtle")
        });

    config
        .or(jwks)
        .or(token)
        .or(authorize)
        .or(register)
        .or(profile)
}
