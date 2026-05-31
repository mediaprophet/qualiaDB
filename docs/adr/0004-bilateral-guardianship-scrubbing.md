# ADR 0004: Bilateral Guardianship Scrubbing

## Status
Accepted

## Context
Mobile edge devices are uniquely vulnerable to forensic extraction. If the database runtime processes sensitive traits (like Bilateral Micro-Commons medical or guardianship connections), simply deallocating memory is insufficient. The data remains in RAM until overwritten, allowing attackers to scrape private connection histories.

## Decision
We utilize the `zeroize` crate to enforce strict memory sanitization.

1. The `QualiaQuin` struct derives `Zeroize`.
2. The `QualiaSuperBlock`, which buffers thousands of Quins during read operations, explicitly implements a custom `Drop` trait containing a volatile scrub:
```rust
impl Drop for QualiaSuperBlock {
    fn drop(&mut self) {
        unsafe {
            std::ptr::write_volatile(self as *mut _, std::mem::zeroed());
        }
    }
}
```

## Consequences
- **Positive:** We mathematically guarantee that decrypted data slices vanish from physical hardware memory the microsecond they exit the current computational scope.
- **Positive:** Meets ultra-high security requirements for verifiable edge health/identity solutions.
- **Negative:** `Zeroize` actively burns CPU cycles overwriting memory regions that standard garbage collectors would normally ignore. We accept this minor performance cost on teardown in exchange for zero-knowledge privacy.
