use bytemuck::{Pod, Zeroable};
use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::path::Path;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ThermalEvictionRecord {
    pub timestamp_ms: u64,
    pub page_id: u32,
    pub fast_entropy: f32,
    pub top1_v: f32,
    pub top2_v: f32,
    pub reserved: [u8; 8], // align to 32 bytes
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ThermalWalHeader {
    pub magic: [u8; 4], // "WAL\0"
    pub version: u32,
    pub head: u32,          // index of next write
    pub capacity: u32,      // total number of records
    pub reserved: [u8; 48], // align to 64 bytes
}

pub struct ThermalWal {
    mmap: MmapMut,
    capacity: usize,
    head: usize,
}

impl ThermalWal {
    pub fn open(path: &Path, capacity_records: usize) -> std::io::Result<Self> {
        let file_size = 64 + (capacity_records * 32);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
        file.set_len(file_size as u64)?;

        let mut mmap = unsafe { MmapOptions::new().map_mut(&file)? };

        // initialize header if new
        let header_ptr = mmap.as_mut_ptr() as *mut ThermalWalHeader;
        let mut head = 0;
        unsafe {
            if (*header_ptr).magic != *b"WAL\0" {
                (*header_ptr).magic = *b"WAL\0";
                (*header_ptr).version = 1;
                (*header_ptr).head = 0;
                (*header_ptr).capacity = capacity_records as u32;
            } else {
                head = (*header_ptr).head as usize;
            }
        }

        Ok(Self {
            mmap,
            capacity: capacity_records,
            head,
        })
    }

    pub fn append(&mut self, record: ThermalEvictionRecord) {
        let offset = 64 + (self.head * 32);
        let record_bytes = bytemuck::bytes_of(&record);
        self.mmap[offset..offset + 32].copy_from_slice(record_bytes);

        self.head = (self.head + 1) % self.capacity;
        let header_ptr = self.mmap.as_mut_ptr() as *mut ThermalWalHeader;
        unsafe {
            (*header_ptr).head = self.head as u32;
        }
    }
}
