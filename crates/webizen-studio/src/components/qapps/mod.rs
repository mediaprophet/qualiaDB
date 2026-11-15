//! QApps catalog and browser component.

pub(crate) mod catalog;
mod component;
pub mod os_services_panel;
mod tests;

#[allow(unused_imports)]
pub use catalog::*;
pub use component::QApps;
pub use os_services_panel::OsSharedServicesPanel;
