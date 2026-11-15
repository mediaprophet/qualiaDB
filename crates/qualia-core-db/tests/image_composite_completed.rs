//! Public API qualification vectors for the nine blend modes, masks and offsets.
use qualia_core_db::hypermedia_authoring::image::{
    composite_into, validate_document, BlendMode, ImageDocument, ImageError, ImageLayer, ImageLimits,
};

fn limits() -> ImageLimits {
    ImageLimits { max_pixels: 64, max_work: 256, max_bytes: 1024 }
}

fn layer(id: &str, rgba: [u8; 4]) -> ImageLayer {
    let mut layer = ImageLayer::new(id, id, 1, 1);
    layer.pixels[0] = rgba;
    layer
}

fn pair(backdrop: [u8; 4], source: [u8; 4]) -> ImageDocument {
    let mut doc = ImageDocument::new("vector", 1, 1);
    doc.layers.push(layer("backdrop", backdrop));
    doc.layers.push(layer("source", source));
    doc
}

fn pixel(doc: &ImageDocument) -> [u8; 4] {
    let mut out = [[0; 4]];
    composite_into(doc, &mut out, limits()).unwrap();
    out[0]
}

#[test]
fn opaque_known_color_vectors_for_each_non_normal_mode() {
    // Independently evaluated W3C separable equations for Cb=(64,128,192)
    // and Cs=(192,96,32), both opaque. The modes must yield distinct results.
    let vectors = [
        (BlendMode::Multiply, [48, 48, 24, 255]),
        (BlendMode::Screen, [208, 176, 200, 255]),
        (BlendMode::Overlay, [96, 97, 145, 255]),
        (BlendMode::SoftLight, [96, 112, 156, 255]),
        (BlendMode::HardLight, [161, 96, 48, 255]),
        (BlendMode::ColorDodge, [255, 205, 220, 255]),
        (BlendMode::ColorBurn, [1, 0, 0, 255]),
        (BlendMode::Difference, [128, 32, 160, 255]),
        (BlendMode::Exclusion, [160, 128, 176, 255]),
    ];
    for (mode, expected) in vectors {
        let mut doc = pair([64, 128, 192, 255], [192, 96, 32, 255]);
        doc.layers[1].blend_mode = mode;
        assert_eq!(pixel(&doc), expected, "{mode:?}");
    }
}

#[test]
fn alpha_interpolation_and_transparent_backdrop() {
    let mut doc = pair([64, 128, 192, 128], [192, 96, 32, 128]);
    doc.layers[1].blend_mode = BlendMode::Multiply;
    assert_eq!(pixel(&doc), [101, 91, 83, 192]);
    doc.layers[0].pixels[0] = [250, 1, 2, 0];
    assert_eq!(pixel(&doc), [192, 96, 32, 128]);
}

#[test]
fn every_blend_mode_has_independent_partial_alpha_and_transparent_backdrop_vectors() {
    // Cb=(64,128,192)/255, Cs=(192,96,32)/255, Ab=As=128/255.
    // The straight output channel before rounding is
    //   [127*(Cb_byte + Cs_byte) + 128*B_byte] / 382,
    // where B is the W3C mode result at those unrounded input fractions.
    // Output alpha is round(255*(2a-a*a)) = 192. The fixed byte vectors
    // below were evaluated independently from that arithmetic, including
    // the cubic/square-root SoftLight and dodge/burn endpoint cases.
    let vectors = [
        (BlendMode::Multiply, [101, 91, 83, 192]),
        (BlendMode::Screen, [155, 133, 141, 192]),
        (BlendMode::Overlay, [117, 107, 123, 192]),
        (BlendMode::SoftLight, [117, 112, 127, 192]),
        (BlendMode::HardLight, [139, 107, 91, 192]),
        (BlendMode::ColorDodge, [171, 143, 148, 192]),
        (BlendMode::ColorBurn, [86, 74, 74, 192]),
        (BlendMode::Difference, [128, 85, 128, 192]),
        (BlendMode::Exclusion, [139, 117, 133, 192]),
    ];
    for (mode, expected) in vectors {
        let mut doc = pair([64, 128, 192, 128], [192, 96, 32, 128]);
        doc.layers[1].blend_mode = mode;
        assert_eq!(pixel(&doc), expected, "partial alpha {mode:?}");

        // A stored RGB value under zero backdrop alpha has no authority over
        // the source colour, even for modes whose B(Cb,Cs) differs sharply.
        doc.layers[0].pixels[0] = [250, 1, 2, 0];
        assert_eq!(pixel(&doc), [192, 96, 32, 128], "transparent backdrop {mode:?}");
    }
}

