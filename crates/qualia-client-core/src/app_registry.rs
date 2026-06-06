use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AppTarget {
    LocalDevDirectory(PathBuf),
    LocalProxyPort(u16),
    IsolatedVault(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppManifest {
    pub name: String,
    pub version: String,
    pub required_shapes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegisteredApp {
    pub did: String,
    pub manifest: AppManifest,
    pub target: AppTarget,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct AppRegistryState {
    pub installed_apps: HashMap<String, RegisteredApp>,
}

impl AppRegistryState {
    pub fn new() -> Self {
        Self {
            installed_apps: HashMap::new(),
        }
    }
}
