# Task 007: Update Documentation to Reflect Reality

## Problem
Documentation significantly overstates capabilities. README and other docs claim features that don't exist or are stubbed.

**Files**: `README.md`, `BUILD_ISSUES.md`, and other documentation  
**Severity**: 🟡 MEDIUM

## Current Inaccuracies

### README.md
1. **"271/271 tests passing"**
   - **Reality**: Had 44 build errors (now fixed to 0)
   - **Actual test count**: 539 test functions in core-db alone
   - **Status**: STALE

2. **"microsecond memory-mapped queries"**
   - **Reality**: mmap_query_subject returns empty, does nothing
   - **Status**: INCORRECT (until Task 005 complete)

3. **"real autoregressive decode" (Linux)**
   - **Reality**: Uses mock_pipeline with placeholder shader
   - **Status**: INCORRECT (until Task 006 complete)

4. **"cryptographically auditable"**
   - **Reality**: zk_proofs verify_proof returns true unconditionally
   - **Status**: INCORRECT (until Task 001 complete)

5. **"post-quantum ML-DSA"**
   - **Reality**: Hand-rolled SHA3 construction, not FIPS 204
   - **Status**: INCORRECT (until Task 003 complete)

6. **"tamper-proof WAL"**
   - **Reality**: ECC parity check is mock
   - **Status**: INCORRECT (until Task 004 complete)

### BUILD_ISSUES.md
- **"44 remaining build errors"**
  - **Reality**: All 82 errors fixed, now 0 errors
  - **Status**: OUTDATED

## Implementation Plan

### Phase 1: Update README.md

1. **Update test count**:
   ```markdown
   ## Testing Status
   - Total test functions: 539
   - Build status: ✅ Compiling (0 errors, as of 2026-06-10)
   - Test execution: Run `cargo test --workspace`
   ```

2. **Update query capabilities**:
   ```markdown
   ## Query Layer
   - Status: Under development
   - Current: Single-pattern matching via mini_parser
   - Planned: Full indexing and mmap queries (Task 005)
   ```

3. **Update LLM/GPU claims**:
   ```markdown
   ## LLM Inference
   - DirectML (Windows): ✅ Real dequantization + GEMM
   - Accelerate/Metal (macOS): ✅ Real dequantization + GEMM
   - wgpu/Vulkan (Linux): ⚠️ Mock pipeline (fix in progress - Task 006)
   - Status: Phase 8 bifurcated compute implemented
   ```

4. **Update security claims**:
   ```markdown
   ## Security Features
   - ZK proofs: ⚠️ Placeholder implementation (Task 001)
   - ML-DSA: ⚠️ Hand-rolled version, not FIPS 204 (Task 003)
   - ECC parity: ⚠️ Mock implementation (Task 004)
   - Ed25519 signatures: ✅ Implemented
   - Status: Security audit in progress
   ```

5. **Add development status section**:
   ```markdown
   ## Development Status
   - Version: 0.0.10-dev (early prototype/research sketch)
   - Build: ✅ Compiles (0 errors)
   - Core functionality: Partially implemented
   - Security features: Stubs requiring implementation
   - Query layer: Requires real implementation
   - GPU inference: Partially functional
   ```

### Phase 2: Update BUILD_ISSUES.md

1. **Mark as resolved**:
   ```markdown
   # Build Issues
   ## Status: RESOLVED ✅
   As of 2026-06-10, all 82 build errors have been fixed.
   Current build status: 0 errors, clean compilation.
   ```

2. **Add historical context**:
   ```markdown
   ## Historical Issues (Resolved)
   - 2026-06-09: 44 build errors in core-db
   - 2026-06-10: All errors fixed, 0 remaining
   ```

### Phase 3: Create ROADMAP.md

Create a new roadmap document:

```markdown
# QualiaDB Roadmap

## Version 0.1.0 (Target: Q3 2026)
- [ ] Fix all security stubs (Tasks 001-004)
- [ ] Implement real query layer (Task 005)
- [ ] Fix mock pipeline (Task 006)
- [ ] Update all documentation

## Version 0.2.0 (Target: Q4 2026)
- [ ] Add real indexing (B-tree/LSM-tree)
- [ ] Implement full SPARQL support
- [ ] Add query optimizer
- [ ] Performance benchmarks

## Version 1.0.0 (Target: 2027)
- [ ] Production-ready security
- [ ] Full query capabilities
- [ ] Comprehensive testing
- [ ] Security audit
```

### Phase 4: Update ARCHITECTURE.md

1. Update LLM section to reflect mock pipeline issue
2. Update security section to note stub implementations
3. Add known limitations section

## Implementation Steps

1. Update README.md test count
2. Update README.md query capabilities
3. Update README.md LLM/GPU claims
4. Update README.md security claims
5. Add development status section to README.md
6. Update BUILD_ISSUES.md to show resolved status
7. Create ROADMAP.md
8. Update ARCHITECTURE.md with known limitations
9. Add TODO references to task files
10. Review all documentation for accuracy

## Success Criteria
- ✅ README reflects current capabilities
- ✅ No overstated claims
- ✅ Test count is accurate
- ✅ Build status is current
- ✅ Security section accurate
- ✅ LLM section accurate
- ✅ ROADMAP.md created
- ✅ BUILD_ISSUES.md updated
- ✅ All documentation reviewed

## Related Files
- `README.md` (main documentation)
- `BUILD_ISSUES.md` (build status)
- `ARCHITECTURE.md` (architecture docs)
- `ROADMAP.md` (to create)
- `CLAUDE.md` (agent orientation)
- `AGENTS.md` (agent coordination)

## Estimated Complexity
- 1-2 days

## Dependencies
- Should be done after Tasks 001-006 (to reflect completed work)
- Can be done incrementally as tasks complete

## Notes
- Version number 0.0.10-dev is honest - keep it
- Don't oversell capabilities
- Be transparent about what's working vs what's stubbed
- Link to task files for implementation details
- Update documentation as tasks complete
- Consider adding "Known Limitations" section
- This is critical for user trust