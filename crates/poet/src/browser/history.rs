//! Canvas undo/redo history — 32-entry bounded stack.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Poet-owned workspace history. Each frame captures a snapshot of container
//! positions, sizes, and manifold seed state so the user can undo/redo layout
//! changes without a Studio dependency.

use std::cell::RefCell;
use std::collections::VecDeque;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, KeyboardEvent};

use crate::tool_chest::core::registry::ManifoldSeed;

/// Maximum frames retained in history.
const MAX_FRAMES: usize = 32;

/// A single snapshot of canvas state at a point in time.
#[derive(Clone, Debug)]
pub struct CanvasFrame {
    /// The manifold seed at this point.
    pub seed: ManifoldSeed,
    /// Human-readable label for this history entry.
    pub label: String,
}

/// Bounded undo/redo stack.
pub struct WorkspaceHistory {
    /// Past states (most recent at the back).
    past: VecDeque<CanvasFrame>,
    /// Future states (most recent at the front).
    future: VecDeque<CanvasFrame>,
    /// Current state.
    current: CanvasFrame,
}

impl WorkspaceHistory {
    /// Create a new history with an initial frame.
    pub fn new(initial: CanvasFrame) -> Self {
        Self {
            past: VecDeque::with_capacity(MAX_FRAMES),
            future: VecDeque::with_capacity(MAX_FRAMES),
            current: initial,
        }
    }

    /// Push a new frame, clearing the redo stack.
    pub fn push(&mut self, frame: CanvasFrame) {
        self.past.push_back(self.current.clone());
        if self.past.len() > MAX_FRAMES {
            self.past.pop_front();
        }
        self.future.clear();
        self.current = frame;
    }

    /// Undo — move current to future, pop past to current.
    /// Returns `true` if undo was possible.
    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.past.pop_back() {
            self.future.push_front(self.current.clone());
            self.current = prev;
            true
        } else {
            false
        }
    }

    /// Redo — move current to past, pop future to current.
    /// Returns `true` if redo was possible.
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.future.pop_front() {
            self.past.push_back(self.current.clone());
            self.current = next;
            true
        } else {
            false
        }
    }

    /// Can undo?
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    /// Can redo?
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    /// Get the current frame.
    pub fn current(&self) -> &CanvasFrame {
        &self.current
    }

    /// Replace the current frame without pushing history.
    pub fn replace_current(&mut self, frame: CanvasFrame) {
        self.current = frame;
    }
}

// ---------------------------------------------------------------------------
// Thread-local singleton + wiring
// ---------------------------------------------------------------------------

thread_local! {
    static CANVAS_HISTORY: RefCell<Option<WorkspaceHistory>> = RefCell::new(None);
}

/// Initialise the canvas history with an initial seed.
/// Called once at startup from `build_app`.
pub fn init_history(seed: ManifoldSeed) {
    CANVAS_HISTORY.with(|h| {
        *h.borrow_mut() = Some(WorkspaceHistory::new(CanvasFrame {
            seed,
            label: "initial".into(),
        }));
    });
}

/// Sync the current seed's container positions from the DOM, then push
/// a new frame with the updated positions. Used after drag/resize ends.
pub fn push_current_frame(label: &str) {
    CANVAS_HISTORY.with(|h| {
        if let Some(history) = h.borrow_mut().as_mut() {
            let seed = snapshot_current_seed(&history.current().seed);
            super::replace_current_seed(&seed);
            history.push(CanvasFrame {
                seed,
                label: label.into(),
            });
        }
    });
}

/// Sync the current frame from the DOM, then push a new manifold seed.
/// Used when switching manifolds so the past frame captures the old
/// manifold's final positions.
pub fn switch_to_manifold(new_seed: ManifoldSeed) {
    CANVAS_HISTORY.with(|h| {
        if let Some(history) = h.borrow_mut().as_mut() {
            // Sync current from DOM so the past frame is accurate.
            let current = snapshot_current_seed(&history.current().seed);
            super::replace_current_seed(&current);
            history.replace_current(CanvasFrame {
                seed: current,
                label: "synced".into(),
            });
            history.push(CanvasFrame {
                seed: new_seed,
                label: "switch manifold".into(),
            });
        }
    });
}

/// Perform undo and re-render the canvas from the restored frame.
pub fn perform_undo() {
    let seed = CANVAS_HISTORY.with(|h| {
        let mut slot = h.borrow_mut();
        let history = slot.as_mut()?;
        if history.undo() {
            Some(history.current().seed.clone())
        } else {
            None
        }
    });
    if let Some(seed) = seed {
        super::replace_current_seed(&seed);
        super::rerender_canvas(&seed);
    }
}

/// Perform redo and re-render the canvas from the restored frame.
pub fn perform_redo() {
    let seed = CANVAS_HISTORY.with(|h| {
        let mut slot = h.borrow_mut();
        let history = slot.as_mut()?;
        if history.redo() {
            Some(history.current().seed.clone())
        } else {
            None
        }
    });
    if let Some(seed) = seed {
        super::replace_current_seed(&seed);
        super::rerender_canvas(&seed);
    }
}

