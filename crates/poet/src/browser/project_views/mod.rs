//! Project container views for Workstream A — Collaborative/ERP/PM.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Every surface persists on the shared COP `/records` ledger. Mock tables
//! were removed; empty families stay empty until a record is saved.

pub mod agent_console_workspace;
pub mod agent_conversation;
pub mod agent_review;
pub mod agent_run_history;
pub mod agent_session_browser;
pub mod budget_model;
pub mod budget_workspace;
pub mod connector_health;
pub mod connector_runs;
pub mod connector_workspace;
pub mod persist;
pub mod persist_ledgers;

pub mod agent_console;
pub mod analytics;
pub mod asset_mgr;
pub mod automation;
pub mod awards;
pub mod bounties;
pub mod budget;
pub mod bulk_import;
pub mod calendar;
pub mod commons;
pub mod cost_base;
pub mod credentials;
pub mod dashboard;
pub mod data_sources;
pub mod deliverable;
pub mod discussion;
pub mod doc_mgmt;
pub mod events;
pub mod gantt;
pub mod governance;
pub mod integrations;
pub mod ip_registry;
pub mod issues;
pub mod kanban;
pub mod knowledge_base;
pub mod news;
pub mod onboarding;
pub mod portfolio;
pub mod project_sheet;
pub mod resource_report;
pub mod retrospective;
pub mod review;
pub mod risk;
pub mod roadmap;
pub mod task_list;
pub mod time_tracking;
pub mod timeline;
pub mod token_mgr;
pub mod voting;
pub mod wiki;
