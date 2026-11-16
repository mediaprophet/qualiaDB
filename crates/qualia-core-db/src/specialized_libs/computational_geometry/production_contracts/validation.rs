use super::*;

fn invalid(field: &'static str) -> ContractError {
    ContractError::new(ReasonCode::InvalidInput, field)
}
fn text(value: &str, limits: ContractLimits) -> Result<(), ContractError> {
    if value.trim().is_empty() || value.len() > limits.string_bytes {
        Err(invalid("text"))
    } else {
        Ok(())
    }
}
fn list<T>(values: &[T], limits: ContractLimits) -> Result<(), ContractError> {
    if values.len() > limits.items_per_list {
        Err(ContractError::new(
            ReasonCode::CapacityExhausted,
            "list_count",
        ))
    } else {
        Ok(())
    }
}
fn texts(values: &[String], limits: ContractLimits) -> Result<(), ContractError> {
    list(values, limits)?;
    for value in values {
        text(value, limits)?;
    }
    Ok(())
}
fn identity(id: ContentId) -> Result<(), ContractError> {
    if id.digest == [0; 32] {
        Err(invalid("uninitialised_identity"))
    } else {
        Ok(())
    }
}
fn identities(values: &[ContentId], limits: ContractLimits) -> Result<(), ContractError> {
    list(values, limits)?;
    for (i, id) in values.iter().enumerate() {
        identity(*id)?;
        if values[..i].contains(id) {
            return Err(invalid("duplicate_dependency"));
        }
    }
    Ok(())
}
fn resource(value: &ResourceRef) -> Result<(), ContractError> {
    identity(value.id)?;
    let compatible = match value.encoding {
        ResourceEncoding::Existing10d => value.kind == ResourceKind::MeshContainer,
        ResourceEncoding::FemcV1 => value.kind == ResourceKind::FemcBindings,
        ResourceEncoding::ClusterDagV1 => value.kind == ResourceKind::ClusterHierarchy,
        ResourceEncoding::ForgeCertification { schema_version } => {
            schema_version == 2 && value.kind == ResourceKind::ForgeManifest
        }
        ResourceEncoding::EtalonReceiptV1 => value.kind == ResourceKind::EtalonRunReceipt,
        ResourceEncoding::WgslUtf8 => value.kind == ResourceKind::ShaderSource,
        ResourceEncoding::ExternalTyped { schema } => {
            identity(schema)?;
            !matches!(
                value.kind,
                ResourceKind::MeshContainer
                    | ResourceKind::FemcBindings
                    | ResourceKind::ClusterHierarchy
                    | ResourceKind::ForgeManifest
                    | ResourceKind::EtalonRunReceipt
                    | ResourceKind::ShaderSource
            )
        }
    };
    if !compatible {
        return Err(ContractError::new(
            ReasonCode::UnsupportedCapability,
            "resource_encoding",
        ));
    }
    Ok(())
}
fn resources(values: &[ResourceRef], limits: ContractLimits) -> Result<(), ContractError> {
    list(values, limits)?;
    for value in values {
        resource(value)?;
    }
    Ok(())
}
fn domains(values: &[DomainLabel], limits: ContractLimits) -> Result<(), ContractError> {
    list(values, limits)?;
    for (i, value) in values.iter().enumerate() {
        resource(&value.labels)?;
        if value.labels.kind != ResourceKind::DomainLabels
            || values[..i]
                .iter()
                .any(|v| v.domain == value.domain && v.kind == value.kind)
        {
            return Err(invalid("domain_labels"));
        }
    }
    Ok(())
}
fn metrics(values: &[NamedMetric], limits: ContractLimits) -> Result<(), ContractError> {
    list(values, limits)?;
    for (i, value) in values.iter().enumerate() {
        text(&value.name, limits)?;
        if values[..i].iter().any(|v| v.name == value.name) {
            return Err(invalid("duplicate_metric"));
        }
        match &value.value {
            MetricValue::NotMeasured { reason } | MetricValue::NotApplicable { reason } => {
                text(reason, limits)?
            }
            MetricValue::Available {
                value,
                units,
                method,
                ..
            } => {
                if !value.is_finite() {
                    return Err(invalid("nonfinite_metric"));
                }
                text(units, limits)?;
                text(method, limits)?;
            }
        }
    }
    Ok(())
}
fn arithmetic(value: &ArithmeticProfile, limits: ContractLimits) -> Result<(), ContractError> {
    text(&value.scaling, limits)?;
    for magnitude in [value.minimum_magnitude, value.maximum_magnitude]
        .into_iter()
        .flatten()
    {
        if !magnitude.is_finite() || magnitude < 0.0 {
            return Err(invalid("arithmetic_range"));
        }
    }
    if let (Some(min), Some(max)) = (value.minimum_magnitude, value.maximum_magnitude) {
        if min > max {
            return Err(invalid("arithmetic_range"));
        }
    }
    Ok(())
}
fn bounds(value: ResourceBounds) -> Result<(), ContractError> {
    if value.cold_bytes == 0 || value.work_items == 0 || value.output_elements == 0 {
        Err(invalid("resource_bounds"))
    } else {
        Ok(())
    }
}

