//! Intentionality assessment — assess intent, classify mistakes.

/// Intentionality classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intentionality {
    Intentional,
    Negligent,
    Reckless,
    Accidental,
    Unknowing,
}

impl Intentionality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Intentional => "intentional",
            Self::Negligent => "negligent",
            Self::Reckless => "reckless",
            Self::Accidental => "accidental",
            Self::Unknowing => "unknowing",
        }
    }
}

/// Mistake classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MistakeType {
    Honest,
    Systematic,
    Repeated,
    Negligent,
    Willful,
}

impl MistakeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Honest => "honest",
            Self::Systematic => "systematic",
            Self::Repeated => "repeated",
            Self::Negligent => "negligent",
            Self::Willful => "willful",
        }
    }
}

/// Assess intentionality from evidence markers.
///
/// Takes evidence markers: `knew_outcome` (bool), `could_prevent` (bool),
/// `repeated_behavior` (bool), `benefited` (bool), `acknowledged` (bool).
pub fn assess_intentionality(
    knew_outcome: bool,
    could_prevent: bool,
    repeated_behavior: bool,
    benefited: bool,
) -> Intentionality {
    if knew_outcome && could_prevent && benefited {
        Intentionality::Intentional
    } else if knew_outcome && !could_prevent && repeated_behavior {
        Intentionality::Reckless
    } else if !knew_outcome && could_prevent && repeated_behavior {
        Intentionality::Negligent
    } else if !knew_outcome && !could_prevent {
        Intentionality::Unknowing
    } else {
        Intentionality::Accidental
    }
}

/// Classify a mistake based on pattern markers.
///
/// Takes: `first_occurrence` (bool), `pattern_matches` (count),
/// `corrected_after_feedback` (bool), `systemic_factor` (bool).
pub fn classify_mistake(
    first_occurrence: bool,
    pattern_matches: usize,
    corrected_after_feedback: bool,
    systemic_factor: bool,
) -> MistakeType {
    if first_occurrence && corrected_after_feedback {
        MistakeType::Honest
    } else if systemic_factor {
        MistakeType::Systematic
    } else if pattern_matches > 2 && !corrected_after_feedback {
        MistakeType::Willful
    } else if pattern_matches > 1 {
        MistakeType::Repeated
    } else {
        MistakeType::Negligent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intentional_assessment() {
        let i = assess_intentionality(true, true, true, true);
        assert_eq!(i, Intentionality::Intentional);
    }

    #[test]
    fn reckless_assessment() {
        let i = assess_intentionality(true, false, true, false);
        assert_eq!(i, Intentionality::Reckless);
    }

    #[test]
    fn negligent_assessment() {
        let i = assess_intentionality(false, true, true, false);
        assert_eq!(i, Intentionality::Negligent);
    }

    #[test]
    fn unknowing_assessment() {
        let i = assess_intentionality(false, false, false, false);
        assert_eq!(i, Intentionality::Unknowing);
    }

    #[test]
    fn accidental_assessment() {
        let i = assess_intentionality(true, true, false, false);
        assert_eq!(i, Intentionality::Accidental);
    }

    #[test]
    fn classify_honest_mistake() {
        let m = classify_mistake(true, 0, true, false);
        assert_eq!(m, MistakeType::Honest);
    }

    #[test]
    fn classify_systematic_mistake() {
        let m = classify_mistake(false, 1, true, true);
        assert_eq!(m, MistakeType::Systematic);
    }

    #[test]
    fn classify_willful_mistake() {
        let m = classify_mistake(false, 5, false, false);
        assert_eq!(m, MistakeType::Willful);
    }

    #[test]
    fn classify_repeated_mistake() {
        let m = classify_mistake(false, 3, true, false);
        assert_eq!(m, MistakeType::Repeated);
    }
}
