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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intrinsic {
    SubgroupReduce { op: SubgroupReduceOp },
    SubgroupShuffle { delta: u32 },
    SubgroupBallot,
    CoopMatMul { m: u32, n: u32, k: u32 },
}