#[test]
fn dodge_burn_endpoint_rules() {
    let mut doc = pair([0, 255, 128, 255], [255, 0, 255, 255]);
    doc.layers[1].blend_mode = BlendMode::ColorDodge;
    assert_eq!(pixel(&doc), [0, 255, 255, 255]);
    doc.layers[1].blend_mode = BlendMode::ColorBurn;
    assert_eq!(pixel(&doc), [0, 255, 128, 255]);
}

#[test]
fn mask_coverage_and_opacity_are_applied_once() {
    let mut doc = pair([64, 128, 192, 128], [192, 96, 32, 128]);
    doc.layers[1].mask = Some(vec![0]);
    assert_eq!(pixel(&doc), [64, 128, 192, 128]);
    doc.layers[1].mask = Some(vec![255]);
    doc.layers[1].opacity = 0.5;
    assert_eq!(pixel(&doc), [115, 115, 128, 160]);
    doc.layers[1].mask = Some(vec![128]);
    assert_eq!(pixel(&doc), [93, 121, 156, 144]);
}

#[test]
fn signed_offsets_clip_in_layer_coordinates() {
    let mut doc = ImageDocument::new("offset", 2, 1);
    let mut layer = ImageLayer::new("l", "l", 2, 1);
    layer.pixels = vec![[255, 0, 0, 255], [0, 255, 0, 255]];
    layer.offset_x = -1;
    doc.layers.push(layer);
    let mut out = [[0; 4]; 2];
    composite_into(&doc, &mut out, limits()).unwrap();
    assert_eq!(out, [[0, 255, 0, 255], [0; 4]]);
    doc.layers[0].offset_x = i32::MIN;
    composite_into(&doc, &mut out, limits()).unwrap();
    assert_eq!(out, [[0; 4]; 2]);
    doc.layers[0].offset_x = i32::MAX;
    composite_into(&doc, &mut out, limits()).unwrap();
    assert_eq!(out, [[0; 4]; 2]);
    doc.layers[0].offset_x = 1;
    composite_into(&doc, &mut out, limits()).unwrap();
    assert_eq!(out, [[0; 4], [255, 0, 0, 255]]);
}

#[test]
fn malformed_hidden_layers_and_all_budget_failures_leave_output_intact() {
    let mut doc = pair([1, 2, 3, 255], [4, 5, 6, 255]);
    let sentinel = [17, 18, 19, 20];
    let mut out = [sentinel];
    doc.layers[1].visible = false;
    doc.layers[1].mask = Some(vec![]);
    assert_eq!(composite_into(&doc, &mut out, limits()), Err(ImageError::Storage));
    assert_eq!(out[0], sentinel);
    doc.layers[1].mask = None;
    doc.layers[1].pixels.clear();
    assert_eq!(composite_into(&doc, &mut out, limits()), Err(ImageError::Storage));
    assert_eq!(out[0], sentinel);
    doc.layers[1].pixels.push([4, 5, 6, 255]);
    doc.layers[1].opacity = f32::NAN;
    assert_eq!(composite_into(&doc, &mut out, limits()), Err(ImageError::Parameters));
    assert_eq!(out[0], sentinel);
    doc.layers[1].opacity = 1.0;
    let tight = ImageLimits { max_pixels: 64, max_work: 2, max_bytes: 1024 };
    assert_eq!(composite_into(&doc, &mut out, tight), Err(ImageError::Budget));
    assert_eq!(out[0], sentinel);
    let tight = ImageLimits { max_pixels: 64, max_work: 256, max_bytes: 11 };
    assert_eq!(composite_into(&doc, &mut out, tight), Err(ImageError::Budget));
    assert_eq!(out[0], sentinel);
    doc.width = 0;
    assert_eq!(composite_into(&doc, &mut out, limits()), Err(ImageError::Dimensions));
    assert_eq!(out[0], sentinel);
    doc.width = u32::MAX;
    doc.height = u32::MAX;
    assert_eq!(validate_document(&doc, limits()), Err(ImageError::Dimensions));
    assert_eq!(out[0], sentinel);
}

#[test]
fn output_size_refusal_and_legacy_normal_rounding() {
    let doc = pair([64, 128, 192, 128], [192, 96, 32, 128]);
    let mut wrong = [[7; 4]; 2];
    assert_eq!(composite_into(&doc, &mut wrong, limits()), Err(ImageError::Output));
    assert_eq!(wrong, [[7; 4]; 2]);
    // Existing source_over::rgba, composed from transparent, has this exact
    // integer rounding for the partial-alpha pair.
    assert_eq!(pixel(&doc), [149, 107, 85, 192]);
}


