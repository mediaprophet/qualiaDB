# QualiaDB Implementation Tasks

This directory contains detailed implementation plans to address all issues identified in the Claude review (2026-06-10) and subsequent analysis.

## Overview

The Claude review identified critical gaps between documentation claims and actual implementation, particularly in security-critical areas. All tasks are scoped to bring the project to a functional, production-ready state.

## Task List

### 🔴 Critical Security Tasks (Must Fix First)

| Task | File | Severity | Status | Est. Time |
|------|------|----------|--------|-----------|
| [001_security_zk_proofs.md](001_security_zk_proofs.md) | zk_proofs.rs | 🔴 CRITICAL | Pending | 2-3 days |
| [002_security_fiduciary_crypto.md](002_security_fiduciary_crypto.md) | fiduciary_crypto.rs | 🔴 CRITICAL | Pending | 1-2 days |
| [003_security_ml_dsa.md](003_security_ml_dsa.md) | fiduciary_crypto.rs | 🔴 HIGH | Pending | 2-3 days |
| [004_security_ecc_parity.md](004_security_ecc_parity.md) | lib.rs | 🔴 HIGH | Pending | 0.5-2 days |

### 🔴 Critical Functionality Tasks

| Task | File | Severity | Status | Est. Time |
|------|------|----------|--------|-----------|
| [005_query_layer_stubs.md](005_query_layer_stubs.md) | query_engine.rs, indexing.rs | 🔴 HIGH | Pending | 5-8 days |
| [006_llm_mock_pipeline_fix.md](006_llm_mock_pipeline_fix.md) | gguf_bridge.rs | 🔴 HIGH | Pending | 2-4 days |

### 🟡 Documentation Tasks

| Task | File | Severity | Status | Est. Time |
|------|------|----------|--------|-----------|
| [007_documentation_accuracy.md](007_documentation_accuracy.md) | README.md, BUILD_ISSUES.md | 🟡 MEDIUM | Pending | 1-2 days |

## Priority Order

### Phase 1: Security First (Week 1-2)
1. Task 001: Fix zk_proofs stub
2. Task 002: Fix fiduciary_crypto stub
3. Task 004: Fix ECC parity (quicker than ML-DSA)
4. Task 003: Fix ML-DSA (or remove claims)

### Phase 2: Core Functionality (Week 3-4)
5. Task 006: Fix mock pipeline (enables real Linux inference)
6. Task 005: Fix query layer (enables real database queries)

### Phase 3: Documentation (Week 5)
7. Task 007: Update all documentation to reflect reality

## Current Status

### ✅ Completed (Prior Work)
- All 82 build errors fixed (0 errors remaining)
- Tokio runtime nesting issues resolved
- Mock pipeline issue identified and documented
- Claude review findings validated

### 🔄 In Progress
- None (tasks not yet started)

### ⏳ Pending
- All tasks listed above

## Total Estimated Time

- **Security tasks**: 5.5-10 days
- **Functionality tasks**: 7-12 days
- **Documentation tasks**: 1-2 days
- **Total**: 13.5-24 days (3-5 weeks for 1 developer)

## Platform Impact After Completion

| Platform | Current | After Tasks |
|----------|---------|-------------|
| **Windows** | DirectML works, mock fallback | Full GPU inference ✅ |
| **macOS** | Accelerate works, mock fallback | Full GPU inference ✅ |
| **Linux** | Mock pipeline only | Full GPU inference ✅ |

## Security Audit

After completing security tasks (001-004), schedule a professional security audit:
- Review ZK proof implementation
- Review ML-DSA implementation
- Review signature verification
- Review ECC parity implementation
- Penetration testing

## Notes

- All tasks are independent unless noted
- Tasks can be parallelized by multiple developers
- Version 0.0.10-dev accurately reflects development state
- Documentation should be updated as tasks complete
- Each task file contains detailed implementation steps
- Success criteria are clearly defined in each task

## References

- [Claude Review Analysis](../docs/claudereport.md) - Validation of review findings
- [Mock Pipeline Fix Documentation](../MOCK_PIPELINE_FIX.md) - Detailed analysis
- [BUILD_ISSUES.md](../BUILD_ISSUES.md) - Historical build issues
- [README.md](../README.md) - Project documentation
- [AGENTS.md](../AGENTS.md) - Agent coordination rules
- [CLAUDE.md](../CLAUDE.md) - AI agent orientation