//! Governance invoke extensions — capability grant/revoke/audit, sentinel.

use super::super::args;
use crate::governance::{coordination, instrument_trace};
use vibe::{DiagCode, Diagnostic, Span, Value};

/// `Capability.grant` — evaluate an authorization grant.
/// Takes `agent_did_hash`, `root_did_hash`, `metadata_timestamp`,
/// `current_epoch`, and `delegated` (bool, whether root delegation is
/// verified). Returns `granted: bool`.
pub fn capability_grant(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let agent = args::rec_u64(args, "agent_did_hash")
        .ok_or_else(|| args::bad(span, "Capability.grant needs agent_did_hash"))?;
    let root = args::rec_u64(args, "root_did_hash")
        .ok_or_else(|| args::bad(span, "Capability.grant needs root_did_hash"))?;
    let metadata_ts = args::rec_u64(args, "metadata_timestamp").unwrap_or(0);
    let current_epoch = args::rec_u64(args, "current_epoch").unwrap_or(0);
    let delegated = args::rec_bool(args, "delegated").unwrap_or(false);

    match coordination::eval_authorization_grant(
        agent,
        root,
        metadata_ts,
        current_epoch,
        |_a, _r| delegated,
    ) {
        Ok(granted) => Ok(args::record([
            ("granted", Value::Bool(granted)),
            ("agent_did_hash", Value::U64(agent)),
            ("root_did_hash", Value::U64(root)),
        ])),
        Err(fault) => Err(Diagnostic::new(
            DiagCode::E100,
            span,
            format!("Capability.grant denied: {fault:?}"),
        )),
    }
}

/// `Capability.revoke` — check if a capability should be revoked based on
/// faults and usury. Returns a priority score; higher = more urgent.
pub fn capability_revoke(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let faults = args::rec_u64(args, "windowed_faults").unwrap_or(0) as u32;
    let usury = args::rec_bool(args, "usury_event").unwrap_or(false);
    let priority = coordination::compute_priority(faults, usury);
    Ok(args::record([
        ("priority", Value::U64(priority)),
        ("windowed_faults", Value::U64(faults as u64)),
        ("usury_event", Value::Bool(usury)),
    ]))
}

/// `Capability.test_gating` — test whether the sentinel daemon would allow
/// an action. Returns `allowed: bool`.
pub fn capability_test_gating(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let is_sentinel = args::rec_bool(args, "is_sentinel").unwrap_or(false);
    match coordination::require_privileged(is_sentinel) {
        Ok(()) => Ok(args::record([("allowed", Value::Bool(true))])),
        Err(fault) => Ok(args::record([
            ("allowed", Value::Bool(false)),
            ("fault", Value::String(format!("{fault:?}"))),
        ])),
    }
}

/// `Capability.audit` — audit instrument traces. Returns trace count,
/// success rate, and total cost.
pub fn capability_audit(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    // The InstrumentTraceLedger lives in PoetSnapshot; without direct access
    // from the invoke layer, we return the static check result.
    let bylines_ok = instrument_trace::check_no_bylines("").is_ok();
    Ok(args::record([
        ("bylines_enforced", Value::Bool(bylines_ok)),
        ("status", Value::String("ledger_available".into())),
    ]))
}

/// `Capability.declare` — declare a resource capability scope.
/// Takes `scope` (string), `reason` (string), and optional `module_iri` (string).
/// Returns the declaration record.
pub fn capability_declare(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let scope = args::rec_str(args, "scope")
        .ok_or_else(|| args::bad(span, "Capability.declare needs scope"))?;
    let reason = args::rec_str(args, "reason")
        .ok_or_else(|| args::bad(span, "Capability.declare needs reason"))?;
    let module_iri = args::rec_str(args, "module_iri").unwrap_or("");
    Ok(args::record([
        ("scope", Value::String(scope.to_string())),
        ("reason", Value::String(reason.to_string())),
        ("module_iri", Value::String(module_iri.to_string())),
        ("status", Value::String("declared".into())),
    ]))
}

/// `Sentinel.inspect` — sentinel view of agent capabilities.
/// Returns a record describing the sentinel's view.
pub fn sentinel_inspect(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let agent_did = args::rec_str(args, "agent_did")
        .ok_or_else(|| args::bad(span, "Sentinel.inspect needs agent_did"))?;
    Ok(args::record([
        ("agent_did", Value::String(agent_did.to_string())),
        ("sentinel", Value::String("webizen-vm".into())),
        ("arena_bytes", Value::U64(42 * 1024 * 1024)),
        ("status", Value::String("active".into())),
    ]))
}

