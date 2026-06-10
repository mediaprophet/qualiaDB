# QualiaDB Logic Modalities Handbook

**Version:** 0.0.10-dev  
**Last Updated:** 2026-06-10  
**Purpose:** Comprehensive documentation of QualiaDB's logic modality implementation status and roadmap

## Supported Logic Families

QualiaDB provides production-grade support for non-classical, extended, graded, and paraconsistent logic families. See [Logic Coverage Assessment](logic-coverage-assessment.md) for a comprehensive mapping of logic families to implementation status.

### Production-Grade Support
- **Paraconsistent Logic** (Belnap 4-valued, explosion barriers, tension scoring)
- **Non-Monotonic Logic** (defeasible rules, default negation in ASP)  
- **Deontic Logic** (normative compliance, violation trails, obligation/permission/forbidden)
- **Epistemic & Doxastic Logic** (knowledge/belief attribution, agent consensus)
- **Linear Logic** (resource consumption, single-use, balance/reconciliation)
- **Temporal Logic** (LTL, Büchi automata, path-dependent traces)
- **Spatio-Temporal Logic** (geofencing, Allen relations, temporal algebra)
- **Probabilistic/Fuzzy Logic** (belief propagation, confidence coefficients)
- **Answer Set Programming** (stable models, grounding)
- **Description Logic** (subsumption, transitive closure, restrictions)
- **Classical/Propositional/Predicate Logic** (Quin atoms, unification, matching)
- **Constraint/Rule Enforcement** (SHACL compiler, N3 rule gating)
- **Clinical/Medical Decision Logic** (contraindications, comorbidity graphs)

### Partial Foundation / Opportunities to Deepen
- **Topological Logic** (RCC8-style spatial relations, connectivity preservation)
- **Quantum Logic** (propositional layer, orthomodular lattices)
- **Interval Logic** (richer reasoning engine beyond Allen intervals)
- **Control Theory** (feedback loops, stability analysis, self-stabilizing agents)

### Intentional Out of Scope
- Classical physics/math domains (better served by specialized libraries)
- Higher-level interpretive frameworks (hermeneutics, semiotics - application layer)

---

## Logic Modality Audit

**Purpose**: Identify and rewire logic modalities in the codebase for better organization and consistency with the calculus modality pattern.

## Current State

### Existing Modalities Structure
```
crates/qualia-core-db/src/modalities/
├── calculus/
│   ├── mod.rs          # Calculus modality entry point
│   ├── gpu.rs          # GPU integration
│   ├── host.rs         # Host integration (disabled on Windows)
│   └── ode_solver.rs   # RK4 ODE solver implementation
```

### Identified Logic Modalities in Root Directory

The following files in `crates/qualia-core-db/src/` appear to be logic modalities that should be moved to `modalities/logic/`:

1. **deontic_logic.rs** (33,458 bytes)
   - Implements deontic logic opcodes (OP_OBLIGATE, OP_PERMIT, OP_FORBID)
   - Already documented in AGENTS.md as a modality
   - Status: ✅ Complete, 10/10 tests
   - Should be moved to: `modalities/logic/deontic.rs`

2. **logic.rs** (27,489 bytes)
   - Core logic modality with WebizenVM, WebizenCompiler
   - Contains LTL opcodes and logic evaluation
   - Should be moved to: `modalities/logic/core.rs`

3. **qubo_compiler.rs** (8,960 bytes)
   - QUBO (Quantum Unconstrained Binary Optimization) compilation
   - Quantum logic formulation
   - Should be moved to: `modalities/logic/qubo.rs`

4. **n3_compiler.rs** (10,714 bytes)
   - N3 rule compilation
   - Logic rule processing
   - Should be moved to: `modalities/logic/n3_compiler.rs`

5. **n3_parser.rs** (9,625 bytes)
   - N3 rule parsing
   - Logic rule parsing
   - Should be moved to: `modalities/logic/n3_parser.rs`

6. **shacl_compiler.rs** (48,852 bytes)
   - SHACL constraint compilation
   - Constraint logic
   - Should be moved to: `modalities/logic/shacl.rs`

