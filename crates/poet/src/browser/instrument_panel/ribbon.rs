//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Ribbon tool descriptor used by container-type and chain catalogs.

#[derive(Clone, Copy)]
pub(super) struct RibbonTool {
    pub(super) id: &'static str,
    pub(super) icon: &'static str,
    pub(super) label: &'static str,
    pub(super) description: &'static str,
}
