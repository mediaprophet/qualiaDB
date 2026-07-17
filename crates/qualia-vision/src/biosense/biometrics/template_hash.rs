//! Face template as FNV-style digest of ROI pixels (not production embedding).
//! Excellence path replaces with licensed embedding under sanctuary crypto.

use crate::biosense::consent::BiosenseConsent;
use crate::biosense::face::FaceRoi;
use crate::cv::buffer::RgbView;
use crate::cv::error::CvError;

#[derive(Debug, Clone, Copy)]
pub struct BiometricTemplate {
    pub hash: u64,
    pub principal_hash: u64,
    pub method: &'static str,
}

pub fn template_hash_from_roi(
    consent: BiosenseConsent,
    src: RgbView<'_>,
    roi: FaceRoi,
) -> Result<BiometricTemplate, CvError> {
    if !consent.may_process() || !consent.allow_store_template {
        return Err(CvError::InvalidParameter);
    }
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let x2 = (roi.x + roi.w).min(src.width);
    let y2 = (roi.y + roi.h).min(src.height);
    for y in roi.y..y2 {
        for x in roi.x..x2 {
            let (r, g, b) = src.pixel(x, y);
            h ^= r as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
            h ^= g as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
            h ^= b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
    }
    Ok(BiometricTemplate {
        hash: h,
        principal_hash: consent.principal_hash,
        method: "roi_fnv_proxy_v1_not_production_embedding",
    })
}

pub fn templates_match(a: BiometricTemplate, b: BiometricTemplate) -> bool {
    a.hash == b.hash && a.principal_hash == b.principal_hash
}
