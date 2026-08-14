//! Caller-buffered sequential SuperBlock cursor.

use std::io;

use super::super::{Q42Volume, QUINS_PER_BLOCK, SUPERBLOCK_HEADER, SUPERBLOCK_SIZE};

/// Metadata for one decoded SuperBlock returned by [`Q42BlockCursor`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42BlockMeta {
    pub block_index: usize,
    pub live_quins: usize,
}

/// Sequential, caller-buffered view of a Q42 volume's SuperBlocks.
pub struct Q42BlockCursor<'a> {
    volume: &'a Q42Volume,
    next_index: usize,
}

impl<'a> Q42BlockCursor<'a> {
    pub(super) fn new(volume: &'a Q42Volume) -> Self {
        Self {
            volume,
            next_index: 0,
        }
    }

    /// Decode the next block into `out`.  No block-sized allocation occurs in
    /// this cursor or the underlying decompressor.
    pub fn next_into(&mut self, out: &mut [u8]) -> io::Result<Option<Q42BlockMeta>> {
        if self.next_index >= self.volume.block_count() as usize {
            return Ok(None);
        }
        let block_index = self.next_index;
        self.next_index += 1;
        let decoded = self.volume.read_superblock_into(block_index, out)?;
        if decoded != SUPERBLOCK_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Q42 block did not decode to a complete SuperBlock",
            ));
        }
        let live_quins = u64::from_le_bytes(out[16..24].try_into().unwrap()) as usize;
        if live_quins > QUINS_PER_BLOCK {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Q42 SuperBlock declares more live Quins than its ledger holds",
            ));
        }
        if SUPERBLOCK_HEADER + live_quins * crate::q42_volume::QUIN_SIZE > SUPERBLOCK_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Q42 SuperBlock live Quin count exceeds its decoded bounds",
            ));
        }
        Ok(Some(Q42BlockMeta {
            block_index,
            live_quins,
        }))
    }

    #[inline]
    pub fn next_index(&self) -> usize {
        self.next_index
    }
}

impl Q42Volume {
    /// Return a sequential cursor which decodes into a caller-owned buffer.
    pub fn block_cursor(&self) -> Q42BlockCursor<'_> {
        Q42BlockCursor::new(self)
    }
}
