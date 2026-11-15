//! Honest capability & preservation profiles across DICOM Transfer Syntaxes.
//! No profile claims pixel interpretation from source preservation alone.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransferSyntaxProfile {
    pub preserve_original: bool,
    pub nested_dataset: bool,
    pub native_integer_samples: bool,
    pub encapsulated_decode: bool,
    pub limitation: Option<&'static str>,
}

/// Return exact preservation, dataset, and decode guarantees for a Transfer Syntax UID.
pub fn transfer_syntax_profile(uid: &str) -> TransferSyntaxProfile {
    let native = matches!(uid, "1.2.840.10008.1.2" | "1.2.840.10008.1.2.1");
    let rle = uid == "1.2.840.10008.1.2.5";
    let jpeg_base = uid == "1.2.840.10008.1.2.4.50";
    let jpeg_ext = uid == "1.2.840.10008.1.2.4.51";
    let jpegls = matches!(uid, "1.2.840.10008.1.2.4.80" | "1.2.840.10008.1.2.4.81");
    let j2k = matches!(uid, "1.2.840.10008.1.2.4.90" | "1.2.840.10008.1.2.4.91");
    let htj2k = matches!(
        uid,
        "1.2.840.10008.1.2.4.201" | "1.2.840.10008.1.2.4.202" | "1.2.840.10008.1.2.4.203"
    );
    let jxl_pixel = matches!(uid, "1.2.840.10008.1.2.4.110" | "1.2.840.10008.1.2.4.112");
    let jxl_recomp = uid == "1.2.840.10008.1.2.4.111";

    let encapsulated_known = rle || jpeg_base || jpeg_ext || jpegls || j2k || htj2k || jxl_pixel || jxl_recomp;
    let encapsulated_decode = rle || jpeg_base || jpeg_ext || jpegls || j2k || jxl_pixel || jxl_recomp;

    let limitation = if native {
        Some("integer MONOCHROME1/2, BitsAllocated8/16/32; floats/packed1-bit/color not selected")
    } else if rle {
        Some("caller-buffered RLE Lossless MONOCHROME1/2,8/16bit; other RLE photometric/packed profiles unsupported")
    } else if jpeg_base {
        Some("caller-buffered JPEG Baseline 8-bit MONOCHROME1/2, RGB, YBR_FULL/422; corrupted/mismatched fragments fail closed")
    } else if jpeg_ext {
        Some("caller-buffered JPEG Extended 8-bit & pure-Rust 12-bit SOF1 DctSequential MONOCHROME1/2, RGB, YBR_FULL/422")
    } else if jpegls {
        Some("pure-Rust ITU-T T.87 JPEG-LS (.80 lossless / .81 near-lossless) MONOCHROME1/2, RGB/YBR, 2..=16-bit")
    } else if j2k {
        Some("pure-Rust ISO 15444-1 JPEG 2000 (.90 5/3 lossless, .91 9/7 & 5/3) MQ/EBCOT/RCT/ICT with multi-tile-part SOT concatenation")
    } else if htj2k {
        Some("HTJ2K framing/MEL/MagSgn & ITU-T T.814 Annex C CxtVLC tables implemented (decode_j2k_part10_with_annex_c_into); None table arg stays fail-closed")
    } else if jxl_pixel {
        Some("pure-Rust ISO 18181-1 JPEG-XL Modular (MA/WP/Palette/RCT/Squeeze), VarDCT (IDCT/CfL/Gaborish/EPF/XYB), and 12-bit multi-bucket rANS")
    } else if jxl_recomp {
        Some("decodes JXL pixel codestream, extracts jbrd box, and reconstructs byte-exact legacy JPEG via reconstruct_jpeg_from_jbrd")
    } else {
        Some("archive-only unsupported dataset transfer syntax")
    };

    TransferSyntaxProfile {
        preserve_original: true,
        nested_dataset: native || encapsulated_known,
        native_integer_samples: native,
        encapsulated_decode,
        limitation: Some(limitation.expect("all branches populate Some")),
    }
}
