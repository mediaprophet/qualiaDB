//! Explicit sanctuary leave choice — never silent.
//!
//! Migrating to Solid requires the principal to choose:
//! 1. **Omit** sanctuary / restricted / bilateral data from the export, or
//! 2. **Reclassify** (change permission structures) so that data may leave
//!    for Solid under owner control.
//!
//! Classified context **never** leaves, regardless of choice.

use crate::{NQuin, PermissiveRoutingLane};

use super::filter::EgressStats;

/// Principal’s decision when sanctuary-bearing quins are present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SanctuaryExportChoice {
    /// No choice recorded — export must fail closed if sanctuary is present.
    #[default]
    Unset,
    /// Leave sanctuary data behind; export only what Solid can hold under
    /// current public/restricted-off policy.
    OmitSanctuary,
    /// Owner accepts that permission structures change so sanctuary/restricted
    /// data may be projected into Solid (still never classified).
    ReclassifyForSolid,
}

impl SanctuaryExportChoice {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unset => "unset",
            Self::OmitSanctuary => "omit-sanctuary",
            Self::ReclassifyForSolid => "reclassify-for-solid",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "unset" | "" => Some(Self::Unset),
            "omit" | "omit-sanctuary" | "leave-behind" => Some(Self::OmitSanctuary),
            "reclassify" | "reclassify-for-solid" | "include-restricted" => {
                Some(Self::ReclassifyForSolid)
            }
            _ => None,
        }
    }

    /// Whether this choice opts sanctuary/restricted into the Solid export.
    pub fn includes_restricted(self) -> bool {
        matches!(self, Self::ReclassifyForSolid)
    }
}

/// Count how many quins would be sanctuary-gated (excluding classified).
pub fn count_sanctuary_quins(quins: &[NQuin]) -> usize {
    quins
        .iter()
        .filter_map(|q| {
            let mut qq = *q;
            if qq.parity == 0 {
                qq.recalculate_parity();
            }
            qq.verify_ecc_parity().then_some(qq)
        })
        .filter(|q| q.get_sensitivity_byte() != NQuin::SENSITIVITY_CLASSIFIED)
        .filter(|q| is_sanctuary_bearing(q))
        .count()
}

pub fn count_classified_quins(quins: &[NQuin]) -> usize {
    quins
        .iter()
        .filter_map(|q| {
            let mut qq = *q;
            if qq.parity == 0 {
                qq.recalculate_parity();
            }
            qq.verify_ecc_parity().then_some(qq)
        })
        .filter(|q| q.get_sensitivity_byte() == NQuin::SENSITIVITY_CLASSIFIED)
        .count()
}

fn is_sanctuary_bearing(quin: &NQuin) -> bool {
    match quin.get_sensitivity_byte() {
        NQuin::SENSITIVITY_RESTRICTED => return true,
        NQuin::SENSITIVITY_CLASSIFIED => return true,
        _ => {}
    }
    match quin.get_sensitivity_tier() {
        NQuin::SENSITIVITY_TIER_LEGAL
        | NQuin::SENSITIVITY_TIER_MEDICAL
        | NQuin::SENSITIVITY_TIER_FIDUCIARY => return true,
        _ => {}
    }
    quin.identify_routing_lane() == PermissiveRoutingLane::EnforceBilateralMicroCommons
}

/// Pre-flight notice for desktop / Databox UI before building a leave bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanctuaryMigrationNotice {
    pub sanctuary_quin_count: usize,
    pub classified_quin_count: usize,
    pub requires_choice: bool,
    pub title: String,
    pub body: String,
    pub choice_omit_label: String,
    pub choice_reclassify_label: String,
    pub classified_note: String,
}

