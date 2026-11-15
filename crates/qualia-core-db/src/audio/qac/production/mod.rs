//! Current-format production and immutable publication from actual signal data.
//! No preview-derived identities, legacy aliases, invented PCM, calibration or
//! transform conventions. Current Hann-provider output is not relabeled rectangular.
#![cfg(not(target_arch = "wasm32"))]
use crate::audio::qac::{
    persistence::{self, QacArtifact, QacPublishError, QacPublishOptions},
    signal::{
        self,
        stft::{StftDocument, StftWriteBuffers, StftWriteOptions},
        writer::{PcmDocument, WriteBuffers, WriteOptions},
    },
    QacStorageOptions,
};
use crate::q42::{
    field_envelope::{self as env, Family, StructuralView},
    field_semantics as sem,
};
use std::path::Path;
#[derive(Clone, Copy, Debug)]
pub struct ProductionOptions {
    pub writer: WriteOptions,
    /// Per published final object, including headers/metadata/padding. Not total disk quota.
    pub publication_max_bytes: usize,
}
impl Default for ProductionOptions {
    fn default() -> Self {
        Self {
            writer: WriteOptions::default(),
            publication_max_bytes: 256 << 20,
        }
    }
}
#[derive(Debug)]
pub enum Error {
    Signal(signal::Error),
    Publication(QacPublishError),
}
impl From<signal::Error> for Error {
    fn from(e: signal::Error) -> Self {
        Self::Signal(e)
    }
}
impl From<QacPublishError> for Error {
    fn from(e: QacPublishError) -> Self {
        Self::Publication(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Signal(e) => write!(f, "current QAC signal admission: {e:?}"),
            Self::Publication(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for Error {}
impl ProductionOptions {
    fn effective_writer(self) -> Result<WriteOptions, Error> {
        let n = u64::try_from(self.publication_max_bytes).map_err(|_| signal::Error::Overflow)?;
        let mut writer = self.writer;
        writer.envelope.file_bytes = writer.envelope.file_bytes.min(n);
        Ok(writer)
    }
    fn publication(self, limits: env::Limits) -> QacPublishOptions {
        QacPublishOptions {
            max_bytes: self.publication_max_bytes,
            storage: QacStorageOptions {
                envelope_limits: limits,
            },
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct StftProductionOptions {
    pub writer: StftWriteOptions,
    pub publication_max_bytes: usize,
}
impl Default for StftProductionOptions {
    fn default() -> Self {
        Self {
            writer: StftWriteOptions::default(),
            publication_max_bytes: 256 << 20,
        }
    }
}
impl From<ProductionOptions> for StftProductionOptions {
    fn from(opts: ProductionOptions) -> Self {
        Self {
            writer: StftWriteOptions {
                envelope: opts.writer.envelope,
                shared: opts.writer.shared,
                signal: opts.writer.signal,
                ..StftWriteOptions::default()
            },
            publication_max_bytes: opts.publication_max_bytes,
        }
    }
}
impl StftProductionOptions {
    fn effective_writer(self) -> Result<StftWriteOptions, Error> {
        let n = u64::try_from(self.publication_max_bytes).map_err(|_| signal::Error::Overflow)?;
        let mut writer = self.writer;
        writer.envelope.file_bytes = writer.envelope.file_bytes.min(n);
        Ok(writer)
    }
    fn publication(self, limits: env::Limits) -> QacPublishOptions {
        QacPublishOptions {
            max_bytes: self.publication_max_bytes,
            storage: QacStorageOptions {
                envelope_limits: limits,
            },
        }
    }
}

/// Encode actual Hann STFT + retained f32 PCM via `etalon_signal`, fully admit all entries,
/// and publish immutably by whole-file BLAKE3 digest.
pub fn produce_stft(
    root: &Path,
    document: &StftDocument<'_>,
    buffers: StftWriteBuffers<'_>,
    scratch: &mut sem::Scratch<'_>,
    final_bytes: &mut [u8],
    options: StftProductionOptions,
) -> Result<QacArtifact, Error> {
    let writer = options.effective_writer()?;
    let n = document.encode_into(buffers, scratch, final_bytes, writer)?;
    Ok(persistence::publish_qac(
        root,
        &final_bytes[..n],
        options.publication(writer.envelope),
    )?)
}

/// Encode actual PCM and its supplied metadata, fully admit the result, and return
/// a full-final-byte BLAKE3 artifact only after canonical no-clobber publication.
/// Encoding errors leave final bytes unchanged. On publication IO/conflict errors,
/// final bytes may contain the valid encoded object, available for an explicit retry.
/// Caller controls a trusted exclusive root; publisher filesystem guarantees apply.
/// No numeric value is inferred from a tensor/colour preview or hearing-band rule.
pub fn produce_pcm(
    root: &Path,
    document: &PcmDocument<'_>,
    buffers: WriteBuffers<'_>,
    scratch: &mut sem::Scratch<'_>,
    final_bytes: &mut [u8],
    options: ProductionOptions,
) -> Result<QacArtifact, Error> {
    let writer = options.effective_writer()?;
    // PcmDocument performs whole-object typed admission before the final prefix copy.
    let n = document.encode_into(buffers, scratch, final_bytes, writer)?;
    Ok(persistence::publish_qac(
        root,
        &final_bytes[..n],
        options.publication(writer.envelope),
    )?)
}
/// Publish an already encoded current PCM/STFT document only after complete typed
/// admission. Unsupported/malformed semantics never create a file or directory.
/// The typed profile must contain at least one actual PCM or admitted STFT entry.
/// Original raw sources are preserved as opaque entries; this does not transcode.
/// Current storage enum spelling does not enable old-format fallback or aliases.
pub fn publish_current_signal(
    root: &Path,
    bytes: &[u8],
    scratch: &mut sem::Scratch<'_>,
    options: ProductionOptions,
) -> Result<QacArtifact, Error> {
    if bytes.len() > options.publication_max_bytes {
        return Err(QacPublishError::Budget.into());
    }
    let writer = options.effective_writer()?;
    let view = StructuralView::open(bytes, Family::QacV1Candidate, writer.envelope)
        .map_err(signal::Error::from)?;
    if view.entry_count() > writer.signal.entries {
        return Err(signal::Error::Limit.into());
    }
    let mut remaining_work = writer.signal.work;
    let mut selected = None;
    for index in 1..=view.entry_count() {
        remaining_work = remaining_work.checked_sub(1).ok_or(signal::Error::Limit)?;
        let e = view.entry(index).map_err(signal::Error::from)?;
        if matches!(e.mode(), 0x100 | 0x101) {
            selected = Some(index);
            break;
        }
    }
    let mut signal_limits = writer.signal;
    signal_limits.work = remaining_work;
    signal::admit(
        &view,
        selected.ok_or(signal::Error::Unsupported)?,
        scratch,
        writer.shared,
        signal_limits,
    )?;
    Ok(persistence::publish_qac(
        root,
        bytes,
        options.publication(writer.envelope),
    )?)
}
#[cfg(test)]
mod tests;
