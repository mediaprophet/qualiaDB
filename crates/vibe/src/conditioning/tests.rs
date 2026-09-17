use super::*;

fn sample_profile() -> ConditioningProfileDto {
    ConditioningProfileDto {
        schema_version: SCHEMA_VERSION,
        profile_id: "urn:qualia:profile:rust-review:v1".into(),
        objective: "Review Rust for bounded resources".into(),
        domains: vec!["urn:qualia:domain:rust".into()],
        requirements: vec![RequirementDto {
            id: "R1".into(),
            class: RequirementClassDto::Enforced,
            rule: "No Tier-1 heap".into(),
            validator: Some("zero-alloc-suite".into()),
            required: true,
            priority: 255,
        }],
        budget: ConditioningBudgetDto {
            input_tokens: 4096,
            output_tokens: 1024,
            tool_rounds: 4,
        },
    }
}

#[test]
fn valid_profile_projects_and_digests_stably() {
    let p = sample_profile();
    let d1 = p.content_digest();
    let d2 = p.content_digest();
    assert_eq!(d1, d2);
    let g = project_profile(&p).unwrap();
    assert!(g.iter().any(|s| s.predicate.contains("contentDigest")));
    assert!(g.iter().any(|s| s.object.contains("EnforcedRequirement")));
}

#[test]
fn invalid_enforced_without_validator() {
    let mut p = sample_profile();
    p.requirements[0].validator = None;
    assert!(p.validate().is_err());
}

#[test]
fn no_invented_conditioning_invoke_ids_in_dto() {
    let p = sample_profile();
    let g = project_profile(&p).unwrap();
    for s in g {
        assert!(!s.object.contains("Conditioning.compile"));
        assert!(!s.object.contains("Conditioning.validate"));
    }
}
