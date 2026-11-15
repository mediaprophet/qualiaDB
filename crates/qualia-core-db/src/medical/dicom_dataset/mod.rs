//! Bounded borrowed Part-10 datasets. Original bytes/unknown values remain intact.
mod header;
mod parser;
mod query;
pub use header::{DatasetError, ParseLimits, TransferSyntax};
pub use parser::{Element, ElementId, ElementKind, ItemId, Part10Dataset};
pub use query::WorkBudget;
#[cfg(test)]
mod tests;
