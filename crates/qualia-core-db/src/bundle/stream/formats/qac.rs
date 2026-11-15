use super::{AuthenticatedRange, FormatError};
use crate::audio::qac::{signal, QacStorageError, QacStorageOptions, QacStorageView};
use crate::q42::{field_envelope, field_semantics};
/// Authenticated complete current .qac entry. Partial ranges and legacy UA4Q refuse.
pub struct QacCompleteView<'a> {
    range: AuthenticatedRange<'a>,
    storage: QacStorageView<'a>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QacAdmitError {
    Format(FormatError),
    Family,
    Version,
    Extent,
    Crc,
    IndexDigest,
    PayloadDigest,
    Limit,
    Unsupported,
    Index,
    Shape,
    Scalar,
    Unit,
    Clock,
    Overflow,
    Nonfinite,
    Shared(field_semantics::Error),
    Envelope(field_envelope::Error),
}
impl From<FormatError> for QacAdmitError {
    fn from(e: FormatError) -> Self {
        Self::Format(e)
    }
}
impl From<field_envelope::Error> for QacAdmitError {
    fn from(e: field_envelope::Error) -> Self {
        match e {
            field_envelope::Error::Family => Self::Family,
            field_envelope::Error::Version => Self::Version,
            field_envelope::Error::Extent => Self::Extent,
            field_envelope::Error::Crc => Self::Crc,
            field_envelope::Error::IndexDigest => Self::IndexDigest,
            field_envelope::Error::PayloadDigest => Self::PayloadDigest,
            field_envelope::Error::Limit => Self::Limit,
            other => Self::Envelope(other),
        }
    }
}
impl From<QacStorageError> for QacAdmitError {
    fn from(e: QacStorageError) -> Self {
        match e {
            QacStorageError::Envelope(err) => err.into(),
        }
    }
}
fn map_signal_error(e: signal::Error) -> QacAdmitError {
    match e {
        signal::Error::Envelope(err) => err.into(),
        signal::Error::Shared(err) => match err {
            field_semantics::Error::UnknownCriticalSchema
            | field_semantics::Error::UnknownCriticalField
            | field_semantics::Error::UnknownCriticalType
            | field_semantics::Error::UnsupportedDomain
            | field_semantics::Error::RequiredCritical
            | field_semantics::Error::MissingField => QacAdmitError::Unsupported,
            field_semantics::Error::Limit => QacAdmitError::Limit,
            field_semantics::Error::Overflow => QacAdmitError::Overflow,
            other => QacAdmitError::Shared(other),
        },
        signal::Error::Unsupported | signal::Error::Profile => QacAdmitError::Unsupported,
        signal::Error::Shape => QacAdmitError::Shape,
        signal::Error::Scalar => QacAdmitError::Scalar,
        signal::Error::Unit => QacAdmitError::Unit,
        signal::Error::Clock => QacAdmitError::Clock,
        signal::Error::Overflow => QacAdmitError::Overflow,
        signal::Error::Nonfinite => QacAdmitError::Nonfinite,
        signal::Error::Index => QacAdmitError::Index,
        signal::Error::Limit => QacAdmitError::Limit,
    }
}
impl<'a> QacCompleteView<'a> {
    /// Whole selected entry only. `QacStorageView::open` dispatches on magic/version bytes.
    pub fn admit(
        range: AuthenticatedRange<'a>,
        options: QacStorageOptions,
    ) -> Result<Self, QacAdmitError> {
        if range.offset() != 0 || range.bytes().len() as u64 != range.selected().logical_length() {
            return Err(QacAdmitError::Extent);
        }
        Ok(Self {
            range,
            storage: QacStorageView::open(range.bytes(), options)?,
        })
    }
    pub const fn range(&self) -> AuthenticatedRange<'a> {
        self.range
    }
    pub fn storage(&self) -> &QacStorageView<'a> {
        &self.storage
    }
    /// Scientific interpretation. Storage admit already succeeded.
    /// Unsupported / Profile / unknown critical → `QacAdmitError::Unsupported`
    /// (caller may still keep the complete entry as OpaqueArchival).
    pub fn admit_signal<'s>(
        &'s self,
        selected: u32,
        scratch: &mut field_semantics::Scratch<'_>,
        shared: field_semantics::Limits,
        limits: signal::Limits,
    ) -> Result<signal::SignalView<'s>, QacAdmitError> {
        signal::admit(
            self.storage().structure(),
            selected,
            scratch,
            shared,
            limits,
        )
        .map_err(map_signal_error)
    }
}