pub fn validate_document(
    document: &ContractDocument,
    limits: ContractLimits,
) -> Result<(), ContractError> {
    limits.validate()?;
    match &document.record {
        ContractRecord::Asset(value) => {
            resource(&value.content)?;
            identity(value.frame.identity)?;
            identity(value.quality_profile)?;
            if value.content.kind != ResourceKind::MeshContainer
                || !value.frame.length_unit_metres.is_finite()
                || value.frame.length_unit_metres <= 0.0
            {
                return Err(invalid("asset_geometry_frame"));
            }
            list(&value.attributes, limits)?;
            for (i, a) in value.attributes.iter().enumerate() {
                text(&a.name, limits)?;
                text(&a.units, limits)?;
                resource(&a.data)?;
                if a.data.kind != ResourceKind::AttributeData
                    || value.attributes[..i]
                        .iter()
                        .any(|b| b.name == a.name && b.domain == a.domain)
                {
                    return Err(invalid("attribute_schema"));
                }
                if let Some(frame) = a.frame {
                    identity(frame)?;
                }
                if a.semantics == FieldSemantics::CornerUv && a.domain != EntityDomain::Corner {
                    return Err(invalid("uv_domain"));
                }
            }
            domains(&value.domains, limits)?;
            resources(&value.protected_entities, limits)?;
            resources(&value.source_assertions, limits)?;
            texts(&value.admitted_operations, limits)?;
            identities(&value.transfer_dependencies, limits)?;
        }
        ContractRecord::Quality(value) => {
            text(&value.name, limits)?;
            if !value.reference_scale.is_finite() || value.reference_scale <= 0.0 {
                return Err(invalid("reference_scale"));
            }
            if !matches!(
                value.assurance_label.as_str(),
                "A0" | "A1" | "A2" | "A3" | "A4"
            ) || !matches!(
                value.fidelity_label.as_str(),
                "F0" | "F1" | "F2" | "F3" | "F4"
            ) {
                return Err(invalid("quality_labels"));
            }
            list(&value.criteria, limits)?;
            for (i, c) in value.criteria.iter().enumerate() {
                text(&c.name, limits)?;
                text(&c.units, limits)?;
                text(&c.method, limits)?;
                if !c.threshold.is_finite()
                    || c.threshold < 0.0
                    || value.criteria[..i].iter().any(|p| p.name == c.name)
                {
                    return Err(invalid("error_criterion"));
                }
            }
            resources(&value.domains, limits)?;
            arithmetic(&value.arithmetic, limits)?;
            bounds(value.resources)?;
            list(&value.expected_rejections, limits)?;
        }
        ContractRecord::Candidate(value) => {
            identity(value.parent)?;
            identity(value.asset)?;
            identity(value.source_revision)?;
            text(&value.algorithm, limits)?;
            if value.parameters.len() > limits.items_per_list {
                return Err(ContractError::new(
                    ReasonCode::CapacityExhausted,
                    "parameters",
                ));
            }
            for (k, v) in &value.parameters {
                text(k, limits)?;
                text(v, limits)?;
            }
            resources(&value.remaps, limits)?;
            identities(&value.transfers, limits)?;
            texts(&value.unresolved_constraints, limits)?;
            metrics(&value.errors, limits)?;
            identities(&value.dependencies, limits)?;
        }
        ContractRecord::Transfer(value) => {
            identity(value.source)?;
            identity(value.target)?;
            identity(value.source_frame)?;
            identity(value.target_frame)?;
            text(&value.units, limits)?;
            text(&value.quantisation, limits)?;
            text(&value.construction_method, limits)?;
            validate_transfer_semantics(value)?;
            resource(&value.support)?;
            domains(&value.domain_restrictions, limits)?;
            resources(&value.validation, limits)?;
            metrics(&value.errors, limits)?;
        }
        ContractRecord::Kernel(value) => {
            identity(value.semantics)?;
            identity(value.discretisation)?;
            resource(&value.specification)?;
            if value.specification.kind != ResourceKind::KernelSpecification {
                return Err(invalid("kernel_specification"));
            }
            list(&value.shapes, limits)?;
            for (i, s) in value.shapes.iter().enumerate() {
                text(&s.name, limits)?;
                text(&s.units, limits)?;
                list(&s.dimensions, limits)?;
                let size = s.dimensions.iter().try_fold(1u64, |n, d| n.checked_mul(*d));
                if s.dimensions.is_empty()
                    || size.is_none()
                    || value.shapes[..i].iter().any(|p| p.name == s.name)
                {
                    return Err(invalid("kernel_shape"));
                }
            }
            list(&value.effects, limits)?;
            for (i, e) in value.effects.iter().enumerate() {
                if value.effects[..i].iter().any(|p| p.binding == e.binding) {
                    return Err(invalid("duplicate_binding"));
                }
            }
            texts(&value.assumptions, limits)?;
            list(&value.lowerings, limits)?;
            if value.lowerings.is_empty() {
                return Err(invalid("kernel_lowerings"));
            }
            arithmetic(&value.arithmetic, limits)?;
            bounds(value.resources)?;
            resource(&value.reference_evaluator)?;
        }
        ContractRecord::Variant(value) => {
            identity(value.kernel)?;
            resource(&value.abi)?;
            resource(&value.emitted_source)?;
            identity(value.adapter.adapter)?;
            identity(value.toolchain)?;
            resource(&value.adapter.limits)?;
            text(&value.adapter.backend, limits)?;
            text(&value.adapter.driver, limits)?;
            text(&value.adapter.compiler, limits)?;
            texts(&value.adapter.features, limits)?;
            identities(&value.dependencies, limits)?;
            if value.emitted_source.kind != ResourceKind::ShaderSource
                || value.schedule.workgroup.contains(&0)
                || value
                    .schedule
                    .workgroup
                    .iter()
                    .try_fold(1u32, |n, d| n.checked_mul(*d))
                    .is_none()
                || value.schedule.items_per_invocation == 0
                || value.schedule.vector_width == 0
            {
                return Err(invalid("execution_variant"));
            }
            arithmetic(&value.precision, limits)?;
            if let Some(manifest) = &value.forge_manifest {
                resource(manifest)?;
                if manifest.kind != ResourceKind::ForgeManifest {
                    return Err(invalid("forge_manifest_type"));
                }
            }
        }
        ContractRecord::Receipt(value) => {
            identities(&value.targets, limits)?;
            resources(&value.fixtures, limits)?;
            list(&value.seeds, limits)?;
            texts(&value.methods, limits)?;
            metrics(&value.metrics, limits)?;
            metrics(&value.resources, limits)?;
            resources(&value.provenance, limits)?;
            if value.targets.is_empty() {
                return Err(invalid("receipt_targets"));
            }
            if value.status == ResultStatus::Passed {
                if value.reason.is_some() || value.explanation.is_some() {
                    return Err(invalid("passing_reason"));
                }
            } else {
                if value.reason.is_none() {
                    return Err(invalid("failure_reason"));
                }
                text(
                    value
                        .explanation
                        .as_deref()
                        .ok_or(invalid("failure_explanation"))?,
                    limits,
                )?;
            }
            if let Some(receipt) = &value.etalon_receipt {
                resource(receipt)?;
                if receipt.kind != ResourceKind::EtalonRunReceipt {
                    return Err(invalid("etalon_receipt_type"));
                }
            }
        }
    }
    Ok(())
}