/// Build the user-facing notice. Call before export when planning a leave.
pub fn plan_sanctuary_migration_notice(quins: &[NQuin]) -> SanctuaryMigrationNotice {
    let sanctuary = count_sanctuary_quins(quins);
    let classified = count_classified_quins(quins);
    let requires_choice = sanctuary > 0;
    SanctuaryMigrationNotice {
        sanctuary_quin_count: sanctuary,
        classified_quin_count: classified,
        requires_choice,
        title: if requires_choice {
            "Sanctuary data needs a choice before Solid export".into()
        } else {
            "Ready to export to Solid".into()
        },
        body: if requires_choice {
            format!(
                "This graph contains {sanctuary} sanctuary / restricted / bilateral record(s). \
                 Solid Pods store open RDF triples and quads — not Qualia sanctuary structures. \
                 You must choose: (1) omit that information from the export, or (2) change \
                 permission structures so it can be reclassified and exported under your control. \
                 Classified records ({classified}) never leave this device."
            )
        } else if classified > 0 {
            format!(
                "No sanctuary-tier records to decide on. {classified} classified record(s) \
                 will stay on this device and will not appear in the Solid export."
            )
        } else {
            "No sanctuary or classified records detected. You can export the public graph to Solid."
                .into()
        },
        choice_omit_label: "Omit sanctuary data — export only what Solid can hold without reclassification"
            .into(),
        choice_reclassify_label:
            "Reclassify for Solid — I change permissions so sanctuary data may leave as RDF under my control"
                .into(),
        classified_note: "Classified context never exports to Solid, regardless of choice.".into(),
    }
}

/// Fail closed when sanctuary is present and the principal has not chosen.
pub fn require_sanctuary_choice(
    quins: &[NQuin],
    choice: SanctuaryExportChoice,
) -> Result<(), String> {
    let sanctuary = count_sanctuary_quins(quins);
    if sanctuary == 0 {
        return Ok(());
    }
    match choice {
        SanctuaryExportChoice::Unset => Err(format!(
            "SANCTUARY_CHOICE_REQUIRED: {sanctuary} sanctuary/restricted record(s) present. \
             Choose omit-sanctuary (leave them behind) or reclassify-for-solid (change \
             permissions so they may export). Classified records never leave. See \
             plan_sanctuary_migration_notice()."
        )),
        SanctuaryExportChoice::OmitSanctuary | SanctuaryExportChoice::ReclassifyForSolid => Ok(()),
    }
}

/// Summarise what an export did, for post-export UI.
pub fn export_outcome_summary(choice: SanctuaryExportChoice, stats: &EgressStats) -> String {
    match choice {
        SanctuaryExportChoice::Unset => "No sanctuary choice was recorded.".into(),
        SanctuaryExportChoice::OmitSanctuary => format!(
            "Exported {} record(s); omitted {} sanctuary/restricted; classified redacted {}.",
            stats.exported, stats.redacted_restricted, stats.redacted_classified
        ),
        SanctuaryExportChoice::ReclassifyForSolid => format!(
            "Exported {} record(s) after reclassification consent; classified still redacted {}.",
            stats.exported, stats.redacted_classified
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    fn public_quin() -> NQuin {
        let mut q = NQuin {
            subject: q_hash("Alice"),
            predicate: q_hash("knows"),
            object: q_hash("Bob"),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q.recalculate_parity();
        q
    }

    #[test]
    fn unset_fails_when_sanctuary_present() {
        let mut q = public_quin();
        q.set_sensitivity_byte(NQuin::SENSITIVITY_RESTRICTED);
        q.recalculate_parity();
        assert!(require_sanctuary_choice(&[q], SanctuaryExportChoice::Unset).is_err());
        assert!(require_sanctuary_choice(&[q], SanctuaryExportChoice::OmitSanctuary).is_ok());
    }

    #[test]
    fn notice_requires_choice_when_restricted() {
        let mut q = public_quin();
        q.set_sensitivity_byte(NQuin::SENSITIVITY_RESTRICTED);
        q.recalculate_parity();
        let n = plan_sanctuary_migration_notice(&[q]);
        assert!(n.requires_choice);
        assert!(n.body.contains("omit"));
        assert!(n.body.contains("reclassif"));
    }
}
