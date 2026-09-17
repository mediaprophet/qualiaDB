//! Pure key classification for layout undo/redo (no Document).
//!
//! `ctrl`/`meta` + z/Z undoes; + y/Y redoes. Shift+Z redo is not handled here.

/// Classified layout history key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutKey {
    Undo,
    Redo,
    Ignore,
}

/// Classify a key event. Host.* is always Ignore.
pub fn classify(key: &str, ctrl: bool, meta: bool) -> LayoutKey {
    if key.contains("Host.") || !(ctrl || meta) {
        return LayoutKey::Ignore;
    }
    match key {
        "z" | "Z" => LayoutKey::Undo,
        "y" | "Y" => LayoutKey::Redo,
        _ => LayoutKey::Ignore,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn z_ctrl_is_undo() {
        assert_eq!(classify("z", true, false), LayoutKey::Undo);
        assert_eq!(classify("Z", false, true), LayoutKey::Undo);
    }

    #[test]
    fn y_ctrl_is_redo() {
        assert_eq!(classify("y", true, false), LayoutKey::Redo);
        assert_eq!(classify("Y", false, true), LayoutKey::Redo);
    }

    #[test]
    fn z_without_ctrl_is_ignored() {
        assert_eq!(classify("z", false, false), LayoutKey::Ignore);
    }

    #[test]
    fn host_key_is_ignored() {
        assert_eq!(classify("Host.", true, false), LayoutKey::Ignore);
    }
}
