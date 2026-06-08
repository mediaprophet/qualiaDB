use serde_json::json;
use warp::Filter;

pub fn oidc_routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let config = warp::path!(".well-known" / "openid-configuration")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&json!({
                "issuer": "http://localhost:4243",
                "authorization_endpoint": "http://localhost:4243/authorize",
                "token_endpoint": "http://localhost:4243/token",
                "jwks_uri": "http://localhost:4243/jwks",
                "subject_types_supported": ["public"]
            }))
        });

    let jwks = warp::path!("jwks").and(warp::get()).map(|| {
        // Mock JWKS returning the local Webizen's public key
        warp::reply::json(&json!({
            "keys": [{
                "kty": "OKP",
                "crv": "Ed25519",
                "use": "sig",
                "kid": "qualia-did-q42-local-key",
                "x": "mock-public-key"
            }]
        }))
    });

    let token = warp::path!("token").and(warp::post()).map(|| {
        warp::reply::json(&json!({
            "access_token": "mock-jwt-signed-by-q42",
            "token_type": "Bearer",
            "id_token": "mock-id-token"
        }))
    });

    let profile = warp::path!("webizen" / "profile" / "card")
        .and(warp::get())
        .map(|| {
            // The WebID translation response
            let turtle = r#"
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix solid: <http://www.w3.org/ns/solid/terms#> .

<#me> a foaf:Person ;
    foaf:name "Local Webizen" ;
    solid:oidcIssuer <http://localhost:4243> .
"#;
            warp::reply::with_header(turtle, "content-type", "text/turtle")
        });

    config.or(jwks).or(token).or(profile)
}