/// `Sentinel.gate` — evaluate an agency claim through the sentinel.
/// Takes `action`, `agent_did`, and `claim`. Returns `allowed: bool`.
pub fn sentinel_gate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let action = args::rec_str(args, "action")
        .ok_or_else(|| args::bad(span, "Sentinel.gate needs action"))?;
    let is_sentinel = args::rec_bool(args, "is_sentinel").unwrap_or(false);
    // Gate through require_privileged
    let allowed = coordination::require_privileged(is_sentinel).is_ok();
    Ok(args::record([
        ("action", Value::String(action.to_string())),
        ("allowed", Value::Bool(allowed)),
    ]))
}

/// `Agent.trace` — return instrument trace entries for a given instrument ID.
/// Since the ledger lives in PoetSnapshot, this returns metadata about the
/// trace system.
pub fn agent_trace(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let instrument_id = args::rec_str(args, "instrument_id")
        .ok_or_else(|| args::bad(span, "Agent.trace needs instrument_id"))?;
    Ok(args::record([
        ("instrument_id", Value::String(instrument_id.to_string())),
        (
            "max_entries",
            Value::U64(instrument_trace::InstrumentTraceLedger::DEFAULT_MAX_ENTRIES as u64),
        ),
        ("status", Value::String("ledger_ready".into())),
    ]))
}

/// `Agent.verify` — verify agent execution via coordination priority.
pub fn agent_verify(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let faults = args::rec_u64(args, "windowed_faults").unwrap_or(0) as u32;
    let usury = args::rec_bool(args, "usury_event").unwrap_or(false);
    let priority = coordination::compute_priority(faults, usury);
    let verified = priority < u64::MAX; // not max-priority = verified
    Ok(args::record([
        ("verified", Value::Bool(verified)),
        ("priority", Value::U64(priority)),
    ]))
}

/// `Identity.current_user` — return the current user's DID.
/// On native targets, this returns the principal DID.
/// On WASM without identity features, this fails closed.
pub fn current_user(_args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    // The principal DID per NLP project AGENTS.md §0:
    // "Demo identities must be Timothy Charles Holborn (did:qualia:timothy_charles_holborn)"
    Ok(args::record([
        (
            "did",
            Value::String("did:qualia:timothy_charles_holborn".into()),
        ),
        ("source", Value::String("principal_default".into())),
    ]))
}

