use super::{
    alphabet,
    budget::{capacity, Work},
    ExecutionBudget, TokenizerError, TokenizerPlan,
};

impl TokenizerPlan<'_, '_> {
    /// Emits exact bytes, including incomplete UTF-8 sequences. Like the legacy
    /// API, capacity must cover the vocabulary string length, even if decoding
    /// contracts it. Empty vocabulary strings validly produce zero bytes.
    pub fn decode_token_bytes_into(
        &self,
        id: u32,
        out: &mut [u8],
        budget: ExecutionBudget,
    ) -> Result<usize, TokenizerError> {
        capacity(&[(out.len(), 1)], budget.capacity_bytes)?;
        let mut work = Work(budget.max_work);
        let mut token = None;
        for entry in self.vocab {
            work.charge(1)?;
            if entry.id == id {
                token = Some(entry.text);
                break;
            }
        }
        let token = token.ok_or(TokenizerError::InvalidTokenId(id))?;
        if out.len() < token.len() {
            return Err(TokenizerError::Capacity);
        }
        work.charge(token.len())?;
        if token.len() == 6 && token.starts_with("<0x") && token.ends_with('>') {
            let hex = token.get(3..5).ok_or(TokenizerError::InvalidByteToken)?;
            let byte = u8::from_str_radix(hex, 16).map_err(|_| TokenizerError::InvalidByteToken)?;
            out[0] = byte;
            return Ok(1);
        }
        let mut written = 0;
        for ch in token.chars() {
            work.charge(256)?; // Conservative complete inverse-alphabet scan.
            if let Some(byte) = alphabet::decode(ch) {
                out[written] = byte;
                written += 1;
            } else {
                let mut utf8 = [0u8; 4];
                let bytes = ch.encode_utf8(&mut utf8).as_bytes();
                out[written..written + bytes.len()].copy_from_slice(bytes);
                written += bytes.len();
            }
        }
        Ok(written)
    }
}
