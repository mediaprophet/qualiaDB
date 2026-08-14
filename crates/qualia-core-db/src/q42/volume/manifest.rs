//! Embedded root-volume descriptor for a logical multi-segment Q42 dataset.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use super::super::{Q42Volume, MAX_COMPRESSED_SUPERBLOCK_SIZE, SUPERBLOCK_SIZE};
use super::range::{verify_source_sha256, Q42RangeSource};
use super::range_volume::Q42RangeVolume;

pub const VOLUME_MANIFEST_MAGIC: [u8; 8] = *b"Q42VOL\0\0";
pub const VOLUME_MANIFEST_VERSION: u16 = 1;
pub const MAX_VOLUME_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_VOLUME_SEGMENTS: usize = 65_536;
const HEADER_BYTES: usize = 32;
const ENTRY_FIXED_BYTES: usize = 66;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// One immutable child segment in a logical Q42 volume.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Q42VolumeSegment {
    /// A root-relative local path today; a future range source will also accept
    /// immutable content-addressed locators such as `ipfs://<cid>`.
    pub locator: String,
    pub byte_length: u64,
    pub first_object_hash: u64,
    pub last_object_hash: u64,
    pub quin_count: u64,
    pub sha256: [u8; 32],
}

/// Front-embedded catalog which makes immutable Q42 segments one snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Q42VolumeManifest {
    pub generation: u64,
    pub segments: Vec<Q42VolumeSegment>,
}

/// A contiguous interval of volume segments that can contain one object hash.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42SegmentMatchRange {
    pub start: usize,
    pub end: usize,
}

