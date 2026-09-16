//! Profile DTO → Conditioning semantic statements.

use super::profile::{ConditioningProfileDto, RequirementClassDto, COND_NS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CondStatement {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

fn term(local: &str) -> String {
    format!("{COND_NS}{local}")
}

pub fn project_profile(profile: &ConditioningProfileDto) -> Result<Vec<CondStatement>, String> {
    profile.validate()?;
    let subj = profile.profile_id.clone();
    let mut out = Vec::new();
    out.push(CondStatement {
        subject: subj.clone(),
        predicate: term("type"),
        object: term("ConditioningProfile"),
    });
    out.push(CondStatement {
        subject: subj.clone(),
        predicate: term("schemaVersion"),
        object: profile.schema_version.to_string(),
    });
    out.push(CondStatement {
        subject: subj.clone(),
        predicate: term("profileId"),
        object: profile.profile_id.clone(),
    });
    out.push(CondStatement {
        subject: subj.clone(),
        predicate: term("contentDigest"),
        object: format!("{:016x}", profile.content_digest()),
    });
    out.push(CondStatement {
        subject: subj.clone(),
        predicate: term("hasObjective"),
        object: profile.objective.clone(),
    });
    for d in &profile.domains {
        out.push(CondStatement {
            subject: subj.clone(),
            predicate: term("hasDomainScope"),
            object: d.clone(),
        });
    }
    for r in &profile.requirements {
        let rid = format!("{}#req-{}", subj, r.id);
        out.push(CondStatement {
            subject: subj.clone(),
            predicate: term("hasRequirement"),
            object: rid.clone(),
        });
        let class = match r.class {
            RequirementClassDto::Enforced => "EnforcedRequirement",
            RequirementClassDto::EvidenceObligation => "EvidenceObligation",
            RequirementClassDto::Guidance => "Guidance",
        };
        out.push(CondStatement {
            subject: rid.clone(),
            predicate: term("type"),
            object: term(class),
        });
        out.push(CondStatement {
            subject: rid.clone(),
            predicate: term("priority"),
            object: r.priority.to_string(),
        });
        out.push(CondStatement {
            subject: rid.clone(),
            predicate: term("isRequired"),
            object: if r.required { "true" } else { "false" }.into(),
        });
        if let Some(v) = &r.validator {
            out.push(CondStatement {
                subject: rid,
                predicate: term("validatorRef"),
                object: v.clone(),
            });
        }
    }
    out.push(CondStatement {
        subject: subj,
        predicate: term("hasBudget"),
        object: format!(
            "in:{} out:{} tools:{}",
            profile.budget.input_tokens, profile.budget.output_tokens, profile.budget.tool_rounds
        ),
    });
    Ok(out)
}
