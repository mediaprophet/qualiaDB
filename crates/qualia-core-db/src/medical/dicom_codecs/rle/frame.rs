use super::{packbits, RleError};
fn word(frame: &[u8], index: usize) -> Result<u32, RleError> {
    let start = index.checked_mul(4).ok_or(RleError::Overflow)?;
    let b = frame
        .get(start..start.checked_add(4).ok_or(RleError::Overflow)?)
        .ok_or(RleError::SegmentHeader)?;
    Ok(u32::from_le_bytes(
        b.try_into().map_err(|_| RleError::SegmentHeader)?,
    ))
}
fn offsets(frame: &[u8], planes: usize) -> Result<[usize; 16], RleError> {
    if frame.len() < 64
        || frame.len() % 2 != 0
        || planes == 0
        || planes > 15
        || word(frame, 0)? as usize != planes
    {
        return Err(RleError::SegmentHeader);
    }
    let mut offsets = [0usize; 16];
    for i in 0..15 {
        let value = word(frame, i + 1)? as usize;
        if i < planes {
            if value < 64
                || value >= frame.len()
                || value % 2 != 0
                || (i == 0 && value != 64)
                || (i > 0 && value <= offsets[i - 1])
            {
                return Err(RleError::SegmentOffsets);
            }
            offsets[i] = value;
        } else if value != 0 {
            return Err(RleError::SegmentOffsets);
        }
    }
    offsets[planes] = frame.len();
    Ok(offsets)
}
pub(super) fn validate(
    frame: &[u8],
    rows: usize,
    columns: usize,
    planes: usize,
) -> Result<(), RleError> {
    let offsets = offsets(frame, planes)?;
    for p in 0..planes {
        packbits::walk(&frame[offsets[p]..offsets[p + 1]], rows, columns, None)?;
    }
    Ok(())
}
/// MSB-first RLE byte planes become canonical native little-endian sample bytes.
pub(super) fn decode(
    frame: &[u8],
    rows: usize,
    columns: usize,
    planes: usize,
    output: &mut [u8],
    scratch: &mut [u8],
) -> Result<(), RleError> {
    let offsets = offsets(frame, planes)?;
    let pixels = rows.checked_mul(columns).ok_or(RleError::Overflow)?;
    let plane = scratch.get_mut(..pixels).ok_or(RleError::ScratchTooSmall)?;
    for p in 0..planes {
        packbits::walk(
            &frame[offsets[p]..offsets[p + 1]],
            rows,
            columns,
            Some(plane),
        )?;
        let byte_index = planes - 1 - p;
        for (pixel, value) in plane.iter().enumerate() {
            let at = pixel
                .checked_mul(planes)
                .and_then(|n| n.checked_add(byte_index))
                .ok_or(RleError::Overflow)?;
            *output.get_mut(at).ok_or(RleError::OutputTooSmall)? = *value;
        }
    }
    Ok(())
}
