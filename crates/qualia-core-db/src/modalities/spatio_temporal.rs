// Epic 21: Spatio-Temporal Logics
// Allen's Interval Algebra & RCC8

pub enum TemporalOp {
    Before,
    Meets,
    Overlaps,
    Starts,
    During,
    Finishes,
    Equals,
}

pub fn evaluate_temporal(op: TemporalOp, t1_start: i64, t1_end: i64, t2_start: i64, t2_end: i64) -> bool {
    match op {
        TemporalOp::Before => t1_end < t2_start,
        TemporalOp::Meets => t1_end == t2_start,
        TemporalOp::Overlaps => t1_start < t2_start && t1_end > t2_start && t1_end < t2_end,
        TemporalOp::Starts => t1_start == t2_start && t1_end < t2_end,
        TemporalOp::During => t1_start > t2_start && t1_end < t2_end,
        TemporalOp::Finishes => t1_end == t2_end && t1_start > t2_start,
        TemporalOp::Equals => t1_start == t2_start && t1_end == t2_end,
    }
}
