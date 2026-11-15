use super::*;
use crate::governance::consent_contract::ConsentScope;
use ed25519_dalek::SigningKey;

const ACTOR: [u8; 32] = [8; 32];
const PURPOSE: u64 = 0x1122;

fn grant() -> ConsentGrant {
    ConsentGrant::new_signed(
        [7; 32],
        &SigningKey::from_bytes(&[3; 32]),
        ACTOR,
        ConsentScope(ConsentScope::ALL),
        PURPOSE,
        10,
        100,
        1,
    )
}

fn request() -> SensingRequest {
    SensingRequest::new(
        ACTOR,
        42,
        SensingModality::Microphone,
        SensingAction::Acquire,
        PURPOSE,
    )
    .unwrap()
}

struct TestIdentity(bool);
impl IdentityAuthority for TestIdentity {
    fn verify_parties(
        &self,
        _grant: &ConsentGrant,
        _actor_id: &[u8; 32],
    ) -> Result<(), AuthorizationError> {
        if self.0 {
            Ok(())
        } else {
            Err(AuthorizationError::IdentityUnverified)
        }
    }
}

struct TestScope(bool);
impl SensingScopeAuthority for TestScope {
    fn verify_scope(
        &self,
        _grant: &ConsentGrant,
        _digest: &[u8; 32],
        _request: &SensingRequest,
    ) -> Result<(), AuthorizationError> {
        if self.0 {
            Ok(())
        } else {
            Err(AuthorizationError::UnsupportedSensorScope)
        }
    }
}

struct TestRevocation(Result<bool, AuthorizationError>);
impl RevocationAuthority for TestRevocation {
    fn is_revoked(
        &self,
        _grant_id: &[u8; 32],
        _grant_digest: &[u8; 32],
        _current_time_unix: u64,
    ) -> Result<bool, AuthorizationError> {
        self.0
    }
}

struct ExactRevocation {
    grant_id: [u8; 32],
    grant_digest: [u8; 32],
    checked_at: u64,
    revoked: bool,
}

impl RevocationAuthority for ExactRevocation {
    fn is_revoked(
        &self,
        grant_id: &[u8; 32],
        grant_digest: &[u8; 32],
        current_time_unix: u64,
    ) -> Result<bool, AuthorizationError> {
        if grant_id != &self.grant_id
            || grant_digest != &self.grant_digest
            || current_time_unix != self.checked_at
        {
            return Err(AuthorizationError::RevocationUnknown);
        }
        Ok(self.revoked)
    }
}

struct TestClock(Result<u64, AuthorizationError>);
impl TrustedUnixClock for TestClock {
    fn now_unix(&mut self) -> Result<u64, AuthorizationError> {
        self.0
    }
}

struct SharedClock<'a>(&'a core::cell::Cell<u64>);
impl TrustedUnixClock for SharedClock<'_> {
    fn now_unix(&mut self) -> Result<u64, AuthorizationError> {
        Ok(self.0.get())
    }
}

struct ScopeChange<'a> {
    time: &'a core::cell::Cell<u64>,
    revoked: &'a core::cell::Cell<bool>,
    next_time: u64,
    next_revoked: bool,
}
impl SensingScopeAuthority for ScopeChange<'_> {
    fn verify_scope(
        &self,
        _: &ConsentGrant,
        _: &[u8; 32],
        _: &SensingRequest,
    ) -> Result<(), AuthorizationError> {
        self.time.set(self.next_time);
        self.revoked.set(self.next_revoked);
        Ok(())
    }
}

struct SharedRevocation<'a>(&'a core::cell::Cell<bool>);
impl RevocationAuthority for SharedRevocation<'_> {
    fn is_revoked(&self, _: &[u8; 32], _: &[u8; 32], _: u64) -> Result<bool, AuthorizationError> {
        Ok(self.0.get())
    }
}

#[test]
fn authority_callback_expiry_or_revocation_is_rechecked_before_permit() {
    use core::cell::Cell;
    for (next_time, next_revoked, expected) in [
        (100, false, AuthorizationError::GrantExpired),
        (60, true, AuthorizationError::Revoked),
        (49, false, AuthorizationError::ClockUnavailable),
    ] {
        let time = Cell::new(50);
        let revoked = Cell::new(false);
        let scope = ScopeChange {
            time: &time,
            revoked: &revoked,
            next_time,
            next_revoked,
        };
        let result = authorize_sensing(
            &grant(),
            request(),
            &TestIdentity(true),
            &scope,
            &SharedRevocation(&revoked),
            &mut SharedClock(&time),
        );
        assert_eq!(result, Err(expected));
    }
}

struct RevocationAdvancesClock<'a>(&'a core::cell::Cell<u64>);
impl RevocationAuthority for RevocationAdvancesClock<'_> {
    fn is_revoked(&self, _: &[u8; 32], _: &[u8; 32], now: u64) -> Result<bool, AuthorizationError> {
        if now == 60 {
            self.0.set(100);
        }
        Ok(false)
    }
}

#[test]
fn final_clock_check_rejects_expiry_during_revocation_lookup() {
    let time = core::cell::Cell::new(50);
    let revoked = core::cell::Cell::new(false);
    let scope = ScopeChange {
        time: &time,
        revoked: &revoked,
        next_time: 60,
        next_revoked: false,
    };
    let result = authorize_sensing(
        &grant(),
        request(),
        &TestIdentity(true),
        &scope,
        &RevocationAdvancesClock(&time),
        &mut SharedClock(&time),
    );
    assert_eq!(result, Err(AuthorizationError::GrantExpired));
}

