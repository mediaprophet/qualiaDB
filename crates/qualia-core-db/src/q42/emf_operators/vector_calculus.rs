//! 3D Staggered & Collocated Electromagnetic Vector Calculus Operators.
//!
//! Delegates the continuum 3D Maxwell differential & integral operators (2nd-order and
//! 4th-order staggered Yee curls $\nabla\times\mathbf{E}$ and $\nabla\times\mathbf{H}$,
//! double-curl $\nabla\times(\mu^{-1}\nabla\times\mathbf{E})$, Gauss divergences
//! $\nabla\cdot\mathbf{D}$ and $\nabla\cdot\mathbf{B}$, vector identity
//! $\nabla\cdot(\nabla\times\mathbf{A}) = 0$, and discrete continuity verification)
//! to `etalon_sim::maxwell_emf::vector_calculus` per C01 ownership.

pub use etalon_sim::maxwell_emf::vector_calculus::*;
