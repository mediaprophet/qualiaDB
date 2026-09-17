//! Storage-backed durable jobs. Recovery is from a file, not process RAM.

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;

use crate::container_10d::crc32c::crc32c;

use super::durable::{Delivery, DurableJob, DurableQueue, MAX_BLOB};

const MAGIC: &[u8; 8] = b"QDUR1\0\0\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableIo {
    Corrupt,
    Capacity,
    Io,
}

impl DurableQueue {
    pub fn persist(&self, path: &Path) -> Result<(), DurableIo> {
        let mut body = Vec::new();
        body.extend_from_slice(MAGIC);
        body.extend_from_slice(&(self.stored_len() as u32).to_be_bytes());
        let mut i = 0;
        while i < self.stored_len() {
            let job = self.stored_job(i).ok_or(DurableIo::Corrupt)?;
            write_job(&mut body, job);
            i += 1;
        }
        let tmp = path.with_extension("tmp");
        File::create(&tmp)
            .and_then(|mut f| f.write_all(&body))
            .map_err(|_| DurableIo::Io)?;
        fs::rename(&tmp, path).map_err(|_| DurableIo::Io)
    }

    pub fn recover(path: &Path) -> Result<Self, DurableIo> {
        let mut bytes = Vec::new();
        File::open(path)
            .and_then(|mut f| f.read_to_end(&mut bytes))
            .map_err(|_| DurableIo::Io)?;
        if bytes.len() < 12 || &bytes[..8] != MAGIC {
            return Err(DurableIo::Corrupt);
        }
        let count = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let mut q = Self::new();
        let mut off = 12usize;
        let mut n = 0usize;
        while n < count {
            let (job, next) = read_job(&bytes, off)?;
            q.restore(job).map_err(|_| DurableIo::Capacity)?;
            off = next;
            n += 1;
        }
        Ok(q)
    }
}

impl DurableQueue {
    pub fn stored_len(&self) -> usize {
        self.len()
    }

    pub fn stored_job(&self, i: usize) -> Option<DurableJob> {
        self.job_at(i)
    }
}

fn write_job(out: &mut Vec<u8>, job: DurableJob) {
    let start = out.len();
    out.extend_from_slice(&job.op_id.to_be_bytes());
    out.extend_from_slice(&job.expiry_unix.to_be_bytes());
    out.push(state_byte(job.state));
    out.extend_from_slice(&job.len.to_be_bytes());
    out.extend_from_slice(&job.blob[..job.len as usize]);
    let crc = crc32c(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

fn read_job(buf: &[u8], off: usize) -> Result<(DurableJob, usize), DurableIo> {
    if off + 8 + 4 + 1 + 2 > buf.len() {
        return Err(DurableIo::Corrupt);
    }
    let op_id = u64::from_be_bytes(buf[off..off + 8].try_into().unwrap());
    let expiry = u32::from_be_bytes(buf[off + 8..off + 12].try_into().unwrap());
    let state = from_state_byte(buf[off + 12]).ok_or(DurableIo::Corrupt)?;
    let len = u16::from_be_bytes([buf[off + 13], buf[off + 14]]);
    if len as usize > MAX_BLOB {
        return Err(DurableIo::Capacity);
    }
    let body_at = off + 15;
    let crc_at = body_at + len as usize;
    if crc_at + 4 > buf.len() {
        return Err(DurableIo::Corrupt);
    }
    let expect = crc32c(&buf[off..crc_at]);
    let got = u32::from_be_bytes(buf[crc_at..crc_at + 4].try_into().unwrap());
    if expect != got {
        return Err(DurableIo::Corrupt);
    }
    let mut job = DurableJob::empty();
    job.op_id = op_id;
    job.expiry_unix = expiry;
    job.state = state;
    job.len = len;
    job.blob[..len as usize].copy_from_slice(&buf[body_at..crc_at]);
    Ok((job, crc_at + 4))
}

fn state_byte(s: Delivery) -> u8 {
    match s {
        Delivery::QueuedLocally => 0,
        Delivery::AcceptedByCustodian => 1,
        Delivery::ReceivedByEndpoint => 2,
        Delivery::AppliedDurably => 3,
        Delivery::Expired => 4,
        Delivery::Unavailable => 5,
    }
}

fn from_state_byte(b: u8) -> Option<Delivery> {
    Some(match b {
        0 => Delivery::QueuedLocally,
        1 => Delivery::AcceptedByCustodian,
        2 => Delivery::ReceivedByEndpoint,
        3 => Delivery::AppliedDurably,
        4 => Delivery::Expired,
        5 => Delivery::Unavailable,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::connectivity::durable::Delivery;

    #[test]
    fn persist_drop_recover_and_corrupt_fail_closed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("jobs.qdur");
        let mut q = DurableQueue::new();
        q.enqueue(42, 99, b"sealed").unwrap();
        q.persist(&path).unwrap();
        let rec = DurableQueue::recover(&path).unwrap();
        assert_eq!(rec.state_of(42), Some(Delivery::QueuedLocally));
        let bytes = fs::read(&path).unwrap();
        fs::write(&path, &bytes[..bytes.len() - 2]).unwrap();
        assert_eq!(DurableQueue::recover(&path), Err(DurableIo::Corrupt));
    }
}
