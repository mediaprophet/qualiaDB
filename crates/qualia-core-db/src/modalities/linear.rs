// Epic 21: Linear Logic
// Resource semantics and fact consumption

pub fn consume_resource(fact_id: &str) -> bool {
    // In a real implementation, this instantly zeroes out or tombstones the fact
    // from the active VM frame so it cannot be double-spent.
    println!("Consumed resource fact: {}", fact_id);
    true
}
