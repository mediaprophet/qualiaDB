//! Operation-specific acting capacity (E17.3). Independent of payment status.

/// Who is acting for this operation. Not a copied RemainingTarget class.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActingClass {
    Personal = 1,
    Corporate = 2,
    Humanitarian = 3,
}

/// Recovery A for this class. Humanitarian and Personal are always 0.
/// Fulfilled obligations recover 0 even for Corporate.
pub fn recovery_for_class(class: ActingClass, fulfilled: bool, remaining: u64) -> u64 {
    if fulfilled {
        return 0;
    }
    match class {
        ActingClass::Personal | ActingClass::Humanitarian => 0,
        ActingClass::Corporate => remaining,
    }
}

/// Employment / corporate capacity never bills personal use.
#[inline]
pub fn employment_bills_personal() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn econ_b_exemption_independent_of_payment() {
        assert!(!employment_bills_personal());
        assert_eq!(recovery_for_class(ActingClass::Personal, false, 1_000), 0);
        assert_eq!(
            recovery_for_class(ActingClass::Humanitarian, false, u64::MAX),
            0
        );
        assert_eq!(recovery_for_class(ActingClass::Corporate, false, 7), 7);
        assert_eq!(recovery_for_class(ActingClass::Corporate, true, 7), 0);
        assert_eq!(recovery_for_class(ActingClass::Corporate, true, 0), 0);
        assert_eq!(recovery_for_class(ActingClass::Personal, true, 9), 0);
    }
}
