//! Opaque process-local incarnation/revision guards for the existing store.
//! This stamp is not an identifier, durable checkpoint or authority grant.
use super::{journal, store, StoredArtifact, StudioArtifact};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct StoreStamp {
    incarnation: Arc<()>,
    revision: u32,
}

impl StoreStamp {
    pub fn revision(&self) -> u32 { self.revision }
    pub(super) fn of(stored: &StoredArtifact) -> Self {
        Self { incarnation: Arc::clone(&stored.incarnation), revision: stored.lamport_clock }
    }
    pub(super) fn matches(&self, stored: &StoredArtifact) -> bool {
        self.revision == stored.lamport_clock && Arc::ptr_eq(&self.incarnation, &stored.incarnation)
    }
}

impl PartialEq for StoreStamp {
    fn eq(&self, other: &Self) -> bool {
        self.revision == other.revision && Arc::ptr_eq(&self.incarnation, &other.incarnation)
    }
}
impl Eq for StoreStamp {}

/// Cheap UI publication check: never clones source or waits on an active edit.
/// Busy/poisoned/absent stores fail closed; no identity or authority is inferred.
pub fn artifact_stamp_is_current(id: &str, stamp: &StoreStamp) -> bool {
    let Ok(source) = store().try_lock() else { return false };
    source.get(id).is_some_and(|stored| stamp.matches(stored))
}

/// Read source and stamp under one mutex. An old stamp retains its Arc, so
/// allocator address reuse cannot create ABA while any old stamp remains held.
pub fn get_artifact_stamped(id: &str) -> Option<(StudioArtifact, StoreStamp)> {
    let source = store().lock().expect("studio store poisoned");
    source.get(id).map(|stored| (stored.artifact.clone(), StoreStamp::of(stored)))
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
pub fn create_artifact_if_absent(artifact: StudioArtifact) -> Result<(StudioArtifact, StoreStamp), String> {
    let mut source = store().lock().expect("studio store poisoned");
    if source.contains_key(&artifact.id) { return Err("Studio artifact already exists; source preserved".into()); }
    let stored = StoredArtifact { artifact, journal: journal::SourceJournal::default(), incarnation: Arc::new(()) };
    journal::admit_initial(&stored)?;
    let result = (stored.artifact.clone(), StoreStamp::of(&stored));
    source.insert(stored.id.clone(), stored);
    Ok(result)
}

fn with_current<F>(id: &str, stamp: &StoreStamp, action: F) -> Result<(StudioArtifact, StoreStamp), String>
where F: FnOnce(&mut StoredArtifact) -> Result<(), String> {
    let mut source = store().lock().expect("studio store poisoned");
    let stored = source.get_mut(id).ok_or_else(|| "Studio artifact is absent".to_string())?;
    if !stamp.matches(stored) { return Err("stale Studio incarnation/revision; source preserved".into()); }
    action(stored)?;
    Ok((stored.artifact.clone(), StoreStamp::of(stored)))
}

/// Fallible detached-candidate edit. Rejection never activates candidate source,
/// advances revision, clears redo, or consumes source/audit history.
pub fn mutate_artifact_stamped<F>(id: &str, stamp: &StoreStamp, action: F) -> Result<(StudioArtifact, StoreStamp), String>
where F: FnOnce(&mut StudioArtifact) -> Result<(), String> {
    with_current(id, stamp, |stored| journal::try_edit(stored, Some(stamp.revision), action))
}

pub fn undo_artifact_stamped(id: &str, stamp: &StoreStamp) -> Result<(StudioArtifact, StoreStamp), String> {
    with_current(id, stamp, |stored| journal::restore(stored, Some(stamp.revision), false))
}

pub fn redo_artifact_stamped(id: &str, stamp: &StoreStamp) -> Result<(StudioArtifact, StoreStamp), String> {
    with_current(id, stamp, |stored| journal::restore(stored, Some(stamp.revision), true))
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
