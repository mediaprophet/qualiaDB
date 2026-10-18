//! Zero-Heap Bitset Input Engine & Mobile Virtual Touch Controller.
//!
//! Provides $O(1)$ zero-allocation input tracking adapted from game engine architectures
//! (replacing dynamic HashSets with fixed CPU word bitsets), paired with responsive
//! multi-touch virtual joysticks and D-pads for mobile phones and tablets.

pub const INPUT_WORDS_4: usize = 4; // 256 discrete key / button channels
pub const MAX_TOUCH_POINTS: usize = 10;

// Standard key codes mapped to flat bit indices (0..255)
pub const KEY_W: u16 = 87;
pub const KEY_A: u16 = 65;
pub const KEY_S: u16 = 83;
pub const KEY_D: u16 = 68;
pub const KEY_SPACE: u16 = 32;
pub const KEY_SHIFT: u16 = 16;
pub const KEY_ESCAPE: u16 = 27;
pub const KEY_E: u16 = 69;
pub const KEY_Q: u16 = 81;
pub const KEY_TAB: u16 = 9;
pub const KEY_ARROW_UP: u16 = 38;
pub const KEY_ARROW_DOWN: u16 = 40;
pub const KEY_ARROW_LEFT: u16 = 37;
pub const KEY_ARROW_RIGHT: u16 = 39;
pub const BUTTON_PRIMARY: u16 = 1;
pub const BUTTON_SECONDARY: u16 = 2;
pub const BUTTON_MIDDLE: u16 = 3;

/// Zero-heap bitset representing 256 discrete button states with single-frame transition events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZeroHeapButtonInput<const WORDS: usize = INPUT_WORDS_4> {
    pub pressed: [u64; WORDS],
    pub just_pressed: [u64; WORDS],
    pub just_released: [u64; WORDS],
}

impl<const WORDS: usize> Default for ZeroHeapButtonInput<WORDS> {
    fn default() -> Self {
        Self {
            pressed: [0; WORDS],
            just_pressed: [0; WORDS],
            just_released: [0; WORDS],
        }
    }
}

impl<const WORDS: usize> ZeroHeapButtonInput<WORDS> {
    pub const fn new() -> Self {
        Self {
            pressed: [0; WORDS],
            just_pressed: [0; WORDS],
            just_released: [0; WORDS],
        }
    }

    /// Clears per-frame transient edge transitions (`just_pressed` and `just_released`).
    /// Call at the start of each frame.
    #[inline]
    pub fn advance_frame(&mut self) {
        self.just_pressed = [0; WORDS];
        self.just_released = [0; WORDS];
    }

    /// Records a button press event.
    #[inline]
    pub fn press(&mut self, key_code: u16) {
        let bit = (key_code as usize) % (WORDS * 64);
        let word = bit / 64;
        let mask = 1u64 << (bit % 64);

        if (self.pressed[word] & mask) == 0 {
            self.pressed[word] |= mask;
            self.just_pressed[word] |= mask;
        }
    }

    /// Records a button release event.
    #[inline]
    pub fn release(&mut self, key_code: u16) {
        let bit = (key_code as usize) % (WORDS * 64);
        let word = bit / 64;
        let mask = 1u64 << (bit % 64);

        if (self.pressed[word] & mask) != 0 {
            self.pressed[word] &= !mask;
            self.just_released[word] |= mask;
        }
    }

    /// Tests whether the button is currently held down.
    #[inline]
    pub fn is_pressed(&self, key_code: u16) -> bool {
        let bit = (key_code as usize) % (WORDS * 64);
        (self.pressed[bit / 64] & (1u64 << (bit % 64))) != 0
    }

    /// Tests whether the button was pressed during this frame.
    #[inline]
    pub fn is_just_pressed(&self, key_code: u16) -> bool {
        let bit = (key_code as usize) % (WORDS * 64);
        (self.just_pressed[bit / 64] & (1u64 << (bit % 64))) != 0
    }

