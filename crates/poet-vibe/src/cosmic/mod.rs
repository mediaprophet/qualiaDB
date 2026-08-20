//! Omniversal Coordinate System (OCS) — cosmic coordinate types.
//!
//! Implements the normative specification from
//! `docs/plans/cosmic-coordinate-systems-specification.md` v2.2.0.
//!
//! ## Modules
//!
//! - [`usri`] — Universal Spacetime & Reality Identifier parsing/generation
//! - [`celestial`] — Celestial body taxonomy and profile types
//! - [`transforms`] — Coordinate transforms (WGS84 ↔ ECEF ↔ ENU, geodetic distance)
//! - [`stardate`] — Piecewise stardate morphism (TOS / TNG / 32nd century)
//! - [`warp`] — Warp scale velocity curve (TOS / TNG + soft saturation)
//! - [`grounding`] — Grounding status and granular collapse operator
//! - [`observer`] — Observer fiber, affective status, epistemic divergence
//! - [`theory`] — Theory packages, law nature, assurance hierarchy
//! - [`nested`] — Nested realm context hashing and time dilation
//! - [`cb_usri`] — Compact Binary USRI for zero-heap hot paths
//!
//! Reference: OCS Specification v2.2.0, Timothy Charles Holborn.

pub mod cb_usri;
pub mod celestial;
pub mod grounding;
pub mod nested;
pub mod observer;
pub mod stardate;
pub mod theory;
pub mod transforms;
pub mod usri;
pub mod warp;
