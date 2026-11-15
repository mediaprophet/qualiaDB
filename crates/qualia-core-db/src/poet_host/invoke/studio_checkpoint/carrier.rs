//! Cold bounded exact source carriers. Opaque metadata stays with its original
//! whole HMC, never reattached to a changed current source entry.
use super::{codec, resources, snapshot, Checkpoint, CheckpointLimits, ResourceData};
use std::io::{self, Write};

const ORIGIN_PREFIX: &str = "authoring/origins/sha256/";

pub(super) fn origin_namespace(key: &str) -> bool {
    const PREFIX: &str = "authoring/origins/";
    key.eq_ignore_ascii_case("authoring/origins")
        || key
            .get(..PREFIX.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(PREFIX))
}

/// Content-locator integrity only; never recursively decodes an inert archive.
pub(super) fn validate_origin(
    key: &str,
    kind: &str,
    bytes: &[u8],
    meta: Option<&[u8]>,
) -> Result<(), String> {
    if !origin_namespace(key) {
        return Ok(());
    }
    let digest = key
        .strip_prefix(ORIGIN_PREFIX)
        .and_then(|s| s.strip_suffix(".hmc"))
        .filter(|s| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
        .ok_or("source origin key is not a canonical SHA-256 locator")?;
    let mut expected = [0; 32];
    hex::decode_to_slice(digest, &mut expected).map_err(|_| "source origin digest invalid")?;
    if kind != "hmc" || meta.is_some() || codec::digest(bytes) != expected {
        return Err("source origin kind/metadata/content digest mismatch".into());
    }
    Ok(())
}

/// Inert verified input, not a principal, permission, review or activation.
/// Whole original HMC bytes preserve original source JSON, entry keys, opaque
/// metadata and index context. Only the canonical decoder constructs this type.
#[derive(Clone)]
pub struct VerifiedSourceCarrier {
    checkpoint: Checkpoint,
    original_hmc: Vec<u8>,
    origin_digest: [u8; 32],
}

/// Read compatibility includes old source metadata. Writing a new source uses
/// `capture` plus `with_source_carrier`, so unknown signatures are not rebased.
pub fn decode_source_carrier(
    bytes: &[u8],
    limits: CheckpointLimits,
) -> Result<VerifiedSourceCarrier, String> {
    let checkpoint = super::decode(bytes, limits)?;
    let origin_digest = checkpoint
        .file_digest()
        .ok_or("verified carrier digest absent")?;
    let mut original_hmc = Vec::new();
    original_hmc
        .try_reserve_exact(bytes.len())
        .map_err(|_| "source carrier allocation refused")?;
    original_hmc.extend_from_slice(bytes);
    Ok(VerifiedSourceCarrier {
        checkpoint,
        original_hmc,
        origin_digest,
    })
}

impl VerifiedSourceCarrier {
    pub fn origin_digest(&self) -> [u8; 32] {
        self.origin_digest
    }
    pub fn checkpoint(&self) -> &Checkpoint {
        &self.checkpoint
    }
    pub fn into_checkpoint(self) -> Checkpoint {
        self.checkpoint
    }
    pub fn resources(&self) -> &[ResourceData] {
        self.checkpoint.resources()
    }
    /// SHA-256 qualifies the exact original package; it does not establish
    /// authorship, identifier equivalence, rights, authority or acceptance.
    pub fn origin_key(&self) -> String {
        format!(
            "authoring/origins/sha256/{}.hmc",
            hex::encode(self.origin_digest)
        )
    }
}

struct CountSource {
    bytes: usize,
    cap: usize,
}
impl Write for CountSource {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .filter(|n| *n <= self.cap)
            .ok_or_else(|| io::Error::other("carrier source exceeds caller budget"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn charge_entry(
    entries: &mut usize,
    total: &mut usize,
    key: &str,
    kind: &str,
    bytes: usize,
    meta: usize,
    limits: CheckpointLimits,
) -> Result<(), String> {
    *entries = entries.checked_add(1).ok_or("entry overflow")?;
    if *entries > limits.entries {
        return Err("retained union exceeds entry budget".into());
    }
    codec::charge(total, key, kind, bytes, meta, limits.file_bytes)
}

impl Checkpoint {
    /// Exact inert input union onto a fresh CURRENT accepted source capture.
    /// Keeps the current record/journal/parent and never copies the selected
    /// input's source body, parent or runtime authority. Host qualifies lineage
    /// separately before encoding, and retains its current operation guard.
    ///
    /// A metadata-bearing original is archived whole at a Core-owned SHA-256
    /// key with kind `hmc` and no outer metadata. Its source metadata still
    /// belongs to the original SOURCE entry INSIDE that exact archive. Distinct
    /// bundle origins can coexist; no semantic equivalence is inferred.
    /// Current source metadata must be absent. All shared resource keys must
    /// agree byte-for-byte in payload, kind and metadata including absence.
    pub fn with_source_carrier(
        mut self,
        input: &VerifiedSourceCarrier,
        limits: CheckpointLimits,
    ) -> Result<Self, String> {
        if self.artifact().id != input.checkpoint.artifact().id
            || self.artifact().domain != input.checkpoint.artifact().domain
        {
            return Err("retained input belongs to another source/domain".into());
        }
        if self.source_meta.is_some() {
            return Err("capture current source before merging a verified source carrier".into());
        }
        if input.original_hmc.len() > limits.file_bytes {
            return Err("source carrier exceeds caller file budget".into());
        }
        let mut source = CountSource {
            bytes: 0,
            cap: limits.source_bytes.min(limits.file_bytes),
        };
        serde_json::to_writer(&mut source, &self.record).map_err(|e| e.to_string())?;
        resources::validate(&self.resources, limits)?;
        resources::validate(input.resources(), limits)?;
        let mut entries = self
            .resources
            .len()
            .checked_add(1)
            .ok_or("entry overflow")?;
        let mut total = 128usize;
        codec::charge(
            &mut total,
            snapshot::SOURCE_KEY,
            snapshot::SOURCE_KIND,
            source.bytes,
            0,
            limits.file_bytes,
        )?;
        for resource in &self.resources {
            codec::charge(
                &mut total,
                &resource.key,
                &resource.kind,
                resource.bytes.len(),
                resource.meta.as_ref().map_or(0, Vec::len),
                limits.file_bytes,
            )?;
        }
        for resource in input.resources() {
            if let Some(old) = self.resources.iter().find(|r| r.key == resource.key) {
                if old != resource {
                    return Err("retained resource payload/kind/metadata conflicts".into());
                }
                continue;
            }
            charge_entry(
                &mut entries,
                &mut total,
                &resource.key,
                &resource.kind,
                resource.bytes.len(),
                resource.meta.as_ref().map_or(0, Vec::len),
                limits,
            )?;
        }
        let archive_key = input
            .checkpoint
            .source_meta
            .as_ref()
            .map(|_| input.origin_key());
        // Archive bytes stay borrowed until their entire output cost is admitted.
        if let Some(key) = &archive_key {
            if let Some(old) = self
                .resources
                .iter()
                .chain(input.resources())
                .find(|r| &r.key == key)
            {
                if old.kind != "hmc" || old.bytes != input.original_hmc || old.meta.is_some() {
                    return Err("retained source archive payload/kind/metadata conflicts".into());
                }
            } else {
                charge_entry(
                    &mut entries,
                    &mut total,
                    key,
                    "hmc",
                    input.original_hmc.len(),
                    0,
                    limits,
                )?;
            }
        }
        snapshot::validate_record(&self.record, limits)?;
        self.resources
            .try_reserve_exact(entries - 1 - self.resources.len())
            .map_err(|_| "retained carrier allocation refused")?;
        for resource in input.resources() {
            if !self.resources.iter().any(|r| r.key == resource.key) {
                self.resources.push(resource.clone());
            }
        }
        if let Some(key) = archive_key {
            if !self.resources.iter().any(|r| r.key == key) {
                self.resources.push(ResourceData {
                    key,
                    kind: "hmc".into(),
                    bytes: input.original_hmc.clone(),
                    meta: None,
                });
            }
        }
        self.file_digest = None;
        Ok(self)
    }
}
