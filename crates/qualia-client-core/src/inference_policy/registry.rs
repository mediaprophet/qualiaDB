//! Bounded weak owner reuse; controller loss does not orphan a locked journal.
use super::{
    configuration::{self, InferenceHostConfiguration},
    store,
};
use qualia_cooperative_core::authoring::DocumentRevision;
use qualia_core_db::poet_host::native_authority::{
    admission_digest, inference::AuthenticatedInferenceAssociation, NativeAuthority, SessionHandle,
    SignedAdmission,
};
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex, OnceLock, Weak,
    },
};
const CAPACITY: usize = 16;
static EPOCH: AtomicU64 = AtomicU64::new(1);
// Fixed tracking includes opaque bindings, bounded by 16 owners x 16 sessions.
// This cold registry reservation is covered by CONFIGURATION_BUDGET_BYTES.
struct TrackedSession {
    handle: SessionHandle,
    association: AuthenticatedInferenceAssociation,
}
struct Entry {
    root: PathBuf,
    anchor: [u8; 32],
    document: DocumentRevision,
    owner: Weak<NativeAuthority>,
    sessions: [Option<TrackedSession>; 16],
}
static OWNERS: OnceLock<Mutex<[Option<Entry>; CAPACITY]>> = OnceLock::new();
fn owners() -> &'static Mutex<[Option<Entry>; CAPACITY]> {
    OWNERS.get_or_init(|| Mutex::new(std::array::from_fn(|_| None)))
}
fn anchor(c: &InferenceHostConfiguration) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"qualia:inference:project-owner:v1\0");
    for s in [&c.project_id, &c.project_issuer] {
        h.update((s.len() as u64).to_le_bytes());
        h.update(s.as_bytes());
    }
    h.update(c.project_public_key);
    h.finalize().into()
}
pub(super) fn epoch() -> Result<u64, String> {
    let e = EPOCH.load(Ordering::Acquire);
    if e == u64::MAX {
        Err("Inference actor epoch exhausted".into())
    } else {
        Ok(e)
    }
}
pub(super) fn preflight_epoch() -> Result<(), String> {
    if epoch()?.checked_add(1) == Some(u64::MAX) {
        Err("Inference actor epoch capacity exhausted".into())
    } else {
        Ok(())
    }
}
pub(super) fn invalidate() {
    // MAX is a permanent fail-closed sentinel, never a usable actor generation.
    let _ = EPOCH.fetch_update(Ordering::AcqRel, Ordering::Acquire, |v| v.checked_add(1));
}
pub(super) fn revoke_tracked() {
    let mut entries = owners().lock().unwrap_or_else(|e| e.into_inner());
    for e in entries.iter_mut().flatten() {
        if let Some(owner) = e.owner.upgrade() {
            for slot in &mut e.sessions {
                if let Some(handle) = slot.take() {
                    let _ = owner.disconnect(&handle.handle);
                }
            }
        } else {
            for slot in &mut e.sessions {
                *slot = None;
            }
        }
    }
}
pub(super) fn track(
    owner: &Arc<NativeAuthority>,
    handle: &SessionHandle,
    association: AuthenticatedInferenceAssociation,
) -> Result<(), String> {
    let mut entries = owners()
        .lock()
        .map_err(|_| "Inference owner registry poisoned")?;
    let entry = entries
        .iter_mut()
        .flatten()
        .find(|e| e.owner.ptr_eq(&Arc::downgrade(owner)))
        .ok_or("Inference owner registry entry missing")?;
    for slot in &mut entry.sessions {
        if slot.as_ref().is_some_and(|s| {
            owner
                .with_current_inference_association(&s.association, |_| ())
                .is_err()
        }) {
            if let Some(stale) = slot.take() {
                let _ = owner.disconnect(&stale.handle);
            }
        }
    }
    if entry.sessions.iter().flatten().any(|s| s.handle == *handle) {
        return Err("Inference session already tracked".into());
    }
    let slot = entry
        .sessions
        .iter_mut()
        .find(|s| s.is_none())
        .ok_or("Inference client session tracking capacity exhausted")?;
    *slot = Some(TrackedSession {
        handle: handle.clone(),
        association,
    });
    Ok(())
}
pub(super) fn forget(owner: &Arc<NativeAuthority>, handle: &SessionHandle) {
    let mut entries = owners().lock().unwrap_or_else(|e| e.into_inner());
    for e in entries
        .iter_mut()
        .flatten()
        .filter(|e| e.owner.ptr_eq(&Arc::downgrade(owner)))
    {
        for slot in &mut e.sessions {
            if slot.as_ref().is_some_and(|s| s.handle == *handle) {
                *slot = None;
            }
        }
    }
}
pub(super) fn preflight(root: &Path, c: &InferenceHostConfiguration) -> Result<(), String> {
    let entries = owners()
        .lock()
        .map_err(|_| "Inference owner registry poisoned")?;
    let key = anchor(c);
    if entries.iter().any(|e| {
        e.as_ref()
            .is_none_or(|e| e.owner.strong_count() == 0 || (e.root == root && e.anchor == key))
    }) {
        Ok(())
    } else {
        Err("Inference owner registry capacity exhausted".into())
    }
}
pub(super) fn open(
    root: &Path,
    c: &InferenceHostConfiguration,
    document: &DocumentRevision,
    initial: Option<SignedAdmission>,
) -> Result<Arc<NativeAuthority>, String> {
    let mut entries = owners()
        .lock()
        .map_err(|_| "Inference owner registry poisoned")?;
    let key = anchor(c);
    for entry in entries.iter().flatten() {
        if entry.root == root && entry.anchor == key {
            if let Some(owner) = entry.owner.upgrade() {
                if entry.document != *document {
                    return Err("Configured source revision differs from retained owner".into());
                }
                if let Some(initial) = initial {
                    let current = owner.enrollment_snapshot()?;
                    if initial.signature != current.signature
                        || admission_digest(&initial.snapshot, configuration::ENTRY_LIMIT)?
                            != admission_digest(&current.snapshot, configuration::ENTRY_LIMIT)?
                    {
                        return Err(
                            "Configured admission differs; install an explicit signed successor"
                                .into(),
                        );
                    }
                }
                return Ok(owner);
            }
        }
    }
    let slot = entries
        .iter()
        .position(|e| e.as_ref().is_none_or(|e| e.owner.strong_count() == 0))
        .ok_or("Inference owner registry capacity exhausted")?;
    let parent = store::child_directory(root, "inference-actors")?;
    let path = store::child_directory(&parent, &hex::encode(key))?;
    let owner = Arc::new(match initial {
        Some(admission) => NativeAuthority::open(
            &path,
            c.anchor(),
            admission,
            document.clone(),
            configuration::ENTRY_LIMIT,
            configuration::ENTRY_COUNT,
        )?,
        None => NativeAuthority::open_retained(
            &path,
            c.anchor(),
            document.clone(),
            configuration::ENTRY_LIMIT,
            configuration::ENTRY_COUNT,
        )?,
    });
    entries[slot] = Some(Entry {
        root: root.to_path_buf(),
        anchor: key,
        document: document.clone(),
        owner: Arc::downgrade(&owner),
        sessions: std::array::from_fn(|_| None),
    });
    Ok(owner)
}
#[cfg(test)]
pub(super) fn set_epoch_for_test(value: u64) {
    EPOCH.store(value, Ordering::Release);
}
