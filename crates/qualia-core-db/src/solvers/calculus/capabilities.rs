//! Machine-readable capability truth for the native calculus surface.
//!
//! Descriptors are static POD-like metadata: discovery allocates nothing and
//! never infers mathematical guarantees from a function name.

use crate::q_hash;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarType {
    F32 = 1,
    F64 = 2,
    Complex64 = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccuracyModel {
    ExactAlgebraic = 1,
    FixedOrder = 2,
    EmbeddedEstimate = 3,
    ResidualControlled = 4,
    DiagnosticOnly = 5,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepControl {
    NotApplicable = 0,
    Fixed = 1,
    Adaptive = 2,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationClass {
    ZeroHeap = 0,
    CallerWorkspace = 1,
    ColdHeap = 2,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Maturity {
    Foundation = 0,
    Verified = 1,
    Certified = 2,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendSet(pub u8);

impl BackendSet {
    pub const SCALAR_CPU: u8 = 1 << 0;
    pub const SIMD_CPU: u8 = 1 << 1;
    pub const GPU: u8 = 1 << 2;
    pub const WASM: u8 = 1 << 3;

    pub const fn new(bits: u8) -> Self {
        Self(bits)
    }

    pub const fn contains(self, backend: u8) -> bool {
        self.0 & backend == backend
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalculusCapability {
    pub operation_id: u64,
    pub name: &'static str,
    pub minimum_dimension: u16,
    /// Zero denotes runtime/caller-bounded dimension.
    pub maximum_dimension: u16,
    pub scalar: ScalarType,
    pub accuracy: AccuracyModel,
    pub formal_order: u8,
    pub step_control: StepControl,
    pub allocation: AllocationClass,
    pub backends: BackendSet,
    pub maturity: Maturity,
}

const CPU: BackendSet = BackendSet::new(BackendSet::SCALAR_CPU | BackendSet::WASM);
const SIMD_CPU: BackendSet =
    BackendSet::new(BackendSet::SCALAR_CPU | BackendSet::SIMD_CPU | BackendSet::WASM);
const PORTABLE_GPU: BackendSet =
    BackendSet::new(BackendSet::SCALAR_CPU | BackendSet::GPU | BackendSet::WASM);

pub const CALCULUS_CAPABILITIES: &[CalculusCapability] = &[
    CalculusCapability {
        operation_id: q_hash("q42:calculus:simpson-1-3"),
        name: "composite Simpson 1/3 quadrature",
        minimum_dimension: 1,
        maximum_dimension: 1,
        scalar: ScalarType::F64,
        accuracy: AccuracyModel::FixedOrder,
        formal_order: 4,
        step_control: StepControl::Fixed,
        allocation: AllocationClass::ZeroHeap,
        backends: SIMD_CPU,
        maturity: Maturity::Verified,
    },
    CalculusCapability {
        operation_id: q_hash("q42:calculus:trapezoid"),
        name: "composite trapezoidal quadrature",
        minimum_dimension: 1,
        maximum_dimension: 1,
        scalar: ScalarType::F64,
        accuracy: AccuracyModel::FixedOrder,
        formal_order: 2,
        step_control: StepControl::Fixed,
        allocation: AllocationClass::ZeroHeap,
        backends: CPU,
        maturity: Maturity::Verified,
    },
    CalculusCapability {
        operation_id: q_hash("q42:calculus:gpu-simpson-1-3-f32"),
        name: "portable GPU Simpson 1/3 quadrature",
        minimum_dimension: 1,
        maximum_dimension: 1,
        scalar: ScalarType::F32,
        accuracy: AccuracyModel::FixedOrder,
        formal_order: 4,
        step_control: StepControl::Fixed,
        allocation: AllocationClass::ColdHeap,
        backends: PORTABLE_GPU,
        maturity: Maturity::Foundation,
    },
    CalculusCapability {
        operation_id: q_hash("q42:calculus:rk4-static"),
        name: "fixed-state classical Runge-Kutta",
        minimum_dimension: 4,
        maximum_dimension: 4,
        scalar: ScalarType::F64,
        accuracy: AccuracyModel::FixedOrder,
        formal_order: 4,
        step_control: StepControl::Fixed,
        allocation: AllocationClass::ZeroHeap,
        backends: CPU,
        maturity: Maturity::Verified,
    },
    CalculusCapability {
        operation_id: q_hash("q42:calculus:rk4-dense"),
        name: "dynamic classical Runge-Kutta",
        minimum_dimension: 1,
        maximum_dimension: 0,
        scalar: ScalarType::F64,
        accuracy: AccuracyModel::FixedOrder,
        formal_order: 4,
        step_control: StepControl::Fixed,
        allocation: AllocationClass::ColdHeap,
        backends: CPU,
        maturity: Maturity::Verified,
    },
    CalculusCapability {
        operation_id: q_hash("q42:calculus:shooting-newton-static"),
        name: "fixed-state Newton shooting BVP",
        minimum_dimension: 4,
        maximum_dimension: 4,
        scalar: ScalarType::F64,
        accuracy: AccuracyModel::ResidualControlled,
        formal_order: 4,
        step_control: StepControl::Fixed,
        allocation: AllocationClass::ZeroHeap,
        backends: CPU,
        maturity: Maturity::Verified,
    },
    CalculusCapability {
        operation_id: q_hash("q42:calculus:bdf1"),
        name: "scalar backward Euler",
        minimum_dimension: 1,
        maximum_dimension: 1,
        scalar: ScalarType::F64,
        accuracy: AccuracyModel::ResidualControlled,
        formal_order: 1,
        step_control: StepControl::Fixed,
        allocation: AllocationClass::ZeroHeap,
        backends: CPU,
        maturity: Maturity::Foundation,
    },
    CalculusCapability {
        operation_id: q_hash("q42:calculus:bdf2"),
        name: "scalar BDF2",
        minimum_dimension: 1,
        maximum_dimension: 1,
        scalar: ScalarType::F64,
        accuracy: AccuracyModel::ResidualControlled,
        formal_order: 2,
        step_control: StepControl::Fixed,
        allocation: AllocationClass::ZeroHeap,
        backends: CPU,
        maturity: Maturity::Foundation,
    },
    CalculusCapability {
        operation_id: q_hash("q42:calculus:symplectic-separable"),
        name: "scalar separable symplectic integration",
        minimum_dimension: 2,
        maximum_dimension: 2,
        scalar: ScalarType::F64,
        accuracy: AccuracyModel::FixedOrder,
        formal_order: 4,
        step_control: StepControl::Fixed,
        allocation: AllocationClass::ColdHeap,
        backends: CPU,
        maturity: Maturity::Verified,
    },
    CalculusCapability {
        operation_id: q_hash("q42:calculus:forward-sensitivity-scalar"),
        name: "scalar forward sensitivity",
        minimum_dimension: 1,
        maximum_dimension: 1,
        scalar: ScalarType::F64,
        accuracy: AccuracyModel::FixedOrder,
        formal_order: 4,
        step_control: StepControl::Fixed,
        allocation: AllocationClass::ColdHeap,
        backends: CPU,
        maturity: Maturity::Verified,
    },
    CalculusCapability {
        operation_id: q_hash("q42:calculus:symbolic-multivariable"),
        name: "symbolic gradient Jacobian and Hessian",
        minimum_dimension: 1,
        maximum_dimension: 0,
        scalar: ScalarType::F64,
        accuracy: AccuracyModel::ExactAlgebraic,
        formal_order: 0,
        step_control: StepControl::NotApplicable,
        allocation: AllocationClass::ColdHeap,
        backends: CPU,
        maturity: Maturity::Verified,
    },
];

pub fn capability(operation_id: u64) -> Option<&'static CalculusCapability> {
    CALCULUS_CAPABILITIES
        .iter()
        .find(|entry| entry.operation_id == operation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_ids_are_unique_and_backend_claims_are_nonempty() {
        for (index, capability) in CALCULUS_CAPABILITIES.iter().enumerate() {
            assert_ne!(capability.operation_id, 0);
            assert_ne!(capability.backends.0, 0);
            for other in &CALCULUS_CAPABILITIES[index + 1..] {
                assert_ne!(capability.operation_id, other.operation_id);
            }
        }
    }

    #[test]
    fn capability_lookup_is_allocation_free_static_discovery() {
        let id = q_hash("q42:calculus:simpson-1-3");
        let descriptor = capability(id).unwrap();
        assert_eq!(descriptor.formal_order, 4);
        assert_eq!(descriptor.maturity, Maturity::Verified);
        assert!(descriptor.backends.contains(BackendSet::SIMD_CPU));
    }
}
