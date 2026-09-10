//! E20.1 inventory of public networking entry points.
//!
//! Every listed call goes through QPR (`qdnf`), not libp2p. This slice does
//! **not** claim Native Independent daemon completion (E20.2).

/// One public networking entry. Names match the application-facing surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryEntry {
    pub name: &'static str,
    pub path: &'static str,
}

const ENTRIES: &[InventoryEntry] = &[
    InventoryEntry {
        name: "pair",
        path: "PeerHost::pair",
    },
    InventoryEntry {
        name: "exchange_protected",
        path: "PeerHost::exchange_protected",
    },
    InventoryEntry {
        name: "announce",
        path: "NativePeer::announce",
    },
    InventoryEntry {
        name: "send_stream",
        path: "NativePeer::send_stream",
    },
    InventoryEntry {
        name: "lookup_qsr",
        path: "NativePeer::lookup_qsr",
    },
    InventoryEntry {
        name: "lookup_qsr_full",
        path: "NativePeer::lookup_qsr_full",
    },
    InventoryEntry {
        name: "open_session",
        path: "NativePeer::open_session",
    },
    InventoryEntry {
        name: "handshake_over_fragments",
        path: "handshake_over_fragments",
    },
];

/// Public QPR entry points. None of these import or call libp2p.
pub fn public_entry_points() -> &'static [InventoryEntry] {
    ENTRIES
}

/// This crate does not import `libp2p`. E20.2 daemon isolation is not claimed.
pub fn libp2p_imported() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_lists_qpr_entry_points() {
        let names: [&str; 8] = [
            "pair",
            "exchange_protected",
            "announce",
            "send_stream",
            "lookup_qsr",
            "lookup_qsr_full",
            "open_session",
            "handshake_over_fragments",
        ];
        let listed = public_entry_points();
        assert_eq!(listed.len(), 8);
        let mut i = 0usize;
        while i < names.len() {
            assert_eq!(listed[i].name, names[i]);
            i = i.saturating_add(1);
        }
        assert!(!libp2p_imported());
    }
}
