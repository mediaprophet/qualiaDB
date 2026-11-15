use super::{fail, Result};
use qualia_core_db::{
    container_10d::{field_section::FieldDescriptor, TypedPayloadResolver},
    sensing::{
        record::ResolvedFieldPayloadBinding, MeasurementModality, PhysicalFieldSemantics,
        SemanticId,
    },
};
use serde::Deserialize;

/// Untrusted declarations: no automatic FIEL/MID2 or URI registry identity.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub container_id_hex: String,
    pub field_id: u64,
    pub field_name_hash: u64,
    pub legacy_unit_hash: u64,
    pub field_type: u8,
    pub support: u8,
    pub representation: u8,
    pub modality: String,
    pub physical_quantity_id: Option<u64>,
    #[serde(default)]
    pub supported_extension_kinds: Vec<u16>,
}

impl Binding {
    pub fn validate(&self) -> Result<()> {
        if self.supported_extension_kinds.len() > 32
            || self.container_id_hex.len() != 32
            || self.physical_quantity_id == Some(0)
        {
            return fail("invalid binding declaration");
        }
        self.modality()?;
        Ok(())
    }

    fn modality(&self) -> Result<MeasurementModality> {
        Ok(match self.modality.as_str() {
            "unknown" => MeasurementModality::Unknown,
            "electromagnetic" => MeasurementModality::Electromagnetic,
            "optical" => MeasurementModality::Optical,
            "acoustic" => MeasurementModality::Acoustic,
            "inertial" => MeasurementModality::Inertial,
            "thermal" => MeasurementModality::Thermal,
            "pressure" => MeasurementModality::Pressure,
            _ => return fail("unsupported modality declaration"),
        })
    }

    pub fn resolve(
        &self,
        resolver: &TypedPayloadResolver<'_, '_>,
    ) -> Result<ResolvedFieldPayloadBinding> {
        let selected = resolver
            .field(self.field_id)?
            .ok_or_else(|| super::error("field not found"))?;
        let field = FieldDescriptor {
            name_hash: self.field_name_hash,
            unit_hash: self.legacy_unit_hash,
            type_kind: self.field_type,
            support: self.support,
            representation: self.representation,
            reserved: 0,
            reserved2: 0,
            sample_offset: 0,
        };
        let physical = PhysicalFieldSemantics::from_existing(
            &field,
            &selected.record,
            self.modality()?,
            SemanticId::from_optional(self.physical_quantity_id.unwrap_or(0)),
        )
        .map_err(|e| super::error(format!("field declaration: {e:?}")))?;
        let mut container_id = [0; 16];
        hex::decode_to_slice(&self.container_id_hex, &mut container_id)?;
        ResolvedFieldPayloadBinding::new(physical, resolver, container_id)
            .map_err(|e| super::error(format!("payload binding: {e:?}")))
    }
}
