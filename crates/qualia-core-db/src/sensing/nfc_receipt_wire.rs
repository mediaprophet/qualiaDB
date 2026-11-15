//! Bounded transport encoding for variable-radio custody receipts.

use super::*;
use crate::sensing::record::RecordClockStamp;

const RECEIPT_MAGIC: [u8; 4] = *b"NCR1";

pub(super) fn encode(
    receipt: &NfcCustodyReceipt,
    out: &mut [u8],
) -> Result<usize, NfcEvidenceError> {
    if out.len() < NFC_RECEIPT_ENCODED_LEN {
        return Err(NfcEvidenceError::OutputTooSmall {
            required: NFC_RECEIPT_ENCODED_LEN,
            available: out.len(),
        });
    }
    let mut cursor = 0;
    put(out, &mut cursor, &RECEIPT_MAGIC);
    put(out, &mut cursor, &NFC_EVIDENCE_VERSION.to_le_bytes());
    put(
        out,
        &mut cursor,
        &(NFC_RECEIPT_ENCODED_LEN as u16).to_le_bytes(),
    );
    put(out, &mut cursor, &receipt.record_id.to_le_bytes());
    put(out, &mut cursor, &receipt.source_id.to_le_bytes());
    put(out, &mut cursor, &receipt.capture_id.to_le_bytes());
    put(out, &mut cursor, &receipt.digest);
    put(out, &mut cursor, &receipt.sequence.to_le_bytes());
    encode_previous(out, &mut cursor, receipt.previous);
    encode_timestamp(out, &mut cursor, receipt.timestamp);
    put(out, &mut cursor, &receipt.signer_key);
    put(out, &mut cursor, &receipt.signature);
    if cursor > NFC_RECEIPT_ENCODED_LEN {
        return Err(NfcEvidenceError::InvalidTransport);
    }
    out[cursor..NFC_RECEIPT_ENCODED_LEN].fill(0);
    Ok(NFC_RECEIPT_ENCODED_LEN)
}

pub(super) fn decode(input: &[u8]) -> Result<NfcCustodyReceipt, NfcEvidenceError> {
    if input.len() != NFC_RECEIPT_ENCODED_LEN {
        return Err(NfcEvidenceError::InvalidTransport);
    }
    let mut cursor = 0;
    if take_array::<4>(input, &mut cursor)? != RECEIPT_MAGIC {
        return Err(NfcEvidenceError::InvalidTransport);
    }
    let version = take_u16(input, &mut cursor)?;
    let length = take_u16(input, &mut cursor)? as usize;
    if version != NFC_EVIDENCE_VERSION || length != NFC_RECEIPT_ENCODED_LEN {
        return Err(NfcEvidenceError::InvalidTransport);
    }
    let record_id = take_u64(input, &mut cursor)?;
    let source_id = take_u64(input, &mut cursor)?;
    let capture_id = take_u64(input, &mut cursor)?;
    let digest = take_array::<32>(input, &mut cursor)?;
    let sequence = take_u64(input, &mut cursor)?;
    let previous = decode_previous(input, &mut cursor)?;
    let timestamp = decode_timestamp(input, &mut cursor)?;
    let signer_key = take_array::<32>(input, &mut cursor)?;
    let signature = take_array::<64>(input, &mut cursor)?;
    let padding_len = NFC_RECEIPT_ENCODED_LEN - cursor;
    if !all_zero(take(input, &mut cursor, padding_len)?)
        || cursor != NFC_RECEIPT_ENCODED_LEN
        || record_id == 0
        || source_id == 0
        || capture_id == 0
    {
        return Err(NfcEvidenceError::InvalidTransport);
    }
    validate_chain(sequence, previous)?;
    if previous.is_some_and(|value| value.iter().all(|byte| *byte == 0)) {
        return Err(NfcEvidenceError::InvalidPrevious);
    }
    Ok(NfcCustodyReceipt {
        record_id,
        source_id,
        capture_id,
        digest,
        sequence,
        previous,
        timestamp,
        signer_key,
        signature,
    })
}

