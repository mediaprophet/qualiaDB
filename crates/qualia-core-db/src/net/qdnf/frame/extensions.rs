//! Authenticated header extensions. Nested parse steps are bounded.

use super::errors::FrameError;

pub const MAX_EXTENSIONS: usize = 8;
pub const MAX_EXTENSION_BYTES: usize = 256;

/// Type-length-value extension after the base header.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extension {
    pub kind: u16,
    pub critical: bool,
    pub len: u16,
}

/// Parse extensions from `src` (the region between base header and payload).
/// Unknown non-critical extensions are counted and skipped. Unknown critical
/// extensions fail.
pub fn parse_extensions(src: &[u8], out: &mut [Extension]) -> Result<usize, FrameError> {
    if src.len() > MAX_EXTENSION_BYTES {
        return Err(FrameError::Capacity);
    }
    let mut offset = 0usize;
    let mut count = 0usize;
    let mut steps = 0usize;
    while offset < src.len() {
        steps = steps.checked_add(1).ok_or(FrameError::Range)?;
        if steps > MAX_EXTENSIONS {
            return Err(FrameError::Capacity);
        }
        let remaining = src.len() - offset;
        if remaining < 4 {
            return Err(FrameError::Truncated);
        }
        let kind = u16::from_be_bytes([src[offset], src[offset + 1]]);
        let len = u16::from_be_bytes([src[offset + 2], src[offset + 3]]);
        let body_end = offset
            .checked_add(4)
            .and_then(|o| o.checked_add(len as usize))
            .ok_or(FrameError::Range)?;
        if body_end > src.len() {
            return Err(FrameError::Truncated);
        }
        let critical = kind & 0x8000 != 0;
        let kind_id = kind & 0x7fff;
        if critical && !known_extension(kind_id) {
            return Err(FrameError::CriticalExtension);
        }
        if count < out.len() {
            out[count] = Extension {
                kind: kind_id,
                critical,
                len,
            };
            count += 1;
        } else {
            return Err(FrameError::Capacity);
        }
        offset = body_end;
    }
    Ok(count)
}

const fn known_extension(kind: u16) -> bool {
    matches!(kind, 1 | 2 | 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_critical_fails() {
        let src = [0x80, 0x99, 0x00, 0x00];
        let mut out = [Extension {
            kind: 0,
            critical: false,
            len: 0,
        }; 4];
        assert_eq!(
            parse_extensions(&src, &mut out),
            Err(FrameError::CriticalExtension)
        );
    }

    #[test]
    fn known_non_critical_is_counted() {
        let src = [0x00, 0x01, 0x00, 0x00];
        let mut out = [Extension {
            kind: 0,
            critical: false,
            len: 0,
        }; 4];
        assert_eq!(parse_extensions(&src, &mut out).unwrap(), 1);
        assert_eq!(out[0].kind, 1);
    }
}
