//! Current QAC envelope admission. Physical interpretation requires a typed reader.
use crate::q42::field_envelope::{self, Family, Limits, StructuralView};
#[derive(Debug, Clone, Copy, Default)]
pub struct QacStorageOptions {
    pub envelope_limits: Limits,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QacStorageError {
    Envelope(field_envelope::Error),
}
#[derive(Debug)]
pub struct QacStorageView<'a>(StructuralView<'a>);
impl<'a> QacStorageView<'a> {
    pub fn open(bytes: &'a [u8], options: QacStorageOptions) -> Result<Self, QacStorageError> {
        StructuralView::open(bytes, Family::QacV1Candidate, options.envelope_limits)
            .map(Self)
            .map_err(QacStorageError::Envelope)
    }
    pub fn structure(&self) -> &StructuralView<'a> {
        &self.0
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const CURRENT: &[u8] =
        include_bytes!("../../q42/field_envelope/fixtures/candidate/qac_candidate-opaque.qac");
    #[test]
    fn current_format_is_default_and_development_bytes_are_rejected() {
        let view = QacStorageView::open(CURRENT, QacStorageOptions::default()).unwrap();
        assert_eq!(view.structure().family(), Family::QacV1Candidate);
        view.structure().verify_chunk(1).unwrap();
        assert!(QacStorageView::open(
            include_bytes!("fixtures/q4au-v2-contract.qac"),
            QacStorageOptions::default()
        )
        .is_err());
    }
    #[test]
    fn invalid_current_version_is_not_reinterpreted() {
        let mut changed = CURRENT.to_vec();
        changed[8] = 99;
        assert_eq!(
            QacStorageView::open(&changed, QacStorageOptions::default()).unwrap_err(),
            QacStorageError::Envelope(field_envelope::Error::Version)
        );
    }
}
