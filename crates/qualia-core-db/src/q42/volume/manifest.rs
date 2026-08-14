//! Embedded root-volume descriptor for a logical multi-segment Q42 dataset.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::super::Q42Volume;

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
            if segment.locator.is_empty() || Path::new(&segment.locator).is_absolute() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Q42 segment locator must be a non-empty relative path",
                ));
            }
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

/// Locally backed multi-segment query snapshot. It deliberately has no remote
/// transport yet: IPFS/HTTP range sources will implement the same contract.
pub struct Q42VolumeSet {
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
        Ok(Self { manifest, segments })
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
