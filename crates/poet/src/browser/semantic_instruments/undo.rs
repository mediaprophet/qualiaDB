//! Pure undo/redo stack for graph layouts.
//!
//! Stores encoded position strings. No Document, no Host IDs.

use std::collections::VecDeque;

/// Bounded layout-history stack. Oldest entries drop when `max` is exceeded.
pub struct LayoutUndo {
    pub max: usize,
    frames: VecDeque<String>,
    redo_frames: VecDeque<String>,
}

impl LayoutUndo {
    pub fn new(max: usize) -> Self {
        Self {
            max,
            frames: VecDeque::new(),
            redo_frames: VecDeque::new(),
        }
    }

    /// Push an encoded layout. Empty strings, Host payloads, and a duplicate of the
    /// current top are ignored. A successful push starts a new branch and clears redo.
    pub fn push(&mut self, encoded: String) {
        if encoded.is_empty() || encoded.contains("Host") {
            return;
        }
        if self.frames.back().is_some_and(|top| top == &encoded) {
            return;
        }
        self.redo_frames.clear();
        self.frames.push_back(encoded);
        while self.frames.len() > self.max {
            self.frames.pop_front();
        }
    }

    /// Most recently pushed layout still on the undo stack (the restore target after `undo`).
    pub fn current(&self) -> Option<&str> {
        self.frames.back().map(String::as_str)
    }

    /// True when more than the initial frame remains, so `undo` can leave a restore target.
    pub fn can_undo(&self) -> bool {
        self.frames.len() > 1
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_frames.is_empty()
    }

    /// Pop the most recently pushed layout onto the redo stack and return it.
    /// Callers restore [`Self::current`] after a successful pop.
    pub fn undo(&mut self) -> Option<String> {
        let popped = self.frames.pop_back()?;
        self.redo_frames.push_back(popped.clone());
        Some(popped)
    }

    /// Pop the most recently undone layout back onto the undo stack and return it.
    pub fn redo(&mut self) -> Option<String> {
        let restored = self.redo_frames.pop_back()?;
        self.frames.push_back(restored.clone());
        Some(restored)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_three_max_two_keeps_last_two() {
        let mut u = LayoutUndo::new(2);
        u.push("a".into());
        u.push("b".into());
        u.push("c".into());
        assert_eq!(u.len(), 2);
        assert_eq!(u.undo(), Some("c".into()));
        assert_eq!(u.undo(), Some("b".into()));
        assert_eq!(u.undo(), None);
    }

    #[test]
    fn undo_is_lifo() {
        let mut u = LayoutUndo::new(8);
        u.push("one".into());
        u.push("two".into());
        u.push("three".into());
        assert_eq!(u.undo(), Some("three".into()));
        assert_eq!(u.undo(), Some("two".into()));
        assert_eq!(u.undo(), Some("one".into()));
        assert_eq!(u.len(), 0);
    }

    #[test]
    fn host_payload_is_ignored() {
        let mut u = LayoutUndo::new(4);
        u.push("10.00,20.00".into());
        u.push("Host.1,2.00".into());
        u.push("Host.".into());
        assert_eq!(u.len(), 1);
        assert_eq!(u.undo(), Some("10.00,20.00".into()));
    }

    #[test]
    fn empty_is_ignored() {
        let mut u = LayoutUndo::new(4);
        u.push(String::new());
        u.push("kept".into());
        u.push(String::new());
        assert_eq!(u.len(), 1);
        assert_eq!(u.undo(), Some("kept".into()));
    }

    #[test]
    fn undo_then_redo_restores() {
        let mut u = LayoutUndo::new(8);
        u.push("a".into());
        u.push("b".into());
        assert_eq!(u.undo(), Some("b".into()));
        assert_eq!(u.len(), 1);
        assert_eq!(u.redo(), Some("b".into()));
        assert_eq!(u.len(), 2);
        assert_eq!(u.undo(), Some("b".into()));
        assert_eq!(u.redo(), Some("b".into()));
        assert_eq!(u.redo(), None);
    }

    #[test]
    fn push_after_undo_drops_redo() {
        let mut u = LayoutUndo::new(8);
        u.push("a".into());
        u.push("b".into());
        assert_eq!(u.undo(), Some("b".into()));
        u.push("c".into());
        assert_eq!(u.redo(), None);
        assert_eq!(u.undo(), Some("c".into()));
        assert_eq!(u.undo(), Some("a".into()));
        assert_eq!(u.undo(), None);
    }

    #[test]
    fn current_is_restore_target_after_undo() {
        let mut u = LayoutUndo::new(8);
        assert_eq!(u.current(), None);
        u.push("a".into());
        u.push("b".into());
        assert_eq!(u.current(), Some("b"));
        assert!(u.can_undo());
        assert!(!u.can_redo());
        assert_eq!(u.undo(), Some("b".into()));
        assert_eq!(u.current(), Some("a"));
        assert!(!u.can_undo());
        assert!(u.can_redo());
        assert_eq!(u.redo(), Some("b".into()));
        assert_eq!(u.current(), Some("b"));
        assert!(u.can_undo());
        assert!(!u.can_redo());
    }

    #[test]
    fn duplicate_top_is_ignored() {
        let mut u = LayoutUndo::new(8);
        u.push("a".into());
        u.push("a".into());
        assert_eq!(u.len(), 1);
        assert_eq!(u.current(), Some("a"));
    }
}