impl Q42SegmentMatchRange {
    pub fn len(self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// One caller-buffered page from a manifest object-hash segment match.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Q42SegmentMatchPage {
    pub range: Q42SegmentMatchRange,
    pub returned: usize,
    pub next_cursor: Option<usize>,
}

impl Q42VolumeManifest {
    pub fn encode(&self) -> io::Result<Vec<u8>> {
        self.validate()?;
        let mut bytes = Vec::with_capacity(HEADER_BYTES + self.segments.len() * ENTRY_FIXED_BYTES);
        bytes.extend_from_slice(&VOLUME_MANIFEST_MAGIC);
        bytes.extend_from_slice(&VOLUME_MANIFEST_VERSION.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&(self.segments.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&self.generation.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        for segment in &self.segments {
            let locator = segment.locator.as_bytes();
            let locator_len = u16::try_from(locator.len()).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Q42 segment locator exceeds u16 length",
                )
            })?;
            bytes.extend_from_slice(&segment.byte_length.to_le_bytes());
            bytes.extend_from_slice(&segment.first_object_hash.to_le_bytes());
            bytes.extend_from_slice(&segment.last_object_hash.to_le_bytes());
            bytes.extend_from_slice(&segment.quin_count.to_le_bytes());
            bytes.extend_from_slice(&segment.sha256);
            bytes.extend_from_slice(&locator_len.to_le_bytes());
            bytes.extend_from_slice(locator);
        }
        if bytes.len() > MAX_VOLUME_MANIFEST_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 volume manifest exceeds the 4 MiB front-matter ceiling",
            ));
        }
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < HEADER_BYTES || bytes.len() > MAX_VOLUME_MANIFEST_BYTES {
            return Err(invalid("invalid Q42 volume manifest length"));
        }
        if bytes[0..8] != VOLUME_MANIFEST_MAGIC {
            return Err(invalid("invalid Q42 volume manifest magic"));
        }
        if u16::from_le_bytes(bytes[8..10].try_into().unwrap()) != VOLUME_MANIFEST_VERSION {
            return Err(invalid("unsupported Q42 volume manifest version"));
        }
        let count = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        if count == 0 || count > MAX_VOLUME_SEGMENTS {
            return Err(invalid("invalid Q42 volume manifest segment count"));
        }
        let generation = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
        let mut offset = HEADER_BYTES;
        let mut segments = Vec::with_capacity(count);
        for _ in 0..count {
            let fixed_end = offset
                .checked_add(ENTRY_FIXED_BYTES)
                .ok_or_else(|| invalid("manifest entry overflow"))?;
            if fixed_end > bytes.len() {
                return Err(invalid("truncated Q42 volume manifest entry"));
            }
            let byte_length = u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap());
            let first_object_hash =
                u64::from_le_bytes(bytes[offset + 8..offset + 16].try_into().unwrap());
            let last_object_hash =
                u64::from_le_bytes(bytes[offset + 16..offset + 24].try_into().unwrap());
            let quin_count =
                u64::from_le_bytes(bytes[offset + 24..offset + 32].try_into().unwrap());
            let sha256 = bytes[offset + 32..offset + 64].try_into().unwrap();
            offset = fixed_end;
            let locator_len =
                u16::from_le_bytes(bytes[offset - 2..offset].try_into().unwrap()) as usize;
            let locator_end = offset
                .checked_add(locator_len)
                .ok_or_else(|| invalid("manifest locator overflow"))?;
            if locator_end > bytes.len() {
                return Err(invalid("truncated Q42 segment locator"));
            }
            let locator = std::str::from_utf8(&bytes[offset..locator_end])
                .map_err(|_| invalid("Q42 segment locator is not UTF-8"))?
                .to_owned();
            segments.push(Q42VolumeSegment {
                locator,
                byte_length,
                first_object_hash,
                last_object_hash,
                quin_count,
                sha256,
            });
            offset = locator_end;
        }
        if offset != bytes.len() {
            return Err(invalid("Q42 volume manifest has trailing bytes"));
        }
        let manifest = Self {
            generation,
            segments,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> io::Result<()> {
        if self.segments.is_empty() || self.segments.len() > MAX_VOLUME_SEGMENTS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 volume must contain 1..=65536 segments",
            ));
        }
        let mut previous_last = None;
        for segment in &self.segments {
            validate_segment_locator(&segment.locator)?;
            if segment.byte_length == 0
                || segment.quin_count == 0
                || segment.first_object_hash > segment.last_object_hash
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Q42 segment metadata is invalid",
                ));
            }
            if previous_last.is_some_and(|last| segment.first_object_hash < last) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Q42 segments are not globally object-sorted",
                ));
            }
            previous_last = Some(segment.last_object_hash);
        }
        Ok(())
    }

    /// Return every segment whose committed object interval can contain the
    /// given value. Equal boundaries remain complete across adjacent volumes.
    pub fn segment_range_for_object(&self, object_hash: u64) -> Option<Q42SegmentMatchRange> {
        let mut lo = 0usize;
        let mut hi = self.segments.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.segments[mid].last_object_hash < object_hash {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let start = lo;
        if start == self.segments.len() || self.segments[start].first_object_hash > object_hash {
            return None;
        }
        lo = start;
        hi = self.segments.len();
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.segments[mid].first_object_hash <= object_hash {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        Some(Q42SegmentMatchRange { start, end: lo })
    }

    /// Fill a bounded page of matching segment indices. This caps caller work
    /// even when a high-frequency object falls on many volume boundaries.
    pub fn segment_indices_for_object_into(
        &self,
        object_hash: u64,
        cursor: usize,
        out: &mut [usize],
    ) -> io::Result<Option<Q42SegmentMatchPage>> {
        let Some(range) = self.segment_range_for_object(object_hash) else {
            return Ok(None);
        };
        if cursor > range.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 manifest segment cursor is beyond the matching interval",
            ));
        }
        if out.is_empty() && cursor < range.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 manifest segment page requires at least one output slot",
            ));
        }
        let returned = (range.len() - cursor).min(out.len());
        for (offset, slot) in out.iter_mut().take(returned).enumerate() {
            *slot = range.start + cursor + offset;
        }
        let next = cursor + returned;
        Ok(Some(Q42SegmentMatchPage {
            range,
            returned,
            next_cursor: (next < range.len()).then_some(next),
        }))
    }

    pub fn segment_from_file(path: &Path, locator: String) -> io::Result<Q42VolumeSegment> {
        let volume = Q42Volume::open(path)?;
        let mut first = None;
        let mut last = 0u64;
        let mut quin_count = 0u64;
        let mut block = [0u8; crate::q42_volume::SUPERBLOCK_SIZE];
        for index in 0..volume.block_count() as usize {
            volume.read_superblock_into(index, &mut block)?;
            let live = u64::from_le_bytes(block[16..24].try_into().unwrap()) as usize;
            for quin_index in 0..live {
                let offset = crate::q42_volume::SUPERBLOCK_HEADER
                    + quin_index * crate::q42_volume::QUIN_SIZE;
                let object =
                    u64::from_le_bytes(block[offset + 16..offset + 24].try_into().unwrap());
                first.get_or_insert(object);
                last = object;
                quin_count += 1;
            }
        }
        let Some(first_object_hash) = first else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 segment has no Quins",
            ));
        };
        Ok(Q42VolumeSegment {
            locator,
            byte_length: std::fs::metadata(path)?.len(),
            first_object_hash,
            last_object_hash: last,
            quin_count,
            sha256: sha256_file(path)?,
        })
    }
}