/// Wire Ctrl+Z (undo) and Ctrl+Y / Ctrl+Shift+Z (redo) keyboard shortcuts.
pub fn wire_undo_redo(document: &Document) {
    let closure = Closure::wrap(Box::new(move |e: KeyboardEvent| {
        let ctrl = e.ctrl_key() || e.meta_key();
        if !ctrl {
            return;
        }
        let key = e.key();
        if key == "z" || key == "Z" {
            e.prevent_default();
            if e.shift_key() {
                perform_redo();
            } else {
                perform_undo();
            }
        } else if key == "y" || key == "Y" {
            e.prevent_default();
            perform_redo();
        }
    }) as Box<dyn FnMut(KeyboardEvent)>);

    document
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// Capture contenteditable document changes when editing focus leaves the
/// surface. This keeps typing responsive while still producing one coherent
/// undo frame per editing session.
pub fn wire_editable_history(document: &Document) {
    let Ok(editors) = document.query_selector_all(".doc-editor[contenteditable=\"true\"]") else {
        return;
    };
    for index in 0..editors.length() {
        let Some(node) = editors.get(index) else {
            continue;
        };
        let Ok(editor) = node.dyn_into::<web_sys::Element>() else {
            continue;
        };
        if !super::dom_bindings::claim(&editor, "editable-history") {
            continue;
        }
        let closure = Closure::wrap(Box::new(move || {
            push_current_frame("edit document");
        }) as Box<dyn FnMut()>);
        let _ = editor.add_event_listener_with_callback("blur", closure.as_ref().unchecked_ref());
        closure.forget();
    }
}

// ---------------------------------------------------------------------------
// DOM sync — read container positions back into a seed
// ---------------------------------------------------------------------------

/// Rebuild the complete serialisable manifold from the mounted canvas.
fn snapshot_current_seed(seed: &ManifoldSeed) -> ManifoldSeed {
    let document = match web_sys::window().and_then(|w| w.document()) {
        Some(d) => d,
        None => return seed.clone(),
    };
    super::canvas_state::snapshot_seed_from_dom(&document, seed)
}

/// Flush the mounted canvas into the persistence store without creating an
/// undo entry. Used immediately before save/export operations.
pub fn sync_persistence_state() {
    CANVAS_HISTORY.with(|slot| {
        if let Some(history) = slot.borrow_mut().as_mut() {
            let seed = snapshot_current_seed(&history.current().seed);
            super::replace_current_seed(&seed);
            let label = history.current().label.clone();
            history.replace_current(CanvasFrame { seed, label });
        }
    });
}

/// Commit a model mutation performed outside the mounted canvas while keeping
/// the previous synced frame available to Undo.
pub fn commit_external_seed(seed: ManifoldSeed, label: &str) {
    super::replace_current_seed(&seed);
    CANVAS_HISTORY.with(|slot| {
        if let Some(history) = slot.borrow_mut().as_mut() {
            history.push(CanvasFrame {
                seed,
                label: label.to_string(),
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_chest::core::registry::ManifoldSeed;

    fn test_seed(id: &str) -> ManifoldSeed {
        ManifoldSeed {
            id: id.into(),
            label: id.into(),
            icon: "test".into(),
            ontology_prefix: "test".into(),
            description: "test".into(),
            containers: vec![],
            connections: vec![],
            panels: vec![],
            ..Default::default()
        }
    }

    #[test]
    fn test_undo_redo_basic() {
        let initial = CanvasFrame {
            seed: test_seed("a"),
            label: "initial".into(),
        };
        let mut hist = WorkspaceHistory::new(initial);

        hist.push(CanvasFrame {
            seed: test_seed("b"),
            label: "edit 1".into(),
        });
        hist.push(CanvasFrame {
            seed: test_seed("c"),
            label: "edit 2".into(),
        });

        assert_eq!(hist.current().seed.id, "c");
        assert!(hist.can_undo());
        assert!(!hist.can_redo());

        assert!(hist.undo());
        assert_eq!(hist.current().seed.id, "b");
        assert!(hist.can_redo());

        assert!(hist.undo());
        assert_eq!(hist.current().seed.id, "a");
        assert!(!hist.can_undo());

        assert!(hist.redo());
        assert_eq!(hist.current().seed.id, "b");
    }

    #[test]
    fn test_push_clears_redo() {
        let initial = CanvasFrame {
            seed: test_seed("a"),
            label: "initial".into(),
        };
        let mut hist = WorkspaceHistory::new(initial);

        hist.push(CanvasFrame {
            seed: test_seed("b"),
            label: "edit 1".into(),
        });
        hist.undo();
        assert!(hist.can_redo());

        hist.push(CanvasFrame {
            seed: test_seed("d"),
            label: "edit 2".into(),
        });
        assert!(!hist.can_redo());
    }

    #[test]
    fn test_max_frames_bounded() {
        let initial = CanvasFrame {
            seed: test_seed("0"),
            label: "initial".into(),
        };
        let mut hist = WorkspaceHistory::new(initial);

        for i in 1..=40 {
            hist.push(CanvasFrame {
                seed: test_seed(&i.to_string()),
                label: format!("edit {}", i),
            });
        }

        // Should only be able to undo 32 times
        let mut undo_count = 0;
        while hist.undo() {
            undo_count += 1;
        }
        assert_eq!(undo_count, 32);
    }
}
