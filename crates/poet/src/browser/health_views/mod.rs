//! Health & Wellbeing container views.
//!
//! Records persist on the COP ledger. Documents go through NLP + the
//! Semantic Library (classified/secret). Sharing is a named-DID disclosure.
//! Conditions are possessions of a Principal, not owl:Thing.

pub mod clinical_models;
pub mod conditions_workspace;
pub mod disclosure_list;
pub mod disclosure_model;
pub mod disclosure_workspace;
pub mod medications_workspace;
pub mod model;
pub mod overview_workspace;
pub mod persist;
pub mod persist_ledgers;
pub mod record_inspection;
pub mod vitals_chart;

pub mod authority_attestations;
pub mod biometrics;
pub mod clinical_reports;
pub mod conditions;
pub mod diet;
pub mod disclosure_log;
pub mod documents;
pub mod family_history;
pub mod health_overview;
pub mod hypotheses;
pub mod immunizations;
pub mod lab_results;
pub mod life_records;
pub mod medications;
pub mod mental_wellbeing;
pub mod physical_activity;
pub mod procedures;
pub mod safeguards;
pub mod sleep;
pub mod therapy_notes;
pub mod vitals;
pub mod welfare_support;