/// Opens an immutable child source identified by the front-embedded manifest.
/// Construction may allocate; all subsequent block reads remain caller-buffered.
pub trait Q42SegmentRangeFactory {
    type Source: Q42RangeSource;

    fn open_segment(&self, segment: &Q42VolumeSegment) -> io::Result<Self::Source>;
}

impl<F, S> Q42SegmentRangeFactory for F
where
    F: Fn(&Q42VolumeSegment) -> io::Result<S>,
    S: Q42RangeSource,
{
    type Source = S;

    fn open_segment(&self, segment: &Q42VolumeSegment) -> io::Result<Self::Source> {
        self(segment)
    }
}

/// Transport-neutral logical Q42 snapshot. Root and child volumes can be
/// opened from HTTP/IPFS range sources just as from a local file.
pub struct Q42RangeVolumeSet<S: Q42RangeSource> {
    manifest: Q42VolumeManifest,
    segments: Vec<Q42RangeVolume<S>>,
}

impl<S: Q42RangeSource> Q42RangeVolumeSet<S> {
    pub fn open_root<R, F>(root: &Q42RangeVolume<R>, factory: &F) -> io::Result<Self>
    where
        R: Q42RangeSource,
        F: Q42SegmentRangeFactory<Source = S>,
    {
        let manifest_length = root
            .volume_manifest_length()?
            .ok_or_else(|| invalid("Q42 root has no embedded volume manifest"))?;
        let mut bytes = vec![0u8; manifest_length];
        root.read_volume_manifest_into(&mut bytes)?;
        let manifest = Q42VolumeManifest::decode(&bytes)?;
        let mut segments = Vec::with_capacity(manifest.segments.len());
        for entry in &manifest.segments {
            let source = factory.open_segment(entry)?;
            if source.length()? != entry.byte_length {
                return Err(invalid(format!(
                    "Q42 segment length differs from root manifest: {}",
                    entry.locator
                )));
            }
            let volume = Q42RangeVolume::open(source)?;
            if volume.object_hash_bounds()?
                != Some((entry.first_object_hash, entry.last_object_hash))
            {
                return Err(invalid(format!(
                    "Q42 segment object bounds differ from root manifest: {}",
                    entry.locator
                )));
            }
            segments.push(volume);
        }
        Ok(Self { manifest, segments })
    }

    pub fn manifest(&self) -> &Q42VolumeManifest {
        &self.manifest
    }

    pub fn segments(&self) -> &[Q42RangeVolume<S>] {
        &self.segments
    }

