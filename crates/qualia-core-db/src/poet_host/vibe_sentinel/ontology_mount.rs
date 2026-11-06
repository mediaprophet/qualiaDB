//! Bounded ontology `NQuin` mounting, authority-filtered capability discovery,
//! and safe provider unmount (`BoundedOntologyMount`, `VS03-11`).

use super::policy_gate::{quin48_to_nquin, WebizenSentinelGate};
use crate::NQuin;
use vibescript_abi::{StableFuncId, VibeNamespaceProvider, VibeTrap};

/// Maximum number of mounted providers tracked by `BoundedOntologyMount`.
pub const MAX_MOUNTED_ONTOLOGY_PROVIDERS: usize = 32;

/// Metadata for a single mounted provider's ontology slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MountedProviderSpan {
    pub provider_iri_hash: u64,
    pub start_idx: usize,
    pub len: usize,
    pub active: bool,
}

/// Descriptor returned by authority-filtered capability discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermittedFunctionSummary {
    pub provider_iri_hash: u64,
    pub func_iri_hash: u64,
    pub stable_id: StableFuncId,
}

/// Caller-buffered ontology store that validates and mounts `static_ontology_nquins()`
/// within an explicit `NQuin` capacity budget and supports clean provider unmounting.
pub struct BoundedOntologyMount<'a> {
    store: &'a mut [NQuin],
    used_len: usize,
    max_quins_per_provider: usize,
    spans: [MountedProviderSpan; MAX_MOUNTED_ONTOLOGY_PROVIDERS],
}

impl<'a> BoundedOntologyMount<'a> {
    /// Creates a new `BoundedOntologyMount` over caller-supplied `store`.
    pub fn new(store: &'a mut [NQuin], max_quins_per_provider: usize) -> Self {
        Self {
            store,
            used_len: 0,
            max_quins_per_provider,
            spans: [MountedProviderSpan::default(); MAX_MOUNTED_ONTOLOGY_PROVIDERS],
        }
    }

    /// Total number of mounted ontology `NQuin` records currently live.
    #[inline]
    pub const fn mounted_quin_count(&self) -> usize {
        self.used_len
    }

    /// Returns the live slice of mounted ontology `NQuin`s.
    #[inline]
    pub fn mounted_quins(&self) -> &[NQuin] {
        &self.store[..self.used_len]
    }

    /// Validates and mounts `provider.static_ontology_nquins()` into `self.store`.
    pub fn mount_provider(
        &mut self,
        provider: &dyn VibeNamespaceProvider,
    ) -> Result<MountedProviderSpan, VibeTrap> {
        let desc = provider.descriptor();
        let quins48 = provider.static_ontology_nquins();
        let provider_hash = desc.canonical_iri.fnv1a_hash;

        // Reject duplicate provider mount
        for s in &self.spans {
            if s.active && s.provider_iri_hash == provider_hash {
                return Err(VibeTrap::DuplicateNamespaceIri);
            }
        }

        // Enforce per-provider and total capacity budgets
        let allowed_for_provider =
            (desc.max_ontology_quins as usize).min(self.max_quins_per_provider);
        if quins48.len() > allowed_for_provider || self.used_len + quins48.len() > self.store.len()
        {
            return Err(VibeTrap::OntologyBudgetExceeded);
        }

        // Verify parity on every NQuin48 before writing any record
        for q48 in quins48 {
            if !q48.verify_parity() {
                return Err(VibeTrap::InvalidQuinParity);
            }
        }

        let free_slot = self
            .spans
            .iter()
            .position(|s| !s.active)
            .ok_or(VibeTrap::RegistryCapacityExceeded)?;

        let start = self.used_len;
        for (i, q48) in quins48.iter().enumerate() {
            self.store[start + i] = quin48_to_nquin(q48);
        }
        self.used_len += quins48.len();

        let span = MountedProviderSpan {
            provider_iri_hash: provider_hash,
            start_idx: start,
            len: quins48.len(),
            active: true,
        };
        self.spans[free_slot] = span;
        Ok(span)
    }

    /// Unmounts `provider_iri_hash`, compacting `self.store` and updating all remaining spans
    /// so no dangling ontology records remain.
    pub fn unmount_provider(&mut self, provider_iri_hash: u64) -> Result<(), VibeTrap> {
        let slot_idx = self
            .spans
            .iter()
            .position(|s| s.active && s.provider_iri_hash == provider_iri_hash)
            .ok_or(VibeTrap::NamespaceNotFound)?;

        let removed = self.spans[slot_idx];
        let tail_start = removed.start_idx + removed.len;
        let tail_len = self.used_len - tail_start;

        // Shift tail records left
        let mut i = 0;
        while i < tail_len {
            self.store[removed.start_idx + i] = self.store[tail_start + i];
            i += 1;
        }
        // Zero out vacated tail
        let new_used = self.used_len - removed.len;
        let mut z = new_used;
        while z < self.used_len {
            self.store[z] = NQuin::default();
            z += 1;
        }
        self.used_len = new_used;
        self.spans[slot_idx].active = false;

        // Update start_idx for spans that followed the removed span
        for s in &mut self.spans {
            if s.active && s.start_idx > removed.start_idx {
                s.start_idx -= removed.len;
            }
        }
        Ok(())
    }

    /// Authority-filtered capability discovery: writes only the functions from `provider`
    /// that `actor_did_hash` is currently permitted to invoke in `context_world_hash`
    /// at `now_unix32` into caller-supplied `out`.
    pub fn discover_permitted_functions(
        &self,
        provider: &dyn VibeNamespaceProvider,
        gate: &WebizenSentinelGate<'_>,
        actor_did_hash: u64,
        context_world_hash: u64,
        now_unix32: u32,
        out: &mut [PermittedFunctionSummary],
    ) -> Result<usize, VibeTrap> {
        let desc = provider.descriptor();
        let provider_hash = desc.canonical_iri.fnv1a_hash;

        // Provider must be currently mounted
        let is_mounted = self
            .spans
            .iter()
            .any(|s| s.active && s.provider_iri_hash == provider_hash);
        if !is_mounted {
            return Err(VibeTrap::NamespaceNotFound);
        }

        let mut written = 0usize;
        for sub in desc.sub_namespaces {
            for func in sub.functions {
                if gate
                    .evaluate_access(
                        actor_did_hash,
                        func.canonical_iri.fnv1a_hash,
                        provider_hash,
                        context_world_hash,
                        now_unix32,
                    )
                    .is_ok()
                {
                    if written >= out.len() {
                        return Err(VibeTrap::OutputBufferTooSmall);
                    }
                    out[written] = PermittedFunctionSummary {
                        provider_iri_hash: provider_hash,
                        func_iri_hash: func.canonical_iri.fnv1a_hash,
                        stable_id: func.stable_id,
                    };
                    written += 1;
                }
            }
        }
        Ok(written)
    }
}