    /// Tests whether the button was released during this frame.
    #[inline]
    pub fn is_just_released(&self, key_code: u16) -> bool {
        let bit = (key_code as usize) % (WORDS * 64);
        (self.just_released[bit / 64] & (1u64 << (bit % 64))) != 0
    }

    /// Clears all button states.
    pub fn reset(&mut self) {
        self.pressed = [0; WORDS];
        self.just_pressed = [0; WORDS];
        self.just_released = [0; WORDS];
    }
}

/// Virtual touch joystick for mobile phones and touchscreen devices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualTouchStick {
    pub center_px: [f32; 2],
    pub current_px: [f32; 2],
    pub radius_px: f32,
    pub deadzone: f32,
    pub axis: [f32; 2], // [-1.0..1.0, -1.0..1.0]
    pub is_active: bool,
    pub active_touch_id: Option<u64>,
}

impl Default for VirtualTouchStick {
    fn default() -> Self {
        Self {
            center_px: [100.0, 100.0],
            current_px: [100.0, 100.0],
            radius_px: 60.0,
            deadzone: 0.1,
            axis: [0.0, 0.0],
            is_active: false,
            active_touch_id: None,
        }
    }
}

impl VirtualTouchStick {
    pub const fn new(center_px: [f32; 2], radius_px: f32) -> Self {
        Self {
            center_px,
            current_px: center_px,
            radius_px,
            deadzone: 0.1,
            axis: [0.0, 0.0],
            is_active: false,
            active_touch_id: None,
        }
    }

    pub fn on_touch_start(&mut self, touch_id: u64, x: f32, y: f32) -> bool {
        if self.is_active {
            return false;
        }

        let dx = x - self.center_px[0];
        let dy = y - self.center_px[1];
        let dist = (dx * dx + dy * dy).sqrt();

        // Allow touch start within 1.5x radius of center
        if dist <= self.radius_px * 1.5 {
            self.is_active = true;
            self.active_touch_id = Some(touch_id);
            self.update_drag(x, y);
            true
        } else {
            false
        }
    }

    pub fn on_touch_move(&mut self, touch_id: u64, x: f32, y: f32) -> bool {
        if self.is_active && self.active_touch_id == Some(touch_id) {
            self.update_drag(x, y);
            true
        } else {
            false
        }
    }

    pub fn on_touch_end(&mut self, touch_id: u64) -> bool {
        if self.is_active && self.active_touch_id == Some(touch_id) {
            self.is_active = false;
            self.active_touch_id = None;
            self.current_px = self.center_px;
            self.axis = [0.0, 0.0];
            true
        } else {
            false
        }
    }

    fn update_drag(&mut self, x: f32, y: f32) {
        let dx = x - self.center_px[0];
        let dy = y - self.center_px[1];
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= 1e-4 {
            self.current_px = self.center_px;
            self.axis = [0.0, 0.0];
            return;
        }

        let clamped_dist = dist.min(self.radius_px);
        let nx = dx / dist;
        let ny = dy / dist;

        self.current_px = [
            self.center_px[0] + nx * clamped_dist,
            self.center_px[1] + ny * clamped_dist,
        ];

        let norm_mag = clamped_dist / self.radius_px;
        if norm_mag <= self.deadzone {
            self.axis = [0.0, 0.0];
        } else {
            // Remap deadzone to 0..1
            let remapped = (norm_mag - self.deadzone) / (1.0 - self.deadzone);
            self.axis = [nx * remapped, ny * remapped];
        }
    }
}

/// Touch contact point record for multi-touch finger gesture tracking.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TouchContact {
    pub id: u64,
    pub pos: [f32; 2],
    pub delta: [f32; 2],
    pub is_active: bool,
}

/// Fixed 10-slot multi-touch tracker without heap allocation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultiTouchTracker {
    pub touches: [TouchContact; MAX_TOUCH_POINTS],
}