    /// Find the first segment whose object interval can contain `object_hash`.
    /// Segment boundary overlap is legal for high-frequency values, so callers
    /// must advance through adjoining intervals when a value spans segments.
    pub fn segment_index_for_object(&self, object_hash: u64) -> Option<usize> {
        self.manifest
            .segment_range_for_object(object_hash)
            .map(|range| range.start)
    }

    pub fn segment_indices_for_object_into(
        &self,
        object_hash: u64,
        cursor: usize,
        out: &mut [usize],
    ) -> io::Result<Option<Q42SegmentMatchPage>> {
        self.manifest
            .segment_indices_for_object_into(object_hash, cursor, out)
    }

    /// Verify every immutable child digest using a caller-owned scratch buffer.
    pub fn verify_segment_hashes(&self, scratch: &mut [u8]) -> io::Result<()> {
        for (entry, segment) in self.manifest.segments.iter().zip(&self.segments) {
            verify_source_sha256(segment.source(), &entry.sha256, scratch)?;
        }
        Ok(())
    }

    /// Verify the committed Quin count in every segment. The supplied buffers
    /// are reused for every block and cap the verifier's memory use.
    pub fn verify_segment_quin_counts(
        &self,
        compressed: &mut [u8],
        decoded: &mut [u8],
    ) -> io::Result<()> {
        if compressed.len() < MAX_COMPRESSED_SUPERBLOCK_SIZE || decoded.len() < SUPERBLOCK_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 segment verifier buffers are too small",
            ));
        }
        for (entry, segment) in self.manifest.segments.iter().zip(&self.segments) {
            let mut actual = 0u64;
            for index in 0..segment.block_count() as usize {
                segment.read_superblock_into(index, compressed, decoded)?;
                let live = u64::from_le_bytes(decoded[16..24].try_into().unwrap());
                if live > crate::QUINS_PER_BLOCK as u64 {
                    return Err(invalid("Q42 SuperBlock exceeds its Quin capacity"));
                }
                actual = actual
                    .checked_add(live)
                    .ok_or_else(|| invalid("Q42 segment Quin count overflow"))?;
            }
            if actual != entry.quin_count {
                return Err(invalid(format!(
                    "Q42 segment Quin count differs from root manifest: {}",
                    entry.locator
                )));
            }
        }
        Ok(())
    }
}

impl Q42VolumeSegment {
    /// Return the immutable CID for an `ipfs://CID` locator.
    pub fn ipfs_cid(&self) -> Option<&str> {
        self.locator.strip_prefix("ipfs://")
    }
}

fn validate_segment_locator(locator: &str) -> io::Result<()> {
    if locator.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Q42 segment locator is empty",
        ));
    }
    if let Some(cid) = locator.strip_prefix("ipfs://") {
        if cid.is_empty() || !cid.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Q42 IPFS locator has an invalid CID",
            ));
        }
        return Ok(());
    }
    let path = Path::new(locator);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Q42 segment locator must be a relative path or ipfs://CID",
        ));
    }
    Ok(())
}

/// Locally backed multi-segment query snapshot. It deliberately has no remote
/// transport yet: IPFS/HTTP range sources will implement the same contract.
pub struct Q42VolumeSet {
    root: Q42Volume,
    manifest: Q42VolumeManifest,
    segments: Vec<Q42Volume>,
}

impl Q42VolumeSet {
    pub fn open_root(path: &Path) -> io::Result<Self> {
        let root = Q42Volume::open(path)?;
        let manifest = root
            .volume_manifest()?
            .ok_or_else(|| invalid("Q42 root has no embedded volume manifest"))?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let mut segments = Vec::with_capacity(manifest.segments.len());
        for entry in &manifest.segments {
            let segment_path = parent.join(&entry.locator);
            if std::fs::metadata(&segment_path)?.len() != entry.byte_length {
                return Err(invalid(format!(
                    "Q42 segment length differs from root manifest: {}",
                    entry.locator
                )));
            }
            let segment = Q42Volume::open(&segment_path)?;
            if segment.object_hash_bounds()
                != Some((entry.first_object_hash, entry.last_object_hash))
            {
                return Err(invalid(format!(
                    "Q42 segment object bounds differ from root manifest: {}",
                    entry.locator
                )));
            }
            segments.push(segment);
        }
        Ok(Self {
            root,
            manifest,
            segments,
        })
    }

