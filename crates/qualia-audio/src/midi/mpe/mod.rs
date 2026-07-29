//! MIDI Polyphonic Expression (zones, per-note expression). Re-exports only (AU-MIDI-SEQ).

pub mod expression;
pub mod note_allocation;
pub mod zone;

pub use expression::{ChannelExpr, MpeExpression, CC_TIMBRE};
pub use note_allocation::MpeNoteAllocator;
pub use zone::MpeZone;
