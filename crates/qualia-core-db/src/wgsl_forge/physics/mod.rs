//! Certified physics forge kernels — complete, deterministic compute shaders with exact
//! CPU oracles, naga validation, and GPU certification.
//!
//! These formalise the previously-orphan `src/shaders/{molecular_dynamics,kinematics}.wgsl`:
//! nothing in the codebase loaded them, and as written they were incomplete or unsafe —
//! the MD shader updated position but left velocity unchanged (half an integrator), and
//! the kinematics shader read and wrote the *same* buffer (a data race making the result
//! non-deterministic). The kernels here are corrected (complete velocity-Verlet;
//! double-buffered, Plummer-softened N-body), embedded from the `.wgsl` via `include_str!`
//! so there is a single source of truth, graded against a Rust oracle, and runnable on any
//! wgpu adapter.
//!
//! **Deliberately not here (reserved for curation, not faked):**
//! - `fluid_dynamics.wgsl` — a `velocity *= 0.99` placeholder labelled "mock Navier–Stokes".
//! - `quantum_bio.wgsl` — non-compiling (invalid inline-struct syntax, builtin shadowing)
//!   and partly demo-grade (one uniform reused as several unrelated physical quantities).
//!
//! Formalising those two requires a decision on the intended physical model (which fluid
//! scheme; which quantum-biology observables and their real parameterisation) — an
//! out-of-band call that belongs to Timothy, so they are left for that direction rather
//! than dressed up as complete.

pub mod kinematics;
pub mod molecular_dynamics;