    /// The front-matter root that owns the snapshot-wide lexicon and manifest.
    pub fn root(&self) -> &Q42Volume {
        &self.root
    }

    pub fn manifest(&self) -> &Q42VolumeManifest {
        &self.manifest
    }
    pub fn segments(&self) -> &[Q42Volume] {
        &self.segments
    }

    pub fn verify_segment_hashes(&self, root_path: &Path) -> io::Result<()> {
        let parent = root_path.parent().unwrap_or_else(|| Path::new("."));
        for entry in &self.manifest.segments {
            if sha256_file(&parent.join(&entry.locator))? != entry.sha256 {
                return Err(invalid(format!(
                    "Q42 segment digest differs from root manifest: {}",
                    entry.locator
                )));
            }
        }
        Ok(())
    }
}

fn sha256_file(path: &Path) -> io::Result<[u8; 32]> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

pub fn root_relative_path(root: &Path, segment: &Path) -> io::Result<String> {
    let parent = root.parent().unwrap_or_else(|| Path::new("."));
    let relative: PathBuf = segment
        .strip_prefix(parent)
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "segment must be below the root Q42 directory",
            )
        })?
        .to_owned();
    relative.to_str().map(str::to_owned).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "segment path is not valid UTF-8",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(locator: &str) -> Q42VolumeSegment {
        Q42VolumeSegment {
            locator: locator.to_owned(),
            byte_length: 1,
            first_object_hash: 1,
            last_object_hash: 1,
            quin_count: 1,
            sha256: [7; 32],
        }
    }

    #[test]
    fn manifest_accepts_immutable_ipfs_cids_and_rejects_escaping_paths() {
        let manifest = Q42VolumeManifest {
            generation: 1,
            segments: vec![segment("ipfs://bafybeigdyrzt5v5cbe")],
        };
        let bytes = manifest.encode().unwrap();
        assert_eq!(Q42VolumeManifest::decode(&bytes).unwrap(), manifest);
        assert_eq!(manifest.segments[0].ipfs_cid(), Some("bafybeigdyrzt5v5cbe"));

        for locator in [
            "../segment.q42",
            "C:\\segment.q42",
            "/segment.q42",
            "ipfs://bad/path",
        ] {
            let invalid = Q42VolumeManifest {
                generation: 1,
                segments: vec![segment(locator)],
            };
            assert!(
                invalid.validate().is_err(),
                "locator {locator:?} must be rejected"
            );
        }
    }

    #[test]
    fn manifest_pages_all_boundary_spanning_segments_without_allocation() {
        let mut first = segment("one.q42");
        let mut second = segment("two.q42");
        let mut third = segment("three.q42");
        first.first_object_hash = 41;
        first.last_object_hash = 42;
        second.first_object_hash = 42;
        second.last_object_hash = 42;
        third.first_object_hash = 42;
        third.last_object_hash = 43;
        let manifest = Q42VolumeManifest {
            generation: 1,
            segments: vec![first, second, third],
        };
        manifest.validate().unwrap();
        let mut page = [usize::MAX; 2];
        let first_page = manifest
            .segment_indices_for_object_into(42, 0, &mut page)
            .unwrap()
            .unwrap();
        assert_eq!(first_page.range, Q42SegmentMatchRange { start: 0, end: 3 });
        assert_eq!(&page, &[0, 1]);
        assert_eq!(first_page.next_cursor, Some(2));
        let second_page = manifest
            .segment_indices_for_object_into(42, 2, &mut page)
            .unwrap()
            .unwrap();
        assert_eq!(second_page.returned, 1);
        assert_eq!(page[0], 2);
        assert_eq!(second_page.next_cursor, None);
    }
}
