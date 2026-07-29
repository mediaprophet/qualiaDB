//! Laplacian variance blur score (higher = sharper).

use crate::cv::buffer::GrayView;

pub fn frame_blur_score(src: GrayView<'_>) -> f32 {
    if src.width < 3 || src.height < 3 {
        return 0.0;
    }
    let mut sum = 0.0f32;
    let mut sum2 = 0.0f32;
    let mut n = 0.0f32;
    for y in 1..src.height - 1 {
        for x in 1..src.width - 1 {
            let c = src.pixel(x, y) as f32;
            let lap = src.pixel(x + 1, y) as f32
                + src.pixel(x - 1, y) as f32
                + src.pixel(x, y + 1) as f32
                + src.pixel(x, y - 1) as f32
                - 4.0 * c;
            sum += lap;
            sum2 += lap * lap;
            n += 1.0;
        }
    }
    if n < 1.0 {
        return 0.0;
    }
    let mean = sum / n;
    (sum2 / n - mean * mean).max(0.0)
}
