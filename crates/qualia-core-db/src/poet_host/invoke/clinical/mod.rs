//! Clinical Decision Support & Pharmacological Risk Scoring Subsystem [WASM-Standalone].

pub mod cha2ds2;
pub mod contraindication;
pub mod framingham;
pub mod score2;

pub use cha2ds2::score as cha2ds2_vasc;
pub use contraindication::{check_condition as check_contraindication, check_drugs as check_drug_interaction};
pub use framingham::score as framingham;
pub use score2::score as score2;
