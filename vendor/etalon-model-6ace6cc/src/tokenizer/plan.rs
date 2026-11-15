use super::{
    budget::{capacity, Work},
    index,
    metadata::{self, Strings},
    *,
};
use crate::gguf::MetadataView;
use core::{cmp::Ordering, mem::size_of};

#[derive(Debug)]
pub struct TokenizerPlan<'data, 'tables> {
    pub(super) vocab: &'tables [VocabEntry<'data>],
    pub(super) merges: &'tables [MergeEntry<'data>],
    pub(super) specials: &'tables [SpecialEntry<'data>],
    metadata: &'data [u8],
    bos: u32,
    eos: u32,
    add_bos: bool,
    profile: TokenizerProfile,
    max_token_bytes: usize,
}
impl TokenizerPlan<'_, '_> {
    pub fn vocab_count(&self) -> usize {
        self.vocab.len()
    }
    pub fn bos_token_id(&self) -> u32 {
        self.bos
    }
    pub fn eos_token_id(&self) -> u32 {
        self.eos
    }
    pub fn add_bos_token(&self) -> bool {
        self.add_bos
    }
    pub fn profile(&self) -> TokenizerProfile {
        self.profile
    }
    pub fn model(&self) -> &'static str {
        "gpt2"
    }
    pub fn max_token_bytes(&self) -> usize {
        self.max_token_bytes
    }
    pub fn metadata_bytes(&self) -> &[u8] {
        self.metadata
    }
}

/// Counts required entries, validating profile/UTF-8/merge grammar. Counts alone
/// cannot construct a plan. Missing byte symbols are checked when encoding needs them.
pub fn requirements(
    view: &MetadataView<'_>,
    budget: PrepareBudget,
) -> Result<TokenizerRequirements, TokenizerError> {
    let mut work = Work(budget.max_work);
    metadata::read(view, &mut work)?.requirements(&mut work)
}

/// Borrows source strings and immutable used table prefixes. No hidden allocation.
/// Table capacities are checked before any write; later validation/work errors
/// may leave tables partly initialized. Fixed-size local control state is excluded.
pub fn prepare<'data, 'tables>(
    view: &MetadataView<'data>,
    tables: TokenizerTables<'tables, 'data>,
    budget: PrepareBudget,
) -> Result<TokenizerPlan<'data, 'tables>, TokenizerError> {
    capacity(
        &[
            (tables.vocab.len(), size_of::<VocabEntry<'_>>()),
            (tables.merges.len(), size_of::<MergeEntry<'_>>()),
            (tables.specials.len(), size_of::<SpecialEntry<'_>>()),
        ],
        budget.capacity_bytes,
    )?;
    let mut work = Work(budget.max_work);
    let fields = metadata::read(view, &mut work)?;
    let counts = fields.requirements(&mut work)?;
    if tables.vocab.len() < counts.vocab
        || tables.merges.len() < counts.merges
        || tables.specials.len() < counts.specials
    {
        return Err(TokenizerError::Capacity);
    }
    let vocab = &mut tables.vocab[..counts.vocab];
    let merges = &mut tables.merges[..counts.merges];
    let specials = &mut tables.specials[..counts.specials];
    let mut strings = Strings::new(fields.tokens, &mut work)?;
    let mut n_specials = 0;
    let mut max_token_bytes = 0;
    for (i, entry) in vocab.iter_mut().enumerate() {
        work.charge(1)?;
        let text = strings.next(&mut work)?.ok_or(TokenizerError::Metadata)?;
        *entry = VocabEntry { text, id: i as u32 };
        max_token_bytes = max_token_bytes.max(text.len());
        if metadata::special(text) {
            specials[n_specials] = SpecialEntry { text, id: i as u32 };
            n_specials += 1;
        }
    }
    let mut strings = Strings::new(fields.merges, &mut work)?;
    for (rank, entry) in merges.iter_mut().enumerate() {
        work.charge(1)?;
        let text = strings.next(&mut work)?.ok_or(TokenizerError::Metadata)?;
        let (left, right) = metadata::pair(text, &mut work)?;
        *entry = MergeEntry { left, right, rank };
    }
    index::sort(vocab, &mut work, |a, b, w| {
        Ok(w.compare(a.text.as_bytes(), b.text.as_bytes())?
            .then(a.id.cmp(&b.id)))
    })?;
    index::sort(merges, &mut work, |a, b, w| {
        let first = w.compare(a.left.as_bytes(), b.left.as_bytes())?;
        let second = if first == Ordering::Equal {
            w.compare(a.right.as_bytes(), b.right.as_bytes())?
        } else {
            first
        };
        Ok(second.then(a.rank.cmp(&b.rank)))
    })?;
    index::sort(specials, &mut work, |a, b, w| {
        w.charge(1)?;
        Ok(b.text.len().cmp(&a.text.len()).then(a.id.cmp(&b.id)))
    })?;
    Ok(TokenizerPlan {
        vocab,
        merges,
        specials,
        metadata: view.metadata_bytes(),
        bos: fields.bos,
        eos: fields.eos,
        add_bos: fields.add_bos,
        profile: fields.profile,
        max_token_bytes,
    })
}
