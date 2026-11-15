use super::{Error, Metadata};
use crate::audio::qac::{
    metadata::FrequencyOffset,
    signal::{decode_sample, ExactTime, Sample},
};

pub(super) struct Window<'a> {
    pub bytes: &'a [u8],
    pub metadata: &'a Metadata,
    pub start: u64,
    pub count: u64,
    pub channel: u32,
}
impl Window<'_> {
    fn sample(&self, record: u64, bin: u32) -> Result<Sample, Error> {
        if record >= self.count || bin >= self.metadata.bins {
            return Err(Error::Index);
        }
        let scalar = self.metadata.scalar;
        let at = record
            .checked_mul(self.metadata.channels.into())
            .and_then(|n| n.checked_add(self.channel.into()))
            .and_then(|n| n.checked_mul(self.metadata.bins.into()))
            .and_then(|n| n.checked_add(bin.into()))
            .and_then(|n| n.checked_mul(scalar.bytes().into()))
            .ok_or(Error::Overflow)?;
        let at = usize::try_from(at).map_err(|_| Error::Overflow)?;
        let bytes = self
            .bytes
            .get(
                at..at
                    .checked_add(scalar.bytes() as usize)
                    .ok_or(Error::Overflow)?,
            )
            .ok_or(Error::Invariant)?;
        Ok(decode_sample(scalar, bytes)?)
    }
    fn time_at(&self, record: u64) -> Result<ExactTime, Error> {
        if record >= self.count {
            return Err(Error::Index);
        }
        Ok(self
            .metadata
            .timing
            .time_at(self.start.checked_add(record).ok_or(Error::Overflow)?)?)
    }
}
/// Borrowed time/channel PCM selection. Samples retain their exact scalar type.
/// Original bytes include all interleaved channels for the selected time range.
pub struct PcmWindow<'a> {
    pub(super) inner: Window<'a>,
}
impl PcmWindow<'_> {
    pub fn records(&self) -> u64 {
        self.inner.count
    }
    pub fn start_record(&self) -> u64 {
        self.inner.start
    }
    pub fn channel(&self) -> u32 {
        self.inner.channel
    }
    pub fn metadata(&self) -> &Metadata {
        self.inner.metadata
    }
    pub fn original_interleaved_bytes(&self) -> &[u8] {
        self.inner.bytes
    }
    pub fn sample(&self, record: u64) -> Result<Sample, Error> {
        self.inner.sample(record, 0)
    }
    pub fn time_at(&self, record: u64) -> Result<ExactTime, Error> {
        self.inner.time_at(record)
    }
}
/// Borrowed frame/channel STFT selection with original full-bin ordering.
/// Frequency is the signed mathematical FFT offset, never an inferred carrier.
pub struct StftWindow<'a> {
    pub(super) inner: Window<'a>,
}
impl StftWindow<'_> {
    pub fn records(&self) -> u64 {
        self.inner.count
    }
    pub fn start_record(&self) -> u64 {
        self.inner.start
    }
    pub fn channel(&self) -> u32 {
        self.inner.channel
    }
    pub fn bins(&self) -> u32 {
        self.inner.metadata.bins
    }
    pub fn metadata(&self) -> &Metadata {
        self.inner.metadata
    }
    pub fn original_interleaved_bytes(&self) -> &[u8] {
        self.inner.bytes
    }
    pub fn sample(&self, frame: u64, bin: u32) -> Result<Sample, Error> {
        self.inner.sample(frame, bin)
    }
    pub fn time_at(&self, frame: u64) -> Result<ExactTime, Error> {
        self.inner.time_at(frame)
    }
    pub fn frequency_offset(&self, bin: u32) -> Result<FrequencyOffset, Error> {
        Ok(self
            .inner
            .metadata
            .transform
            .ok_or(Error::Invariant)?
            .frequency_offset(bin)?)
    }
}
