//! Typed failures for portable operator validation and apply.

/// Recoverable operator contract failure.
///
/// `Copy` so hot paths can return without allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorError {
    /// Rank, feature, or batch dimensions are zero or inconsistent with the views.
    UnsupportedShape,
    /// Caller workspace is smaller than [`super::workspace_requirement`].
    WorkspaceTooSmall,
    /// Payload length, alignment, or checksum does not match the descriptor.
    InvalidPayload,
    /// A tile or codebook index is outside the declared bounds.
    IndexOutOfRange,
    /// Descriptor fields contradict each other (kind vs scale layout, digest empty, etc.).
    MalformedDescriptor,
    /// Kind is unknown, or apply was asked for a kind this schedule does not implement.
    UnsupportedKind,
}

impl OperatorError {
    /// Stable token for receipts and tests.
    pub const fn token(self) -> &'static str {
        match self {
            Self::UnsupportedShape => "unsupported_shape",
            Self::WorkspaceTooSmall => "workspace_too_small",
            Self::InvalidPayload => "invalid_payload",
            Self::IndexOutOfRange => "index_out_of_range",
            Self::MalformedDescriptor => "malformed_descriptor",
            Self::UnsupportedKind => "unsupported_kind",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OperatorError;

    #[test]
    fn error_tokens_are_distinct() {
        let all = [
            OperatorError::UnsupportedShape,
            OperatorError::WorkspaceTooSmall,
            OperatorError::InvalidPayload,
            OperatorError::IndexOutOfRange,
            OperatorError::MalformedDescriptor,
            OperatorError::UnsupportedKind,
        ];
        for (i, a) in all.iter().enumerate() {
            for (j, b) in all.iter().enumerate() {
                if i != j {
                    assert_ne!(a.token(), b.token());
                    assert_ne!(*a, *b);
                }
            }
        }
    }
}
