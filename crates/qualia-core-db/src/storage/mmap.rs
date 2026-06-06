use memmap2::MmapMut;
use std::fs::OpenOptions;
use crate::QualiaQuin;

pub struct MmapStore {
    mmap: MmapMut,
    capacity_quins: usize,
    active_quins: usize,
}

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
    pub fn append(&mut self, quin: &QualiaQuin) -> Result<(), std::io::Error> {
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
    pub fn as_slice(&self) -> &[QualiaQuin] {
        let active_bytes = self.active_quins * 48;
        let slice = &self.mmap[..active_bytes];
        bytemuck::cast_slice(slice)
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
        
        let quin = QualiaQuin {
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
