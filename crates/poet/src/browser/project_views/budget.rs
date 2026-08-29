//! Project economics — auditable plan/actual/funding/royalty/tax workflow.

use web_sys::{Document, Element};

pub fn build_budget_view(document: &Document) -> Element {
    super::budget_workspace::build_budget_view(document)
}