7. **owl_to_shacl.rs** (26,546 bytes)
   - OWL to SHACL conversion
   - Ontology logic
   - Should be moved to: `modalities/logic/owl.rs`

8. **rules.rs** (62 bytes)
   - Rules definition (likely stub)
   - Should be moved to: `modalities/logic/rules.rs`

## Proposed Structure

```
crates/qualia-core-db/src/modalities/
├── calculus/
│   ├── mod.rs
│   ├── gpu.rs
│   ├── host.rs
│   └── ode_solver.rs
└── logic/
    ├── mod.rs              # Logic modality entry point
    ├── core.rs             # Core logic (from logic.rs)
    ├── deontic.rs          # Deontic logic (from deontic_logic.rs)
    ├── qubo.rs             # QUBO compilation (from qubo_compiler.rs)
    ├── n3_compiler.rs      # N3 rule compilation (from n3_compiler.rs)
    ├── n3_parser.rs        # N3 rule parsing (from n3_parser.rs)
    ├── shacl.rs            # SHACL compilation (from shacl_compiler.rs)
    ├── owl.rs              # OWL to SHACL (from owl_to_shacl.rs)
    └── rules.rs            # Rules definition (from rules.rs)
```

## Rewiring Plan

### Phase 1: Create Logic Modality Structure
1. Create `modalities/logic/` directory
2. Create `modalities/logic/mod.rs` with logic opcodes and exports
3. Move files to new locations
4. Update imports throughout codebase

### Phase 2: Update Opcodes
- Define logic opcodes in `modalities/logic/mod.rs`
- Ensure consistency with existing opcode numbering
- Update AGENTS.md with new structure

### Phase 3: Rewire Imports
- Update `lib.rs` to import from new location
- Update all files that import these modules
- Ensure feature gates are preserved
- Update tests to use new paths

### Phase 4: Validation
- Run all tests to ensure no breakage
- Verify opcodes are correctly defined
- Check that no circular dependencies are introduced
- Validate that Windows-specific issues are handled

## Opcodes to Define

Based on AGENTS.md and existing code:

### Deontic Logic Opcodes
- `OP_OBLIGATE = 0x10`
- `OP_PERMIT = 0x11`
- `OP_FORBID = 0x12`

### Core Logic Opcodes
- LTL opcodes (from logic.rs)
- WebizenVM opcodes
- Logic evaluation opcodes

### QUBO Opcodes
- QUBO compilation opcodes
- Quantum logic opcodes

### N3 Rule Opcodes
- Rule compilation opcodes
- Rule parsing opcodes

### SHACL Opcodes
- Constraint opcodes
- Validation opcodes

### OWL Opcodes
- Ontology opcodes
- Conversion opcodes

## Dependencies

### External Dependencies
- serde, serde_json (for serialization)
- tokio (for async operations)
- log (for logging)

### Internal Dependencies
- `resolver.rs` (for hash resolution)
- `lexicon.rs` (for tokenization)
- `webizen.rs` (for VM integration)

## Testing Strategy

1. **Unit Tests**: Ensure all existing tests pass after move
2. **Integration Tests**: Verify logic modalities work with VM
3. **Opcode Tests**: Test all opcode definitions
4. **Feature Gate Tests**: Ensure feature gates work correctly

## Risk Assessment

### High Risk
- Breaking changes to imports across codebase
- Potential circular dependencies
- Windows-specific issues with some modules

### Medium Risk
- Test failures due to path changes
- Feature gate misconfiguration
- Opcode numbering conflicts

### Low Risk
- Documentation updates
- Comment updates
- Minor refactoring

## Timeline

- **Phase 1**: Create structure and move files (2 hours)
- **Phase 2**: Update opcodes (1 hour)
- **Phase 3**: Rewire imports (3 hours)
- **Phase 4**: Validation (2 hours)

**Total Estimated Time**: 8 hours

## Notes

- The calculus modality serves as a template for the logic modality structure
- Deontic logic is already well-documented in AGENTS.md
- SHACL compiler is the largest module and will require careful handling
- Some modules may have Windows-specific issues (similar to calculus/host.rs)
