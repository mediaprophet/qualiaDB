//! Image editing — layers, brushes, selection, filters, masks, colour, vector.
//!
//! ~40 required functions. This module provides the data structures and
//! deterministic operations for image editing.

use std::collections::BTreeMap;

/// Blend mode for layer compositing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    HardLight,
    ColorDodge,
    ColorBurn,
    Difference,
    Exclusion,
}

/// A layer in an image document.
#[derive(Debug, Clone)]
pub struct ImageLayer {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[u8; 4]>, // RGBA
    pub opacity: f32,
    pub blend_mode: BlendMode,
    pub visible: bool,
    pub offset_x: i32,
    pub offset_y: i32,
    pub mask: Option<Vec<u8>>,
}

impl ImageLayer {
    pub fn new(id: &str, name: &str, width: u32, height: u32) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            width,
            height,
            pixels: vec![[0, 0, 0, 0]; (width as usize) * (height as usize)],
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            visible: true,
            offset_x: 0,
            offset_y: 0,
            mask: None,
        }
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x < self.width && y < self.height {
            let idx = (y as usize) * (self.width as usize) + (x as usize);
            self.pixels[idx] = rgba;
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x < self.width && y < self.height {
            let idx = (y as usize) * (self.width as usize) + (x as usize);
            self.pixels[idx]
        } else {
            [0, 0, 0, 0]
        }
    }

    pub fn fill(&mut self, rgba: [u8; 4]) {
        for px in &mut self.pixels {
            *px = rgba;
        }
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.0, 1.0);
    }

    pub fn set_blend_mode(&mut self, mode: BlendMode) {
        self.blend_mode = mode;
    }

    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub fn set_offset(&mut self, x: i32, y: i32) {
        self.offset_x = x;
        self.offset_y = y;
    }

    /// Apply a rectangular selection mask.
    pub fn set_rect_mask(&mut self, x: u32, y: u32, w: u32, h: u32) {
        let mut mask = vec![0u8; (self.width as usize) * (self.height as usize)];
        for my in y..(y + h).min(self.height) {
            for mx in x..(x + w).min(self.width) {
                let idx = (my as usize) * (self.width as usize) + (mx as usize);
                mask[idx] = 255;
            }
        }
        self.mask = Some(mask);
    }

    pub fn clear_mask(&mut self) {
        self.mask = None;
    }
}

/// A brush stroke — sequence of points with size and colour.
#[derive(Debug, Clone)]
pub struct BrushStroke {
    pub points: Vec<(f32, f32)>, // (x, y)
    pub size: f32,
    pub colour: [u8; 4],
    pub opacity: f32,
    pub hardness: f32,
}

impl BrushStroke {
    pub fn new(size: f32, colour: [u8; 4]) -> Self {
        Self {
            points: Vec::new(),
            size,
            colour,
            opacity: 1.0,
            hardness: 0.8,
        }
    }

    pub fn add_point(&mut self, x: f32, y: f32) {
        self.points.push((x, y));
    }

    /// Apply the brush stroke to a layer.
    pub fn apply_to(&self, layer: &mut ImageLayer) {
        let radius = self.size / 2.0;
        for &(px, py) in &self.points {
            let x0 = (px - radius).max(0.0) as u32;
            let y0 = (py - radius).max(0.0) as u32;
            let x1 = ((px + radius) as u32).min(layer.width);
            let y1 = ((py + radius) as u32).min(layer.height);
            for y in y0..y1 {
                for x in x0..x1 {
                    let dx = x as f32 - px;
                    let dy = y as f32 - py;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist <= radius {
                        let alpha = if dist < radius * self.hardness {
                            self.opacity
                        } else {
                            self.opacity
                                * (1.0
                                    - (dist - radius * self.hardness)
                                        / (radius * (1.0 - self.hardness)))
                        };
                        let mut px_val = layer.get_pixel(x, y);
                        px_val[0] = (self.colour[0] as f32 * alpha
                            + px_val[0] as f32 * (1.0 - alpha))
                            as u8;
                        px_val[1] = (self.colour[1] as f32 * alpha
                            + px_val[1] as f32 * (1.0 - alpha))
                            as u8;
                        px_val[2] = (self.colour[2] as f32 * alpha
                            + px_val[2] as f32 * (1.0 - alpha))
                            as u8;
                        px_val[3] = ((alpha * 255.0) as u8).max(px_val[3]);
                        layer.set_pixel(x, y, px_val);
                    }
                }
            }
        }
    }
}

/// A selection region.
#[derive(Debug, Clone)]
pub struct Selection {
    pub id: String,
    pub shape: SelectionShape,
    pub feather: f32,
}

#[derive(Debug, Clone)]
pub enum SelectionShape {
    Rectangle {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Ellipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
    },
    Lasso {
        points: Vec<(f32, f32)>,
    },
}

/// Image filter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFilter {
    GaussianBlur,
    Sharpen,
    Grayscale,
    Invert,
    Sepia,
    Brightness,
    Contrast,
    HueRotate,
    EdgeDetect,
    Emboss,
}

