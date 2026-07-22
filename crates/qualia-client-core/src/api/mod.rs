//! Client API surface â€” split into domain sub-modules.
//! All items are re-exported here to preserve the flat pi::function_name() API.

pub mod superblock;
pub mod qapp_vault;
pub mod system;
pub mod tokens;
pub mod chat;
pub mod agents;
pub mod jobs;
pub mod profile;
pub mod domains;
pub mod mail;
pub mod connect;
pub mod projects;
pub mod agreements;
pub mod model;
pub mod downloads;
pub mod daemon;
pub mod qapp_launcher;
pub mod dashboard;
pub mod protocol;
pub mod windows;
pub mod guardianship;

pub use superblock::*;
pub use qapp_vault::*;
pub use system::*;
pub use tokens::*;
pub use chat::*;
pub use agents::*;
pub use jobs::*;
pub use profile::*;
pub use domains::*;
pub use mail::*;
pub use connect::*;
pub use projects::*;
pub use agreements::*;
pub use model::*;
pub use downloads::*;
pub use daemon::*;
pub use qapp_launcher::*;
pub use dashboard::*;
pub use protocol::*;
pub use windows::*;
pub use guardianship::*;