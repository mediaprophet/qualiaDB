use super::RleError;
/// Normative DICOM PackBits semantics with strict row boundaries. Unlike the
/// existing PSD Vec decoder, overlong runs are rejected, never truncated.
pub(super) fn walk(
    segment: &[u8],
    rows: usize,
    columns: usize,
    mut output: Option<&mut [u8]>,
) -> Result<(), RleError> {
    let pixels = rows.checked_mul(columns).ok_or(RleError::Overflow)?;
    if output.as_ref().is_some_and(|o| o.len() != pixels) {
        return Err(RleError::OutputTooSmall);
    }
    let mut input = 0usize;
    let mut produced = 0usize;
    for _ in 0..rows {
        let row_end = produced.checked_add(columns).ok_or(RleError::Overflow)?;
        while produced < row_end {
            let control = *segment.get(input).ok_or(RleError::TruncatedRun)?;
            input = input.checked_add(1).ok_or(RleError::Overflow)?;
            if control == 128 {
                continue;
            }
            let count = if control <= 127 {
                control as usize + 1
            } else {
                257 - control as usize
            };
            let end = produced.checked_add(count).ok_or(RleError::Overflow)?;
            if end > row_end {
                return Err(RleError::RunCrossesRow);
            }
            if control <= 127 {
                let stop = input.checked_add(count).ok_or(RleError::Overflow)?;
                let literal = segment.get(input..stop).ok_or(RleError::TruncatedRun)?;
                if let Some(out) = output.as_deref_mut() {
                    out.get_mut(produced..end)
                        .ok_or(RleError::OutputTooSmall)?
                        .copy_from_slice(literal);
                }
                input = stop;
            } else {
                let byte = *segment.get(input).ok_or(RleError::TruncatedRun)?;
                input = input.checked_add(1).ok_or(RleError::Overflow)?;
                if let Some(out) = output.as_deref_mut() {
                    out.get_mut(produced..end)
                        .ok_or(RleError::OutputTooSmall)?
                        .fill(byte);
                }
            }
            produced = end;
        }
    }
    // Decoder no-ops may remain; at most one final zero is even-length padding.
    while segment.get(input) == Some(&128) {
        input = input.checked_add(1).ok_or(RleError::Overflow)?;
    }
    match segment.get(input..) {
        Some([]) | Some([0]) => Ok(()),
        _ => Err(RleError::TrailingSegmentData),
    }
}