/// Verifies declared required thresholds; it does not authenticate observations.
pub fn validate_receipt_against_profile(
    receipt: &ValidationReceipt,
    profile: &QualityProfile,
    target: ContentId,
) -> Result<(), ContractError> {
    let limits = ContractLimits::default();
    identities(&receipt.targets, limits)?;
    metrics(&receipt.metrics, limits)?;
    list(&profile.criteria, limits)?;
    for (i, c) in profile.criteria.iter().enumerate() {
        text(&c.name, limits)?;
        text(&c.units, limits)?;
        text(&c.method, limits)?;
        if !c.threshold.is_finite()
            || c.threshold < 0.0
            || profile.criteria[..i].iter().any(|p| p.name == c.name)
        {
            return Err(invalid("error_criterion"));
        }
    }
    if receipt.reason.is_some() || receipt.explanation.is_some() {
        return Err(invalid("passing_reason"));
    }
    if receipt.status != ResultStatus::Passed || !receipt.targets.contains(&target) {
        return Err(ContractError::new(
            ReasonCode::UnsatisfiedConstraint,
            "receipt_target_or_status",
        ));
    }
    for criterion in profile.criteria.iter().filter(|c| c.required) {
        let metric = receipt
            .metrics
            .iter()
            .find(|m| m.name == criterion.name)
            .ok_or(ContractError::new(
                ReasonCode::InconclusiveVerification,
                "required_metric",
            ))?;
        match &metric.value {
            MetricValue::Available {
                value,
                units,
                class,
                method,
            } if value.is_finite()
                && *value >= 0.0
                && *value <= criterion.threshold
                && *units == criterion.units
                && *class == criterion.class
                && *method == criterion.method => {}
            _ => {
                return Err(ContractError::new(
                    ReasonCode::UnsatisfiedConstraint,
                    "metric_threshold_or_method",
                ))
            }
        }
    }
    Ok(())
}
