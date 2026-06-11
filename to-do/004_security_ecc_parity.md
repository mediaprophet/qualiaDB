# Task 004: Fix verify_ecc_parity Mock in lib.rs

## Problem
`lib.rs` contains `verify_ecc_parity` which is a "Mock ECC parity check" that returns true unless parity == u64::MAX. The "ECC Parity and Checksum" vector advertised in the Quin layout doesn't actually protect anything.

**File**: `crates/qualia-core-db/src/lib.rs`  
**Severity**: 🔴 HIGH

## Current State
```rust
// Mock ECC parity check
pub fn verify_ecc_parity(quin: &NQuin) -> bool {
    let parity = quin.parity;
    if parity == u64::MAX {
        return false;
    }
    return true; // Always true otherwise - no actual ECC check
}
```

## Implementation Plan

### Implement Real ECC Parity Check
The parity field should be a XOR fold of all other Quin fields for integrity checking.

1. **Calculate actual parity**:
   ```rust
   pub fn calculate_parity(quin: &NQuin) -> u64 {
       quin.subject ^ quin.predicate ^ quin.object ^ quin.context ^ quin.metadata
   }
   ```

2. **Verify parity**:
   ```rust
   pub fn verify_ecc_parity(quin: &NQuin) -> bool {
       let expected_parity = calculate_parity(quin);
       quin.parity == expected_parity
   }
   ```

3. **Add parity calculation on creation**:
   ```rust
   impl NQuin {
       pub fn new(
           subject: u64,
           predicate: u64,
           object: u64,
           context: u64,
           metadata: u64,
       ) -> Self {
           let quin = NQuin {
               subject,
               predicate,
               object,
               context,
               metadata,
               parity: 0, // Will be calculated
           };
           let parity = calculate_parity(&quin);
           NQuin { parity, ..quin }
       }
   }
   ```

4. **Add Reed-Solomon ECC (Optional but recommended)**:
   For stronger error detection/correction, implement Reed-Solomon:
   ```rust
   use reed_solomon::Encoder;

   pub fn calculate_reed_solomon_ecc(quin: &NQuin) -> u64 {
       let encoder = Encoder::new(2); // 2 parity bytes
       let data = [
           quin.subject.to_le_bytes(),
           quin.predicate.to_le_bytes(),
           quin.object.to_le_bytes(),
           quin.context.to_le_bytes(),
           quin.metadata.to_le_bytes(),
       ].concat();
       let encoded = encoder.encode(&data);
       // Extract parity from encoded data
       u64::from_le_bytes(encoded[encoded.len()-8..].try_into().unwrap())
   }
   ```

## Implementation Steps

1. Implement `calculate_parity()` function
2. Update `verify_ecc_parity()` to use real calculation
3. Add `NQuin::new()` constructor with automatic parity calculation
4. Update all Quin creation sites to use constructor
5. Add Reed-Solomon ECC (optional but recommended for stronger protection)
6. Write comprehensive tests:
   - Valid parity verification
   - Tampered field detection
   - Parity calculation consistency
   - Reed-Solomon error detection (if implemented)

## Success Criteria
- ✅ `verify_ecc_parity()` detects tampered Quins
- ✅ Parity is automatically calculated on creation
- ✅ All Quin creation uses constructor with parity
- ✅ Tests detect single-bit flips
- ✅ Tests detect multi-bit corruption
- ✅ Optional: Reed-Solomon ECC for error correction

## Related Files
- `crates/qualia-core-db/src/lib.rs` (main)
- `crates/qualia-core-db/src/crdt.rs` (Quin usage)
- `crates/qualia-core-db/src/storage.rs` (Quin persistence)
- `crates/qualia-core-db/src/wal.rs` (Quin logging)
- All files creating NQuin instances

## Estimated Complexity
- Basic XOR parity: 0.5-1 day
- With Reed-Solomon ECC: 1-2 days

## Dependencies
- None (can be done independently)
- Reed-Solomon: requires `reed-solomon` crate

## Notes
- This is a data integrity check, not cryptographic authentication
- XOR parity provides detection, not correction
- Reed-Solomon provides both detection and correction
- Consider performance impact on hot paths
- The parity field is part of the 48-byte Quin structure
- This should have minimal performance impact
- Update AGENTS.md to reflect real ECC implementation