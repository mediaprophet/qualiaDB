//! Path payload budget including WireGuard and inner IPv6/UDP.

/// Conservative QFrame budget on an outer IPv6/UDP path.
///
/// `floor((outer_path_MTU - 40 - 8 - 32) / 16) * 16 - 48`
pub const fn qframe_budget_ipv6(outer_path_mtu: u16) -> u16 {
    if outer_path_mtu < 128 {
        return 0;
    }
    let inner = outer_path_mtu.saturating_sub(40 + 8 + 32);
    let aligned = (inner / 16) * 16;
    aligned.saturating_sub(48)
}

pub const QFRAME_ON_1280: u16 = qframe_budget_ipv6(1280);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservative_1280_is_1152() {
        assert_eq!(QFRAME_ON_1280, 1152);
        assert_eq!(qframe_budget_ipv6(1280), 1152);
    }
}
