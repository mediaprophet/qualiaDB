//! Caller-buffered token-piece decoding for autoregressive hot paths.

use super::{gpt2_unicode_to_byte, GgufTokenizer};

impl GgufTokenizer {
    /// Decode one token into caller-owned storage.
    ///
    /// Returns `None` for an unknown token or when `out` is too small. A token's decoded byte
    /// representation is never longer than its vocabulary string, so callers can use a bounded
    /// stack buffer without intermediate heap allocation.
    pub fn decode_token_bytes_into(&self, id: u32, out: &mut [u8]) -> Option<usize> {
        let token = self.vocab.get(id as usize)?.as_str();
        if out.len() < token.len() {
            return None;
        }

        if token.len() == 6 && token.starts_with("<0x") && token.ends_with('>') {
            let byte = u8::from_str_radix(&token[3..5], 16).ok()?;
            out[0] = byte;
            return Some(1);
        }

        if self.uses_gpt2_byte_decoder() {
            let mut written = 0usize;
            for symbol in token.chars() {
                if let Some(byte) = gpt2_unicode_to_byte(symbol) {
                    out[written] = byte;
                    written += 1;
                } else {
                    let mut encoded = [0u8; 4];
                    let bytes = symbol.encode_utf8(&mut encoded).as_bytes();
                    out[written..written + bytes.len()].copy_from_slice(bytes);
                    written += bytes.len();
                }
            }
            return Some(written);
        }

        if let Some(rest) = token
            .strip_prefix('\u{2581}')
            .or_else(|| token.strip_prefix('\u{0120}'))
        {
            out[0] = b' ';
            out[1..1 + rest.len()].copy_from_slice(rest.as_bytes());
            return Some(1 + rest.len());
        }

        out[..token.len()].copy_from_slice(token.as_bytes());
        Some(token.len())
    }
}
