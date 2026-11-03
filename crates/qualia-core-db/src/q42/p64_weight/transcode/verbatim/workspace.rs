use super::SafetensorTranscodeValidationError as Error;
use etalon_model::p64::build::{TensorLayout, WorkspaceRequirements};
use etalon_model::p64::P64TensorEntry;

/// Bounds adapter workspace and output, excluding parsed source metadata and the sink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SafetensorTranscodeBudget {
    pub output_bytes: usize,
    pub scratch_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SafetensorTranscodeRequirements {
    pub tensor_count: usize,
    pub manifold_bytes: usize,
    pub scratch_bytes: usize,
}

/// Every supplied capacity is charged; only required prefixes are modified.
pub struct SafetensorTranscodeWorkspace<'w, 'p> {
    pub layouts: &'w mut [TensorLayout<'p>],
    pub entries: &'w mut [P64TensorEntry],
    pub requirements: &'w mut [WorkspaceRequirements],
    pub checksums: &'w mut [u32],
    pub manifolds: &'w mut [u8],
}

pub(super) fn bytes(
    layouts: usize,
    entries: usize,
    requirements: usize,
    checksums: usize,
    manifolds: usize,
) -> Result<usize, Error> {
    let mut total = manifolds;
    for (count, width) in [
        (layouts, core::mem::size_of::<TensorLayout<'_>>()),
        (entries, core::mem::size_of::<P64TensorEntry>()),
        (requirements, core::mem::size_of::<WorkspaceRequirements>()),
        (checksums, core::mem::size_of::<u32>()),
    ] {
        total = total
            .checked_add(count.checked_mul(width).ok_or(Error::WorkspaceOverflow)?)
            .ok_or(Error::WorkspaceOverflow)?;
    }
    Ok(total)
}

impl SafetensorTranscodeWorkspace<'_, '_> {
    pub(super) fn admit(
        &self,
        needed: SafetensorTranscodeRequirements,
        budget: SafetensorTranscodeBudget,
    ) -> Result<(), Error> {
        let required = bytes(
            self.layouts.len(),
            self.entries.len(),
            self.requirements.len(),
            self.checksums.len(),
            self.manifolds.len(),
        )?;
        if required > budget.scratch_bytes {
            return Err(Error::ScratchBudget {
                required,
                available: budget.scratch_bytes,
            });
        }
        for (field, required, available) in [
            ("layouts", needed.tensor_count, self.layouts.len()),
            ("entries", needed.tensor_count, self.entries.len()),
            ("requirements", needed.tensor_count, self.requirements.len()),
            ("checksums", needed.tensor_count, self.checksums.len()),
            ("manifolds", needed.manifold_bytes, self.manifolds.len()),
        ] {
            if available < required {
                return Err(Error::WorkspaceCapacity {
                    field,
                    required,
                    available,
                });
            }
        }
        Ok(())
    }
}
