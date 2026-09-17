//! Pure keyboard model for the instrument node graph (no Document).
//!
//! Focus is an index into a fixed-length node list. Ends clamp.
//! Dispatch stays index-only — never Host IDs.

/// Focus cursor over a node list of `len` items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphFocus {
    pub index: usize,
    pub len: usize,
}

impl GraphFocus {
    pub fn new(len: usize) -> Self {
        Self { index: 0, len }
    }

    pub fn right(&mut self) {
        if self.len > 0 && self.index + 1 < self.len {
            self.index += 1;
        }
    }

    pub fn left(&mut self) {
        if self.index > 0 {
            self.index -= 1;
        }
    }

    pub fn home(&mut self) {
        self.index = 0;
    }

    pub fn end(&mut self) {
        self.index = self.len.saturating_sub(1);
    }

    /// ArrowRight, ArrowLeft, Home, End. Other keys (including Host.*) are ignored.
    pub fn handle(&mut self, key: &str) {
        match key {
            "ArrowRight" => self.right(),
            "ArrowLeft" => self.left(),
            "Home" => self.home(),
            "End" => self.end(),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_nodes_right_from_zero_is_one() {
        let mut focus = GraphFocus::new(4);
        focus.right();
        assert_eq!(focus.index, 1);
        focus.handle("ArrowRight");
        assert_eq!(focus.index, 2);
    }

    #[test]
    fn left_from_zero_stays_zero() {
        let mut focus = GraphFocus::new(4);
        focus.left();
        assert_eq!(focus.index, 0);
        focus.handle("ArrowLeft");
        assert_eq!(focus.index, 0);
    }

    #[test]
    fn end_goes_to_last_and_right_clamps() {
        let mut focus = GraphFocus::new(4);
        focus.end();
        assert_eq!(focus.index, 3);
        focus.right();
        assert_eq!(focus.index, 3);
        focus.handle("Home");
        assert_eq!(focus.index, 0);
        focus.handle("End");
        assert_eq!(focus.index, 3);
    }

    #[test]
    fn host_key_is_ignored() {
        let mut focus = GraphFocus::new(4);
        focus.handle("Host.navigate");
        assert_eq!(focus.index, 0);
        focus.handle("Escape");
        assert_eq!(focus.index, 0);
    }
}
