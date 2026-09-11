//! Caller-paged larger-than-RAM replica stream (E06.6).
//!
//! A file is read in fixed [`PAGE_BYTES`] windows. The window is the only
//! payload buffer on this path: the dataset is never loaded as a whole, and
//! tests must not allocate a 42 MiB (or file-sized) replica buffer to prove
//! paging. Sequential SHA-384 covers every page in order.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use sha2::{Digest, Sha384};

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Replica page window. Not a dataset allocation and not a 64 KiB QNF chunk.
pub const PAGE_BYTES: usize = 4096;

/// Caller-owned 4 KiB window plus fill/peak counters.
pub struct PageWindow {
    buf: [u8; PAGE_BYTES],
    filled: usize,
    peak_filled: usize,
    pages_seen: u32,
}

impl PageWindow {
    pub const fn new() -> Self {
        Self {
            buf: [0u8; PAGE_BYTES],
            filled: 0,
            peak_filled: 0,
            pages_seen: 0,
        }
    }

    #[inline]
    pub const fn capacity(&self) -> usize {
        PAGE_BYTES
    }

    #[inline]
    pub fn filled(&self) -> usize {
        self.filled
    }

    /// Peak bytes placed in this window. Never the file length.
    #[inline]
    pub fn peak_bytes(&self) -> usize {
        self.peak_filled
    }

    #[inline]
    pub fn pages_seen(&self) -> u32 {
        self.pages_seen
    }

    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.buf[..self.filled]
    }

    fn accept(&mut self, n: usize) {
        self.filled = n;
        if n > self.peak_filled {
            self.peak_filled = n;
        }
        if n > 0 {
            self.pages_seen = self.pages_seen.saturating_add(1);
        }
    }
}

impl Default for PageWindow {
    fn default() -> Self {
        Self::new()
    }
}

/// Open replica file. Length is metadata, not a loaded buffer.
pub struct PagedFile {
    file: File,
    len: u64,
}

impl PagedFile {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, QdnfError> {
        let file = File::open(path.as_ref()).map_err(|_| QdnfError::Incomplete)?;
        let len = file.metadata().map_err(|_| QdnfError::Incomplete)?.len();
        Ok(Self { file, len })
    }

    #[inline]
    pub fn logical_len(&self) -> u64 {
        self.len
    }

    /// Copy one page at `offset` into `window`. Offset at/past EOF yields 0.
    pub fn read_page(&mut self, offset: u64, window: &mut PageWindow) -> Result<usize, QdnfError> {
        if offset >= self.len {
            window.accept(0);
            return Ok(0);
        }
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|_| QdnfError::Incomplete)?;
        let remain = (self.len - offset) as usize;
        let want = if remain < PAGE_BYTES {
            remain
        } else {
            PAGE_BYTES
        };
        let n = self
            .file
            .read(&mut window.buf[..want])
            .map_err(|_| QdnfError::Incomplete)?;
        window.accept(n);
        Ok(n)
    }
}

/// Write `page_count` patterned pages. Holds at most one page in RAM.
pub fn write_patterned_pages(
    path: impl AsRef<Path>,
    page_count: u32,
    seed: u8,
) -> Result<StrongDigest, QdnfError> {
    if page_count == 0 {
        return Err(QdnfError::Malformed);
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path.as_ref())
        .map_err(|_| QdnfError::Incomplete)?;
    let mut page = [0u8; PAGE_BYTES];
    let mut hasher = Sha384::new();
    let mut i = 0u32;
    while i < page_count {
        fill_page(&mut page, seed, i);
        file.write_all(&page).map_err(|_| QdnfError::Incomplete)?;
        hasher.update(&page);
        i += 1;
    }
    file.sync_all().map_err(|_| QdnfError::Incomplete)?;
    Ok(finish(hasher))
}