/// Apply a filter to a layer's pixels.
pub fn apply_filter(layer: &mut ImageLayer, filter: ImageFilter, intensity: f32) {
    let intensity = intensity.clamp(0.0, 1.0);
    match filter {
        ImageFilter::Grayscale => {
            for px in &mut layer.pixels {
                let gray = (px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000;
                px[0] = gray as u8;
                px[1] = gray as u8;
                px[2] = gray as u8;
            }
        }
        ImageFilter::Invert => {
            for px in &mut layer.pixels {
                px[0] = 255 - px[0];
                px[1] = 255 - px[1];
                px[2] = 255 - px[2];
            }
        }
        ImageFilter::Sepia => {
            for px in &mut layer.pixels {
                let r = px[0] as f32;
                let g = px[1] as f32;
                let b = px[2] as f32;
                let nr = (0.393 * r + 0.769 * g + 0.189 * b).min(255.0);
                let ng = (0.349 * r + 0.686 * g + 0.168 * b).min(255.0);
                let nb = (0.272 * r + 0.534 * g + 0.131 * b).min(255.0);
                px[0] = (nr * intensity + r * (1.0 - intensity)) as u8;
                px[1] = (ng * intensity + g * (1.0 - intensity)) as u8;
                px[2] = (nb * intensity + b * (1.0 - intensity)) as u8;
            }
        }
        ImageFilter::Brightness => {
            let delta = (intensity * 100.0) as i32;
            for px in &mut layer.pixels {
                px[0] = (px[0] as i32 + delta).clamp(0, 255) as u8;
                px[1] = (px[1] as i32 + delta).clamp(0, 255) as u8;
                px[2] = (px[2] as i32 + delta).clamp(0, 255) as u8;
            }
        }
        ImageFilter::Contrast => {
            let factor = 1.0 + intensity;
            for px in &mut layer.pixels {
                px[0] = (((px[0] as f32 - 128.0) * factor) + 128.0).clamp(0.0, 255.0) as u8;
                px[1] = (((px[1] as f32 - 128.0) * factor) + 128.0).clamp(0.0, 255.0) as u8;
                px[2] = (((px[2] as f32 - 128.0) * factor) + 128.0).clamp(0.0, 255.0) as u8;
            }
        }
        ImageFilter::EdgeDetect => {
            // Simple Sobel edge detection.
            let w = layer.width as usize;
            let h = layer.height as usize;
            let original = layer.pixels.clone();
            for y in 1..h - 1 {
                for x in 1..w - 1 {
                    let idx = y * w + x;
                    let p = |offset: usize| original[offset][0] as i32;
                    let gx = -p(idx - w - 1) - 2 * p(idx - 1) - p(idx + w - 1)
                        + p(idx - w + 1)
                        + 2 * p(idx + 1)
                        + p(idx + w + 1);
                    let gy = -p(idx - w - 1) - 2 * p(idx - w) - p(idx - w + 1)
                        + p(idx + w - 1)
                        + 2 * p(idx + w)
                        + p(idx + w + 1);
                    let mag = ((gx * gx + gy * gy) as f32).sqrt().min(255.0) as u8;
                    let val = (mag as f32 * intensity) as u8;
                    layer.pixels[idx] = [val, val, val, original[idx][3]];
                }
            }
        }
        _ => {
            // GaussianBlur, Sharpen, HueRotate, Emboss — would need convolution kernels.
            // For now, these are no-ops with the intensity parameter stored.
        }
    }
}

/// An image document with multiple layers.
#[derive(Debug, Clone)]
pub struct ImageDocument {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub layers: Vec<ImageLayer>,
    pub active_layer: usize,
    pub selections: BTreeMap<String, Selection>,
}

impl ImageDocument {
    pub fn new(id: &str, width: u32, height: u32) -> Self {
        Self {
            id: id.to_string(),
            width,
            height,
            layers: Vec::new(),
            active_layer: 0,
            selections: BTreeMap::new(),
        }
    }

    pub fn add_layer(&mut self, name: &str) -> &mut ImageLayer {
        let id = format!("layer_{}", self.layers.len());
        let layer = ImageLayer::new(&id, name, self.width, self.height);
        self.layers.push(layer);
        self.active_layer = self.layers.len() - 1;
        self.layers.last_mut().unwrap()
    }

    pub fn remove_layer(&mut self, index: usize) -> bool {
        if index < self.layers.len() {
            self.layers.remove(index);
            if self.active_layer >= self.layers.len() && !self.layers.is_empty() {
                self.active_layer = self.layers.len() - 1;
            }
            true
        } else {
            false
        }
    }

    pub fn active_layer_mut(&mut self) -> Option<&mut ImageLayer> {
        self.layers.get_mut(self.active_layer)
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Composite all visible layers into a single RGBA buffer.
    pub fn composite(&self) -> Vec<[u8; 4]> {
        let mut result = vec![[0u8; 4]; (self.width as usize) * (self.height as usize)];
        for layer in &self.layers {
            if !layer.visible {
                continue;
            }
            for (i, px) in layer.pixels.iter().enumerate() {
                let alpha = (px[3] as f32 / 255.0) * layer.opacity;
                let r = (px[0] as f32 * alpha + result[i][0] as f32 * (1.0 - alpha)) as u8;
                let g = (px[1] as f32 * alpha + result[i][1] as f32 * (1.0 - alpha)) as u8;
                let b = (px[2] as f32 * alpha + result[i][2] as f32 * (1.0 - alpha)) as u8;
                let a = ((alpha * 255.0) as u8).max(result[i][3]);
                result[i] = [r, g, b, a];
            }
        }
        result
    }

    pub fn add_selection(&mut self, selection: Selection) {
        self.selections.insert(selection.id.clone(), selection);
    }

    pub fn clear_selections(&mut self) {
        self.selections.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layer_creation() {
        let layer = ImageLayer::new("l1", "Background", 100, 100);
        assert_eq!(layer.width, 100);
        assert_eq!(layer.pixels.len(), 10000);
        assert!(layer.visible);
    }

    #[test]
    fn layer_set_pixel() {
        let mut layer = ImageLayer::new("l1", "test", 10, 10);
        layer.set_pixel(5, 5, [255, 0, 0, 255]);
        assert_eq!(layer.get_pixel(5, 5), [255, 0, 0, 255]);
    }

    #[test]
    fn layer_fill() {
        let mut layer = ImageLayer::new("l1", "test", 10, 10);
        layer.fill([128, 64, 32, 255]);
        assert_eq!(layer.get_pixel(0, 0), [128, 64, 32, 255]);
    }

    #[test]
    fn brush_stroke_apply() {
        let mut layer = ImageLayer::new("l1", "test", 50, 50);
        let mut stroke = BrushStroke::new(10.0, [255, 0, 0, 255]);
        stroke.add_point(25.0, 25.0);
        stroke.apply_to(&mut layer);
        // Center pixel should be red.
        let px = layer.get_pixel(25, 25);
        assert!(px[0] > 200);
    }

    #[test]
    fn filter_grayscale() {
        let mut layer = ImageLayer::new("l1", "test", 10, 10);
        layer.fill([255, 0, 0, 255]);
        apply_filter(&mut layer, ImageFilter::Grayscale, 1.0);
        let px = layer.get_pixel(0, 0);
        // Gray should be ~76 (0.299 * 255).
        assert!((px[0] as i32 - 76).abs() < 2);
        assert_eq!(px[0], px[1]);
        assert_eq!(px[1], px[2]);
    }

    #[test]
    fn filter_invert() {
        let mut layer = ImageLayer::new("l1", "test", 10, 10);
        layer.fill([100, 50, 200, 255]);
        apply_filter(&mut layer, ImageFilter::Invert, 1.0);
        let px = layer.get_pixel(0, 0);
        assert_eq!(px[0], 155);
        assert_eq!(px[1], 205);
        assert_eq!(px[2], 55);
    }

    #[test]
    fn filter_brightness() {
        let mut layer = ImageLayer::new("l1", "test", 10, 10);
        layer.fill([100, 100, 100, 255]);
        apply_filter(&mut layer, ImageFilter::Brightness, 0.5);
        let px = layer.get_pixel(0, 0);
        assert!(px[0] > 100);
    }

    #[test]
    fn document_add_layer() {
        let mut doc = ImageDocument::new("doc1", 100, 100);
        doc.add_layer("Background");
        doc.add_layer("Foreground");
        assert_eq!(doc.layer_count(), 2);
    }

    #[test]
    fn document_composite() {
        let mut doc = ImageDocument::new("doc1", 10, 10);
        let bg = doc.add_layer("bg");
        bg.fill([0, 0, 255, 255]);
        let fg = doc.add_layer("fg");
        fg.fill([255, 0, 0, 128]);
        let composite = doc.composite();
        // Should be a blend of blue and red.
        assert!(composite[0][0] > 0);
        assert!(composite[0][2] > 0);
    }

    #[test]
    fn document_remove_layer() {
        let mut doc = ImageDocument::new("doc1", 10, 10);
        doc.add_layer("bg");
        doc.add_layer("fg");
        assert!(doc.remove_layer(0));
        assert_eq!(doc.layer_count(), 1);
    }

    #[test]
    fn layer_mask() {
        let mut layer = ImageLayer::new("l1", "test", 20, 20);
        layer.set_rect_mask(5, 5, 10, 10);
        assert!(layer.mask.is_some());
        let mask = layer.mask.as_ref().unwrap();
        // Center should be 255, corners should be 0.
        assert_eq!(mask[10 * 20 + 10], 255);
        assert_eq!(mask[0], 0);
    }

    #[test]
    fn filter_edge_detect() {
        let mut layer = ImageLayer::new("l1", "test", 10, 10);
        // Create a sharp edge: left half white, right half black.
        for y in 0..10 {
            for x in 0..10 {
                if x < 5 {
                    layer.set_pixel(x, y, [255, 255, 255, 255]);
                }
            }
        }
        apply_filter(&mut layer, ImageFilter::EdgeDetect, 1.0);
        // Edge should be detected around x=4-5.
        let edge_px = layer.get_pixel(4, 5);
        assert!(edge_px[0] > 0);
    }
}
