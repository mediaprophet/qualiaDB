use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubgroupReduceOp {
    Add,
    Mul,
    Min,
    Max,
    And,
    Or,
    Xor,
}

/// Hardware family an [`Intrinsic`] maps onto. Used by the capability checker to
/// decide whether a schedule that relies on the intrinsic can run natively, be
/// lowered to a portable equivalent, or must be excluded on the local adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntrinsicClass {
    /// Warp/subgroup operations (shuffle, ballot, reductions).
    Subgroup,
    /// Cooperative-matrix / Tensor-core matrix-multiply-accumulate.
    CooperativeMatrix,
    /// Ray-query intersection tests (RT cores).
    RayTracing,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intrinsic {
    SubgroupReduce { op: SubgroupReduceOp },
    SubgroupShuffle { delta: u32 },
    SubgroupBallot,
    CoopMatMul { m: u32, n: u32, k: u32 },
    /// Hardware ray-query intersection against a bound acceleration structure.
    /// `destination` receives the committed-hit distance (or a miss sentinel).
    RayQuery {
        acceleration_structure: String,
        origin: String,
        direction: String,
        t_min: String,
        t_max: String,
        destination: String,
    },
}

impl Intrinsic {
    pub const fn class(&self) -> IntrinsicClass {
        match self {
            Self::SubgroupReduce { .. } | Self::SubgroupShuffle { .. } | Self::SubgroupBallot => {
                IntrinsicClass::Subgroup
            }
            Self::CoopMatMul { .. } => IntrinsicClass::CooperativeMatrix,
            Self::RayQuery { .. } => IntrinsicClass::RayTracing,
        }
    }
}