/// Sequential SHA-384 over every page. `window` is the only payload buffer.
pub fn hash_file_pages(
    file: &mut PagedFile,
    window: &mut PageWindow,
    page_digests: &mut [StrongDigest],
) -> Result<(StrongDigest, u32), QdnfError> {
    if file.logical_len() == 0 {
        return Err(QdnfError::Range);
    }
    let mut hasher = Sha384::new();
    let mut offset = 0u64;
    let mut pages = 0u32;
    while offset < file.logical_len() {
        let n = file.read_page(offset, window)?;
        if n == 0 {
            return Err(QdnfError::Range);
        }
        hasher.update(&window.buf[..n]);
        if (pages as usize) < page_digests.len() {
            page_digests[pages as usize] = page_digest(&window.buf[..n]);
        }
        offset = offset
            .checked_add(n as u64)
            .ok_or(QdnfError::Range)?;
        pages = pages.checked_add(1).ok_or(QdnfError::Range)?;
    }
    if offset != file.logical_len() {
        return Err(QdnfError::Range);
    }
    Ok((finish(hasher), pages))
}

/// Hash every page and require `expected_len` plus `expected` digest.
pub fn verify_file_pages(
    file: &mut PagedFile,
    expected_len: u64,
    expected: StrongDigest,
    window: &mut PageWindow,
    page_digests: &mut [StrongDigest],
) -> Result<(StrongDigest, u32), QdnfError> {
    if expected == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    if expected_len == 0 || file.logical_len() != expected_len {
        return Err(QdnfError::Range);
    }
    let (digest, pages) = hash_file_pages(file, window, page_digests)?;
    if digest != expected {
        return Err(QdnfError::Conflict);
    }
    Ok((digest, pages))
}

fn fill_page(page: &mut [u8; PAGE_BYTES], seed: u8, index: u32) {
    let mut i = 0usize;
    while i < PAGE_BYTES {
        page[i] = seed.wrapping_add(index as u8).wrapping_add(i as u8);
        i += 1;
    }
}

fn page_digest(bytes: &[u8]) -> StrongDigest {
    let mut hasher = Sha384::new();
    hasher.update(bytes);
    finish(hasher)
}

fn finish(hasher: Sha384) -> StrongDigest {
    let out = hasher.finalize();
    let mut d = StrongDigest::ZERO;
    d.0.copy_from_slice(&out);
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const FILE_PAGES: u32 = 16;
    const FILE_BYTES: u64 = (FILE_PAGES as u64) * (PAGE_BYTES as u64);

    #[test]
    fn sixty_four_kib_file_pages_at_four_kib() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("replica.bin");
        let expected = write_patterned_pages(&path, FILE_PAGES, 0xA5).unwrap();
        let mut file = PagedFile::open(&path).unwrap();
        assert_eq!(file.logical_len(), FILE_BYTES);
        assert!(FILE_BYTES > PAGE_BYTES as u64);

        let mut window = PageWindow::new();
        let mut page_digests = [StrongDigest::ZERO; FILE_PAGES as usize];
        let (digest, pages) = verify_file_pages(
            &mut file,
            FILE_BYTES,
            expected,
            &mut window,
            &mut page_digests,
        )
        .unwrap();
        assert_eq!(digest, expected);
        assert_eq!(pages, FILE_PAGES);
        assert_eq!(window.pages_seen(), FILE_PAGES);
        assert_eq!(window.capacity(), PAGE_BYTES);
        assert_eq!(window.peak_bytes(), PAGE_BYTES);
        assert!(core::mem::size_of::<PageWindow>() >= PAGE_BYTES);
        assert!(core::mem::size_of::<PageWindow>() < FILE_BYTES as usize);

        let mut i = 0usize;
        while i < FILE_PAGES as usize {
            assert_ne!(page_digests[i], StrongDigest::ZERO);
            if i > 0 {
                assert_ne!(page_digests[i], page_digests[i - 1]);
            }
            i += 1;
        }

        let mut wrong = expected;
        wrong.0[0] ^= 1;
        let mut window2 = PageWindow::new();
        let mut digests2 = [StrongDigest::ZERO; FILE_PAGES as usize];
        assert_eq!(
            verify_file_pages(&mut file, FILE_BYTES, wrong, &mut window2, &mut digests2),
            Err(QdnfError::Conflict)
        );
        assert_eq!(window2.peak_bytes(), PAGE_BYTES);
    }

    #[test]
    fn length_mismatch_is_range() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("short.bin");
        let expected = write_patterned_pages(&path, 2, 0x11).unwrap();
        let mut file = PagedFile::open(&path).unwrap();
        let mut window = PageWindow::new();
        let mut digests = [StrongDigest::ZERO; 2];
        assert_eq!(
            verify_file_pages(&mut file, 4096, expected, &mut window, &mut digests),
            Err(QdnfError::Range)
        );
    }
}
