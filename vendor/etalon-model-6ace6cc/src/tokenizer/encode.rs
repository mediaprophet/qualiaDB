use super::{
    alphabet::{self, ALPHABET},
    budget::{capacity, Work},
    index, pretoken, *,
};
use core::mem::size_of;

fn push(
    out: &mut [u32],
    written: &mut usize,
    id: u32,
    work: &mut Work,
) -> Result<(), TokenizerError> {
    work.charge(1)?;
    *out.get_mut(*written).ok_or(TokenizerError::Capacity)? = id;
    *written += 1;
    Ok(())
}

impl TokenizerPlan<'_, '_> {
    /// Encodes raw completion text. No template, BOS, EOS or stop marker is added.
    pub fn encode_raw_into(
        &self,
        text: &str,
        workspace: ExecutionWorkspace<'_>,
        out: &mut [u32],
        budget: ExecutionBudget,
    ) -> Result<usize, TokenizerError> {
        self.encode(text, workspace, out, budget, false)
    }
    /// Raw encoding followed only by declared BOS insertion when not already first.
    pub fn encode_prompt_into(
        &self,
        text: &str,
        workspace: ExecutionWorkspace<'_>,
        out: &mut [u32],
        budget: ExecutionBudget,
    ) -> Result<usize, TokenizerError> {
        self.encode(text, workspace, out, budget, true)
    }
    fn encode(
        &self,
        text: &str,
        workspace: ExecutionWorkspace<'_>,
        out: &mut [u32],
        budget: ExecutionBudget,
        prompt: bool,
    ) -> Result<usize, TokenizerError> {
        capacity(
            &[
                (workspace.spans.len(), size_of::<PretokenSpan>()),
                (workspace.encoded.len(), 1),
                (workspace.symbols.len(), size_of::<SymbolSpan>()),
                (out.len(), size_of::<u32>()),
            ],
            budget.capacity_bytes,
        )?;
        let mut work = Work(budget.max_work);
        let mut written = 0;
        let mut remaining = text;
        while !remaining.is_empty() {
            work.charge(1)?;
            let mut matched = None;
            for special in self.specials {
                work.charge(1)?;
                if remaining.len() >= special.text.len()
                    && work.equal(
                        &remaining.as_bytes()[..special.text.len()],
                        special.text.as_bytes(),
                    )?
                {
                    matched = Some(*special);
                    break;
                }
            }
            if let Some(special) = matched {
                push(out, &mut written, special.id, &mut work)?;
                remaining = &remaining[special.text.len()..];
                continue;
            }
            let mut next_special = remaining.len();
            for special in self.specials {
                work.charge(1)?;
                if special.text.len() > remaining.len() {
                    continue;
                }
                for pos in 0..=remaining.len() - special.text.len() {
                    work.charge(1)?;
                    if work.equal(
                        &remaining.as_bytes()[pos..pos + special.text.len()],
                        special.text.as_bytes(),
                    )? {
                        next_special = next_special.min(pos);
                        break;
                    }
                }
            }
            let segment = &remaining[..next_special];
            let count = pretoken::scan(segment, workspace.spans, &mut work)?;
            for span in &workspace.spans[..count] {
                self.piece(
                    &segment[span.start..span.end],
                    workspace.encoded,
                    workspace.symbols,
                    out,
                    &mut written,
                    &mut work,
                )?;
            }
            remaining = &remaining[next_special..];
        }
        if prompt && self.add_bos_token() && (written == 0 || out[0] != self.bos_token_id()) {
            if written == out.len() {
                return Err(TokenizerError::Capacity);
            }
            work.charge(
                written
                    .checked_add(1)
                    .ok_or(TokenizerError::ArithmeticOverflow)?,
            )?;
            out.copy_within(..written, 1);
            out[0] = self.bos_token_id();
            written += 1;
        }
        Ok(written)
    }

    fn piece(
        &self,
        piece: &str,
        encoded: &mut [u8],
        symbols: &mut [SymbolSpan],
        out: &mut [u32],
        written: &mut usize,
        work: &mut Work,
    ) -> Result<(), TokenizerError> {
        if let Some(id) = index::vocab(self.vocab, piece.as_bytes(), work)? {
            return push(out, written, id, work);
        }
        let (mut end, mut count) = (0usize, 0usize);
        for byte in piece.bytes() {
            work.charge(1)?;
            let mut utf8 = [0u8; 4];
            let value = ALPHABET[byte as usize].encode_utf8(&mut utf8).as_bytes();
            let next = end
                .checked_add(value.len())
                .ok_or(TokenizerError::ArithmeticOverflow)?;
            let target = encoded.get_mut(end..next).ok_or(TokenizerError::Capacity)?;
            let symbol = symbols.get_mut(count).ok_or(TokenizerError::Capacity)?;
            work.charge(value.len())?;
            target.copy_from_slice(value);
            *symbol = SymbolSpan {
                start: end,
                end: next,
            };
            end = next;
            count += 1;
        }
        let word = &encoded[..end];
        if let Some(id) = index::vocab(self.vocab, word, work)? {
            return push(out, written, id, work);
        }
        loop {
            work.charge(1)?;
            let mut best: Option<(usize, usize)> = None;
            for i in 0..count.saturating_sub(1) {
                work.charge(1)?;
                let a = symbols[i];
                let b = symbols[i + 1];
                if let Some(rank) = index::merge(
                    self.merges,
                    &word[a.start..a.end],
                    &word[b.start..b.end],
                    work,
                )? {
                    if best.is_none_or(|(old, _)| rank < old) {
                        best = Some((rank, i));
                    }
                }
            }
            let Some((_, i)) = best else {
                break;
            };
            work.charge(count - i)?; // merged boundary plus shifted symbol slots.
            symbols[i].end = symbols[i + 1].end;
            symbols.copy_within(i + 2..count, i + 1);
            count -= 1;
        }
        for symbol in &symbols[..count] {
            work.charge(1)?;
            let text = &word[symbol.start..symbol.end];
            if let Some(id) = index::vocab(self.vocab, text, work)? {
                push(out, written, id, work)?;
            } else {
                work.charge(text.len())?;
                let text = core::str::from_utf8(text).map_err(|_| TokenizerError::Metadata)?;
                for ch in text.chars() {
                    work.charge(1)?;
                    let mut utf8 = [0u8; 4];
                    let id =
                        match index::vocab(self.vocab, ch.encode_utf8(&mut utf8).as_bytes(), work)?
                        {
                            Some(id) => id,
                            None => {
                                work.charge(256)?;
                                let byte = alphabet::decode(ch).ok_or(TokenizerError::Metadata)?;
                                return Err(TokenizerError::MissingByteSymbol(byte));
                            }
                        };
                    push(out, written, id, work)?;
                }
            }
        }
        Ok(())
    }
}
