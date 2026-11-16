use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;

use super::{validate_document, ContractDocument};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    InvalidInput,
    UnsupportedCapability,
    UnsatisfiedConstraint,
    CapacityExhausted,
    Cancelled,
    OracleMismatch,
    InconclusiveVerification,
    StaleIdentity,
    InvalidSchema,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractError {
    pub code: ReasonCode,
    pub field: &'static str,
}

impl ContractError {
    pub const fn new(code: ReasonCode, field: &'static str) -> Self {
        Self { code, field }
    }
}
impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.field)
    }
}
impl std::error::Error for ContractError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HashNamespace {
    #[serde(rename = "q42.sha256/v1")]
    Sha256V1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentId {
    pub namespace: HashNamespace,
    pub digest: [u8; 32],
}
impl ContentId {
    pub fn sha256(bytes: &[u8]) -> Self {
        Self {
            namespace: HashNamespace::Sha256V1,
            digest: Sha256::digest(bytes).into(),
        }
    }
    pub fn verify(self, bytes: &[u8]) -> Result<(), ContractError> {
        if self == Self::sha256(bytes) {
            Ok(())
        } else {
            Err(ContractError::new(
                ReasonCode::StaleIdentity,
                "content_digest",
            ))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ContractLimits {
    pub envelope_bytes: usize,
    pub items_per_list: usize,
    pub string_bytes: usize,
}
impl Default for ContractLimits {
    fn default() -> Self {
        Self {
            envelope_bytes: 1024 * 1024,
            items_per_list: 128,
            string_bytes: 512,
        }
    }
}
impl ContractLimits {
    pub fn validate(self) -> Result<(), ContractError> {
        if self.envelope_bytes == 0
            || self.envelope_bytes > 1024 * 1024
            || self.items_per_list == 0
            || self.items_per_list > 128
            || self.string_bytes == 0
            || self.string_bytes > 512
        {
            return Err(ContractError::new(
                ReasonCode::CapacityExhausted,
                "contract_limits",
            ));
        }
        Ok(())
    }
}

pub fn parse_document(
    bytes: &[u8],
    limits: ContractLimits,
) -> Result<ContractDocument, ContractError> {
    limits.validate()?;
    if bytes.len() > limits.envelope_bytes {
        return Err(ContractError::new(
            ReasonCode::CapacityExhausted,
            "envelope_bytes",
        ));
    }
    let document: ContractDocument = serde_json::from_slice(bytes)
        .map_err(|_| ContractError::new(ReasonCode::InvalidSchema, "contract_json"))?;
    validate_document(&document, limits)?;
    Ok(document)
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
}
impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("canonical envelope capacity"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// v1 canonical bytes are compact UTF-8 JSON in the fixed field order declared by
/// this versioned schema. Parameter BTreeMap keys are lexical; array order is normative.
/// Typed parsing removes input object order. No generic JSON tree is admitted.
/// Floating numbers use serde_json roundtrip encoding; encoding changes require v2.
/// Identity is not authentication, and timestamps belong in separate evidence records.
pub fn canonical_bytes(
    document: &ContractDocument,
    limits: ContractLimits,
) -> Result<Vec<u8>, ContractError> {
    validate_document(document, limits)?;
    let mut writer = BoundedWriter {
        bytes: Vec::with_capacity(limits.envelope_bytes),
        limit: limits.envelope_bytes,
    };
    serde_json::to_writer(&mut writer, document)
        .map_err(|_| ContractError::new(ReasonCode::CapacityExhausted, "canonical_bytes"))?;
    Ok(writer.bytes)
}

pub fn document_identity(
    document: &ContractDocument,
    limits: ContractLimits,
) -> Result<ContentId, ContractError> {
    Ok(ContentId::sha256(&canonical_bytes(document, limits)?))
}

/// A resolver must compare bytes as well as hashes if a digest key already exists.
pub fn verify_existing_identity(
    id: ContentId,
    existing: &[u8],
    candidate: &[u8],
) -> Result<(), ContractError> {
    id.verify(existing)?;
    id.verify(candidate)?;
    if existing != candidate {
        return Err(ContractError::new(
            ReasonCode::StaleIdentity,
            "digest_collision",
        ));
    }
    Ok(())
}
/// Every direct dependency must be present as a contract or an explicitly supplied
/// immutable external identity. Typed resources retain their own codecs and bytes.
pub fn validate_dependency_graph(
    nodes: &[(ContentId, &ContractDocument)],
    external: &[ContentId],
    limits: ContractLimits,
) -> Result<(), ContractError> {
    limits.validate()?;
    if nodes.len() > limits.items_per_list || external.len() > limits.items_per_list {
        return Err(ContractError::new(
            ReasonCode::CapacityExhausted,
            "dependency_graph",
        ));
    }
    for (i, (id, document)) in nodes.iter().enumerate() {
        if *id != document_identity(document, limits)?
            || nodes[..i].iter().any(|(p, _)| p == id)
            || external.contains(id)
        {
            return Err(ContractError::new(ReasonCode::StaleIdentity, "graph_node"));
        }
    }
    let mut completed = [false; 128];
    let mut remaining = nodes.len();
    while remaining > 0 {
        let mut progress = false;
        for (index, (_, document)) in nodes.iter().enumerate() {
            if completed[index] {
                continue;
            }
            let mut ready = true;
            visit_dependencies(document, &mut |dependency| {
                if let Some(position) = nodes.iter().position(|(id, _)| *id == dependency) {
                    ready &= completed[position];
                } else if !external.contains(&dependency) {
                    return Err(ContractError::new(
                        ReasonCode::StaleIdentity,
                        "missing_dependency",
                    ));
                }
                Ok(())
            })?;
            if ready {
                completed[index] = true;
                remaining -= 1;
                progress = true;
            }
        }
        if !progress {
            return Err(ContractError::new(
                ReasonCode::UnsatisfiedConstraint,
                "dependency_cycle",
            ));
        }
    }
    Ok(())
}

fn visit_resource(
    resource: &super::ResourceRef,
    f: &mut impl FnMut(ContentId) -> Result<(), ContractError>,
) -> Result<(), ContractError> {
    f(resource.id)?;
    if let super::ResourceEncoding::ExternalTyped { schema } = resource.encoding {
        f(schema)?;
    }
    Ok(())
}
fn visit_resources(
    resources: &[super::ResourceRef],
    f: &mut impl FnMut(ContentId) -> Result<(), ContractError>,
) -> Result<(), ContractError> {
    for resource in resources {
        visit_resource(resource, f)?;
    }
    Ok(())
}
fn visit_dependencies(
    document: &ContractDocument,
    f: &mut impl FnMut(ContentId) -> Result<(), ContractError>,
) -> Result<(), ContractError> {
    use super::ContractRecord;
    match &document.record {
        ContractRecord::Asset(v) => {
            visit_resource(&v.content, f)?;
            f(v.frame.identity)?;
            f(v.quality_profile)?;
            for a in &v.attributes {
                if let Some(frame) = a.frame {
                    f(frame)?;
                }
                visit_resource(&a.data, f)?;
            }
            for d in &v.domains {
                visit_resource(&d.labels, f)?;
            }
            visit_resources(&v.protected_entities, f)?;
            visit_resources(&v.source_assertions, f)?;
            for id in &v.transfer_dependencies {
                f(*id)?;
            }
        }
        ContractRecord::Quality(v) => visit_resources(&v.domains, f)?,
        ContractRecord::Candidate(v) => {
            f(v.parent)?;
            f(v.asset)?;
            f(v.source_revision)?;
            visit_resources(&v.remaps, f)?;
            for id in v.transfers.iter().chain(&v.dependencies) {
                f(*id)?;
            }
        }
        ContractRecord::Transfer(v) => {
            for id in [v.source, v.target, v.source_frame, v.target_frame] {
                f(id)?;
            }
            visit_resource(&v.support, f)?;
            visit_resources(&v.validation, f)?;
            for d in &v.domain_restrictions {
                visit_resource(&d.labels, f)?;
            }
        }
        ContractRecord::Kernel(v) => {
            f(v.semantics)?;
            f(v.discretisation)?;
            visit_resource(&v.specification, f)?;
            visit_resource(&v.reference_evaluator, f)?;
        }
        ContractRecord::Variant(v) => {
            f(v.kernel)?;
            f(v.adapter.adapter)?;
            f(v.toolchain)?;
            visit_resource(&v.abi, f)?;
            visit_resource(&v.emitted_source, f)?;
            visit_resource(&v.adapter.limits, f)?;
            if let Some(manifest) = &v.forge_manifest {
                visit_resource(manifest, f)?;
            }
            for id in &v.dependencies {
                f(*id)?;
            }
        }
        ContractRecord::Receipt(v) => {
            for id in &v.targets {
                f(*id)?;
            }
            visit_resources(&v.fixtures, f)?;
            visit_resources(&v.provenance, f)?;
            if let Some(receipt) = &v.etalon_receipt {
                visit_resource(receipt, f)?;
            }
        }
    }
    Ok(())
}

pub(super) fn unique_parameters<'de, D>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Unique;
    impl<'de> serde::de::Visitor<'de> for Unique {
        type Value = std::collections::BTreeMap<String, String>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("unique parameter names")
        }
        fn visit_map<M>(self, mut input: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut result = std::collections::BTreeMap::new();
            while let Some((key, value)) = input.next_entry::<String, String>()? {
                if result.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate normative parameter"));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Unique)
}
