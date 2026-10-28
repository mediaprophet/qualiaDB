//! Opaque process-local incarnation/revision guards for the existing store.
//! This stamp is not an identifier, durable checkpoint or authority grant.
use super::{journal, store, StoredArtifact, StudioArtifact};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};

/// Lock-free publication state owned by the canonical StoredArtifact entry.
#[derive(Debug)]
pub(super) struct PublicationState {
    artifact_id: String,
    revision: AtomicU32,
    busy: AtomicBool,
    retired: AtomicBool,
}
impl PublicationState {
    pub(super) fn new(artifact_id: &str, revision: u32) -> Self {
        Self {
            artifact_id: artifact_id.to_owned(),
            revision: AtomicU32::new(revision),
            busy: AtomicBool::new(false),
            retired: AtomicBool::new(false),
        }
    }
    pub(super) fn begin(self: &Arc<Self>, revision: u32) -> Result<PublicationMutation, String> {
        if self.retired.load(Ordering::Acquire)
            || self.revision.load(Ordering::Acquire) != revision
            || self
                .busy
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
        {
            return Err("source publication is busy, retired, or stale".into());
        }
        Ok(PublicationMutation {
            state: Arc::clone(self),
            previous: revision,
            finished: false,
        })
    }
    pub(super) fn retire(&self) {
        self.retired.store(true, Ordering::Release);
    }
    pub(super) fn matches(&self, id: &str, revision: u32) -> bool {
        if id != self.artifact_id
            || self.retired.load(Ordering::Acquire)
            || self.busy.load(Ordering::Acquire)
            || self.revision.load(Ordering::Acquire) != revision
        {
            return false;
        }
        !self.busy.load(Ordering::Acquire)
            && !self.retired.load(Ordering::Acquire)
            && self.revision.load(Ordering::Acquire) == revision
    }
}
pub(super) struct PublicationMutation {
    state: Arc<PublicationState>,
    previous: u32,
    finished: bool,
}
impl PublicationMutation {
    pub(super) fn finish(mut self, revision: u32) {
        self.state.revision.store(revision, Ordering::Release);
        self.state.busy.store(false, Ordering::Release);
        self.finished = true;
    }
}
impl Drop for PublicationMutation {
    fn drop(&mut self) {
        if !self.finished {
            if std::thread::panicking() {
                // Keep this incarnation unusable until the enclosing store
                // mutex has completed poisoning during unwind. The mutation
                // may have changed source before panicking.
                self.state.retired.store(true, Ordering::Release);
            } else {
                // Ordinary rejected candidates are detached and can restore
                // their prior publication without invalidating accepted work.
                self.state.revision.store(self.previous, Ordering::Release);
                self.state.busy.store(false, Ordering::Release);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct StoreStamp {
    publication: Arc<PublicationState>,
    revision: u32,
}

impl StoreStamp {
    pub fn revision(&self) -> u32 {
        self.revision
    }
    pub(super) fn of(stored: &StoredArtifact) -> Self {
        Self {
            publication: Arc::clone(&stored.publication),
            revision: stored.lamport_clock,
        }
    }
    pub(super) fn matches(&self, stored: &StoredArtifact) -> bool {
        self.revision == stored.lamport_clock && Arc::ptr_eq(&self.publication, &stored.publication)
    }
}

impl PartialEq for StoreStamp {
    fn eq(&self, other: &Self) -> bool {
        self.revision == other.revision && Arc::ptr_eq(&self.publication, &other.publication)
    }
}
impl Eq for StoreStamp {}

/// Cheap UI publication check: never clones source or waits on an active edit.
/// Busy/poisoned/absent stores fail closed; no identity or authority is inferred.
pub fn artifact_stamp_is_current(id: &str, stamp: &StoreStamp) -> bool {
    stamp_is_current(id, stamp, store().is_poisoned())
}

pub(super) fn stamp_is_current(id: &str, stamp: &StoreStamp, store_poisoned: bool) -> bool {
    !store_poisoned && stamp.publication.matches(id, stamp.revision)
}

/// Read source and stamp under one mutex. An old stamp retains its Arc, so
/// allocator address reuse cannot create ABA while any old stamp remains held.
pub fn get_artifact_stamped(id: &str) -> Option<(StudioArtifact, StoreStamp)> {
    let source = store().lock().expect("studio store poisoned");
    source
        .get(id)
        .map(|stored| (stored.artifact.clone(), StoreStamp::of(stored)))
}

/// Inspect the existing undo predecessor under the exact current store stamp.
/// No session or permission is inferred; native hosts must first authorize read.
pub fn previous_source_snapshot(
    id: &str,
    stamp: &StoreStamp,
) -> Result<Option<journal::EditableSourceSnapshot>, String> {
    let source = store().lock().map_err(|_| "studio store poisoned")?;
    let stored = source.get(id).ok_or("Studio artifact is absent")?;
    if !stamp.matches(stored) {
        return Err("stale Studio predecessor inspection".into());
    }
    Ok(journal::previous_source_snapshot(stored))
}

/// Atomic admission of a new identified record. Never replaces accepted work.
/// The native caller owns admission/rights; existence is not authorization.
pub fn create_artifact_if_absent(
    artifact: StudioArtifact,
) -> Result<(StudioArtifact, StoreStamp), String> {
    let mut source = store().lock().expect("studio store poisoned");
    if source.contains_key(&artifact.id) {
        return Err("Studio artifact already exists; source preserved".into());
    }
    let publication = Arc::new(PublicationState::new(&artifact.id, artifact.lamport_clock));
    let stored = StoredArtifact {
        artifact,
        journal: journal::SourceJournal::default(),
        publication,
    };
    journal::admit_initial(&stored)?;
    let result = (stored.artifact.clone(), StoreStamp::of(&stored));
    source.insert(stored.id.clone(), stored);
    Ok(result)
}

fn with_current<F>(
    id: &str,
    stamp: &StoreStamp,
    action: F,
) -> Result<(StudioArtifact, StoreStamp), String>
where
    F: FnOnce(&mut StoredArtifact) -> Result<(), String>,
{
    let mut source = store().lock().expect("studio store poisoned");
    let stored = source
        .get_mut(id)
        .ok_or_else(|| "Studio artifact is absent".to_string())?;
    if !stamp.matches(stored) {
        return Err("stale Studio incarnation/revision; source preserved".into());
    }
    let publication = Arc::clone(&stored.publication);
    let mutation = publication.begin(stored.lamport_clock)?;
    action(stored)?;
    mutation.finish(stored.lamport_clock);
    Ok((stored.artifact.clone(), StoreStamp::of(stored)))
}

/// Fallible detached-candidate edit. Rejection never activates candidate source,
/// advances revision, clears redo, or consumes source/audit history.
pub fn mutate_artifact_stamped<F>(
    id: &str,
    stamp: &StoreStamp,
    action: F,
) -> Result<(StudioArtifact, StoreStamp), String>
where
    F: FnOnce(&mut StudioArtifact) -> Result<(), String>,
{
    with_current(id, stamp, |stored| {
        journal::try_edit(stored, Some(stamp.revision), action)
    })
}

pub fn undo_artifact_stamped(
    id: &str,
    stamp: &StoreStamp,
) -> Result<(StudioArtifact, StoreStamp), String> {
    with_current(id, stamp, |stored| {
        journal::restore(stored, Some(stamp.revision), false)
    })
}

pub fn redo_artifact_stamped(
    id: &str,
    stamp: &StoreStamp,
) -> Result<(StudioArtifact, StoreStamp), String> {
    with_current(id, stamp, |stored| {
        journal::restore(stored, Some(stamp.revision), true)
    })
}

#[cfg(test)]
mod predecessor_tests {
    use super::*;
    #[test]
    fn predecessor_is_actual_undo_source_and_stale_stamp_cannot_read_it() {
        let id = "urn:qualia:test:source-predecessor-stamp";
        let mut artifact = StudioArtifact::new(id, "test", "Original title");
        artifact
            .properties
            .insert("retained".into(), "original source".into());
        let (_, before) = create_artifact_if_absent(artifact).unwrap();
        assert!(previous_source_snapshot(id, &before).unwrap().is_none());
        let (_, after) = mutate_artifact_stamped(id, &before, |source| {
            source.title = "New title".into();
            source
                .properties
                .insert("retained".into(), "changed source".into());
            Ok(())
        })
        .unwrap();
        assert!(previous_source_snapshot(id, &before).is_err());
        let previous = previous_source_snapshot(id, &after).unwrap().unwrap();
        assert_eq!(previous.title, "Original title");
        assert_eq!(previous.properties["retained"], "original source");
    }
}
