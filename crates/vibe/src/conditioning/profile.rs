//! Validated conditioning profile DTO (authoring-side).

pub const SCHEMA_VERSION: u16 = 1;
pub const COND_NS: &str = "https://qualiadb.org/schema/conditioning#";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementClassDto {
    Enforced,
    EvidenceObligation,
    Guidance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequirementDto {
    pub id: String,
    pub class: RequirementClassDto,
    pub rule: String,
    pub validator: Option<String>,
    pub required: bool,
    pub priority: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditioningBudgetDto {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub tool_rounds: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditioningProfileDto {
    pub schema_version: u16,
    pub profile_id: String,
    pub objective: String,
    pub domains: Vec<String>,
    pub requirements: Vec<RequirementDto>,
    pub budget: ConditioningBudgetDto,
}

impl ConditioningProfileDto {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(format!("unsupported schema_version {}", self.schema_version));
        }
        if self.profile_id.trim().is_empty() {
            return Err("profile_id must be non-empty".into());
        }
        if self.objective.trim().is_empty() {
            return Err("objective must be non-empty".into());
        }
        if self.requirements.len() > 64 {
            return Err("requirements exceed bound 64".into());
        }
        if self.domains.len() > 32 {
            return Err("domains exceed bound 32".into());
        }
        let mut seen = Vec::new();
        for r in &self.requirements {
            if r.id.trim().is_empty() {
                return Err("requirement id empty".into());
            }
            if seen.contains(&r.id) {
                return Err(format!("duplicate requirement id {}", r.id));
            }
            seen.push(r.id.clone());
            if matches!(r.class, RequirementClassDto::Enforced) && r.validator.is_none() {
                return Err(format!("enforced requirement {} needs validator", r.id));
            }
        }
        Ok(())
    }

    /// FNV-1a content digest over canonical fields (stable identity).
    pub fn content_digest(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in self.profile_id.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        for b in self.objective.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        for r in &self.requirements {
            for b in r.id.as_bytes() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h ^= r.priority as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }
}
