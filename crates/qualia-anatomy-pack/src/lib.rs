//! The reference-body packer, compiled without the desktop client.
//! The source files stay in `qualia-client-core` so there is one packer.

#[path = "../../qualia-client-core/src/wellfair/ccf_resolver.rs"]
mod ccf_resolver;

#[path = "../../qualia-client-core/src/wellfair/anatomy_body.rs"]
mod anatomy_body;

#[path = "../../qualia-client-core/src/wellfair/anatomy_pack.rs"]
pub mod anatomy_pack;

#[path = "../../qualia-client-core/src/wellfair/body_constitution.rs"]
pub mod body_constitution;
