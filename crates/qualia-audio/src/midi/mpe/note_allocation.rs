//! MPE note allocation — assign incoming notes to member channels, rotating.
//!
//! In MPE each sounding note gets its own member channel so its per-note
//! expression is independent. [`MpeNoteAllocator`] hands each new note the next
//! free member channel using a **rotating** cursor (round-robin), which spreads
//! successive notes across channels — the standard way to avoid immediately
//! reusing a just-released channel and to keep pitch-bend glide artifacts apart.
//! Occupancy is a fixed `[Option<u8>; 16]` table indexed by channel, so
//! allocation and release are allocation-free and safe on the real-time thread.

use crate::types::AudioError;

use super::zone::MpeZone;

/// Rotating (round-robin) allocator of member channels to active notes.
#[derive(Debug, Clone)]
pub struct MpeNoteAllocator {
    zone: MpeZone,
    /// Per-channel active note number (`None` = free). Indexed by channel 0..16.
    active: [Option<u8>; 16],
    /// Rotating cursor: the next member channel to try.
    cursor: u8,
}

impl MpeNoteAllocator {
    /// Create an allocator over `zone` with all member channels free.
    pub fn new(zone: MpeZone) -> Self {
        Self {
            zone,
            active: [None; 16],
            cursor: zone.member_low,
        }
    }

    /// Allocate a member channel for `note`, rotating from the current cursor.
    ///
    /// Returns the assigned channel index. Errors with
    /// [`AudioError::BackendUnavailable`] if every member channel is occupied.
    pub fn allocate(&mut self, note: u8) -> Result<u8, AudioError> {
        let count = self.zone.member_count();
        let low = self.zone.member_low;
        // Try each member channel once, starting at the cursor.
        for step in 0..count {
            let ch = low + ((self.cursor - low + step) % count);
            if self.active[ch as usize].is_none() {
                self.active[ch as usize] = Some(note);
                // Advance the cursor past the channel we just used.
                self.cursor = low + ((ch - low + 1) % count);
                return Ok(ch);
            }
        }
        Err(AudioError::BackendUnavailable)
    }

    /// Release the channel currently holding `note`. Returns the freed channel,
    /// or `None` if the note was not active.
    pub fn release(&mut self, note: u8) -> Option<u8> {
        let low = self.zone.member_low as usize;
        let high = self.zone.member_high as usize;
        for ch in low..=high {
            if self.active[ch] == Some(note) {
                self.active[ch] = None;
                return Some(ch as u8);
            }
        }
        None
    }

    /// The note currently sounding on `channel`, if any.
    #[inline]
    pub fn note_on_channel(&self, channel: u8) -> Option<u8> {
        self.active.get(channel as usize).copied().flatten()
    }

    /// Number of member channels currently occupied.
    pub fn active_count(&self) -> usize {
        let low = self.zone.member_low as usize;
        let high = self.zone.member_high as usize;
        self.active[low..=high]
            .iter()
            .filter(|c| c.is_some())
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_three_notes_three_distinct_channels() {
        let mut alloc = MpeNoteAllocator::new(MpeZone::lower(7).unwrap());
        let c0 = alloc.allocate(60).unwrap();
        let c1 = alloc.allocate(64).unwrap();
        let c2 = alloc.allocate(67).unwrap();
        assert_ne!(c0, c1);
        assert_ne!(c1, c2);
        assert_ne!(c0, c2);
        // All must be valid member channels.
        for &c in &[c0, c1, c2] {
            assert!((1..=7).contains(&c));
        }
    }

    #[test]
    fn rotates_and_reuses_after_release() {
        let mut alloc = MpeNoteAllocator::new(MpeZone::lower(3).unwrap()); // channels 1,2,3
        let a = alloc.allocate(60).unwrap();
        let b = alloc.allocate(61).unwrap();
        let c = alloc.allocate(62).unwrap();
        assert_eq!([a, b, c], [1, 2, 3]);
        assert!(alloc.allocate(63).is_err()); // full
        assert_eq!(alloc.release(61), Some(2));
        assert_eq!(alloc.allocate(63).unwrap(), 2); // reuses freed channel
    }

    #[test]
    fn tracks_active_notes() {
        let mut alloc = MpeNoteAllocator::new(MpeZone::lower(5).unwrap());
        let ch = alloc.allocate(72).unwrap();
        assert_eq!(alloc.note_on_channel(ch), Some(72));
        assert_eq!(alloc.active_count(), 1);
        alloc.release(72);
        assert_eq!(alloc.active_count(), 0);
        assert_eq!(alloc.note_on_channel(ch), None);
    }
}