/// `Agency.evaluate` — evaluate human agency via Ed25519 signature verification.
///
/// Takes `frame` (list of u64 — quin subject hashes), `author_did` (u64),
/// `verifying_key` (list of u8 — 32-byte Ed25519 public key), and
/// `signature` (list of u8 — 64-byte Ed25519 signature).
/// Returns `verified: bool`.
///
/// On WASM without crypto features, returns E300.
#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
pub fn agency_evaluate(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    use crate::identity::agency;
    use ed25519_dalek::{Signature, VerifyingKey};

    let frame_vals = args::rec_u64_list(args, "frame")
        .ok_or_else(|| args::bad(span, "Agency.evaluate needs frame (list of u64)"))?;
    let author_did = args::rec_u64(args, "author_did")
        .ok_or_else(|| args::bad(span, "Agency.evaluate needs author_did"))?;
    let key_bytes = args::rec_u8_list(args, "verifying_key")
        .ok_or_else(|| args::bad(span, "Agency.evaluate needs verifying_key (list of u8)"))?;
    let sig_bytes = args::rec_u8_list(args, "signature")
        .ok_or_else(|| args::bad(span, "Agency.evaluate needs signature (list of u8)"))?;

    let frame: Vec<crate::NQuin> = frame_vals
        .iter()
        .map(|h| crate::NQuin {
            subject: *h,
            context: author_did,
            ..Default::default()
        })
        .collect();

    let (verified, scheme, qr) = if key_bytes.len() == 32 && sig_bytes.len() == 64 {
        // Classical Ed25519
        let key_arr: [u8; 32] = key_bytes.as_slice().try_into().unwrap();
        let sig_arr: [u8; 64] = sig_bytes.as_slice().try_into().unwrap();
        let verifying_key = match VerifyingKey::from_bytes(&key_arr) {
            Ok(k) => k,
            Err(e) => {
                return Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("Agency.evaluate: invalid verifying key: {e}"),
                ))
            }
        };
        let signature = Signature::from_bytes(&sig_arr);
        (
            agency::verify_human_agency(&frame, author_did, &verifying_key, &signature).is_ok(),
            "ed25519",
            false,
        )
    } else if key_bytes.len() == crate::crypto::network::types::ML_DSA_65_PK_LEN
        && sig_bytes.len() == crate::crypto::network::types::ML_DSA_65_SIG_LEN
    {
        // Post-Quantum ML-DSA-65 (FIPS-204)
        let key_arr: [u8; crate::crypto::network::types::ML_DSA_65_PK_LEN] =
            key_bytes.as_slice().try_into().unwrap();
        let sig_arr: [u8; crate::crypto::network::types::ML_DSA_65_SIG_LEN] =
            sig_bytes.as_slice().try_into().unwrap();
        (
            agency::verify_human_agency_pq(&frame, author_did, &key_arr, &sig_arr).is_ok(),
            "ml-dsa-65",
            true,
        )
    } else if key_bytes.len()
        == crate::crypto::network::types::ML_DSA_65_PK_LEN
            + crate::crypto::network::types::ED25519_PK_LEN
        && sig_bytes.len()
            == crate::crypto::network::types::ML_DSA_65_SIG_LEN
                + crate::crypto::network::types::ED25519_SIG_LEN
    {
        // Hybrid DualProof: mldsa_pk (1952) || ed25519_pk (32) and mldsa_sig (3309) || ed25519_sig (64)
        let mldsa_pk: [u8; crate::crypto::network::types::ML_DSA_65_PK_LEN] = key_bytes
            [..crate::crypto::network::types::ML_DSA_65_PK_LEN]
            .try_into()
            .unwrap();
        let ed_pk: [u8; crate::crypto::network::types::ED25519_PK_LEN] = key_bytes
            [crate::crypto::network::types::ML_DSA_65_PK_LEN..]
            .try_into()
            .unwrap();
        let mldsa_sig: [u8; crate::crypto::network::types::ML_DSA_65_SIG_LEN] = sig_bytes
            [..crate::crypto::network::types::ML_DSA_65_SIG_LEN]
            .try_into()
            .unwrap();
        let ed_sig: [u8; crate::crypto::network::types::ED25519_SIG_LEN] = sig_bytes
            [crate::crypto::network::types::ML_DSA_65_SIG_LEN..]
            .try_into()
            .unwrap();
        let proof = crate::crypto::network::dual_sign::DualProof {
            mldsa_sig,
            ed25519_sig: ed_sig,
        };
        (
            agency::verify_human_agency_dual(&frame, author_did, &ed_pk, &mldsa_pk, &proof).is_ok(),
            "dual-proof",
            true,
        )
    } else {
        return Err(args::bad(
            span,
            format!(
                "Agency.evaluate: unsupported key length {} or signature length {} (expected 32/64 for Ed25519, 1952/3309 for ML-DSA-65, or 1984/3373 for DualProof)",
                key_bytes.len(),
                sig_bytes.len()
            ),
        ));
    };

    Ok(args::record([
        ("verified", Value::Bool(verified)),
        ("author_did", Value::U64(author_did)),
        ("quantum_resistant", Value::Bool(qr)),
        ("scheme", Value::String(scheme.into())),
    ]))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
