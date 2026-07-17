//! Fail-closed quality gate for biosense windows.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityReject {
    Ok,
    TooBlurry,
    TooMuchMotion,
    InsufficientSignal,
}

/// `blur_score` from laplacian variance; `motion` mean abs diff; thresholds excellence-tuned defaults.
pub fn reject_low_quality(blur_score: f32, motion: f32, min_blur: f32, max_motion: f32) -> QualityReject {
    if blur_score < min_blur {
        return QualityReject::TooBlurry;
    }
    if motion > max_motion {
        return QualityReject::TooMuchMotion;
    }
    QualityReject::Ok
}
