//! CRC-backed CSCP receipts. Truncation fails closed. Not a live path.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use crate::container_10d::crc32c::crc32c;

use super::kernel::FabricError;
use super::receipt::{OpReceipt, ReceiptStatus};
use super::wire::{decode_receipt, encode_receipt};

const MAGIC: &[u8; 8] = b"CSCPRC1\0";
pub const MAX_STORED: usize = 8;

pub fn persist_receipts(receipts: &[OpReceipt], path: &Path) -> Result<(), FabricError> {
    if receipts.len() > MAX_STORED {
        return Err(FabricError::Capacity);
    }
    let mut body = Vec::new();
    body.extend_from_slice(MAGIC);
    body.extend_from_slice(&(receipts.len() as u32).to_be_bytes());
    for r in receipts {
        let mut rec = [0u8; 256];
        let n = encode_receipt(r, &mut rec).map_err(|_| FabricError::Illegal)?;
        body.extend_from_slice(&(n as u16).to_be_bytes());
        body.extend_from_slice(&rec[..n]);
        let crc = crc32c(&rec[..n]);
        body.extend_from_slice(&crc.to_be_bytes());
    }
    let tmp = path.with_extension("tmp");
    File::create(&tmp)
        .and_then(|mut f| f.write_all(&body))
        .map_err(|_| FabricError::Capacity)?;
    fs::rename(&tmp, path).map_err(|_| FabricError::Capacity)
}

pub fn recover_receipts(path: &Path) -> Result<[Option<OpReceipt>; MAX_STORED], FabricError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|_| FabricError::Illegal)?;
    if bytes.len() < 12 || &bytes[..8] != MAGIC {
        return Err(FabricError::Illegal);
    }
    let count = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    if count > MAX_STORED {
        return Err(FabricError::Capacity);
    }
    let mut out = [None; MAX_STORED];
    let mut off = 12usize;
    let mut i = 0usize;
    while i < count {
        if off + 2 > bytes.len() {
            return Err(FabricError::Illegal);
        }
        let n = u16::from_be_bytes([bytes[off], bytes[off + 1]]) as usize;
        off += 2;
        if off + n + 4 > bytes.len() {
            return Err(FabricError::Illegal);
        }
        let rec = &bytes[off..off + n];
        let crc_got = u32::from_be_bytes([
            bytes[off + n],
            bytes[off + n + 1],
            bytes[off + n + 2],
            bytes[off + n + 3],
        ]);
        if crc32c(rec) != crc_got {
            return Err(FabricError::Illegal);
        }
        out[i] = Some(decode_receipt(rec).map_err(|_| FabricError::Illegal)?);
        off += n + 4;
        i += 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn persist_recover_and_truncated_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("r.cscp");
        let r = OpReceipt::queued(11, [0xAAu8; 32], [0xBBu8; 32], 2, 500);
        persist_receipts(&[r], &path).unwrap();
        let got = recover_receipts(&path).unwrap();
        assert_eq!(got[0].unwrap().op_id, 11);
        assert_eq!(got[0].unwrap().status, ReceiptStatus::Queued);
        let mut f = File::create(&path).unwrap();
        f.write_all(b"CSCPRC1").unwrap();
        assert!(recover_receipts(&path).is_err());
        persist_receipts(&[r], &path).unwrap();
        let recovered = recover_receipts(&path).unwrap()[0].unwrap();
        assert_eq!(
            recovered.replay(11, &[0xAAu8; 32], false, 2, 10),
            ReceiptStatus::Denied
        );
        assert_eq!(
            recovered.replay(11, &[0xAAu8; 32], true, 2, 10),
            ReceiptStatus::Committed
        );
    }
}
