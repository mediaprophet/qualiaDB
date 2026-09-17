//! Studio layout undo/redo (SI-09/10). Duplicate of Poet LayoutUndo.
//! Studio does not depend on poet. No Host IDs.

use std::collections::VecDeque;

/// Classified layout history key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKey {
    Undo,
    Redo,
    Ignore,
}

/// `ctrl`/`meta` + z/Z undoes; + y/Y redoes. Host.* is always Ignore.
pub fn classify(key: &str, ctrl: bool) -> LayoutKey {
    if key.contains("Host.") || !ctrl {
        return LayoutKey::Ignore;
    }
    match key {
        "z" | "Z" => LayoutKey::Undo,
        "y" | "Y" => LayoutKey::Redo,
        _ => LayoutKey::Ignore,
    }
}

/// Bounded layout-history stack. Clone so Dioxus signals can hold it.
#[derive(Clone, Debug)]
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

    pub fn current(&self) -> Option<&str> {
        self.frames.back().map(String::as_str)
    }

    pub fn can_undo(&self) -> bool {
        self.frames.len() > 1
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_frames.is_empty()
    }

    pub fn undo(&mut self) -> Option<String> {
        let popped = self.frames.pop_back()?;
        self.redo_frames.push_back(popped.clone());
        Some(popped)
    }

    pub fn redo(&mut self) -> Option<String> {
        let restored = self.redo_frames.pop_back()?;
        self.frames.push_back(restored.clone());
        Some(restored)
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_ctrl_is_undo_y_is_redo() {
        assert_eq!(classify("z", true), LayoutKey::Undo);
        assert_eq!(classify("Y", true), LayoutKey::Redo);
        assert_eq!(classify("z", false), LayoutKey::Ignore);
        assert_eq!(classify("Host.", true), LayoutKey::Ignore);
    }

    #[test]
    fn current_is_restore_target_after_undo() {
        let mut u = LayoutUndo::new(8);
        u.push("a".into());
        u.push("b".into());
        assert!(u.can_undo());
        u.undo();
        assert_eq!(u.current(), Some("a"));
        assert_eq!(u.redo().as_deref(), Some("b"));
        u.push("b".into());
        assert_eq!(u.len(), 2);
    }

    #[test]
    fn len_and_is_empty_track_history() {
        let mut u = LayoutUndo::new(4);
        assert!(u.is_empty());
        assert_eq!(u.len(), 0);
        u.push("p1".into());
        assert!(!u.is_empty());
        assert_eq!(u.len(), 1);
        u.push("p2".into());
        assert_eq!(u.len(), 2);
        u.undo();
        assert_eq!(u.len(), 1);
        u.redo();
        assert_eq!(u.len(), 2);
    }
}
