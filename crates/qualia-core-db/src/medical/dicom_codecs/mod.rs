//! Explicit preserve/decode/interpret profiles; native integer samples are not display pixels.
pub mod encapsulated;
pub mod float_pixels;
pub mod j2k;
pub mod jpeg;
#[cfg(not(target_arch = "wasm32"))]
pub mod jpeg_12bit;
#[cfg(not(target_arch = "wasm32"))]
mod jpeg_12bit_scan;
pub mod jpegls;
pub mod jxl;
mod metadata;
mod native;
pub mod padding;
mod profile;
mod rle;

pub use encapsulated::{EncapError, EncapLimits};
pub use j2k::{
    annex_c_ht_vlc_tables, decode_ht_cblk_annex_c, decode_j2k_codestream_into,
    decode_j2k_part10_dataset_into, decode_j2k_part10_into, decode_j2k_part10_with_annex_c_into,
    encode_ht_cleanup_segment, HtVlcEntry, HtVlcTables, J2kError, J2kFrameInfo, J2kLimits,
    ProgressionOrder, WaveletTransform, ANNEX_C_HT_VLC_TABLES, ANNEX_C_TBL0, ANNEX_C_TBL1,
    TS_HTJ2K, TS_HTJ2K_LOSSLESS, TS_HTJ2K_RPCL, TS_J2K, TS_J2K_LOSSLESS,
};
pub use jpeg::{
    decode_jpeg_into, JpegColorFormat, JpegDecodedInfo, JpegError, JpegLimits,
    TS_JPEG_BASELINE, TS_JPEG_EXTENDED,
};
pub use jpegls::{
    decode_jpegls_codestream_into, decode_jpegls_part10_into, JpegLsError, JpegLsFrameInfo,
    JpegLsLimits, TS_JPEGLS_LOSSLESS, TS_JPEGLS_NEAR_LOSSLESS,
};
pub use jxl::{
    decode_jxl_codestream_into, decode_jxl_part10_into, encode_jbrd_box_from_jpeg,
    extract_jbrd_payload, reconstruct_jpeg_from_jbrd, reconstruct_jpeg_from_jxl_container,
    JbrdExtractedCoeffs, JxlDecodedCodestream, JxlEncoding, JxlError, JxlFrameInfo, JxlFrameType,
    JxlImageHeader, JxlLimits, JxlTransferFunction, TS_JXL, TS_JXL_JPEG_RECOMPRESSION,
    TS_JXL_LOSSLESS,
};
pub use native::{NativePixelView, PixelError, StoredSample};
pub use profile::{transfer_syntax_profile, TransferSyntaxProfile};
pub use rle::{decode_rle_lossless_into, RleError, RleLimits};

#[cfg(test)]
mod tests;
