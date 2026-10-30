use std::sync::OnceLock;

use ort::environment::Environment;

/// Default environment shared by Qualia's optional vision adapters.
/// Applications owning an ORT environment must use the explicit-environment loaders.
pub fn shared_environment() -> Result<&'static Environment, String> {
    static ENVIRONMENT: OnceLock<Result<Environment, String>> = OnceLock::new();
    match ENVIRONMENT.get_or_init(|| {
        ort::init().build().map_err(|error| {
            format!("Qualia ORT environment initialization failed; use a caller-owned environment loader if the application already initialized ORT: {error}")
        })
    }) {
        Ok(environment) => Ok(environment),
        Err(error) => Err(error.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_vision_loads_share_one_environment_without_model_assets() {
        let first = shared_environment().expect("ORT environment");
        let second = shared_environment().expect("same ORT environment");
        assert!(std::ptr::eq(first, second));
        let first_session = ort::session::Session::builder(first).expect("first session builder");
        drop(first_session);
        let second_session =
            ort::session::Session::builder(second).expect("second session builder");
        drop(second_session);
        assert!(std::ptr::eq(first, shared_environment().unwrap()));
    }
}
