use crate::{query_compiler::QueryCompiler, QualiaQuin};

/// A zero-allocation stream processor trait that iterates over lines or chunks
/// of bytes/strings without allocating to the heap.
pub trait ZeroCopyStream<'a> {
    type Item;
    fn stream_parse(&self) -> impl Iterator<Item = Self::Item> + 'a;
}

/// The Zero-Allocation Ingestion Pipeline.
/// Designed for extremely high-throughput ingestion of N-Triples*, N-Quads*,
/// JSON-LD*, and N3Logic direct to hardware Quins.
pub struct IngestionPipeline<'a> {
    payload: &'a str,
}

impl<'a> IngestionPipeline<'a> {
    pub fn new(payload: &'a str) -> Self {
        Self { payload }
    }

    /// Parses a single line natively, determining hardware routing based on the syntax.
    pub fn parse_line(line: &str) -> Option<QualiaQuin> {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None; // Skip empty lines and comments
        }

        let mut metadata: u64 = 0;

        // 1. N3Logic Detection -> Route to Core 1 (Prolog Webizen)
        if trimmed.contains("=>") || trimmed.contains("@forAll") || trimmed.contains("@forSome") {
            // N3Logic Implication / Quantification requires deep inference
            // Routing Tier: 0b10 (Bits 61-62)
            metadata |= 0b10 << 61;
            // Embedded N3Logic identifier payload (example mask)
            metadata |= 0x00A3;
        }
        // 2. RDF-Star Nesting (<< >>) -> Route based on complexity
        else if trimmed.contains("<<") && trimmed.contains(">>") {
            if trimmed.contains("qualia:guardian") || trimmed.contains("SPIN:") {
                // Nested identity logic -> Core 1
                metadata |= 0b10 << 61;
            } else {
                // Standard structural nesting -> Core 2 (SIMD)
                // Routing Tier: 0b00 (Bits 61-62)
                metadata |= 0b00 << 61;
            }
        }
        // 3. Fallback to the QueryCompiler logic gates for other semantics
        else if let Some(quin) = QueryCompiler::compile_to_quin(trimmed) {
            return Some(quin);
        } else {
            // Standard Passthrough
            metadata |= 0b00 << 61;
        }

        Some(QualiaQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata,
            parity: 0,
        })
    }
}

impl<'a> ZeroCopyStream<'a> for IngestionPipeline<'a> {
    type Item = QualiaQuin;

    /// Iterates over the string payload line-by-line, compiling immediately.
    fn stream_parse(&self) -> impl Iterator<Item = Self::Item> + 'a {
        self.payload
            .lines()
            .filter_map(|line| Self::parse_line(line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_n3logic_routing() {
        let n3_rule = "{ ?x a :Man } => { ?x a :Mortal } .";
        let quin = IngestionPipeline::parse_line(n3_rule).unwrap();

        // Expected route: 0b10 (Core 1 Prolog Webizen)
        let expected_routing = 0b10 << 61;
        assert_eq!(
            quin.metadata & (0b11 << 61),
            expected_routing,
            "N3Logic implication failed to route to Core 1"
        );
    }

    #[test]
    fn test_rdf_star_nesting() {
        let nested_triple = "<< :AgentX :prescribed :MedicationY >> :assertedBy :DoctorZ .";
        let quin = IngestionPipeline::parse_line(nested_triple).unwrap();

        // Expected route: 0b00 (Core 2 SIMD matcher) for structural nesting
        let expected_routing = 0b00 << 61;
        assert_eq!(
            quin.metadata & (0b11 << 61),
            expected_routing,
            "RDF-Star nesting failed to route correctly"
        );
    }
}
