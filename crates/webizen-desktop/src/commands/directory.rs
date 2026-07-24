//! Directory and agents

#![allow(non_snake_case)]

use qualia_client_core::api;
use qualia_client_core::state::{Actor, DelegationRule, FrontDoor};
use tauri::command;

// ── Directory / agents ────────────────────────────────────────────────────────

#[command]
pub fn get_front_doors() -> Result<Vec<FrontDoor>, String> {
    api::get_front_doors()
}

#[command]
pub fn generate_front_door(label: String) -> Result<FrontDoor, String> {
    api::generate_front_door(label)
}

#[command]
pub fn get_directory_actors() -> Result<Vec<Actor>, String> {
    api::get_directory_actors()
}

#[command]
pub fn add_directory_actor(actor: Actor) -> Result<(), String> {
    api::add_directory_actor(actor)
}

#[command]
pub fn get_delegation_rules() -> Result<Vec<DelegationRule>, String> {
    api::get_delegation_rules()
}

#[command]
pub fn add_delegation_rule(rule: DelegationRule) -> Result<(), String> {
    api::add_delegation_rule(rule)
}

