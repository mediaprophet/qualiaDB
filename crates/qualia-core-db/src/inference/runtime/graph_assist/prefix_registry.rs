use super::identity::PrefixIdentity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryError {
    EmptyPageSet,
    PageCapacity,
    RegistryFull,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrefixPageSet<const PAGES: usize> {
    pub identity: PrefixIdentity,
    pub pages: [u32; PAGES],
    pub page_count: u16,
    pub token_count: u32,
}

impl<const PAGES: usize> PrefixPageSet<PAGES> {
    pub const fn empty() -> Self {
        Self {
            identity: PrefixIdentity { words: [0; 4] },
            pages: [u32::MAX; PAGES],
            page_count: 0,
            token_count: 0,
        }
    }

    pub fn new(
        identity: PrefixIdentity,
        pages: &[u32],
        token_count: u32,
    ) -> Result<Self, RegistryError> {
        if pages.is_empty() {
            return Err(RegistryError::EmptyPageSet);
        }
        if pages.len() > PAGES || pages.len() > u16::MAX as usize {
            return Err(RegistryError::PageCapacity);
        }
        let mut set = Self::empty();
        set.identity = identity;
        set.pages[..pages.len()].copy_from_slice(pages);
        set.page_count = pages.len() as u16;
        set.token_count = token_count;
        Ok(set)
    }

    pub fn page_slice(&self) -> &[u32] {
        &self.pages[..self.page_count as usize]
    }
}

/// Fixed-capacity prefix-page index. Lookup/insert/evict never allocate.
pub struct PrefixPageRegistry<const ENTRIES: usize, const PAGES: usize> {
    entries: [PrefixPageSet<PAGES>; ENTRIES],
    occupied: [bool; ENTRIES],
    next_victim: usize,
}

impl<const ENTRIES: usize, const PAGES: usize> PrefixPageRegistry<ENTRIES, PAGES> {
    pub const fn new() -> Self {
        Self {
            entries: [PrefixPageSet::empty(); ENTRIES],
            occupied: [false; ENTRIES],
            next_victim: 0,
        }
    }

    pub fn get(&self, identity: PrefixIdentity) -> Option<&PrefixPageSet<PAGES>> {
        self.entries
            .iter()
            .zip(self.occupied)
            .find_map(|(entry, occupied)| (occupied && entry.identity == identity).then_some(entry))
    }

    /// Insert or update. Once full, deterministic round-robin replacement bounds retention.
    pub fn insert(&mut self, set: PrefixPageSet<PAGES>) -> Result<(), RegistryError> {
        self.insert_replacing(set).map(|_| ())
    }

    /// Insert or update and return the replaced page set to its resource owner.
    ///
    /// The registry itself stores identifiers only; a KV prefix owner uses this return value to
    /// release physical-page references without leaking them on update or round-robin eviction.
    pub fn insert_replacing(
        &mut self,
        set: PrefixPageSet<PAGES>,
    ) -> Result<Option<PrefixPageSet<PAGES>>, RegistryError> {
        if ENTRIES == 0 {
            return Err(RegistryError::RegistryFull);
        }
        if let Some(index) = self
            .entries
            .iter()
            .zip(self.occupied)
            .position(|(entry, occupied)| occupied && entry.identity == set.identity)
            .or_else(|| self.occupied.iter().position(|occupied| !occupied))
        {
            let replaced = self.occupied[index].then_some(self.entries[index]);
            self.entries[index] = set;
            self.occupied[index] = true;
            return Ok(replaced);
        }
        let victim = self.next_victim % ENTRIES;
        let replaced = self.occupied[victim].then_some(self.entries[victim]);
        self.entries[victim] = set;
        self.occupied[victim] = true;
        self.next_victim = (victim + 1) % ENTRIES;
        Ok(replaced)
    }

    pub fn remove(&mut self, identity: PrefixIdentity) -> bool {
        self.remove_entry(identity).is_some()
    }

    /// Remove and return an entry so the physical-page owner can release its references.
    pub fn remove_entry(&mut self, identity: PrefixIdentity) -> Option<PrefixPageSet<PAGES>> {
        if let Some(index) = self
            .entries
            .iter()
            .zip(self.occupied)
            .position(|(entry, occupied)| occupied && entry.identity == identity)
        {
            let removed = self.entries[index];
            self.occupied[index] = false;
            self.entries[index] = PrefixPageSet::empty();
            Some(removed)
        } else {
            None
        }
    }
}

impl<const ENTRIES: usize, const PAGES: usize> Default for PrefixPageRegistry<ENTRIES, PAGES> {
    fn default() -> Self {
        Self::new()
    }
}
