//! Bounded session overlay for custom typography / unicode glyphs (B-009).
//!
//! Compile-time `ALL_ICONS` stays the hot static table. This overlay is a
//! fixed 32-slot session table for user/custom buttons. It is not a Vibe
//! keyword and does not add Host methods.

use super::icon_registry::{icon_entry, IconCategory, PuaCodepoint};

/// Session glyph slots (cold UI path; bounded, not a Vec in a hot kernel).
pub const MAX_SESSION_ICONS: usize = 32;
const MAX_ID_BYTES: usize = 48;
const MAX_LABEL_BYTES: usize = 24;
/// First PUA codepoint reserved for session glyphs (static icons end ~U+E086).
pub const SESSION_PUA_START: u32 = 0xE100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IconSessionError {
    EmptyId,
    IdTooLong,
    LabelTooLong,
    StaticIdReserved,
    DuplicateId,
    TableFull,
    InvalidFallback,
    PuaExhausted,
}

impl core::fmt::Display for IconSessionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyId => write!(f, "icon id is empty"),
            Self::IdTooLong => write!(f, "icon id exceeds {MAX_ID_BYTES} bytes"),
            Self::LabelTooLong => write!(f, "ascii label exceeds {MAX_LABEL_BYTES} bytes"),
            Self::StaticIdReserved => write!(f, "id collides with a compile-time icon"),
            Self::DuplicateId => write!(f, "id already registered in this session"),
            Self::TableFull => write!(f, "session icon table is full ({MAX_SESSION_ICONS})"),
            Self::InvalidFallback => write!(f, "unicode fallback must be a visible character"),
            Self::PuaExhausted => write!(f, "session PUA range exhausted"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionIcon {
    pub id: String,
    pub pua: char,
    pub unicode_fallback: char,
    pub ascii_label: String,
    pub category: IconCategory,
}

struct SessionTable {
    slots: [Option<SessionIcon>; MAX_SESSION_ICONS],
    used: usize,
}

impl SessionTable {
    fn new() -> Self {
        Self {
            slots: [const { None }; MAX_SESSION_ICONS],
            used: 0,
        }
    }
}

thread_local! {
    static TABLE: core::cell::RefCell<SessionTable> = core::cell::RefCell::new(SessionTable::new());
}

fn next_pua(used: usize) -> Result<char, IconSessionError> {
    let code = SESSION_PUA_START.saturating_add(used as u32);
    let pua = PuaCodepoint(code);
    if !pua.is_valid() {
        return Err(IconSessionError::PuaExhausted);
    }
    Ok(pua.as_char())
}

/// Register a session-only glyph. Does not mutate `ALL_ICONS`.
pub fn register_session_icon(
    id: &str,
    unicode_fallback: char,
    ascii_label: &str,
    category: IconCategory,
) -> Result<char, IconSessionError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(IconSessionError::EmptyId);
    }
    if id.len() > MAX_ID_BYTES {
        return Err(IconSessionError::IdTooLong);
    }
    if ascii_label.len() > MAX_LABEL_BYTES {
        return Err(IconSessionError::LabelTooLong);
    }
    if unicode_fallback == '\0' || unicode_fallback == '\u{FFFD}' {
        return Err(IconSessionError::InvalidFallback);
    }
    if icon_entry(id).is_some() {
        return Err(IconSessionError::StaticIdReserved);
    }

    TABLE.with(|cell| {
        let mut table = cell.borrow_mut();
        if table.slots.iter().flatten().any(|slot| slot.id == id) {
            return Err(IconSessionError::DuplicateId);
        }
        if table.used >= MAX_SESSION_ICONS {
            return Err(IconSessionError::TableFull);
        }
        let pua = next_pua(table.used)?;
        let slot = table
            .slots
            .iter_mut()
            .find(|s| s.is_none())
            .ok_or(IconSessionError::TableFull)?;
        *slot = Some(SessionIcon {
            id: id.to_string(),
            pua,
            unicode_fallback,
            ascii_label: ascii_label.to_string(),
            category,
        });
        table.used += 1;
        Ok(pua)
    })
}

pub fn session_icon(id: &str) -> Option<SessionIcon> {
    TABLE.with(|cell| {
        cell.borrow()
            .slots
            .iter()
            .flatten()
            .find(|slot| slot.id == id)
            .cloned()
    })
}

pub fn session_icon_count() -> usize {
    TABLE.with(|cell| cell.borrow().used)
}

pub fn clear_session_icons() {
    TABLE.with(|cell| *cell.borrow_mut() = SessionTable::new());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset() {
        clear_session_icons();
    }

    #[test]
    fn registers_bounded_custom_glyph() {
        reset();
        let pua = register_session_icon("custom-mark", '★', "Star", IconCategory::Toolbox)
            .expect("register");
        assert_eq!(pua, char::from_u32(SESSION_PUA_START).unwrap());
        assert_eq!(session_icon_count(), 1);
        let icon = session_icon("custom-mark").unwrap();
        assert_eq!(icon.unicode_fallback, '★');
        assert_eq!(icon.ascii_label, "Star");
        reset();
    }

    #[test]
    fn refuses_static_id_and_overflow() {
        reset();
        assert_eq!(
            register_session_icon("nquin", 'x', "Nope", IconCategory::System),
            Err(IconSessionError::StaticIdReserved)
        );
        for i in 0..MAX_SESSION_ICONS {
            register_session_icon(&format!("g{i}"), '•', "g", IconCategory::Toolbox).expect("fill");
        }
        assert_eq!(
            register_session_icon("overflow", '•', "g", IconCategory::Toolbox),
            Err(IconSessionError::TableFull)
        );
        reset();
    }
}
