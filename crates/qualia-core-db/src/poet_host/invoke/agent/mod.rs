//! Agent orchestration invoke family.
//!
//! Contains the DAG executor (R3), agent context builder, and stalk
//! isolation. These are the wiring layers that connect VibeScript agent
//! primitives (A1–A9) to the host capability dispatch and blackboard.

pub mod dag_executor;
