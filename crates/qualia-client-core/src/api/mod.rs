//! Client API surface — split into domain sub-modules.
//! All items are re-exported here to preserve the flat pi::function_name() API.

pub mod agents;
pub mod agreements;
pub mod chat;
pub mod connect;
pub mod daemon;
pub mod dashboard;
pub mod domains;
pub mod downloads;
pub mod guardianship;
pub mod jobs;
pub mod mail;
pub mod model;
pub mod profile;
pub mod projects;
pub mod protocol;
pub mod qapp_launcher;
pub mod qapp_vault;
pub mod superblock;
pub mod system;
pub mod tokens;
pub mod windows;

pub use agents::*;
pub use agreements::*;
pub use chat::*;
pub use connect::*;
pub use daemon::*;
pub use dashboard::*;
pub use domains::*;
pub use downloads::*;
pub use guardianship::*;
pub use jobs::*;
pub use mail::*;
pub use model::*;
pub use profile::*;
pub use projects::*;
pub use protocol::*;
pub use qapp_launcher::*;
pub use qapp_vault::*;
pub use superblock::*;
pub use system::*;
pub use tokens::*;
pub use windows::*;
