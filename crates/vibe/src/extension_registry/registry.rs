//! Atomic, bounded extension provider registry (`ExtensionRegistry`) for `crates/vibe`.

use vibescript_abi::{
    q_hash_str, FuncDescriptor, ProviderDescriptor, RegistryGeneration,
    SubNamespaceDescriptor, VibeNamespaceProvider, VibeOpCode, VibeTrap,
};

/// Maximum number of concurrently registered extension providers in a fixed-size registry.
pub const MAX_EXTENSION_PROVIDERS: usize = 32;

/// Fixed-capacity, generational registry of `VibeNamespaceProvider` implementations.
///
/// Registration is cold and atomic: if any validation check fails (hash mismatch,
/// duplicate IRI/alias/ID, `q_hash` collision, or invalid ontology parity/budget),
/// the registry remains completely unchanged.
pub struct ExtensionRegistry<'p> {
    slots: [Option<&'p dyn VibeNamespaceProvider>; MAX_EXTENSION_PROVIDERS],
    generation: RegistryGeneration,
    active_count: usize,
}

impl<'p> Default for ExtensionRegistry<'p> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'p> ExtensionRegistry<'p> {
    /// Creates an empty `ExtensionRegistry` at `RegistryGeneration::INITIAL`.
    #[inline]
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_EXTENSION_PROVIDERS],
            generation: RegistryGeneration::INITIAL,
            active_count: 0,
        }
    }

    /// Current registry generation counter.
    #[inline]
    pub const fn generation(&self) -> RegistryGeneration {
        self.generation
    }

    /// Number of currently active providers.
    #[inline]
    pub const fn active_count(&self) -> usize {
        self.active_count
    }

    /// Atomically validates and registers `provider`, returning its assigned `ns_slot`
    /// and advancing `self.generation`.
    pub fn register(&mut self, provider: &'p dyn VibeNamespaceProvider) -> Result<u32, VibeTrap> {
        let free_slot = self
            .slots
            .iter()
            .position(|s| s.is_none())
            .ok_or(VibeTrap::RegistryCapacityExceeded)?;

        let desc = provider.descriptor();
        let ontology = provider.static_ontology_nquins();

        // 1. Validate canonical IRI and short alias
        if desc.canonical_iri.iri.is_empty() || desc.short_alias.is_empty() {
            return Err(VibeTrap::InvalidCanonicalIri);
        }
        if desc.canonical_iri.fnv1a_hash != q_hash_str(desc.canonical_iri.iri) {
            return Err(VibeTrap::HashMismatch);
        }

        // 2. Validate 5-axis version compatibility
        if !desc.versions.is_engine_compatible() {
            return Err(VibeTrap::IncompatibleVersionAxes);
        }

        // 3. Validate ontology budget and NQuin48 XOR parity
        if ontology.len() > desc.max_ontology_quins as usize {
            return Err(VibeTrap::OntologyBudgetExceeded);
        }
        for quin in ontology {
            if !quin.verify_parity() {
                return Err(VibeTrap::InvalidQuinParity);
            }
        }

        // 4. Validate intra-provider sub-namespaces, functions, and ontology spans
        for (sub_idx_a, sub_a) in desc.sub_namespaces.iter().enumerate() {
            if sub_a.name.is_empty() {
                return Err(VibeTrap::InvalidCanonicalIri);
            }
            for sub_b in &desc.sub_namespaces[..sub_idx_a] {
                if sub_a.sub_id == sub_b.sub_id || sub_a.name == sub_b.name {
                    return Err(VibeTrap::DuplicateStableFuncId);
                }
            }
            for (fn_idx_a, fn_a) in sub_a.functions.iter().enumerate() {
                if fn_a.name.is_empty() || fn_a.canonical_iri.iri.is_empty() {
                    return Err(VibeTrap::InvalidCanonicalIri);
                }
                if fn_a.canonical_iri.fnv1a_hash != q_hash_str(fn_a.canonical_iri.iri) {
                    return Err(VibeTrap::HashMismatch);
                }
                if fn_a.stable_id.sub_namespace_id != sub_a.sub_id {
                    return Err(VibeTrap::DuplicateStableFuncId);
                }
                if !fn_a.contract.validate_spans(ontology.len()) {
                    return Err(VibeTrap::InvalidOntologySpan);
                }
                // Check against earlier functions in the same sub-namespace
                for fn_b in &sub_a.functions[..fn_idx_a] {
                    Self::check_intra_provider_function_conflict(fn_a, fn_b, true)?;
                }
                // Check against functions in earlier sub-namespaces of the same provider
                for prev_sub in &desc.sub_namespaces[..sub_idx_a] {
                    for fn_b in prev_sub.functions {
                        Self::check_intra_provider_function_conflict(fn_a, fn_b, false)?;
                    }
                }
            }
        }

        // 5. Validate cross-provider collisions against all currently registered providers
        for existing in self.slots.iter().flatten() {
            let ex_desc = existing.descriptor();
            if ex_desc.canonical_iri.iri == desc.canonical_iri.iri {
                return Err(VibeTrap::DuplicateNamespaceIri);
            }
            if ex_desc.canonical_iri.fnv1a_hash == desc.canonical_iri.fnv1a_hash {
                return Err(VibeTrap::HashCollisionDetected);
            }
            if ex_desc.short_alias == desc.short_alias {
                return Err(VibeTrap::DuplicateShortAlias);
            }
            for sub_new in desc.sub_namespaces {
                for fn_new in sub_new.functions {
                    for sub_ex in ex_desc.sub_namespaces {
                        for fn_ex in sub_ex.functions {
                            Self::check_cross_provider_function_conflict(fn_new, fn_ex)?;
                        }
                    }
                }
            }
        }

        // 6. Commit atomically and advance generation
        self.slots[free_slot] = Some(provider);
        self.active_count += 1;
        self.generation = self.generation.next();
        Ok(free_slot as u32)
    }

    /// Unregisters the provider at `slot` and advances `self.generation` so all
    /// previously prepared handles fail closed with `VibeTrap::StaleRegistryGeneration`.
    pub fn unregister(&mut self, slot: u32) -> Result<(), VibeTrap> {
        let idx = slot as usize;
        if idx >= MAX_EXTENSION_PROVIDERS || self.slots[idx].is_none() {
            return Err(VibeTrap::NamespaceNotFound);
        }
        self.slots[idx] = None;
        self.active_count = self.active_count.saturating_sub(1);
        self.generation = self.generation.next();
        Ok(())
    }

    /// Looks up a registered provider by slot index.
    #[inline]
    pub fn provider_at(&self, slot: u32) -> Option<&'p dyn VibeNamespaceProvider> {
        self.slots.get(slot as usize).copied().flatten()
    }

    /// Resolves a `(provider, FuncDescriptor)` in bounded time from a `VibeOpCode`.
    #[inline]
    pub fn resolve_opcode(
        &self,
        opcode: VibeOpCode,
    ) -> Result<(&'p dyn VibeNamespaceProvider, &'static FuncDescriptor), VibeTrap> {
        let provider = self
            .provider_at(opcode.ns_slot())
            .ok_or(VibeTrap::NamespaceNotFound)?;
        let desc = provider.descriptor();
        let target_sub = opcode.sub_namespace_id();
        let target_fn = opcode.func_id();
        let mut s_idx = 0;
        while s_idx < desc.sub_namespaces.len() {
            let sub = &desc.sub_namespaces[s_idx];
            if sub.sub_id == target_sub {
                let mut f_idx = 0;
                while f_idx < sub.functions.len() {
                    let func = &sub.functions[f_idx];
                    if func.stable_id.func_id == target_fn {
                        return Ok((provider, func));
                    }
                    f_idx += 1;
                }
                return Err(VibeTrap::FunctionNotFound);
            }
            s_idx += 1;
        }
        Err(VibeTrap::FunctionNotFound)
    }

    /// Finds a registered provider by canonical IRI or short alias.
    pub fn find_by_namespace_or_alias(
        &self,
        namespace_or_alias: &str,
    ) -> Option<(u32, &'p dyn VibeNamespaceProvider, &'static ProviderDescriptor)> {
        for (idx, slot) in self.slots.iter().enumerate() {
            if let Some(provider) = slot {
                let desc = provider.descriptor();
                if desc.canonical_iri.iri == namespace_or_alias
                    || desc.short_alias == namespace_or_alias
                {
                    return Some((idx as u32, *provider, desc));
                }
            }
        }
        None
    }

    /// Iterates over all active provider descriptors.
    pub fn find_sub_namespace<'a>(
        desc: &'a ProviderDescriptor,
        sub_name: &str,
    ) -> Option<&'a SubNamespaceDescriptor> {
        desc.sub_namespaces.iter().find(|s| s.name == sub_name)
    }

    fn check_intra_provider_function_conflict(
        a: &FuncDescriptor,
        b: &FuncDescriptor,
        same_sub_namespace: bool,
    ) -> Result<(), VibeTrap> {
        if a.canonical_iri.iri == b.canonical_iri.iri {
            return Err(VibeTrap::DuplicateFunctionIri);
        }
        if a.canonical_iri.fnv1a_hash == b.canonical_iri.fnv1a_hash {
            return Err(VibeTrap::HashCollisionDetected);
        }
        if a.stable_id == b.stable_id || (same_sub_namespace && a.name == b.name) {
            return Err(VibeTrap::DuplicateStableFuncId);
        }
        Ok(())
    }

    fn check_cross_provider_function_conflict(
        a: &FuncDescriptor,
        b: &FuncDescriptor,
    ) -> Result<(), VibeTrap> {
        if a.canonical_iri.iri == b.canonical_iri.iri {
            return Err(VibeTrap::DuplicateFunctionIri);
        }
        if a.canonical_iri.fnv1a_hash == b.canonical_iri.fnv1a_hash {
            return Err(VibeTrap::HashCollisionDetected);
        }
        Ok(())
    }

    /// Checks whether a `StableFuncId` is duplicated within the same provider.
    pub fn validate_unique_stable_ids(desc: &ProviderDescriptor) -> Result<(), VibeTrap> {
        for (i, sub_a) in desc.sub_namespaces.iter().enumerate() {
            for (j, fn_a) in sub_a.functions.iter().enumerate() {
                for fn_b in &sub_a.functions[..j] {
                    if fn_a.stable_id == fn_b.stable_id || fn_a.name == fn_b.name {
                        return Err(VibeTrap::DuplicateStableFuncId);
                    }
                }
                for sub_b in &desc.sub_namespaces[..i] {
                    for fn_b in sub_b.functions {
                        if fn_a.stable_id == fn_b.stable_id {
                            return Err(VibeTrap::DuplicateStableFuncId);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
