use crate::NQuin;
#[cfg(not(target_arch = "wasm32"))]
use memmap2::MmapMut;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::OpenOptions;

#[cfg(not(target_arch = "wasm32"))]
pub struct MmapStore {
    mmap: MmapMut,
    capacity_quins: usize,
    active_quins: usize,
}

#[cfg(not(target_arch = "wasm32"))]
impl MmapStore {
    // Opens or creates a file of exactly `capacity_quins * 48` bytes.
    pub fn open(path: &str, capacity_quins: usize) -> Result<Self, std::io::Error> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        let required_len = (capacity_quins * 48) as u64;
        file.set_len(required_len)?;

        let mmap = unsafe { MmapMut::map_mut(&file)? };

        Ok(Self {
            mmap,
            capacity_quins,
            active_quins: 0,
        })
    }

    // Appends a Quin. Returns error if capacity exceeded.
    pub fn append(&mut self, quin: &NQuin) -> Result<(), std::io::Error> {
        if self.active_quins >= self.capacity_quins {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "MmapStore capacity exceeded",
            ));
        }

        let offset = self.active_quins * 48;
        let bytes = bytemuck::bytes_of(quin);
        self.mmap[offset..offset + 48].copy_from_slice(bytes);

        self.active_quins += 1;
        Ok(())
    }

    // Returns a zero-copy slice of all active Quins.
    pub fn as_slice(&self) -> &[NQuin] {
        let active_bytes = self.active_quins * 48;
        let slice = &self.mmap[..active_bytes];
        bytemuck::cast_slice(slice)
    }
}

#[cfg(target_arch = "wasm32")]
pub struct MmapStore {
    data: Vec<NQuin>,
    capacity_quins: usize,
}

#[cfg(target_arch = "wasm32")]
impl MmapStore {
    pub fn open(_path: &str, capacity_quins: usize) -> Result<Self, std::io::Error> {
        Ok(Self {
            data: Vec::with_capacity(capacity_quins),
            capacity_quins,
        })
    }

    pub fn append(&mut self, quin: &NQuin) -> Result<(), std::io::Error> {
        if self.data.len() >= self.capacity_quins {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "MmapStore capacity exceeded",
            ));
        }

        self.data.push(*quin);
        Ok(())
    }

    pub fn as_slice(&self) -> &[NQuin] {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_mmap_store_open() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        let store = MmapStore::open(path, 10).unwrap();
        assert_eq!(store.capacity_quins, 10);
        assert_eq!(store.active_quins, 0);
    }

    #[test]
    fn test_mmap_store_append_and_slice() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        let mut store = MmapStore::open(path, 10).unwrap();

        let quin = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 4,
            metadata: 5,
            parity: 6,
        };

        store.append(&quin).unwrap();
        assert_eq!(store.active_quins, 1);

        let slice = store.as_slice();
        assert_eq!(slice.len(), 1);
        assert_eq!(slice[0].subject, 1);
        assert_eq!(slice[0].predicate, 2);
        assert_eq!(slice[0].object, 3);
        assert_eq!(slice[0].context, 4);
        assert_eq!(slice[0].metadata, 5);
        assert_eq!(slice[0].parity, 6);
    }
}
