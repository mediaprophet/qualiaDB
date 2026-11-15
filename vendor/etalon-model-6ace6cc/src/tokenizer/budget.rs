use super::TokenizerError;
use core::cmp::Ordering;

pub(super) struct Work(pub u64);
impl Work {
    pub fn charge(&mut self, n: usize) -> Result<(), TokenizerError> {
        let n = u64::try_from(n).map_err(|_| TokenizerError::ArithmeticOverflow)?;
        self.0 = self.0.checked_sub(n).ok_or(TokenizerError::WorkBudget)?;
        Ok(())
    }
    pub fn compare(&mut self, a: &[u8], b: &[u8]) -> Result<Ordering, TokenizerError> {
        self.charge(1)?;
        for (&x, &y) in a.iter().zip(b) {
            self.charge(1)?;
            if x != y {
                return Ok(x.cmp(&y));
            }
        }
        Ok(a.len().cmp(&b.len()))
    }
    pub fn equal(&mut self, a: &[u8], b: &[u8]) -> Result<bool, TokenizerError> {
        self.charge(1)?;
        if a.len() != b.len() {
            return Ok(false);
        }
        Ok(self.compare(a, b)? == Ordering::Equal)
    }
}

pub(super) fn capacity(parts: &[(usize, usize)], limit: usize) -> Result<(), TokenizerError> {
    let mut total = 0usize;
    for &(count, size) in parts {
        total = total
            .checked_add(
                count
                    .checked_mul(size)
                    .ok_or(TokenizerError::ArithmeticOverflow)?,
            )
            .ok_or(TokenizerError::ArithmeticOverflow)?;
    }
    if total > limit {
        return Err(TokenizerError::CapacityBudget);
    }
    Ok(())
}
