/// Value-free errors deliberately omit original names, offsets and private metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProtectionError {
    Source,
    Capacity,
    Limit,
    Overflow,
    Coverage,
}
