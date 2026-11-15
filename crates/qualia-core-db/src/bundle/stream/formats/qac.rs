use super::{AuthenticatedRange, FormatError};
use crate::audio::qac::{QacStorageError, QacStorageOptions, QacStorageView};
use crate::q42::field_envelope;
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
}
