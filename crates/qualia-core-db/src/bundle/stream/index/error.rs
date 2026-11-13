//! Fixed-size errors; conversion to historical owned errors is cold only.
use crate::bundle::BundleError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndexError {
    HeaderLength,
    BadMagic,
    UnsupportedVersion(u16),
    Flags,
    Reserved,
    Extent,
    ExtentLimit,
    Overflow,
    Footer,
    Alignment,
    EncodedLimit,
    Count,
    ResidentLimit,
    Malformed,
    Field,
    DuplicateField,
    MissingField,
    Text,
    Digest,
    EntryBounds,
    DuplicateKey,
    Overlap,
    Scratch,
    Allocation,
}

impl IndexError {
    pub(crate) fn into_bundle_error(self) -> BundleError {
        match self {
            Self::HeaderLength => BundleError::TooShort,
            Self::BadMagic => BundleError::BadMagic,
            Self::UnsupportedVersion(v) => BundleError::UnsupportedVersion(v),
            _ => BundleError::Cbor(self.message().into()),
        }
    }

    pub const fn message(self) -> &'static str {
        match self {
            Self::HeaderLength => "QBDL header must be exactly 64 bytes",
            Self::BadMagic => "QBDL magic differs",
            Self::UnsupportedVersion(_) => "QBDL version unsupported",
            Self::Flags => "QBDL flags unsupported",
            Self::Reserved => "QBDL reserved header bytes are nonzero",
            Self::Extent => "QBDL logical extent differs",
            Self::ExtentLimit => "QBDL logical extent exceeds limit",
            Self::Overflow => "QBDL arithmetic overflow",
            Self::Footer => "QBDL index is not the exact footer",
            Self::Alignment => "QBDL alignment differs",
            Self::EncodedLimit => "QBDL index exceeds encoded limit",
            Self::Count => "QBDL entry count differs or exceeds limit",
            Self::ResidentLimit => "QBDL index exceeds aggregate resident reservation",
            Self::Malformed => "QBDL index encoding is malformed or unsupported",
            Self::Field => "QBDL index field is unknown or invalid",
            Self::DuplicateField => "QBDL index field occurs twice",
            Self::MissingField => "QBDL index required field is missing",
            Self::Text => "QBDL index text is empty or exceeds limit",
            Self::Digest => "QBDL entry requires exactly 32 digest bytes",
            Self::EntryBounds => "QBDL entry exceeds payload bounds",
            Self::DuplicateKey => "QBDL entry key occurs twice",
            Self::Overlap => "QBDL nonempty payload ranges overlap",
            Self::Scratch => "QBDL caller scratch is insufficient",
            Self::Allocation => "QBDL cold index allocation unavailable",
        }
    }
}

impl core::fmt::Display for IndexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.message())
    }
}
impl std::error::Error for IndexError {}