#[test]
fn explicit_transparent_and_encoded_blend_identity_vectors() {
    let modes = [BlendMode::Normal, BlendMode::Multiply, BlendMode::Screen,
        BlendMode::Overlay, BlendMode::SoftLight, BlendMode::HardLight,
        BlendMode::ColorDodge, BlendMode::ColorBurn, BlendMode::Difference, BlendMode::Exclusion];
    for mode in modes {
        let mut doc = pair([64, 128, 192, 128], [197, 83, 11, 0]);
        doc.layers[1].blend_mode = mode;
        assert_eq!(pixel(&doc), [64, 128, 192, 128], "transparent source {mode:?}");
    }
    // Fixed endpoint/neutral vectors from the declared encoded W3C equations.
    // RGBA8 cannot encode exactly 0.5: HardLight's neighbours have a one-LSB
    // bias; Overlay/SoftLight round both neighbours back to these base bytes.
    for (mode, source, expected) in [
        (BlendMode::Multiply, [255,255,255,255], [64,128,192,255]),
        (BlendMode::Screen, [0,0,0,255], [64,128,192,255]),
        (BlendMode::Screen, [255,255,255,255], [255,255,255,255]),
        (BlendMode::Overlay, [127,127,127,255], [64,128,192,255]),
        (BlendMode::Overlay, [128,128,128,255], [64,128,192,255]),
        (BlendMode::SoftLight, [127,127,127,255], [64,128,192,255]),
        (BlendMode::SoftLight, [128,128,128,255], [64,128,192,255]),
        (BlendMode::HardLight, [127,127,127,255], [64,127,191,255]),
        (BlendMode::HardLight, [128,128,128,255], [65,128,192,255]),
        (BlendMode::ColorDodge, [0,0,0,255], [64,128,192,255]),
        (BlendMode::ColorBurn, [255,255,255,255], [64,128,192,255]),
        (BlendMode::Difference, [64,128,192,255], [0,0,0,255]),
        (BlendMode::Exclusion, [0,0,0,255], [64,128,192,255]),
        (BlendMode::Exclusion, [255,255,255,255], [191,127,63,255]),
    ] {
        let mut doc = pair([64,128,192,255], source);
        doc.layers[1].blend_mode = mode;
        assert_eq!(pixel(&doc), expected, "identity/neutral {mode:?} {source:?}");
    }
}

#[test]
fn full_mask_identity_and_partial_coverage_match_for_every_mode() {
    for mode in [BlendMode::Normal, BlendMode::Multiply, BlendMode::Screen,
        BlendMode::Overlay, BlendMode::SoftLight, BlendMode::HardLight,
        BlendMode::ColorDodge, BlendMode::ColorBurn, BlendMode::Difference, BlendMode::Exclusion] {
        let mut doc = pair([64,128,192,128], [192,96,32,128]);
        doc.layers[1].blend_mode = mode;
        doc.layers[1].opacity = 0.5;
        let unmasked = pixel(&doc);
        doc.layers[1].mask = Some(vec![255]);
        assert_eq!(pixel(&doc), unmasked, "full mask {mode:?}");
        doc.layers[1].mask = Some(vec![128]);
        let half_mask = pixel(&doc);
        doc.layers[1].mask = None;
        doc.layers[1].opacity = 0.5 * 128.0 / 255.0;
        assert_eq!(pixel(&doc), half_mask, "coverage once {mode:?}");
    }
}

#[test]
fn vertical_diagonal_and_different_extent_offset_vectors() {
    let r=[255,0,0,255];let g=[0,255,0,255];let b=[0,0,255,255];let y=[255,255,0,255];let z=[0;4];
    let mut doc=ImageDocument::new("xy",2,2);
    let mut l=ImageLayer::new("l","l",2,2);l.pixels=vec![r,g,b,y];doc.layers.push(l);
    for (offset,expected) in [([0,1],[z,z,r,g]),([0,-1],[b,y,z,z]),([1,1],[z,z,z,r]),([-1,-1],[y,z,z,z])] {
        doc.layers[0].offset_x=offset[0];doc.layers[0].offset_y=offset[1];
        let mut out=[z;4];composite_into(&doc,&mut out,limits()).unwrap();assert_eq!(out,expected,"{offset:?}");
    }
    let mut narrow=ImageLayer::new("n","n",1,2);narrow.pixels=vec![r,b];narrow.offset_x=1;doc.layers=vec![narrow];
    let mut out=[z;4];composite_into(&doc,&mut out,limits()).unwrap();assert_eq!(out,[z,r,z,b]);
    doc.layers[0].mask=Some(vec![255,0]);composite_into(&doc,&mut out,limits()).unwrap();assert_eq!(out,[z,r,z,z]);
    let mut wide=ImageLayer::new("w","w",3,1);wide.pixels=vec![r,g,b];wide.offset_x=-1;doc.layers=vec![wide];
    composite_into(&doc,&mut out,limits()).unwrap();assert_eq!(out,[g,b,z,z]);
}
