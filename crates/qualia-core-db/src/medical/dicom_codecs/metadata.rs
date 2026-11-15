use super::PixelError;
use crate::medical::dicom_dataset::{Part10Dataset, WorkBudget};
/// Shared admitted native-layout metadata. No decoder may invent these values.
pub(super) struct IntegerImageInfo<'a> {
    pub source_blake3: [u8; 32],
    pub rows: u16,
    pub columns: u16,
    pub frames: u32,
    pub bits_allocated: u16,
    pub bits_stored: u16,
    pub signed: bool,
    pub photometric: &'a str,
    pub samples_per_frame: usize,
    pub padding_declared: bool,
    pub bytes: usize,
}
impl<'a> IntegerImageInfo<'a> {
    pub fn from_dataset(
        ds: &Part10Dataset<'a>,
        max_pixel_bytes: usize,
        w: &mut WorkBudget,
    ) -> Result<Self, PixelError> {
        if ds.find(None, 0x7fe00008, w)?.is_some() || ds.find(None, 0x7fe00009, w)?.is_some() {
            return Err(PixelError::Unsupported(
                "float pixel data is not selected by integer view",
            ));
        }
        let required = |tag, w: &mut WorkBudget| {
            ds.unsigned_short(tag, w)
                .map_err(PixelError::from)?
                .ok_or(PixelError::Missing(tag))
        };
        let rows = required(0x00280010, w)?;
        let columns = required(0x00280011, w)?;
        if rows == 0 || columns == 0 {
            return Err(PixelError::Invalid(0x00280010));
        }
        if required(0x00280002, w)? != 1 {
            return Err(PixelError::Unsupported("samples per pixel other than1"));
        }
        let photometric = ds
            .text(None, 0x00280004, *b"CS", w)?
            .ok_or(PixelError::Missing(0x00280004))?;
        if !matches!(photometric, "MONOCHROME1" | "MONOCHROME2") {
            return Err(PixelError::Unsupported(
                "non-monochrome photometric interpretation",
            ));
        }
        let bits_allocated = required(0x00280100, w)?;
        if !matches!(bits_allocated, 8 | 16 | 32) {
            return Err(PixelError::Unsupported("BitsAllocated outside8/16/32"));
        }
        let bits_stored = required(0x00280101, w)?;
        let high = required(0x00280102, w)?;
        let rep = required(0x00280103, w)?;
        if bits_stored == 0 || bits_stored > bits_allocated || high != bits_stored - 1 || rep > 1 {
            return Err(PixelError::Invalid(0x00280101));
        }
        let padding_declared =
            ds.find(None, 0x00280120, w)?.is_some() || ds.find(None, 0x00280121, w)?.is_some();
        let frames = ds.frame_count(w)?;
        let samples_per_frame = (rows as usize)
            .checked_mul(columns as usize)
            .ok_or(PixelError::Overflow)?;
        let need = samples_per_frame
            .checked_mul(frames as usize)
            .and_then(|v| v.checked_mul(bits_allocated as usize / 8))
            .ok_or(PixelError::Overflow)?;
        if need > max_pixel_bytes {
            return Err(PixelError::ByteLimit);
        }

        Ok(Self {
            source_blake3: ds.source_blake3(),
            rows,
            columns,
            frames,
            bits_allocated,
            bits_stored,
            signed: rep == 1,
            photometric,
            samples_per_frame,
            padding_declared,
            bytes: need,
        })
    }
}
