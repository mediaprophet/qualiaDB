use super::{
    ContentId, ContractError, DomainLabel, EntityDomain, FieldSemantics, NamedMetric, ReasonCode,
    ResourceKind, ResourceRef,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferRule {
    Interpolation,
    ConservativeIntegration,
    ExactLabelCompatibility,
    FrameTransport,
    JacobianTranspose,
    NormalRecompute,
    TangentRecompute,
    NormalizedSkinWeights,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    SourceToTarget,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferContract {
    pub source: ContentId,
    pub target: ContentId,
    pub direction: TransferDirection,
    pub source_dofs: u64,
    pub target_dofs: u64,
    pub construction_method: String,
    pub domain: EntityDomain,
    pub semantics: FieldSemantics,
    pub units: String,
    pub source_frame: ContentId,
    pub target_frame: ContentId,
    pub rule: TransferRule,
    pub support: ResourceRef,
    pub maximum_nonzeros: u64,
    pub quantisation: String,
    pub domain_restrictions: Vec<DomainLabel>,
    pub validation: Vec<ResourceRef>,
    pub errors: Vec<NamedMetric>,
}

pub fn validate_transfer_endpoints(
    contract: &TransferContract,
    source: ContentId,
    target: ContentId,
) -> Result<(), ContractError> {
    if contract.source != source || contract.target != target {
        return Err(ContractError::new(
            ReasonCode::StaleIdentity,
            "transfer_endpoints",
        ));
    }
    Ok(())
}

pub fn validate_transfer_semantics(contract: &TransferContract) -> Result<(), ContractError> {
    let compatible = matches!(
        (contract.semantics, contract.rule),
        (
            FieldSemantics::Intensive | FieldSemantics::Displacement | FieldSemantics::CornerUv,
            TransferRule::Interpolation
        ) | (
            FieldSemantics::Extensive,
            TransferRule::ConservativeIntegration
        ) | (
            FieldSemantics::Categorical,
            TransferRule::ExactLabelCompatibility
        ) | (FieldSemantics::TangentVector, TransferRule::FrameTransport)
            | (FieldSemantics::Force, TransferRule::JacobianTranspose)
            | (FieldSemantics::Normal, TransferRule::NormalRecompute)
            | (FieldSemantics::Tangent, TransferRule::TangentRecompute)
            | (
                FieldSemantics::SkinWeights,
                TransferRule::NormalizedSkinWeights
            )
    );
    if !compatible
        || (contract.support.kind == ResourceKind::IndexRemap
            && contract.rule != TransferRule::ExactLabelCompatibility)
    {
        return Err(ContractError::new(
            ReasonCode::UnsatisfiedConstraint,
            "transfer_semantics",
        ));
    }
    if contract.semantics == FieldSemantics::CornerUv
        && (contract.domain != EntityDomain::Corner
            || ![
                super::LabelKind::Chart,
                super::LabelKind::Material,
                super::LabelKind::Provenance,
            ]
            .iter()
            .all(|kind| {
                contract
                    .domain_restrictions
                    .iter()
                    .any(|d| d.domain == EntityDomain::Corner && d.kind == *kind)
            }))
    {
        return Err(ContractError::new(
            ReasonCode::UnsupportedCapability,
            "corner_chart_domains",
        ));
    }
    if contract.source_dofs == 0 || contract.target_dofs == 0 || contract.maximum_nonzeros == 0 {
        return Err(ContractError::new(
            ReasonCode::InvalidInput,
            "transfer_dimensions",
        ));
    }
    if !matches!(
        contract.support.kind,
        ResourceKind::FemcBindings
            | ResourceKind::SparseMap
            | ResourceKind::IndexRemap
            | ResourceKind::Operator
    ) {
        return Err(ContractError::new(
            ReasonCode::UnsupportedCapability,
            "transfer_support_type",
        ));
    }
    if contract.support.kind == ResourceKind::FemcBindings
        && !matches!(
            contract.semantics,
            FieldSemantics::Intensive | FieldSemantics::Displacement | FieldSemantics::Force
        )
    {
        return Err(ContractError::new(
            ReasonCode::UnsupportedCapability,
            "femc_field_semantics",
        ));
    }
    Ok(())
}
