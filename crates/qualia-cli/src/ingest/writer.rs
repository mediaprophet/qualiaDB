use std::fs::File;
use std::io::{BufWriter, Write};
use qualia_core_db::NQuin; // Using NQuin

pub struct SuperBlockWriter {
    writer: BufWriter<File>,
    buffer: [NQuin; 850],
    cursor: usize,
    blocks_written: u32,
}

impl SuperBlockWriter {
    pub fn new(output_path: &std::path::Path) -> std::io::Result<Self> {
        let file = File::create(output_path)?;
        // Use a large system buffer to prevent constant OS context switches
        let writer = BufWriter::with_capacity(1024 * 1024, file); 
        
        Ok(Self {
            writer,
            buffer: [NQuin::default(); 850], 
            cursor: 0,
            blocks_written: 0,
        })
    }

    #[inline(always)]
    pub fn push(&mut self, quin: NQuin) -> std::io::Result<()> {
        self.buffer[self.cursor] = quin;
        self.cursor += 1;

        if self.cursor == 850 {
            self.flush_block()?;
        }
        Ok(())
    }

    pub fn flush_block(&mut self) -> std::io::Result<()> {
        if self.cursor == 0 {
            return Ok(());
        }

        // 1. Build the 160-byte Header
        // [4 bytes Magic Number "Q42B"] + [4 bytes block index] + [152 bytes reserved/padding]
        let mut header = [0u8; 160];
        header[0..4].copy_from_slice(b"Q42B");
        header[4..8].copy_from_slice(&self.blocks_written.to_le_bytes());
        
        // 2. Write Header
        self.writer.write_all(&header)?;

        // 3. Write Data (Zero-cost slice cast via bytemuck)
        // Note: If cursor < 850 (e.g. final block), we still write the full 850 
        // to maintain strict 40KB alignment, leaving trailing structs as Zero/Default.
        let bytes: &[u8] = bytemuck::cast_slice(&self.buffer);
        self.writer.write_all(bytes)?;

        // 4. Reset Buffer
        self.cursor = 0;
        // Zero out buffer so leftover old records aren't written if this is the last block
        self.buffer = [NQuin::default(); 850];
        self.blocks_written += 1;

        Ok(())
    }
}

impl Drop for SuperBlockWriter {
    fn drop(&mut self) {
        // Ensure final partial block is flushed when the writer goes out of scope
        let _ = self.flush_block();
        let _ = self.writer.flush();
    }
}
