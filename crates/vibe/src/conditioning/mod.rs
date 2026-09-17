//! Vibe-facing conditioning profile DTO and graph projection (P1B).
//! No invented Host invoke IDs. Neutral statements only.

mod profile;
mod project;
#[cfg(test)]
mod tests;

pub use profile::{
    ConditioningBudgetDto, ConditioningProfileDto, RequirementClassDto, RequirementDto,
    SCHEMA_VERSION,
};
pub use project::{project_profile, CondStatement};