fn authorize(
    grant: &ConsentGrant,
    request: SensingRequest,
    identity: &TestIdentity,
    scope: &TestScope,
    revoked: Result<bool, AuthorizationError>,
    now: Result<u64, AuthorizationError>,
) -> Result<SensingPermit, AuthorizationError> {
    authorize_sensing(
        grant,
        request,
        identity,
        scope,
        &TestRevocation(revoked),
        &mut TestClock(now),
    )
}

#[test]
fn issues_request_bound_permit_only_after_every_gate() {
    let signed = grant();
    let permit = authorize(
        &signed,
        request(),
        &TestIdentity(true),
        &TestScope(true),
        Ok(false),
        Ok(50),
    )
    .unwrap();
    assert_eq!(permit.grant_id(), signed.grant_id);
    assert_eq!(permit.grant_digest(), signed.compute_digest());
    assert_eq!(permit.request(), request());
    assert_eq!(permit.issued_at_unix(), 50);
    assert_eq!(permit.expires_at_unix(), signed.expires_at);
}

#[test]
fn fails_closed_for_actor_purpose_expiry_and_unknown_revocation() {
    let signed = grant();
    let other_actor = SensingRequest::new(
        [9; 32],
        42,
        SensingModality::Microphone,
        SensingAction::Acquire,
        PURPOSE,
    )
    .unwrap();
    assert_eq!(
        authorize(
            &signed,
            other_actor,
            &TestIdentity(true),
            &TestScope(true),
            Ok(false),
            Ok(50)
        ),
        Err(AuthorizationError::ActorMismatch)
    );
    let wrong_purpose = SensingRequest::new(
        ACTOR,
        42,
        SensingModality::Microphone,
        SensingAction::Acquire,
        9,
    )
    .unwrap();
    assert_eq!(
        authorize(
            &signed,
            wrong_purpose,
            &TestIdentity(true),
            &TestScope(true),
            Ok(false),
            Ok(50)
        ),
        Err(AuthorizationError::PurposeMismatch)
    );
    assert_eq!(
        authorize(
            &signed,
            request(),
            &TestIdentity(true),
            &TestScope(true),
            Ok(false),
            Ok(100)
        ),
        Err(AuthorizationError::GrantExpired)
    );
    assert_eq!(
        authorize(
            &signed,
            request(),
            &TestIdentity(true),
            &TestScope(true),
            Err(AuthorizationError::RevocationUnknown),
            Ok(50)
        ),
        Err(AuthorizationError::RevocationUnknown)
    );
}

#[test]
fn identity_and_sensor_scope_must_be_explicitly_approved() {
    let signed = grant();
    assert_eq!(
        authorize(
            &signed,
            request(),
            &TestIdentity(false),
            &TestScope(true),
            Ok(false),
            Ok(50)
        ),
        Err(AuthorizationError::IdentityUnverified)
    );
    assert_eq!(
        authorize(
            &signed,
            request(),
            &TestIdentity(true),
            &TestScope(false),
            Ok(false),
            Ok(50)
        ),
        Err(AuthorizationError::UnsupportedSensorScope)
    );
    assert_eq!(
        authorize(
            &signed,
            request(),
            &TestIdentity(true),
            &TestScope(true),
            Ok(true),
            Ok(50)
        ),
        Err(AuthorizationError::Revoked)
    );
}

#[test]
fn invalid_identifiers_and_future_grant_fail_closed() {
    assert_eq!(
        SensingRequest::new(
            ACTOR,
            0,
            SensingModality::Wifi,
            SensingAction::Acquire,
            PURPOSE
        ),
        Err(AuthorizationError::InvalidRequest)
    );
    assert_eq!(
        authorize(
            &grant(),
            request(),
            &TestIdentity(true),
            &TestScope(true),
            Ok(false),
            Ok(9)
        ),
        Err(AuthorizationError::GrantNotYetValid)
    );
}

#[test]
fn permit_is_only_a_decision_and_revalidation_rechecks_live_revocation_and_time() {
    let signed = grant();
    let decision = authorize(
        &signed,
        request(),
        &TestIdentity(true),
        &TestScope(true),
        Ok(false),
        Ok(50),
    )
    .unwrap();
    assert_eq!(
        decision.revalidate(
            &signed,
            &TestIdentity(true),
            &TestScope(true),
            &TestRevocation(Ok(true)),
            &mut TestClock(Ok(60)),
        ),
        Err(AuthorizationError::Revoked)
    );

    let decision = authorize(
        &signed,
        request(),
        &TestIdentity(true),
        &TestScope(true),
        Ok(false),
        Ok(50),
    )
    .unwrap();
    assert_eq!(
        decision.revalidate(
            &signed,
            &TestIdentity(true),
            &TestScope(true),
            &TestRevocation(Ok(false)),
            &mut TestClock(Ok(100)),
        ),
        Err(AuthorizationError::GrantExpired)
    );
}

#[test]
fn broker_queries_revocation_by_exact_grant_digest_at_trusted_time_each_pass() {
    let signed = grant();
    let digest = signed.compute_digest();
    let decision = authorize_sensing(
        &signed,
        request(),
        &TestIdentity(true),
        &TestScope(true),
        &ExactRevocation {
            grant_id: signed.grant_id,
            grant_digest: digest,
            checked_at: 40,
            revoked: false,
        },
        &mut TestClock(Ok(40)),
    )
    .unwrap();

    let use_token = decision
        .revalidate(
            &signed,
            &TestIdentity(true),
            &TestScope(true),
            &ExactRevocation {
                grant_id: signed.grant_id,
                grant_digest: digest,
                checked_at: 60,
                revoked: false,
            },
            &mut TestClock(Ok(60)),
        )
        .unwrap();
    assert_eq!(use_token.grant_digest(), digest);
    assert_eq!(use_token.checked_at_unix(), 60);
}
