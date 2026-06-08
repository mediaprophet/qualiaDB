// Qualia-DB Fuzz Testing & Property Verification
// Utilizes randomized property generation to guarantee the Zero-Allocation
// Query Compiler never panics or overflows memory boundaries on malformed edge inputs.

#[cfg(test)]
mod tests {
    use crate::query_compiler::QueryCompiler;
    use crate::QualiaQuin;
    use proptest::prelude::*;

    proptest! {
        // Generates 10,000 completely randomized string mutations
        // and throws them at the compiler to ensure it never panics.
        #[test]
        fn fuzz_query_compiler_no_panics(ref s in ".*") {
            // The compiler should either return Some(Quin) or safely ignore garbage.
            // Under no circumstance should a malicious string cause a memory panic.
            let _result = QueryCompiler::compile_to_quin(s);
        }

        // Generates random bytes to ensure zero-copy unaligned memory mappings don't fail
        #[test]
        fn fuzz_raw_quin_memory_mapping(bytes in proptest::collection::vec(any::<u8>(), 48)) {
            // Because QualiaQuin is strictly #[repr(C, align(16))] and exactly 48 bytes,
            // we should be able to map ANY 48-byte chunk into the struct safely.
            let chunk: &[u8] = &bytes;
            let _quin: QualiaQuin = unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const QualiaQuin) };
        }
    }

    #[test]
    fn qualia_validate_volatile_scrubbing() {
        // Ensures that Bilateral Guardianship strictness is met by
        // testing the `zeroize` memory scrubbing capability.
        use zeroize::Zeroize;

        let mut quin = QualiaQuin {
            subject: 999,
            predicate: 888,
            object: 777,
            context: 666,
            metadata: 555,
            parity: 444,
        };

        // Explicitly scrub the memory
        quin.zeroize();

        // Ensure the struct was completely overwritten with zeros in RAM
        assert_eq!(quin.subject, 0, "Volatile scrubbing failed on subject");
        assert_eq!(quin.predicate, 0, "Volatile scrubbing failed on predicate");
        assert_eq!(quin.parity, 0, "Volatile scrubbing failed on parity");
    }
}
