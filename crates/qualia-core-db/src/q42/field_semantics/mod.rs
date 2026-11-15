//! Bounded candidate SHARED metadata/reference admission, not domain physics.
//! Scientific modes, units beyond base strings, domain schemas and imported frame
//! profiles require a supplied registry. No model execution/device authority follows.
use super::field_envelope::{Block, Entry, Field, StructuralView};
mod entries;
mod graph;
mod rules;
mod shared;
pub use rules::{validate_fields, FieldRule};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Scratch,
    Limit,
    Overflow,
    MissingField,
    FieldType,
    RequiredCritical,
    UnknownCriticalSchema,
    UnknownCriticalField,
    UnknownCriticalType,
    UnsupportedDomain,
    Reference,
    ReferenceSchema,
    Cycle,
    Depth,
    Value,
    Frame,
    Time,
    Descriptor,
    Provenance,
    Validity,
    PayloadIntegrity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub blocks: usize,
    pub entries: usize,
    pub edges: usize,
    pub depth: u32,
    /// Aggregate conservative work units; includes rescans, lookups and mask bytes.
    pub work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            blocks: 4096,
            entries: 4096,
            edges: 16384,
            depth: 32,
            work: 8_000_000,
        }
    }
}
/// Caller-owned deterministic graph workspace. Contents may change on failure.
#[derive(Clone, Copy, Debug, Default)]
pub struct NodeState {
    id: u32,
    indegree: u32,
    depth: u32,
    done: bool,
}
pub struct Scratch<'a> {
    pub metadata: &'a mut [NodeState],
    pub entries: &'a mut [NodeState],
}
/// Domain registries own exact domain field rules and numeric/reference semantics.
/// Returning rules commits the registry to validating that schema, not preserving it.
/// Callbacks must charge variable work through Context::charge_work before doing it.
pub trait DomainRegistry {
    fn rules(&self, _schema: u16, _revision: u16) -> Option<&'static [FieldRule]> {
        None
    }
    fn validate_block(&self, _block: Block<'_>, _context: &Context<'_, '_>) -> Result<(), Error> {
        Err(Error::UnsupportedDomain)
    }
    /// Descriptor keys4/5/6 name family medium/transform/calibration schemas.
    fn descriptor_target_schema(&self, _key: u16) -> Option<u16> {
        None
    }
    fn validate_units(&self, _units: &str, _context: &Context<'_, '_>) -> Result<(), Error> {
        Err(Error::UnsupportedDomain)
    }
    /// A family-specific grid order may omit shared key7 only after explicit admission.
    /// Positive spacing and nonzero dimensions have already been checked.
    fn validate_grid_frame(
        &self,
        _frame: Block<'_>,
        _context: &Context<'_, '_>,
    ) -> Result<(), Error> {
        Err(Error::UnsupportedDomain)
    }
    /// Additional family spatial-key/brick bounds. Called for every spatial entry.
    /// A grid without shared key7 has no generic ordering and fails closed by default.
    fn validate_spatial_entry(
        &self,
        _entry: Entry<'_>,
        frame: Block<'_>,
        _context: &Context<'_, '_>,
    ) -> Result<(), Error> {
        if field(frame, 7).is_some() {
            Ok(())
        } else {
            Err(Error::UnsupportedDomain)
        }
    }
    /// Needed for georeferenced or imported/nonbase frame conventions.
    fn validate_frame_profile(
        &self,
        _frame: Block<'_>,
        _context: &Context<'_, '_>,
    ) -> Result<(), Error> {
        Err(Error::UnsupportedDomain)
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct SharedOnly;
impl DomainRegistry for SharedOnly {}
#[derive(Debug)]
pub struct Context<'v, 'a> {
    view: &'v StructuralView<'a>,
    work: std::cell::Cell<u64>,
}
impl<'v, 'a> Context<'v, 'a> {
    /// Charge bounded registry work too; exhaustion fails closed.
    pub fn charge_work(&self, amount: u64) -> Result<(), Error> {
        let left = self.work.get().checked_sub(amount).ok_or(Error::Limit)?;
        self.work.set(left);
        Ok(())
    }
    pub fn remaining_work(&self) -> u64 {
        self.work.get()
    }
    /// Budgeted metadata traversal for domain cross-reference validation.
    pub fn blocks(&self) -> impl Iterator<Item = Result<Block<'a>, Error>> + '_ {
        self.view.blocks().map(move |block| {
            self.charge_work(1)?;
            Ok(block)
        })
    }
    pub fn block(&self, id: u32) -> Result<Block<'a>, Error> {
        for block in self.view.blocks() {
            self.charge_work(1)?;
            if block.id() == id {
                return Ok(block);
            }
        }
        Err(Error::Reference)
    }
    pub fn entry(&self, id: u32) -> Result<Entry<'a>, Error> {
        self.charge_work(1)?;
        self.view.entry(id).map_err(|_| Error::Reference)
    }
    pub fn require_schema(&self, id: u32, schema: u16) -> Result<Block<'a>, Error> {
        let block = self.block(id)?;
        if block.schema_type() != schema || block.revision() != 1 {
            return Err(Error::ReferenceSchema);
        }
        Ok(block)
    }
    fn require_admitted(
        &self,
        id: u32,
        registry: &impl DomainRegistry,
    ) -> Result<Block<'a>, Error> {
        let block = self.block(id)?;
        if rules::shared_rules(block.schema_type(), block.revision()).is_none()
            && registry
                .rules(block.schema_type(), block.revision())
                .is_none()
        {
            return Err(Error::UnsupportedDomain);
        }
        Ok(block)
    }
}
/// Only shared metadata and reference closure are admitted. Domain payloads and
/// numeric providers remain separate; whole requested chunks still need verification.
pub struct SharedAdmission<'v, 'a> {
    pub context: Context<'v, 'a>,
    pub preserved_unknown_blocks: u32,
}
pub fn admit_shared<'v, 'a>(
    view: &'v StructuralView<'a>,
    registry: &impl DomainRegistry,
    scratch: &mut Scratch<'_>,
    limits: Limits,
) -> Result<SharedAdmission<'v, 'a>, Error> {
    let context = Context {
        view,
        work: std::cell::Cell::new(limits.work),
    };
    let mut count = 0usize;
    for _ in view.blocks() {
        context.charge_work(1)?;
        count += 1;
        if count > limits.blocks {
            return Err(Error::Limit);
        }
    }
    if count > limits.blocks || view.entry_count() as usize > limits.entries {
        return Err(Error::Limit);
    }
    if scratch.metadata.len() < count || scratch.entries.len() < view.entry_count() as usize {
        return Err(Error::Scratch);
    }
    let mut unknown = 0u32;
    for block in view.blocks() {
        context.charge_work(block.raw_bytes().len() as u64)?;
        let fields = block.fields().count() as u64;
        context.charge_work(fields.checked_mul(64).ok_or(Error::Overflow)?)?;
        for field in block.fields() {
            if !(1..=11).contains(&field.type_tag()) && field.flags() & 1 != 0 {
                return Err(Error::UnknownCriticalType);
            }
        }
        if let Some(rules) = rules::shared_rules(block.schema_type(), block.revision()) {
            context.charge_work(
                fields
                    .checked_mul(rules.len() as u64 + 1)
                    .ok_or(Error::Overflow)?,
            )?;
            validate_fields(block, rules)?;
            shared::validate(block, &context, registry)?;
        } else if let Some(rules) = registry.rules(block.schema_type(), block.revision()) {
            validate_fields(block, rules)?;
            registry.validate_block(block, &context)?;
        } else {
            if block.flags() & 1 != 0 {
                return Err(Error::UnknownCriticalSchema);
            }
            if block.fields().any(|f| f.flags() & 1 != 0) {
                return Err(Error::UnknownCriticalField);
            }
            unknown = unknown.checked_add(1).ok_or(Error::Overflow)?;
        }
    }
    entries::validate(&context, registry)?;
    graph::metadata(&context, &mut scratch.metadata[..count], limits)?;
    graph::ancestry(
        &context,
        &mut scratch.entries[..view.entry_count() as usize],
        limits,
    )?;
    Ok(SharedAdmission {
        context,
        preserved_unknown_blocks: unknown,
    })
}
pub fn field<'a>(block: Block<'a>, key: u16) -> Option<Field<'a>> {
    block.fields().find(|f| f.key() == key)
}
fn required<'a>(block: Block<'a>, key: u16) -> Result<Field<'a>, Error> {
    field(block, key).ok_or(Error::MissingField)
}
fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}
fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}
fn scalar32(block: Block<'_>, key: u16) -> Result<u32, Error> {
    Ok(u32_at(required(block, key)?.value(), 0))
}
fn scalar64(block: Block<'_>, key: u16) -> Result<u64, Error> {
    Ok(u64_at(required(block, key)?.value(), 0))
}
fn text(field: Field<'_>) -> Result<&str, Error> {
    std::str::from_utf8(field.value()).map_err(|_| Error::Value)
}
fn identifier(field: Field<'_>) -> Result<(), Error> {
    let s = text(field)?;
    if s.is_empty() || s.chars().any(|c| c.is_control()) {
        return Err(Error::Value);
    }
    Ok(())
}
#[cfg(test)]
mod tests;
