//! Bounded fixed-size transport for radio custody receipts.

use super::*;

pub(super) fn encode(
    receipt: &RadioCustodyReceipt,
    out: &mut [u8],
) -> Result<usize, RadioReceiptError> {
    if out.len() < RADIO_RECEIPT_ENCODED_LEN {
        return Err(RadioReceiptError::OutputTooSmall {
            required: RADIO_RECEIPT_ENCODED_LEN,
            available: out.len(),
        });
    }
    let mut cursor = 0;
    put(out, &mut cursor, &RECEIPT_MAGIC);
    put(out, &mut cursor, &receipt.version.to_le_bytes());
    put(
        out,
        &mut cursor,
        &(RADIO_RECEIPT_ENCODED_LEN as u16).to_le_bytes(),
    );
    put(out, &mut cursor, &receipt.record_id.to_le_bytes());
    put(out, &mut cursor, &receipt.source_id.to_le_bytes());
    put(out, &mut cursor, &receipt.capture_id.to_le_bytes());
    put(out, &mut cursor, &receipt.record_digest);
    put(out, &mut cursor, &receipt.sequence.to_le_bytes());
    encode_previous(out, &mut cursor, receipt.previous_checkpoint);
    encode_timestamp(out, &mut cursor, receipt.timestamp);
    encode_policy(out, &mut cursor, receipt.policy);
    put(out, &mut cursor, &receipt.signer_public_key);
    put(out, &mut cursor, &receipt.signature);
    if cursor > RADIO_RECEIPT_ENCODED_LEN {
        return Err(RadioReceiptError::InvalidTransport);
    }
    out[cursor..RADIO_RECEIPT_ENCODED_LEN].fill(0);
    Ok(RADIO_RECEIPT_ENCODED_LEN)
}

pub(super) fn decode(input: &[u8]) -> Result<RadioCustodyReceipt, RadioReceiptError> {
    if input.len() != RADIO_RECEIPT_ENCODED_LEN {
        return Err(RadioReceiptError::InvalidTransport);
    }
    let mut cursor = 0;
    if take_array::<4>(input, &mut cursor)? != RECEIPT_MAGIC {
        return Err(RadioReceiptError::InvalidTransport);
    }
    let version = take_u16(input, &mut cursor)?;
    let length = take_u16(input, &mut cursor)? as usize;
    if version != RADIO_RECEIPT_VERSION || length != RADIO_RECEIPT_ENCODED_LEN {
        return Err(RadioReceiptError::InvalidTransport);
    }
    let record_id = take_u64(input, &mut cursor)?;
    let source_id = take_u64(input, &mut cursor)?;
    let capture_id = take_u64(input, &mut cursor)?;
    let record_digest = take_array::<32>(input, &mut cursor)?;
    let sequence = take_u64(input, &mut cursor)?;
    let previous_checkpoint = decode_previous(input, &mut cursor)?;
    let timestamp = decode_timestamp(input, &mut cursor)?;
    let policy = decode_policy(input, &mut cursor)?;
    let signer_public_key = take_array::<32>(input, &mut cursor)?;
    let signature = take_array::<64>(input, &mut cursor)?;
    let padding_len = RADIO_RECEIPT_ENCODED_LEN - cursor;
    if !all_zero(take(input, &mut cursor, padding_len)?)
        || cursor != RADIO_RECEIPT_ENCODED_LEN
        || record_id == 0
        || source_id == 0
        || capture_id == 0
    {
        return Err(RadioReceiptError::InvalidTransport);
    }
    validate_chain_position(sequence, previous_checkpoint)?;
    if previous_checkpoint.is_some_and(|digest| all_zero(&digest)) {
        return Err(RadioReceiptError::InvalidPreviousCheckpoint);
    }
    Ok(RadioCustodyReceipt {
        version,
        record_id,
        source_id,
        capture_id,
        record_digest,
        sequence,
        previous_checkpoint,
        timestamp,
        policy,
        signer_public_key,
        signature,
    })
}

