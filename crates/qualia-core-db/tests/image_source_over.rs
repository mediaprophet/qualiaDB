//! Public canonical owner regressions; also mounted in the native Poet closure.
use qualia_core_db::hypermedia_authoring::image::{BrushStroke, ImageDocument};

fn dab(colour: [u8; 4], opacity: f32) -> BrushStroke {
    let mut s = BrushStroke::new(4.0, colour);
    s.opacity = opacity;
    s.hardness = 1.0;
    s.add_point(4.0, 4.0);
    s
}
#[test]
fn transparent_red_is_straight_and_composite_applies_alpha_once() {
    let mut d = ImageDocument::new("alpha", 8, 8);
    dab([255, 0, 0, 255], 0.5).apply_to(d.add_layer("paint"));
    assert_eq!(d.layers[0].get_pixel(4, 4), [255, 0, 0, 128]);
    assert_eq!(d.composite()[36], [255, 0, 0, 128]);
    let mut over_black = ImageDocument::new("display", 8, 8);
    over_black.add_layer("black").fill([0, 0, 0, 255]);
    over_black.layers.push(d.layers[0].clone());
    assert_eq!(over_black.composite()[36], [128, 0, 0, 255]);
    d.layers[0].opacity = 0.5;
    assert_eq!(d.composite()[36], [255, 0, 0, 64]);
}
#[test]
fn intersecting_transparent_strokes_accumulate_source_over_coverage() {
    let mut d = ImageDocument::new("overlap", 8, 8);
    let layer = d.add_layer("paint");
    dab([255, 0, 0, 255], 0.5).apply_to(layer);
    dab([0, 0, 255, 255], 0.5).apply_to(layer);
    assert_eq!(layer.get_pixel(4, 4), [85, 0, 170, 192]);
    assert_eq!(d.composite()[36], [85, 0, 170, 192]);
}
#[test]
fn opaque_base_layer_opacity_and_colour_alpha_each_apply_once() {
    let mut d = ImageDocument::new("opaque", 8, 8);
    d.add_layer("base").fill([0, 0, 0, 255]);
    dab([255, 0, 0, 128], 1.0).apply_to(d.add_layer("paint"));
    assert_eq!(d.composite()[36], [128, 0, 0, 255]);
    d.layers[1].opacity = 0.5;
    assert_eq!(d.composite()[36], [64, 0, 0, 255]);
    d.layers[1].visible = false;
    assert_eq!(d.composite()[36], [0, 0, 0, 255]);
}
#[test]
fn opaque_legacy_fill_and_full_hardness_boundary_remain_finite() {
    let mut d = ImageDocument::new("legacy", 8, 8);
    let layer = d.add_layer("opaque");
    layer.fill([10, 20, 30, 255]);
    dab([100, 110, 120, 255], 1.0).apply_to(layer);
    assert_eq!(layer.get_pixel(4, 4), [100, 110, 120, 255]);
    assert_eq!(layer.get_pixel(2, 4), [100, 110, 120, 255]);
    assert_eq!(layer.get_pixel(0, 0), [10, 20, 30, 255]);
    assert_eq!(d.composite()[36], [100, 110, 120, 255]);
}
