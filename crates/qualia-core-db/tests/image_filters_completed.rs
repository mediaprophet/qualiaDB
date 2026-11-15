//! Independent public-API cases for the four deferred image filters.

use qualia_core_db::hypermedia_authoring::image::{
    filter_plan, filter_rgba8_into, ImageError, ImageFilterOp, ImageLimits,
};

fn limits() -> ImageLimits {
    ImageLimits {
        max_pixels: 4 * 1024 * 1024,
        max_work: 1024 * 1024 * 1024,
        max_bytes: 256 * 1024 * 1024,
    }
}

fn run(src: &[[u8; 4]], w: u32, h: u32, op: ImageFilterOp) -> Vec<[u8; 4]> {
    let plan = filter_plan(w, h, op, limits()).unwrap();
    let mut out = vec![[77; 4]; src.len()];
    let mut scratch = vec![[0.0; 4]; plan.scratch_pixels];
    filter_rgba8_into(src, w, h, op, limits(), &mut out, &mut scratch).unwrap();
    out
}

#[test]
fn gaussian_impulse_is_symmetric_conservative_and_alpha_aware() {
    // Radius three fits on both sides of the impulse, so clamped boundaries
    // cannot fold the impulse out of the image. These are independent rounded
    // samples of the normalized sigma=1 Gaussian.
    let mut src = [[0, 0, 0, 255]; 9];
    src[4] = [255, 255, 255, 255];
    let out = run(&src, 9, 1, ImageFilterOp::GaussianBlur { sigma: 1.0 });
    let expected = [0, 1, 14, 62, 102, 62, 14, 1, 0];
    assert_eq!(out.iter().map(|p| p[0]).collect::<Vec<_>>(), expected);
    assert!(out.iter().zip(out.iter().rev()).all(|(a, b)| a == b));
    assert!((out.iter().map(|p| p[0] as i32).sum::<i32>() - 255).abs() <= 2);
    assert!(out.iter().all(|p| p[3] == 255));

    let transparent_red = [[255, 0, 0, 0], [0, 0, 255, 255], [255, 0, 0, 0]];
    let out = run(&transparent_red, 3, 1, ImageFilterOp::GaussianBlur { sigma: 1.0 });
    assert!(out.iter().all(|p| p[0] == 0 && p[1] == 0 && p[2] == 255));
    assert!(out.iter().all(|p| p[3] > 0));
}

#[test]
fn gaussian_and_sharpen_tiny_images_and_exact_identities() {
    let single = [[12, 34, 56, 90]];
    assert_eq!(run(&single, 1, 1, ImageFilterOp::GaussianBlur { sigma: 2.0 }), single);
    assert_eq!(run(&single, 1, 1, ImageFilterOp::Sharpen { sigma: 1.0, amount: 3.0 }), single);
    let hidden_color = [[200, 100, 50, 0]];
    assert_eq!(run(&hidden_color, 1, 1, ImageFilterOp::GaussianBlur { sigma: 0.0 }), hidden_color);
    assert_eq!(run(&hidden_color, 1, 1, ImageFilterOp::Sharpen { sigma: 1.0, amount: 0.0 }), hidden_color);
    assert_eq!(run(&hidden_color, 1, 1, ImageFilterOp::Sharpen { sigma: 0.0, amount: 4.0 }), hidden_color);
    assert_eq!(run(&hidden_color, 1, 1, ImageFilterOp::GaussianBlur { sigma: 1.0 }), [[0, 0, 0, 0]]);
}

#[test]
fn sharpen_preserves_alpha_and_increases_opaque_impulse_contrast() {
    let src = [[0, 0, 0, 64], [128, 128, 128, 128], [0, 0, 0, 255]];
    let out = run(&src, 3, 1, ImageFilterOp::Sharpen { sigma: 1.0, amount: 1.0 });
    assert_eq!(out.iter().map(|p| p[3]).collect::<Vec<_>>(), vec![64, 128, 255]);
    assert!(out[1][0] > 128);
    assert_eq!(out[0][0], 0);
    assert_eq!(out[2][0], 0);
}

#[test]
fn hue_uses_svg_matrix_and_preserves_achromatics_alpha_and_turn_identity() {
    let src = [[255, 0, 0, 101], [90, 90, 90, 0]];
    let out = run(&src, 2, 1, ImageFilterOp::HueRotate { degrees: 90.0 });
    assert_eq!(out[0], [0, 91, 0, 101]);
    assert_eq!(out[1], src[1]);
    assert_eq!(run(&src, 2, 1, ImageFilterOp::HueRotate { degrees: 360.0 }), src);
    assert_eq!(run(&src, 2, 1, ImageFilterOp::HueRotate { degrees: -360.0 }), src);
}

