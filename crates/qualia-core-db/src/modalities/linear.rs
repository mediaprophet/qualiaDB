use crate::QualiaQuin;

// Marks a Quin as consumed by setting metadata bit 59 (CONSUMED_BIT)
pub const CONSUMED_BIT: u64 = 1u64 << 59;

pub fn consume_quin(q: &mut QualiaQuin) {
    q.metadata |= CONSUMED_BIT;
}

pub fn is_consumed(q: &QualiaQuin) -> bool {
    (q.metadata & CONSUMED_BIT) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_quin() {
        let mut q = QualiaQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        assert!(!is_consumed(&q));
        consume_quin(&mut q);
        assert!(is_consumed(&q));
    }
}