fn decode_previous(
    input: &[u8],
    cursor: &mut usize,
) -> Result<Option<[u8; 32]>, RadioReceiptError> {
    let tag = take_u8(input, cursor)?;
    let digest = take_array::<32>(input, cursor)?;
    match tag {
        0 if all_zero(&digest) => Ok(None),
        1 => Ok(Some(digest)),
        _ => Err(RadioReceiptError::InvalidTransport),
    }
}

fn decode_timestamp(
    input: &[u8],
    cursor: &mut usize,
) -> Result<ReceiptTimestamp, RadioReceiptError> {
    let tag = take_u8(input, cursor)?;
    let bytes = take(input, cursor, 28)?;
    match tag {
        0 if all_zero(bytes) => Ok(ReceiptTimestamp::Unknown),
        1 if all_zero(&bytes[8..]) => Ok(ReceiptTimestamp::UnixSeconds(u64::from_le_bytes(
            bytes[..8]
                .try_into()
                .map_err(|_| RadioReceiptError::InvalidTransport)?,
        ))),
        2 => {
            let clock_id = u64::from_le_bytes(
                bytes[0..8]
                    .try_into()
                    .map_err(|_| RadioReceiptError::InvalidTransport)?,
            );
            let restart_epoch = u32::from_le_bytes(
                bytes[8..12]
                    .try_into()
                    .map_err(|_| RadioReceiptError::InvalidTransport)?,
            );
            let ticks_ns = u64::from_le_bytes(
                bytes[12..20]
                    .try_into()
                    .map_err(|_| RadioReceiptError::InvalidTransport)?,
            );
            let uncertainty_ns = u64::from_le_bytes(
                bytes[20..28]
                    .try_into()
                    .map_err(|_| RadioReceiptError::InvalidTransport)?,
            );
            Ok(ReceiptTimestamp::Monotonic(
                RecordClockStamp::new(clock_id, restart_epoch, ticks_ns, uncertainty_ns)
                    .map_err(|_| RadioReceiptError::InvalidTransport)?,
            ))
        }
        3 if all_zero(&bytes[8..]) => Ok(ReceiptTimestamp::WindowsUtc100ns(i64::from_le_bytes(
            bytes[..8]
                .try_into()
                .map_err(|_| RadioReceiptError::InvalidTransport)?,
        ))),
        _ => Err(RadioReceiptError::InvalidTransport),
    }
}

fn decode_policy(
    input: &[u8],
    cursor: &mut usize,
) -> Result<Option<PolicyDecisionReference>, RadioReceiptError> {
    let tag = take_u8(input, cursor)?;
    let grant = take_array::<32>(input, cursor)?;
    let request = take_array::<32>(input, cursor)?;
    let decision = take_array::<32>(input, cursor)?;
    match tag {
        0 if all_zero(&grant) && all_zero(&request) && all_zero(&decision) => Ok(None),
        1 => PolicyDecisionReference::new(grant, request, decision).map(Some),
        _ => Err(RadioReceiptError::InvalidTransport),
    }
}

fn take<'a>(
    input: &'a [u8],
    cursor: &mut usize,
    length: usize,
) -> Result<&'a [u8], RadioReceiptError> {
    let end = (*cursor)
        .checked_add(length)
        .ok_or(RadioReceiptError::InvalidTransport)?;
    let bytes = input
        .get(*cursor..end)
        .ok_or(RadioReceiptError::InvalidTransport)?;
    *cursor = end;
    Ok(bytes)
}
fn take_array<const N: usize>(
    input: &[u8],
    cursor: &mut usize,
) -> Result<[u8; N], RadioReceiptError> {
    take(input, cursor, N)?
        .try_into()
        .map_err(|_| RadioReceiptError::InvalidTransport)
}
fn take_u8(input: &[u8], cursor: &mut usize) -> Result<u8, RadioReceiptError> {
    Ok(take(input, cursor, 1)?[0])
}
fn take_u16(input: &[u8], cursor: &mut usize) -> Result<u16, RadioReceiptError> {
    Ok(u16::from_le_bytes(take_array(input, cursor)?))
}
fn take_u64(input: &[u8], cursor: &mut usize) -> Result<u64, RadioReceiptError> {
    Ok(u64::from_le_bytes(take_array(input, cursor)?))
}
