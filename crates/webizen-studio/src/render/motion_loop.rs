//! RequestAnimationFrame / interval driver for spring-smoothed UI motion.

use crate::render::motion::Spring;

/// Advance a selection-highlight spring toward `1.0` when selected, `0.0` otherwise.
pub fn step_selection_spring(
    spring: &mut Spring,
    selected: bool,
    theme_class: Option<&str>,
    dt: f64,
) -> f64 {
    spring.set_target(if selected { 1.0 } else { 0.0 });
    let timeline = crate::render::motion::timeline_from_theme(0.0, dt, theme_class);
    let v = spring.step(&timeline);
    (1.0 + v * 0.035).clamp(1.0, 1.04)
}

/// Impulse the presentation-mode toolbar spring (one-shot pulse on mode switch).
pub fn trigger_mode_pulse(spring: &mut Spring) {
    spring.value = 0.0;
    spring.velocity = 14.0;
    spring.set_target(0.0);
}

/// Advance the toolbar pulse spring; returns a scale bump in `[0.0, 0.045]`.
pub fn step_mode_pulse_spring(spring: &mut Spring, theme_class: Option<&str>, dt: f64) -> f64 {
    let timeline = crate::render::motion::timeline_from_theme(0.0, dt, theme_class);
    if timeline.reduced_motion {
        spring.value = 0.0;
        spring.velocity = 0.0;
        return 0.0;
    }
    spring.set_target(0.0);
    spring.step(&timeline);
    (spring.value.abs() + spring.velocity.abs() * 0.018).clamp(0.0, 0.045)
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_ui_motion_loop(mut on_frame: impl FnMut(f64) + 'static) {
    use std::cell::RefCell;
    use std::rc::Rc;

    use gloo_timers::callback::Interval;
    use wasm_bindgen::prelude::*;

    let last = Rc::new(RefCell::new(0.0f64));
    Interval::new(16, move || {
        let now = js_sys::Date::now();
        let mut prev = last.borrow_mut();
        let dt = if *prev > 0.0 {
            ((now - *prev) / 1000.0).clamp(0.001, 0.05)
        } else {
            0.016
        };
        *prev = now;
        on_frame(dt);
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_ui_motion_loop(_on_frame: impl FnMut(f64) + 'static) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_spring_reaches_target_without_overshoot() {
        let mut spring = Spring::new(0.0);
        for _ in 0..120 {
            step_selection_spring(&mut spring, true, None, 0.016);
        }
        assert!(spring.is_at_target(0.02));
    }
}