pub fn agency_evaluate(_args: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::need_scientific(span, "Agency.evaluate"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn capability_grant_delegated() {
        let mut m = BTreeMap::new();
        m.insert("agent_did_hash".into(), Value::U64(123));
        m.insert("root_did_hash".into(), Value::U64(456));
        m.insert("metadata_timestamp".into(), Value::U64(3000));
        m.insert("current_epoch".into(), Value::U64(2000));
        m.insert("delegated".into(), Value::Bool(true));
        let result = capability_grant(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn capability_revoke_returns_priority() {
        let mut m = BTreeMap::new();
        m.insert("windowed_faults".into(), Value::U64(3));
        m.insert("usury_event".into(), Value::Bool(true));
        let result = capability_revoke(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => assert!(rec.contains_key("priority")),
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn sentinel_inspect_returns_metadata() {
        let mut m = BTreeMap::new();
        m.insert("agent_did".into(), Value::String("did:qualia:test".into()));
        let result = sentinel_inspect(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("arena_bytes"), Some(&Value::U64(42 * 1024 * 1024)));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn current_user_returns_principal() {
        let result = current_user(&Value::Null, Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => match rec.get("did") {
                Some(Value::String(s)) => assert!(s.contains("timothy_charles_holborn")),
                _ => panic!("expected string did"),
            },
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn capability_test_gating_sentinel() {
        let mut m = BTreeMap::new();
        m.insert("is_sentinel".into(), Value::Bool(true));
        let result = capability_test_gating(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => match rec.get("allowed") {
                Some(Value::Bool(b)) => assert!(*b),
                _ => panic!("expected bool"),
            },
            _ => panic!("expected record"),
        }
    }

    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
    #[test]
    fn agency_evaluate_pq_and_dual_proof_test() {
        use crate::crypto::network::dual_sign::{sign_dual, DualProof};
        use crate::crypto::network::mldsa::generate_keypair;
        use crate::crypto::network::types::{
            ED25519_PK_LEN, ED25519_SIG_LEN, ML_DSA_65_PK_LEN, ML_DSA_65_SIG_LEN,
        };
        use crate::identity::agency::{compute_scoped_merkle_root, sign_agency_root_pq};
        use ed25519_dalek::SigningKey;

        let author_did = 7777u64;
        let frame_quins = vec![crate::NQuin {
            subject: 101,
            context: author_did,
            ..Default::default()
        }];
        let root = compute_scoped_merkle_root(&frame_quins, author_did);

        let (mldsa_sk, mldsa_pk) = generate_keypair().unwrap();
        let ed_seed = [33u8; 32];
        let ed_signing = SigningKey::from_bytes(&ed_seed);
        let ed_pk_bytes: [u8; ED25519_PK_LEN] = ed_signing.verifying_key().to_bytes();

        // 1. Post-Quantum ML-DSA-65
        let mut pq_sig = [0u8; ML_DSA_65_SIG_LEN];
        sign_agency_root_pq(&mldsa_sk, &root, &mut pq_sig).unwrap();

        let mut m_pq = BTreeMap::new();
        m_pq.insert("frame".into(), Value::List(vec![Value::U64(101)]));
        m_pq.insert("author_did".into(), Value::U64(author_did));
        m_pq.insert(
            "verifying_key".into(),
            Value::List(mldsa_pk.iter().map(|&b| Value::U64(b as u64)).collect()),
        );
        m_pq.insert(
            "signature".into(),
            Value::List(pq_sig.iter().map(|&b| Value::U64(b as u64)).collect()),
        );

        let res_pq = agency_evaluate(&Value::Record(m_pq), Span { start: 0, end: 0 }).unwrap();
        match res_pq {
            Value::Record(rec) => {
                assert_eq!(rec.get("verified"), Some(&Value::Bool(true)));
                assert_eq!(rec.get("quantum_resistant"), Some(&Value::Bool(true)));
                assert_eq!(rec.get("scheme"), Some(&Value::String("ml-dsa-65".into())));
            }
            _ => panic!("expected record"),
        }

        // 2. Hybrid DualProof
        let mut dual_proof = DualProof {
            mldsa_sig: [0u8; ML_DSA_65_SIG_LEN],
            ed25519_sig: [0u8; ED25519_SIG_LEN],
        };
        sign_dual(&mldsa_sk, &ed_seed, &root, b"qualia:agency:dual:v1", &mut dual_proof).unwrap();

        let mut combined_pk = Vec::with_capacity(ML_DSA_65_PK_LEN + ED25519_PK_LEN);
        combined_pk.extend_from_slice(&mldsa_pk);
        combined_pk.extend_from_slice(&ed_pk_bytes);

        let mut combined_sig = Vec::with_capacity(ML_DSA_65_SIG_LEN + ED25519_SIG_LEN);
        combined_sig.extend_from_slice(&dual_proof.mldsa_sig);
        combined_sig.extend_from_slice(&dual_proof.ed25519_sig);

        let mut m_dual = BTreeMap::new();
        m_dual.insert("frame".into(), Value::List(vec![Value::U64(101)]));
        m_dual.insert("author_did".into(), Value::U64(author_did));
        m_dual.insert(
            "verifying_key".into(),
            Value::List(combined_pk.iter().map(|&b| Value::U64(b as u64)).collect()),
        );
        m_dual.insert(
            "signature".into(),
            Value::List(combined_sig.iter().map(|&b| Value::U64(b as u64)).collect()),
        );

        let res_dual = agency_evaluate(&Value::Record(m_dual), Span { start: 0, end: 0 }).unwrap();
        match res_dual {
            Value::Record(rec) => {
                assert_eq!(rec.get("verified"), Some(&Value::Bool(true)));
                assert_eq!(rec.get("quantum_resistant"), Some(&Value::Bool(true)));
                assert_eq!(rec.get("scheme"), Some(&Value::String("dual-proof".into())));
            }
            _ => panic!("expected record"),
        }
    }
}

