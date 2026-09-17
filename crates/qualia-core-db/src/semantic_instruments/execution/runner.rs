//! SI-06 generic demo runner.
//!
//! Dispatch is by entry-point name (`assess` / `recognise` only). This is a
//! labelled identity-transform demo, not a clinical or Host kernel.

use super::receipt::ExecutionReceipt;
use crate::semantic_instruments::errors::InstrumentError;
use crate::semantic_instruments::resolve::{InstalledRelease, LocalRegistry};

/// Demo entry points only. Anything else is [`InstrumentError::UnknownEntryPoint`].
const DEMO_ENTRY_POINTS: &[&str] = &["assess", "recognise"];

pub struct RunRequest<'a> {
    pub release_id: &'a str,
    pub entry_point: &'a str,
    pub input: &'a [u8],
    pub now: u32,
    pub cancelled: bool,
    pub max_input_bytes: usize,
    pub max_steps: u32,
    pub parent_receipt: Option<String>,
}

pub enum RunOutcome {
    Completed { receipt: ExecutionReceipt },
    Held { receipt: ExecutionReceipt },
    Refused { error: InstrumentError },
}

/// Activate a closed install. Open records fail [`InstrumentError::NotClosed`].
pub fn preflight<'a>(
    registry: &'a LocalRegistry,
    req: &RunRequest<'_>,
) -> Result<&'a InstalledRelease, InstrumentError> {
    registry.activate(req.release_id)
}

/// Run a preflighted install. Empty input is held; unknown entries are refused.
pub fn run_entry(
    registry: &LocalRegistry,
    installed: &InstalledRelease,
    req: &RunRequest<'_>,
) -> RunOutcome {
    let _ = req.now;
    if let Err(error) = registry.activate(req.release_id) {
        return RunOutcome::Refused { error };
    }
    if req.cancelled {
        return RunOutcome::Refused {
            error: InstrumentError::Cancelled,
        };
    }
    if !DEMO_ENTRY_POINTS.contains(&req.entry_point) {
        return RunOutcome::Refused {
            error: InstrumentError::UnknownEntryPoint,
        };
    }
    if req.max_steps == 0 || req.input.len() > req.max_input_bytes {
        return RunOutcome::Refused {
            error: InstrumentError::ResourceLimit,
        };
    }
    if req.input.is_empty() {
        return RunOutcome::Held {
            receipt: bind_receipt(installed, req, &[], "held"),
        };
    }
    // Labelled demo: identity transform so callers can check I/O commitments.
    RunOutcome::Completed {
        receipt: bind_receipt(installed, req, req.input, "completed"),
    }
}

