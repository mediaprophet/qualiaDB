//! Prepared operator runtime module (W3: EOS-031).
//!
//! Provides the prepared operator adapter bridging companion packages into inference runs.

pub mod adapter;
pub mod cpu_lookup;
pub mod gpu_schedule;
pub mod moe_operator;

pub use adapter::{PreparedOperator, PreparedOperatorError};
pub use cpu_lookup::{execute_cpu_q4k_lookup, precompute_activation_luts, CpuScheduleConfig};
pub use gpu_schedule::{GpuOccupancyDiagnostics, GpuOperatorSchedule, GpuScheduleKind};
pub use moe_operator::{
    dispatch_clustered_moe_step, ClusterAnchor, ClusteredMoeScratch, ClusteredMoEOperator,
    IndependentExpertWeights, LowRankDelta, RoutedExpert,
};

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_inference_kernel::operators::{
        AccumKind, OperatorDescriptor, OperatorKind, ScaleLayout,
    };
    use crate::inference::operator_package::{
        FidelityContract, OperatorPackage, OperatorRecord, PackageBuilder, SegmentKind,
    };

    fn make_dense_package() -> OperatorPackage {
        let mut builder = PackageBuilder::new(
            0x1111,
            0x2222,
            FidelityContract::SourceBytePreserving,
        );
        // 2x3 matrix: [[1, 2, 3], [4, 5, 6]]
        let weights: [f32; 6] = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let mut wbytes = Vec::new();
        for &w in &weights {
            wbytes.extend_from_slice(&w.to_le_bytes());
        }

        builder.add_segment_payload(1, SegmentKind::BitPlaneTiles, wbytes).unwrap();
        builder.add_operator(OperatorRecord {
            tensor_role: 1,
            name: "test.dense".to_string(),
            descriptor: OperatorDescriptor {
                kind: OperatorKind::DenseF32,
                in_features: 3,
                out_features: 2,
                batch_hint: 1,
                tile_elems: 3,
                scale_layout: ScaleLayout::None,
                accum: AccumKind::F32,
                max_workspace_bytes: 0,
                representation_digest: 0x2222,
            },
            source_segment_id: 1,
            primary_segment_id: 1,
            scale_segment_id: 0,
        }).unwrap();

        let bytes = builder.build().unwrap();
        OperatorPackage::from_bytes(bytes).unwrap()
    }

    #[test]
    fn prepared_operator_lifecycle_and_stale_generation_fail_closed() {
        let pkg = make_dense_package();
        let generation = 42u64;
        let prep = PreparedOperator::prepare(&pkg, "test.dense", generation)
            .expect("prepare must succeed");

        assert_eq!(prep.name, "test.dense");
        assert_eq!(prep.residency_generation, 42);

        let input = [1.0f32, 2.0, 3.0];
        let mut output = [0.0f32; 2];
        let mut ws = [];

        // Correct generation succeeds
        prep.execute(&input, &mut output, &mut ws, 42).expect("execute succeeds");
        // [1*1 + 2*2 + 3*3 = 14, 4*1 + 5*2 + 6*3 = 32]
        assert_eq!(output, [14.0, 32.0]);

        // Stale generation fails closed immediately
        let err = prep.execute(&input, &mut output, &mut ws, 43).err();
        assert_eq!(
            err,
            Some(PreparedOperatorError::StaleResidencyGeneration {
                expected: 42,
                actual: 43,
            })
        );
    }

    #[test]
    fn digest_mismatch_rejected_on_prepare() {
        let mut builder = PackageBuilder::new(
            0x1111,
            0x2222, // package manifest digest
            FidelityContract::OperatorFidelity,
        );
        let weights = [1.0f32, 2.0, 3.0];
        let mut wbytes = Vec::new();
        for &w in &weights {
            wbytes.extend_from_slice(&w.to_le_bytes());
        }
        builder.add_segment_payload(1, SegmentKind::BitPlaneTiles, wbytes).unwrap();
        builder.add_operator(OperatorRecord {
            tensor_role: 1,
            name: "test.tampered".to_string(),
            descriptor: OperatorDescriptor {
                kind: OperatorKind::DenseF32,
                in_features: 3,
                out_features: 1,
                batch_hint: 1,
                tile_elems: 3,
                scale_layout: ScaleLayout::None,
                accum: AccumKind::F32,
                max_workspace_bytes: 0,
                representation_digest: 0x9999, // tampered digest
            },
            source_segment_id: 1,
            primary_segment_id: 1,
            scale_segment_id: 0,
        }).unwrap();

        let pkg = OperatorPackage::from_bytes(builder.build().unwrap()).unwrap();
        let res = PreparedOperator::prepare(&pkg, "test.tampered", 1);
        assert!(matches!(res, Err(PreparedOperatorError::DigestMismatch { .. })));
    }
}