impl Default for MultiTouchTracker {
    fn default() -> Self {
        Self {
            touches: [TouchContact::default(); MAX_TOUCH_POINTS],
        }
    }
}

impl MultiTouchTracker {
    pub const fn new() -> Self {
        Self {
            touches: [TouchContact {
                id: 0,
                pos: [0.0, 0.0],
                delta: [0.0, 0.0],
                is_active: false,
            }; MAX_TOUCH_POINTS],
        }
    }

    pub fn touch_start(&mut self, id: u64, x: f32, y: f32) {
        for slot in self.touches.iter_mut() {
            if !slot.is_active {
                slot.id = id;
                slot.pos = [x, y];
                slot.delta = [0.0, 0.0];
                slot.is_active = true;
                return;
            }
        }
    }

    pub fn touch_move(&mut self, id: u64, x: f32, y: f32) {
        for slot in self.touches.iter_mut() {
            if slot.is_active && slot.id == id {
                slot.delta = [x - slot.pos[0], y - slot.pos[1]];
                slot.pos = [x, y];
                return;
            }
        }
    }

    pub fn touch_end(&mut self, id: u64) {
        for slot in self.touches.iter_mut() {
            if slot.is_active && slot.id == id {
                slot.is_active = false;
                slot.delta = [0.0, 0.0];
                return;
            }
        }
    }

    pub fn active_count(&self) -> usize {
        self.touches.iter().filter(|t| t.is_active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_heap_button_transitions() {
        let mut input = ZeroHeapButtonInput::<4>::new();

        assert!(!input.is_pressed(KEY_W));
        assert!(!input.is_just_pressed(KEY_W));

        // Press KEY_W
        input.press(KEY_W);
        assert!(input.is_pressed(KEY_W));
        assert!(input.is_just_pressed(KEY_W));
        assert!(!input.is_just_released(KEY_W));

        // Next frame: remains pressed, but no longer just_pressed
        input.advance_frame();
        assert!(input.is_pressed(KEY_W));
        assert!(!input.is_just_pressed(KEY_W));
        assert!(!input.is_just_released(KEY_W));

        // Release KEY_W
        input.release(KEY_W);
        assert!(!input.is_pressed(KEY_W));
        assert!(!input.is_just_pressed(KEY_W));
        assert!(input.is_just_released(KEY_W));

        // Next frame: clean
        input.advance_frame();
        assert!(!input.is_pressed(KEY_W));
        assert!(!input.is_just_released(KEY_W));
    }

    #[test]
    fn test_virtual_touch_stick_deadzone_and_clamp() {
        let mut stick = VirtualTouchStick::new([100.0, 100.0], 50.0);
        assert_eq!(stick.axis, [0.0, 0.0]);

        // Touch slightly off center (within deadzone of 0.1 * 50 = 5px)
        assert!(stick.on_touch_start(1, 103.0, 100.0));
        assert_eq!(stick.axis, [0.0, 0.0]);

        // Drag fully to right beyond radius (100 + 75 = 175px)
        assert!(stick.on_touch_move(1, 175.0, 100.0));
        assert!((stick.axis[0] - 1.0).abs() < 1e-4);
        assert_eq!(stick.axis[1], 0.0);

        // Release touch
        assert!(stick.on_touch_end(1));
        assert_eq!(stick.axis, [0.0, 0.0]);
        assert!(!stick.is_active);
    }

    #[test]
    fn test_multi_touch_tracker() {
        let mut tracker = MultiTouchTracker::new();
        assert_eq!(tracker.active_count(), 0);

        tracker.touch_start(101, 50.0, 50.0);
        tracker.touch_start(102, 150.0, 150.0);
        assert_eq!(tracker.active_count(), 2);

        tracker.touch_move(101, 55.0, 52.0);
        assert_eq!(tracker.touches[0].delta, [5.0, 2.0]);

        tracker.touch_end(101);
        assert_eq!(tracker.active_count(), 1);
    }
}