#[test]
fn emboss_uses_signed_central_gradient_and_clamped_edges() {
    let src = [[0, 0, 0, 10], [128, 128, 128, 20], [255, 255, 255, 30]];
    let out = run(&src, 3, 1, ImageFilterOp::Emboss { azimuth_degrees: 0.0, strength: 1.0 });
    assert_eq!(out, [[192, 192, 192, 10], [255, 255, 255, 20], [192, 192, 192, 30]]);
    let reverse = run(&src, 3, 1, ImageFilterOp::Emboss { azimuth_degrees: 180.0, strength: 1.0 });
    assert_eq!(reverse[1], [1, 1, 1, 20]);
    assert_eq!(run(&src, 3, 1, ImageFilterOp::Emboss { azimuth_degrees: 0.0, strength: 0.0 }), src);
    assert_eq!(run(&src[..1], 1, 1, ImageFilterOp::Emboss { azimuth_degrees: 0.0, strength: 1.0 }), [[128, 128, 128, 10]]);
}

#[test]
fn hue_and_emboss_order_is_observably_noncommuting() {
    // One pixel makes the clamped left/right/up/down luminances equal, so a
    // full-strength emboss relief is exactly 128. At half strength it averages
    // each input channel with 128. For red, the SVG 90-degree hue matrix gives
    // (0,91,0). Thus hue then emboss gives (64,110,64). Emboss first gives
    // (192,64,64); multiplying that by the SVG 90-degree matrix, with byte
    // clipping, gives (64,110,0). Alpha passes through both operations.
    let red = [[255, 0, 0, 137]];
    let hue = ImageFilterOp::HueRotate { degrees: 90.0 };
    let emboss = ImageFilterOp::Emboss { azimuth_degrees: 0.0, strength: 0.5 };
    let hue_then_emboss = run(&run(&red, 1, 1, hue), 1, 1, emboss);
    let emboss_then_hue = run(&run(&red, 1, 1, emboss), 1, 1, hue);
    assert_eq!(hue_then_emboss, [[64, 110, 64, 137]]);
    assert_eq!(emboss_then_hue, [[64, 110, 0, 137]]);
    assert_ne!(hue_then_emboss, emboss_then_hue);
}

#[test]
fn preflight_refuses_invalid_inputs_without_changing_output() {
    let src = [[2, 3, 4, 5]];
    let operations = [
        ImageFilterOp::GaussianBlur { sigma: f32::NAN },
        ImageFilterOp::GaussianBlur { sigma: 16.1 },
        ImageFilterOp::Sharpen { sigma: 1.0, amount: f32::INFINITY },
        ImageFilterOp::HueRotate { degrees: -361.0 },
        ImageFilterOp::Emboss { azimuth_degrees: 0.0, strength: 1.1 },
    ];
    for op in operations {
        let mut out = [[9; 4]];
        assert_eq!(filter_rgba8_into(&src, 1, 1, op, limits(), &mut out, &mut []), Err(ImageError::Parameters));
        assert_eq!(out, [[9; 4]]);
    }
    let op = ImageFilterOp::GaussianBlur { sigma: 1.0 };
    let mut out = [[9; 4]];
    assert_eq!(filter_rgba8_into(&src, 0, 1, op, limits(), &mut out, &mut []), Err(ImageError::Dimensions));
    assert_eq!(filter_rgba8_into(&[], 1, 1, op, limits(), &mut out, &mut []), Err(ImageError::Storage));
    assert_eq!(filter_rgba8_into(&src, 1, 1, op, limits(), &mut [], &mut []), Err(ImageError::Output));
    assert_eq!(filter_rgba8_into(&src, 1, 1, op, limits(), &mut out, &mut []), Err(ImageError::Scratch));
    assert_eq!(out, [[9; 4]]);
    let tight = ImageLimits { max_pixels: 1, max_work: 100, max_bytes: 23 };
    assert_eq!(filter_rgba8_into(&src, 1, 1, op, tight, &mut out, &mut []), Err(ImageError::Budget));
    assert_eq!(out, [[9; 4]]);
    assert!(matches!(
        filter_plan(u32::MAX, u32::MAX, op, limits()),
        Err(ImageError::Budget | ImageError::Dimensions)
    ));
    assert_eq!(filter_plan(1, 1, op, ImageLimits { max_pixels: 1, max_work: 1, max_bytes: 100 }).err(), Some(ImageError::Budget));
    assert_eq!(filter_plan(1, 1, op, ImageLimits { max_pixels: 1, max_work: 100, max_bytes: 23 }).err(), Some(ImageError::Budget));
}
