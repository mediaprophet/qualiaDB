#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenizerProfile {
    Gpt2,
    Smollm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenizerError {
    Metadata,
    DuplicateField,
    UnsupportedProfile,
    MalformedMerge,
    MissingByteSymbol(u8),
    InvalidTokenId(u32),
    InvalidByteToken,
    Capacity,
    CapacityBudget,
    WorkBudget,
    ArithmeticOverflow,
}

impl core::fmt::Display for TokenizerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "tokenizer: {self:?}")
    }
}
impl core::error::Error for TokenizerError {}

#[derive(Clone, Copy, Debug)]
pub struct PrepareBudget {
    pub capacity_bytes: usize,
    pub max_work: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct ExecutionBudget {
    pub capacity_bytes: usize,
    pub max_work: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TokenizerRequirements {
    pub vocab: usize,
    pub merges: usize,
    pub specials: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct VocabEntry<'a> {
    pub(super) text: &'a str,
    pub(super) id: u32,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct MergeEntry<'a> {
    pub(super) left: &'a str,
    pub(super) right: &'a str,
    pub(super) rank: usize,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct SpecialEntry<'a> {
    pub(super) text: &'a str,
    pub(super) id: u32,
}

#[derive(Debug)]
pub struct TokenizerTables<'work, 'data> {
    pub vocab: &'work mut [VocabEntry<'data>],
    pub merges: &'work mut [MergeEntry<'data>],
    pub specials: &'work mut [SpecialEntry<'data>],
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PretokenSpan {
    pub start: usize,
    pub end: usize,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct SymbolSpan {
    pub(super) start: usize,
    pub(super) end: usize,
}

#[derive(Debug)]
pub struct ExecutionWorkspace<'a> {
    pub spans: &'a mut [PretokenSpan],
    pub encoded: &'a mut [u8],
    pub symbols: &'a mut [SymbolSpan],
}
