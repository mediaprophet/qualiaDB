# Task 009: Comprehensive Documentation Audit and Update

## Problem
The repository has 66 .md files, many of which are old, stale, or contain incorrect information about what's actually implemented. Documentation needs to be audited and updated to match the current codebase state.

## Files to Audit (66 total)

### 🔴 Critical Priority (Core Documentation)
- [ ] `README.md` - Main project documentation
- [ ] `ARCHITECTURE.md` - Architecture overview
- [ ] `AGENTS.md` - Agent coordination rules
- [ ] `CLAUDE.md` - AI agent orientation

### 🟡 High Priority (Status & Build)
- [ ] `BUILD_ISSUES.md` - Build status (likely outdated - errors fixed)
- [ ] `CHANGELOG.md` - Change history
- [ ] `docs/PROJECT_STATE.md` - Current project state
- [ ] `FIX_PLAN.md` - Fix plan (likely outdated)
- [ ] `TODO.md` - Project TODO (likely outdated)

### 🟢 Medium Priority (Technical Specs)
- [ ] `docs/manuals/ARCHITECTURE.md` - Manual architecture
- [ ] `docs/manuals/DEVELOPMENT.md` - Development guide
- [ ] `docs/manuals/developer-guide.md` - Developer guide
- [ ] `docs/manuals/glossary.md` - Glossary
- [ ] `docs/manuals/webizen-protocol-rfc.md` - Webizen protocol
- [ ] `docs/manuals/adr/*` - Architecture Decision Records (6 files)

### 🔵 Low Priority / Likely Stale
- [ ] `legacy/*` - Legacy documentation (3 files)
- [ ] `bundled/qapps/*` - App-specific docs (4 files)
- [ ] `docs/planning/*` - Planning docs (2 files)
- [ ] `docs/sdo-info/*` - SDO standards (6 files)
- [ ] `crates/*/README.md` - Crate-specific docs (4 files)

## Audit Process

For each file:

1. **Read the file** to understand what it claims
2. **Check relevant code sections** to verify implementation
3. **Identify inaccuracies**:
   - Outdated version numbers
   - Features claimed but not implemented
   - Features implemented but not documented
   - Incorrect build/test status
   - Stale TODO items
   - Wrong architecture descriptions
4. **Update or mark for deletion**:
   - Update if still relevant
   - Mark as deprecated if outdated
   - Delete if completely obsolete
   - Move to legacy/ if historical

## Known Issues (From Previous Work)

### README.md
- ❌ "271/271 tests passing" - Should be 539 tests, 0 build errors
- ❌ "microsecond memory-mapped queries" - mmap_query_subject is stub
- ❌ "real autoregressive decode (Linux)" - Uses mock pipeline
- ❌ "cryptographically auditable" - zk_proofs is stub
- ❌ "post-quantum ML-DSA" - Hand-rolled, not FIPS 204
- ❌ "tamper-proof WAL" - ECC parity is mock

### BUILD_ISSUES.md
- ❌ "44 remaining build errors" - All errors fixed (0 remaining)

### ARCHITECTURE.md
- ? Need to check for accuracy against current implementation
- ? LLM section may mention llama.cpp (corrected in CLAUDE.md)

### AGENTS.md
- ✅ Generally accurate (recently reviewed)
- ? May need updates for completed tasks

## Implementation Steps

### Phase 1: Critical Files (Day 1)
1. Audit README.md against codebase
2. Audit ARCHITECTURE.md against codebase
3. Audit AGENTS.md against codebase
4. Audit CLAUDE.md against codebase

### Phase 2: Status Files (Day 2)
5. Audit BUILD_ISSUES.md (mark as resolved)
6. Audit CHANGELOG.md
7. Audit PROJECT_STATE.md
8. Review FIX_PLAN.md and TODO.md (likely obsolete)

### Phase 3: Technical Specs (Day 3-4)
9. Audit docs/manuals/ARCHITECTURE.md
10. Audit docs/manuals/DEVELOPMENT.md
11. Audit docs/manuals/developer-guide.md
12. Audit ADR files for currency

### Phase 4: Cleanup (Day 5)
13. Review legacy/ files for deletion/archival
14. Review planning/ files for relevance
15. Review bundled/ app docs for accuracy
16. Create index of all documentation

## Success Criteria
- ✅ All critical documentation matches current implementation
- ✅ Build status accurately reflects 0 errors
- ✅ No overstated capabilities
- ✅ Stale files marked deprecated or deleted
- ✅ Documentation index created
- ✅ Version numbers accurate

## Related Files
- All .md files in repository
- Codebase for verification

## Estimated Complexity
- Critical files: 1 day
- Status files: 1 day
- Technical specs: 2 days
- Cleanup: 1 day
- **Total**: 5 days

## Dependencies
- None (can be done independently)
- Should be done after Tasks 001-006 (to reflect completed work)

## Notes
- This is a large task - focus on high-impact files first
- Version 0.0.10-dev is accurate - keep it
- Be honest about what's implemented vs stubbed
- Consider creating a DOCUMENTATION_STATUS.md to track audit progress
- Some files may be historical - preserve if they have value
- Link to task files for implementation details