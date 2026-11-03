//! Optional structural index checks; registration and namespace trust belong to the host.
use super::{error, fail, Result};
use qualia_core_db::{
    container_10d::TypedPayloadResolver,
    sensing::{
        container::SensingIndexView,
        record::{MeasurementRecord, ResolvedFieldPayloadBinding},
    },
};

#[derive(Clone, Copy)]
pub(super) struct Selection {
    field_id: u64,
    row: usize,
    kind: u16,
}

impl Selection {
    pub fn optional(
        field_id: Option<u64>,
        row: Option<usize>,
        kind: Option<u16>,
    ) -> Result<Option<Self>> {
        match (field_id, row, kind) {
            (None, None, None) => Ok(None),
            (Some(field_id), Some(row), Some(kind)) if field_id != 0 && kind != 0 =>
                Ok(Some(Self { field_id, row, kind })),
            _ => fail("index field id, row and kind must be supplied together; ids and kind must be nonzero"),
        }
    }

    pub fn require_registered(self, supported: &[u16]) -> Result<()> {
        if !supported.contains(&self.kind) {
            return fail("selected index kind is absent from the supplied extension allow-list");
        }
        Ok(())
    }

    pub fn verify(
        self,
        resolver: &TypedPayloadResolver<'_, '_>,
        binding: ResolvedFieldPayloadBinding,
        sidecar: &MeasurementRecord,
    ) -> Result<()> {
        let view = SensingIndexView::open(
            resolver,
            self.field_id,
            self.kind,
            binding.raw_payload().container_id(),
        )
        .map_err(|e| error(format!("sensing index: {e:?}")))?;
        let record = view
            .record(self.row, binding.physical())
            .map_err(|e| error(format!("sensing index row: {e:?}")))?;
        if record != *sidecar {
            return fail("selected index record differs from the canonical sidecar record");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
