use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameHandle {
    CpuRgbaSlot(u8),
}

#[derive(Clone)]
pub struct SharedFrameBuffer {
    slots: Arc<[Mutex<Box<[u8]>>; 2]>,
    byte_len: usize,
}

impl SharedFrameBuffer {
    pub fn new(byte_len: usize) -> Self {
        Self {
            slots: Arc::new([
                Mutex::new(vec![0u8; byte_len].into_boxed_slice()),
                Mutex::new(vec![0u8; byte_len].into_boxed_slice()),
            ]),
            byte_len,
        }
    }

    pub fn byte_len(&self) -> usize {
        self.byte_len
    }

    pub fn with_slot<R>(&self, slot: u8, reader: impl FnOnce(&[u8]) -> R) -> Option<R> {
        let slot = self.slots.get(slot as usize)?;
        let guard = slot.lock().ok()?;
        Some(reader(&guard))
    }

    pub(crate) fn overwrite_slot(&self, slot: usize, writer: impl FnOnce(&mut [u8])) -> Option<()> {
        let slot = self.slots.get(slot)?;
        let mut guard = slot.lock().ok()?;
        writer(&mut guard);
        Some(())
    }
}

pub type StateHash = [u8; 32];

pub(crate) fn compute_state_hash(bytes: &[u8]) -> StateHash {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}

#[derive(Debug, Clone)]
pub struct SimulationSnapshot {
    pub epoch: u64,
    pub dimensions: (u32, u32),
    pub state_hash: StateHash,
    pub frame: FrameHandle,
}
