//! Admission of existing Poet GeometryHost operations, without executing a host.
use vibe::{catalog, session::{SessionLimits, SourceSession}, DiagCode};

const OPERATIONS: [&str; 4] = [
    "ComputationalGeometry.profile_sweep",
    "ComputationalGeometry.revolve_profile",
    "ComputationalGeometry.torus",
    "ComputationalGeometry.uv_sphere",
];

fn session(grants: &[&str]) -> SourceSession {
    SourceSession::new(grants.iter().map(|g| (*g).into()).collect(), SessionLimits::default())
}

fn source(operation: &str, request: bool, effect: &str) -> String {
    let target = if request {
        format!(r#", capability("{operation}")"#)
    } else {
        String::new()
    };
    format!(r#"requires [capability("capability.invoke"){target}];
        {effect} fn build() -> Record {{ return capability.invoke("{operation}", {{}}); }} build();"#)
}
#[test]
fn exact_requests_and_host_grants_admit_existing_poet_operations() {
    for operation in OPERATIONS {
        assert!(catalog::is_known(operation));
        assert!(catalog::granted_covers(&[operation], operation));
        let admitted = session(&["capability.invoke", operation]);
        assert!(admitted.prepare(&source(operation, true, "effect")).is_ok(), "{operation}");
        assert_eq!(admitted.revision(), 0); // Preparation never executes geometry.
        assert!(admitted.sources().is_empty());
    }
}

#[test]
fn family_host_grants_still_require_explicit_source_requests() {
    for operation in OPERATIONS {
        assert!(catalog::granted_covers(&["ComputationalGeometry"], operation));
        let admitted = session(&["capability.invoke", "ComputationalGeometry"]);
        assert!(admitted.prepare(&source(operation, true, "effect")).is_ok(), "{operation}");
        let denied = admitted.prepare(&source(operation, false, "effect")).err().expect("missing request");
        assert_eq!(denied.code, DiagCode::E300, "{operation}");
    }
}

#[test]
fn source_requests_cannot_supply_missing_host_grants() {
    for operation in OPERATIONS {
        let denied = session(&["capability.invoke"])
            .prepare(&source(operation, true, "effect")).err().expect("missing host grant");
        assert_eq!(denied.code, DiagCode::E300, "{operation}");
    }
}

#[test]
fn invoke_grant_cannot_be_inferred_from_a_geometry_request() {
    for operation in OPERATIONS {
        let denied = session(&[operation])
            .prepare(&source(operation, true, "effect")).err().expect("missing invoke grant");
        assert_eq!(denied.code, DiagCode::E300, "{operation}");
    }
}

#[test]
fn unknown_geometry_siblings_remain_denied_even_with_exact_host_grants() {
    let unknown = "ComputationalGeometry.unregistered_profile_sweep";
    assert!(!catalog::is_known(unknown));
    assert!(!catalog::granted_covers(&[unknown, "ComputationalGeometry"], unknown));
    let denied = session(&["capability.invoke", unknown, "ComputationalGeometry"])
        .prepare(&source(unknown, true, "effect")).err().expect("unknown sibling");
    assert_eq!(denied.code, DiagCode::E300);
}

#[test]
fn geometry_grants_do_not_relax_pure_function_effect_checks() {
    for operation in OPERATIONS {
        let denied = session(&["capability.invoke", operation])
            .prepare(&source(operation, true, "pure")).err().expect("pure function host effect");
        assert_eq!(denied.code, DiagCode::E200, "{operation}");
    }
}