fn encode_previous(out: &mut [u8], cursor: &mut usize, previous: Option<[u8; 32]>) {
    match previous {
        Some(value) => {
            put(out, cursor, &[1]);
            put(out, cursor, &value);
        }
        None => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 32]);
        }
    }
}
fn encode_timestamp(out: &mut [u8], cursor: &mut usize, timestamp: ReceiptTimestamp) {
    match timestamp {
        ReceiptTimestamp::Unknown => {
            put(out, cursor, &[0]);
            put(out, cursor, &[0; 28]);
        }
        ReceiptTimestamp::UnixSeconds(value) => {
            put(out, cursor, &[1]);
            put(out, cursor, &value.to_le_bytes());
            put(out, cursor, &[0; 20]);
        }
        ReceiptTimestamp::Monotonic(value) => {
            put(out, cursor, &[2]);
            put(out, cursor, &value.clock_id().to_le_bytes());
            put(out, cursor, &value.restart_epoch().to_le_bytes());
            put(out, cursor, &value.ticks_ns().to_le_bytes());
            put(out, cursor, &value.uncertainty_ns().to_le_bytes());
        }
        ReceiptTimestamp::WindowsUtc100ns(value) => {
            put(out, cursor, &[3]);
            put(out, cursor, &value.to_le_bytes());
            put(out, cursor, &[0; 20]);
        }
    }
}
fn decode_previous(input: &[u8], cursor: &mut usize) -> Result<Option<[u8; 32]>, NfcEvidenceError> {
    let tag = take_u8(input, cursor)?;
    let digest = take_array::<32>(input, cursor)?;
    match tag {
        0 if all_zero(&digest) => Ok(None),
        1 => Ok(Some(digest)),
        _ => Err(NfcEvidenceError::InvalidTransport),
    }
}
fn decode_timestamp(
    input: &[u8],
    cursor: &mut usize,
) -> Result<ReceiptTimestamp, NfcEvidenceError> {
    let tag = take_u8(input, cursor)?;
    let bytes = take(input, cursor, 28)?;
    match tag {
        0 if all_zero(bytes) => Ok(ReceiptTimestamp::Unknown),
        1 if all_zero(&bytes[8..]) => Ok(ReceiptTimestamp::UnixSeconds(u64::from_le_bytes(
            bytes[..8]
                .try_into()
                .map_err(|_| NfcEvidenceError::InvalidTransport)?,
        ))),
        2 => {
            let clock = u64::from_le_bytes(
                bytes[0..8]
                    .try_into()
                    .map_err(|_| NfcEvidenceError::InvalidTransport)?,
            );
            let epoch = u32::from_le_bytes(
                bytes[8..12]
                    .try_into()
                    .map_err(|_| NfcEvidenceError::InvalidTransport)?,
            );
            let ticks = u64::from_le_bytes(
                bytes[12..20]
                    .try_into()
                    .map_err(|_| NfcEvidenceError::InvalidTransport)?,
            );
            let uncertainty = u64::from_le_bytes(
                bytes[20..28]
                    .try_into()
                    .map_err(|_| NfcEvidenceError::InvalidTransport)?,
            );
            Ok(ReceiptTimestamp::Monotonic(
                RecordClockStamp::new(clock, epoch, ticks, uncertainty)
                    .map_err(|_| NfcEvidenceError::InvalidTransport)?,
            ))
        }
        3 if all_zero(&bytes[8..]) => Ok(ReceiptTimestamp::WindowsUtc100ns(i64::from_le_bytes(
            bytes[..8]
                .try_into()
                .map_err(|_| NfcEvidenceError::InvalidTransport)?,
        ))),
        _ => Err(NfcEvidenceError::InvalidTransport),
    }
}
fn take<'a>(input: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], NfcEvidenceError> {
    let end = (*cursor)
        .checked_add(len)
        .ok_or(NfcEvidenceError::InvalidTransport)?;
    let value = input
        .get(*cursor..end)
        .ok_or(NfcEvidenceError::InvalidTransport)?;
    *cursor = end;
    Ok(value)
}
fn take_array<const N: usize>(
    input: &[u8],
    cursor: &mut usize,
) -> Result<[u8; N], NfcEvidenceError> {
    take(input, cursor, N)?
        .try_into()
        .map_err(|_| NfcEvidenceError::InvalidTransport)
}
fn take_u8(input: &[u8], cursor: &mut usize) -> Result<u8, NfcEvidenceError> {
    Ok(take(input, cursor, 1)?[0])
}
fn take_u16(input: &[u8], cursor: &mut usize) -> Result<u16, NfcEvidenceError> {
    Ok(u16::from_le_bytes(take_array(input, cursor)?))
}
fn take_u64(input: &[u8], cursor: &mut usize) -> Result<u64, NfcEvidenceError> {
    Ok(u64::from_le_bytes(take_array(input, cursor)?))
}

fn all_zero(bytes: &[u8]) -> bool {
    bytes.iter().all(|byte| *byte == 0)
}