fn bind_receipt(
    installed: &InstalledRelease,
    req: &RunRequest<'_>,
    output: &[u8],
    outcome: &str,
) -> ExecutionReceipt {
    ExecutionReceipt::bind(
        installed.release_id.as_str(),
        installed.content_digest.as_str(),
        &installed.lock.digest(),
        req.entry_point,
        req.input,
        output,
        req.parent_receipt.clone(),
        outcome,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_instruments::canonical::sha256_prefixed;
    use crate::semantic_instruments::resolve::{DependencyLock, InstalledRelease, LocalRegistry};

    const RELEASE: &str = "https://ns.webizen.org/demo/unit-convert/releases/1.0.0";
    const DIGEST: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn empty_lock(release_id: &str) -> DependencyLock {
        DependencyLock {
            release_id: release_id.into(),
            entries: Vec::new(),
        }
    }

    fn demo_install(closed: bool) -> InstalledRelease {
        InstalledRelease {
            release_id: RELEASE.into(),
            content_digest: DIGEST.into(),
            lock: empty_lock(RELEASE),
            closed,
        }
    }

    fn request<'a>(entry_point: &'a str, input: &'a [u8]) -> RunRequest<'a> {
        RunRequest {
            release_id: RELEASE,
            entry_point,
            input,
            now: 1,
            cancelled: false,
            max_input_bytes: 64,
            max_steps: 8,
            parent_receipt: None,
        }
    }

    fn closed_registry() -> LocalRegistry {
        let mut registry = LocalRegistry::new();
        registry.install(demo_install(true)).unwrap();
        registry
    }

    fn completed(outcome: RunOutcome) -> ExecutionReceipt {
        match outcome {
            RunOutcome::Completed { receipt } => receipt,
            _ => panic!("expected Completed"),
        }
    }

    fn held(outcome: RunOutcome) -> ExecutionReceipt {
        match outcome {
            RunOutcome::Held { receipt } => receipt,
            _ => panic!("expected Held"),
        }
    }

    fn refused(outcome: RunOutcome) -> InstrumentError {
        match outcome {
            RunOutcome::Refused { error } => error,
            _ => panic!("expected Refused"),
        }
    }

    #[test]
    fn closed_assess_identity_completes() {
        let registry = closed_registry();
        let req = request("assess", b"demo-ok");
        let installed = preflight(&registry, &req).unwrap();
        let receipt = completed(run_entry(&registry, installed, &req));
        assert_eq!(receipt.release_id, RELEASE);
        assert_eq!(receipt.content_digest, DIGEST);
        assert_eq!(receipt.lock_digest, installed.lock.digest());
        assert_eq!(receipt.entry_point, "assess");
        assert_eq!(receipt.outcome, "completed");
        assert_eq!(receipt.input_commitment, sha256_prefixed(b"demo-ok"));
        assert_eq!(receipt.output_commitment, sha256_prefixed(b"demo-ok"));
        assert!(receipt.parent_receipt.is_none());
    }

    #[test]
    fn empty_input_is_held() {
        let registry = closed_registry();
        let req = request("recognise", b"");
        let installed = preflight(&registry, &req).unwrap();
        let receipt = held(run_entry(&registry, installed, &req));
        assert_eq!(receipt.outcome, "held");
        assert_eq!(receipt.input_commitment, sha256_prefixed(b""));
        assert_eq!(receipt.output_commitment, sha256_prefixed(b""));
        assert_eq!(receipt.entry_point, "recognise");
    }

    #[test]
    fn unknown_host_entry_is_refused() {
        let registry = closed_registry();
        let req = request("Host.ClinicalRisk.framingham", b"demo-ok");
        let installed = preflight(&registry, &req).unwrap();
        assert_eq!(
            refused(run_entry(&registry, installed, &req)),
            InstrumentError::UnknownEntryPoint
        );
    }

    #[test]
    fn cancelled_is_refused() {
        let registry = closed_registry();
        let mut req = request("assess", b"demo-ok");
        req.cancelled = true;
        let installed = preflight(&registry, &req).unwrap();
        assert_eq!(
            refused(run_entry(&registry, installed, &req)),
            InstrumentError::Cancelled
        );
    }

    #[test]
    fn open_install_preflight_is_not_closed() {
        let mut registry = LocalRegistry::new();
        registry.install(demo_install(false)).unwrap();
        let req = request("assess", b"demo-ok");
        assert_eq!(preflight(&registry, &req), Err(InstrumentError::NotClosed));
    }

    #[test]
    fn huge_input_is_resource_limit() {
        let registry = closed_registry();
        let mut req = request("assess", b"demo-ok");
        req.max_input_bytes = 3;
        let installed = preflight(&registry, &req).unwrap();
        assert_eq!(
            refused(run_entry(&registry, installed, &req)),
            InstrumentError::ResourceLimit
        );
    }

    #[test]
    fn zero_steps_is_resource_limit() {
        let registry = closed_registry();
        let mut req = request("assess", b"demo-ok");
        req.max_steps = 0;
        let installed = preflight(&registry, &req).unwrap();
        assert_eq!(
            refused(run_entry(&registry, installed, &req)),
            InstrumentError::ResourceLimit
        );
    }
}
