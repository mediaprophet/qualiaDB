//! Privileged native policy persistence. One host instance owns a configured
//! policy store; all its admissions share liveness and generation state.
//! A separate trusted authority directory anchors the exact accepted record.
//! Anchor-first publication is fail-closed, not a two-file atomic transaction:
//! interruption leaves diagnostic bytes and requires recovery, never stale use.
//! Rolling back the entire trusted host storage remains outside this assurance.
use super::{
    anchor::{
        atomic_json_write, read_bounded, validate_json_size, AuthorityAnchor, RecordIdentity,
    },
    controller::{AuthorityState, InstalledController},
    policy_digest,
    signature::{signature_parts, verify_signature},
    GovernedTokenSession, InferenceAdmission, PolicyDocument, PolicyError, PolicyRules,
    SignedPolicy, TrustedPolicyIdentity, MAX_POLICY_JSON_BYTES, POLICY_COLD_BUDGET_BYTES,
    POLICY_VERSION,
};
use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{atomic::Ordering, Arc},
};

const FILE_NAME: &str = "sentinel-inference-policy.json";

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    serial: u64,
    generation: u64,
    next_session: u64,
    active: Option<SignedPolicy>,
    signature: [[u8; 32]; 2],
}
impl Record {
    fn empty() -> Self {
        Self {
            version: 1,
            serial: 0,
            generation: 0,
            next_session: 0,
            active: None,
            signature: [[0; 32]; 2],
        }
    }
    fn digest(&self, trust: &TrustedPolicyIdentity) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(b"qualia:sentinel:policy-store:v1\0");
        h.update(self.version.to_le_bytes());
        h.update(trust.key);
        for v in [
            trust.principal,
            trust.audience,
            trust.controller_generation,
            self.serial,
            self.generation,
            self.next_session,
        ] {
            h.update(v.to_le_bytes());
        }
        match &self.active {
            None => h.update([0]),
            Some(policy) => {
                h.update([1]);
                h.update(policy_digest(&policy.document));
                h.update(policy.signature[0]);
                h.update(policy.signature[1]);
            }
        }
        h.finalize().into()
    }
    fn sign(&mut self, trust: &TrustedPolicyIdentity, key: &SigningKey) {
        self.signature = signature_parts(key.sign(&self.digest(trust)).to_bytes());
    }
    fn verify(&self, trust: &TrustedPolicyIdentity) -> Result<(), PolicyError> {
        if self.version != 1 || self.serial == 0 || self.generation == 0 {
            return Err(PolicyError::InvalidPolicy);
        }
        verify_signature(trust.key, self.digest(trust), self.signature)?;
        if let Some(policy) = &self.active {
            if policy.document.policy_generation != self.generation {
                return Err(PolicyError::StaleGeneration);
            }
            // Keep expired installations loadable for explicit replacement or
            // revocation; session admission separately checks the current time.
            policy.verify(trust, policy.document.not_before)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PolicyStatus {
    pub installed: bool,
    pub generation: u64,
    pub serial: u64,
    pub digest: Option<[u8; 32]>,
}

pub struct NativePolicyHost {
    root: PathBuf,
    trust: TrustedPolicyIdentity,
    record: Record,
    authority: Arc<AuthorityState>,
    controller: Option<Arc<InstalledController>>,
    anchor: AuthorityAnchor,
    // Persistent lockfile is never removed: unlinking would permit split locks.
    _lock: fs::File,
    #[cfg(test)]
    fault: Option<PublishFault>,
}
#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PublishFault {
    BeforeAnchor,
    AfterAnchor,
    AfterPolicy,
    PanicAfterAnchor,
}
impl Drop for NativePolicyHost {
    fn drop(&mut self) {
        self.authority.disconnect();
    }
}
impl NativePolicyHost {
    /// Only the authenticated host supplies directory and trusted identity.
    /// Do not construct a fresh host for every request: replacement/revocation
    /// must invalidate the shared handles owned by this instance.
    pub fn open(
        root: &Path,
        authority_root: &Path,
        trust: TrustedPolicyIdentity,
        budget: usize,
    ) -> Result<Self, PolicyError> {
        check_budget(budget)?;
        if root.as_os_str().len() > 4096 || authority_root.as_os_str().len() > 4096 {
            return Err(PolicyError::Capacity);
        }
        fs::create_dir_all(root).map_err(|_| PolicyError::Io)?;
        let root = fs::canonicalize(root).map_err(|_| PolicyError::Io)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("sentinel-inference-policy.lock"))
            .map_err(|_| PolicyError::Io)?;
        lock.try_lock().map_err(|_| PolicyError::Io)?;
        fs::create_dir_all(authority_root).map_err(|_| PolicyError::Io)?;
        let authority_root = fs::canonicalize(authority_root).map_err(|_| PolicyError::Io)?;
        let anchor = AuthorityAnchor::new(&root, &authority_root)?;
        if !root.join(FILE_NAME).exists() && anchor.exists() {
            return Err(PolicyError::StaleGeneration);
        }
        let mut host = Self {
            root,
            trust,
            record: Record::empty(),
            authority: Arc::new(AuthorityState::new(0)),
            controller: None,
            anchor,
            _lock: lock,
            #[cfg(test)]
            fault: None,
        };
        if host.root.join(FILE_NAME).exists() {
            host.reload(budget)?;
        }
        Ok(host)
    }
    pub fn status(&self) -> PolicyStatus {
        PolicyStatus {
            installed: self.record.active.is_some(),
            generation: self.record.generation,
            serial: self.record.serial,
            digest: self
                .record
                .active
                .as_ref()
                .map(|p| policy_digest(&p.document)),
        }
    }
    pub fn trusted_identity(&self) -> TrustedPolicyIdentity {
        self.trust
    }
    pub fn disconnect(&self) {
        self.authority.disconnect();
    }
    #[cfg(test)]
    pub(super) fn fail_next_publication(&mut self, fault: PublishFault) {
        self.fault = Some(fault);
    }
    pub fn reload(&mut self, budget: usize) -> Result<(), PolicyError> {
        check_budget(budget)?;
        let result = self.read_record();
        match result {
            Ok(record) => {
                if record.serial < self.record.serial
                    || record.generation < self.record.generation
                    || record.next_session < self.record.next_session
                {
                    self.disconnect();
                    return Err(PolicyError::StaleGeneration);
                }
                self.publish(record)
            }
            Err(error) => {
                self.disconnect();
                Err(error)
            }
        }
    }
    /// Explicit privileged human/host installation. Rules contain no principal
    /// or signing key: these come from configured trust. The caller must have
    /// authorized this operation; never expose it as an unauthenticated RPC.
    pub fn install(
        &mut self,
        key: &SigningKey,
        rules: PolicyRules,
        not_before: u64,
        expires_at: u64,
        now: u64,
        expected_generation: u64,
        budget: usize,
    ) -> Result<PolicyStatus, PolicyError> {
        check_budget(budget)?;
        self.check_key(key)?;
        self.check_generation(expected_generation)?;
        let generation = self
            .record
            .generation
            .checked_add(1)
            .ok_or(PolicyError::CounterExhausted)?;
        let doc = PolicyDocument {
            version: POLICY_VERSION,
            audience: self.trust.audience,
            principal: self.trust.principal,
            controller_generation: self.trust.controller_generation,
            policy_generation: generation,
            not_before,
            expires_at,
            rules,
        };
        let policy = SignedPolicy::sign(doc, key)?;
        policy.verify(&self.trust, now)?;
        let mut next = self.next_record()?;
        next.generation = generation;
        next.active = Some(policy);
        self.persist(&mut next, key)?;
        self.publish(next)?;
        Ok(self.status())
    }
    pub fn revoke(
        &mut self,
        key: &SigningKey,
        expected_generation: u64,
        budget: usize,
    ) -> Result<PolicyStatus, PolicyError> {
        check_budget(budget)?;
        self.check_key(key)?;
        self.check_generation(expected_generation)?;
        let mut next = self.next_record()?;
        next.generation = next
            .generation
            .checked_add(1)
            .ok_or(PolicyError::CounterExhausted)?;
        next.active = None;
        self.persist(&mut next, key)?;
        self.publish(next)?;
        Ok(self.status())
    }
    /// Cold admission reserves a durable, never-reused session number before
    /// returning a session. Signature verification and disk I/O are not hot-path work.
    pub fn admit(
        &mut self,
        key: &SigningKey,
        request: InferenceAdmission,
        budget: usize,
        owner: Arc<crate::poet_host::native_authority::NativeAuthority>,
        association: crate::poet_host::native_authority::inference::AuthenticatedInferenceAssociation,
    ) -> Result<GovernedTokenSession, PolicyError> {
        self.admit_clock(key, request, None, budget, owner, association)
    }
    #[cfg(test)]
    pub(super) fn admit_at(
        &mut self,
        key: &SigningKey,
        request: InferenceAdmission,
        now: u64,
        budget: usize,
        owner: Arc<crate::poet_host::native_authority::NativeAuthority>,
        association: crate::poet_host::native_authority::inference::AuthenticatedInferenceAssociation,
    ) -> Result<GovernedTokenSession, PolicyError> {
        self.admit_clock(key, request, Some(now), budget, owner, association)
    }
    fn admit_clock(
        &mut self,
        key: &SigningKey,
        request: InferenceAdmission,
        clock_override: Option<u64>,
        budget: usize,
        owner: Arc<crate::poet_host::native_authority::NativeAuthority>,
        association: crate::poet_host::native_authority::inference::AuthenticatedInferenceAssociation,
    ) -> Result<GovernedTokenSession, PolicyError> {
        check_budget(budget)?;
        self.check_key(key)?;
        let controller = self.controller.as_ref().ok_or(PolicyError::MissingPolicy)?;
        let session_id = self
            .record
            .next_session
            .checked_add(1)
            .ok_or(PolicyError::CounterExhausted)?;
        let session = GovernedTokenSession::admit(
            Arc::clone(controller),
            request,
            session_id,
            clock_override,
            owner,
            association,
        )?;
        let mut next = self.next_record()?;
        next.next_session = session_id;
        self.persist(&mut next, key)?;
        // Session-number bookkeeping does not replace or reconnect authority.
        self.record = next;
        Ok(session)
    }
    fn check_key(&self, key: &SigningKey) -> Result<(), PolicyError> {
        if key.verifying_key().to_bytes() != self.trust.key {
            Err(PolicyError::WrongAuthority)
        } else {
            Ok(())
        }
    }
    fn check_generation(&self, expected: u64) -> Result<(), PolicyError> {
        if expected != self.record.generation {
            Err(PolicyError::StaleGeneration)
        } else {
            Ok(())
        }
    }
    fn next_record(&self) -> Result<Record, PolicyError> {
        let mut next = self.record.clone();
        next.serial = next
            .serial
            .checked_add(1)
            .ok_or(PolicyError::CounterExhausted)?;
        Ok(next)
    }
    fn publish(&mut self, record: Record) -> Result<(), PolicyError> {
        let _gate = self.authority.lock()?;
        let epoch = match self
            .authority
            .generation
            .load(Ordering::Acquire)
            .checked_add(1)
        {
            Some(epoch) => epoch,
            None => {
                self.authority.connected.store(false, Ordering::Release);
                return Err(PolicyError::CounterExhausted);
            }
        };
        self.authority.generation.store(epoch, Ordering::Release);
        self.controller = record.active.as_ref().map(|policy| {
            Arc::new(InstalledController {
                policy: policy.clone(),
                key: self.trust.key,
                authority: Arc::clone(&self.authority),
                epoch,
            })
        });
        self.record = record;
        self.authority.connected.store(true, Ordering::Release);
        Ok(())
    }
    fn read_record(&self) -> Result<Record, PolicyError> {
        let bytes = read_bounded(&self.root.join(FILE_NAME), MAX_POLICY_JSON_BYTES)?;
        let record: Record = serde_json::from_slice(&bytes).map_err(|_| PolicyError::Encoding)?;
        record.verify(&self.trust)?;
        self.anchor.verify(
            RecordIdentity {
                serial: record.serial,
                generation: record.generation,
                digest: record.digest(&self.trust),
            },
            &self.trust,
        )?;
        Ok(record)
    }
    fn persist(&mut self, record: &mut Record, key: &SigningKey) -> Result<(), PolicyError> {
        record.sign(&self.trust, key);
        validate_json_size(record, MAX_POLICY_JSON_BYTES)?;
        let current = if self.record.serial == 0 {
            if self.root.join(FILE_NAME).exists() || self.anchor.exists() {
                Err(PolicyError::StaleGeneration)
            } else {
                Ok(())
            }
        } else {
            self.read_record().and_then(|disk| {
                if disk.digest(&self.trust) == self.record.digest(&self.trust) {
                    Ok(())
                } else {
                    Err(PolicyError::StaleGeneration)
                }
            })
        };
        if let Err(error) = current {
            self.disconnect();
            return Err(error);
        }
        #[cfg(test)]
        let fault = self.fault.take();
        #[cfg(test)]
        if fault == Some(PublishFault::BeforeAnchor) {
            return Err(PolicyError::Io);
        }
        self.anchor.publish(
            RecordIdentity {
                serial: record.serial,
                generation: record.generation,
                digest: record.digest(&self.trust),
            },
            &self.trust,
            key,
        )?;
        // Any failure or unwind after anchor replacement invalidates live
        // admissions immediately, even if the caller catches the error/panic.
        let mut guard = PublicationGuard {
            authority: &self.authority,
            complete: false,
        };
        #[cfg(test)]
        match fault {
            Some(PublishFault::AfterAnchor) => return Err(PolicyError::Io),
            Some(PublishFault::PanicAfterAnchor) => panic!("injected policy publication unwind"),
            _ => {}
        }
        atomic_json_write(&self.root.join(FILE_NAME), record, MAX_POLICY_JSON_BYTES)?;
        #[cfg(test)]
        if fault == Some(PublishFault::AfterPolicy) {
            return Err(PolicyError::Io);
        }
        guard.complete = true;
        Ok(())
    }
}
fn check_budget(budget: usize) -> Result<(), PolicyError> {
    if budget < POLICY_COLD_BUDGET_BYTES {
        Err(PolicyError::Budget)
    } else {
        Ok(())
    }
}
struct PublicationGuard<'a> {
    authority: &'a AuthorityState,
    complete: bool,
}
impl Drop for PublicationGuard<'_> {
    fn drop(&mut self) {
        if !self.complete {
            self.authority.disconnect();
        }
    }
}
