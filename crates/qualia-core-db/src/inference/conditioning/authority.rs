//! Authority / disclosure view (borrowed).

#[derive(Debug, Clone, Copy)]
pub struct AuthorityView<'a> {
    pub principal_did_hash: u64,
    pub disclosure_ceiling: u8,
    pub allowed_graph_scopes: &'a [u64],
    pub tools_allowed: bool,
}
