//! Machine schema entries for all 8 Etalon VibeScript providers (`etalon-vibe`, 30 functions).
//!
//! Dynamically projects `etalon_vibe::ETALON_VIBE_PROVIDERS` descriptors into `SchemaEntry`
//! records so `all_schemas()` and `schema_for()` never drift from the canonical ABI contracts.

use super::{EffectClass, HonestyLabel, SchemaArg, SchemaEntry};
use etalon_vibe::ETALON_VIBE_PROVIDERS;
use vibescript_abi::{EffectSet, ParamSpec, PrimitiveKind};

fn primitive_type_name(kind: PrimitiveKind) -> &'static str {
    match kind {
        PrimitiveKind::F32 => "f32",
        PrimitiveKind::F64 => "f64",
        PrimitiveKind::I32 => "i32",
        PrimitiveKind::I64 => "i64",
        PrimitiveKind::U32 => "u32",
        PrimitiveKind::U64 => "u64",
        PrimitiveKind::U8 => "u8",
        PrimitiveKind::Bool => "bool",
        PrimitiveKind::Utf8Bytes => "bytes",
        PrimitiveKind::NQuin48 => "NQuin48",
        PrimitiveKind::ResourceHandle => "ResourceHandle",
    }
}

fn format_param_type(param: &ParamSpec) -> String {
    let base = primitive_type_name(param.kind);
    if param.min_elems == 1 && param.max_elems == 1 {
        base.to_string()
    } else if param.kind == PrimitiveKind::U8 || param.kind == PrimitiveKind::Utf8Bytes {
        "bytes".to_string()
    } else {
        format!("List<{base}>")
    }
}

fn format_return_type(kind: PrimitiveKind, required_output_elems: u32) -> String {
    let base = primitive_type_name(kind);
    if required_output_elems <= 1 {
        base.to_string()
    } else if kind == PrimitiveKind::U8 || kind == PrimitiveKind::Utf8Bytes {
        "bytes".to_string()
    } else {
        format!("List<{base}>")
    }
}

fn map_effect_class(effects: EffectSet) -> EffectClass {
    if effects.contains_all(EffectSet::GRAPH_MUTATE) {
        EffectClass::Write
    } else if effects.contains_all(EffectSet::GRAPH_READ) {
        EffectClass::Read
    } else if effects.contains_all(EffectSet::HARDWARE_IO)
        || effects.0 != EffectSet::PURE_COMPUTE.0
    {
        EffectClass::External
    } else {
        EffectClass::Pure
    }
}

/// Generates `SchemaEntry` records for all functions across `ETALON_VIBE_PROVIDERS`.
pub fn entries() -> Vec<SchemaEntry> {
    let mut out = Vec::with_capacity(30);
    for provider in ETALON_VIBE_PROVIDERS {
        let d = provider.descriptor();
        for sub in d.sub_namespaces {
            for f in sub.functions {
                let id = format!("{}.{}.{}", d.short_alias, sub.name, f.name);
                let effect = map_effect_class(f.effects);
                let returns = format_return_type(f.output_kind, f.required_output_elems);
                let mut entry =
                    SchemaEntry::new(&id, effect, HonestyLabel::CapabilityLease, &returns)
                        .with_description(&format!(
                            "Etalon VibeScript extension `{}` ({})",
                            id, f.canonical_iri.iri
                        ));
                for p in f.params {
                    entry = entry.with_arg(SchemaArg::new(p.name, &format_param_type(p), true));
                }
                out.push(entry);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability_schema::{ all_schemas, schema_for };

    #[test]
    fn t51_etalon_vibe_entries_all_30_functions_present() {
        let etalon_entries = entries();
        assert_eq!(etalon_entries.len(), 30);
        let all = all_schemas();
        assert!(all.len() >= 30);
        for id in [
            "gpu.compute.affine_f32",
            "gpu.compute.gemv4_f32",
            "gpu.compute.topk2_f32",
            "gpu.compute.rmsnorm_f32",
            "gpu.compute.rope_f32",
            "mech.kinematics.verlet_step",
            "mech.kinematics.kinetic_energy",
            "mech.kinematics.spring_damper_force",
            "mech.geometry.aabb_volume",
            "mech.geometry.excavation_prism_volume",
            "forge.shader.tile_schedule_gemm",
            "forge.shader.iq_power_reference",
            "forge.shader.coop_matrix_tile_ops",
            "render.spatial.project_10d_to_3d",
            "render.spatial.sdf_torus_eval",
            "render.spatial.skeletal_lerp_joint",
            "render.spatial.brep_euler_characteristic",
            "signal.dsp.signal_rms_peak",
            "signal.dsp.cfar_cell_average",
            "signal.dsp.biquad_df1_step",
            "signal.vision.region_mean_u8",
            "sense.telemetry.clock_stamp_drift_ns",
            "sense.telemetry.spectrum_bin_hz",
            "sense.telemetry.rssi_log_distance_m",
            "p64.runtime.p64_align_offset",
            "p64.runtime.moe_top2_softmax",
            "p64.runtime.residency_admit_check",
            "nquin.wire.compute_wire_parity",
            "nquin.wire.verify_wire_words",
            "nquin.wire.radix_bucket_byte",
        ] {
            let entry = schema_for(id).unwrap_or_else(|| panic!("missing schema for {id}"));
            assert_eq!(entry.honesty, HonestyLabel::CapabilityLease);
            assert!(!entry.args.is_empty());
        }
    }
}

