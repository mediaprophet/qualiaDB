//! The **birth transition**, the **Digital Birth Record**, and the **developmental accrual of agency** —
//! the domain model of `docs/manuals/standards/init-draft-standards-wip-main/DigitalBirthRecord` (Timothy's
//! draft standard), built on the correct primitive: **relational stewardship, not self-sovereignty.**
//!
//! ## Why this is *not* "self-sovereign" (the infant case falsifies it)
//!
//! An infant has no "sovereign self" to own or control anything — yet they are a full rights-bearing human.
//! Their support is **stewardship via a permissive commons, defined *between* guardians** (plural, with
//! distributed and differing roles: the medical decision-maker need not be the day-to-day carer). Personhood
//! then is **not a switch that flips at birth** — it **accrues gradually and relationally** as the child
//! develops (neurologically and otherwise), agency supports slowly yielding to the child's own say, toward
//! the capacity to maintain **adult personhood — *if healthy***. For those who never reach independent adult
//! agency, continued stewardship is **supported personhood, not a lesser status** (CRPD supported-, not
//! substituted-, decision-making). "Self-sovereign identity" can model *none* of this — the infant, the
//! gradient, or the lifelong-supported adult — which is exactly why it is the wrong, and harmful, model.
//! Agency/personhood is a **developmental, relational fabric**, never an atomic sovereign property.
//!
//! ## What this module builds
//!
//! At birth the gestational [`MaternalFetalDyad`] resolves into a born person who **owns** a foundational,
//! biometric-extended [`DigitalBirthRecord`] ("an inalienable digital prosthetic extension of a person"),
//! held under a [`Guardianship`] **commons of [`Steward`]s** (default the biological parents, subject to
//! official credentials), and carrying an [`AgencyStage`] on the developmental gradient. Biometrics are
//! referenced **by class** ([`BiometricClass`]); the datum is `Sanctuary`-class, held in the person's data
//! store and referenced, never inlined (data-minimisation).
//!
//! **What this is / isn't.** Domain machinery + the invariants, aligned to the standard's `br:` ontology
//! (`hasGuardian`/`isGuardianOf`/`hasCredential`/`hasConsentFrom`). It does **not** encode the RDF `br:`
//! vocabulary, biometric wire-formats, VC issuance, or wallet/`did:q42` integration (coordinate), and it
//! does **not** settle the values/legal specifics — the stage boundaries, when adult status is recognised,
//! the legal role definitions, guardianship-transfer policy — those are Timothy's / an expert's (§9.4).

use serde::{Deserialize, Serialize};

use super::dyad::{DyadInvalid, MaternalFetalDyad, Parentage, PrincipalRef};
use super::scorecard::ForumClass;

/// A **class** of biometric a birth record references — the record's *structure*, never the datum. The
/// actual biometric (a DNA sequence, a blood type) is `Sanctuary`-class, held in the person's data store
/// and referenced, not inlined here (data-minimisation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiometricClass {
    /// DNA — also the evidentiary basis of biological parentage and ancestry use-cases.
    Dna,
    BloodType,
    Fingerprint,
    /// Neonatal footprint (a historical birth-record biometric).
    Footprint,
    Other(String),
}

/// The kind of official credential that establishes or overrides a stewardship relation (`br:hasCredential`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardianshipCredential {
    /// DNA / official attestation confirming a biological parent as steward.
    ProofOfParentage,
    /// An adoption order — assigns stewardship to adoptive parent(s).
    AdoptionOrder,
    /// A surrogacy legal-parentage order — assigns stewardship to the intended parent(s).
    SurrogacyParentageOrder,
    /// A court/official grant of guardianship to a non-parent (kinship carer, state guardian, …).
    GuardianshipGrant,
    Other(String),
}

/// A reference to a verifiable credential (`br:Credential` / `br:hasCredential`). Abstract here — the VC /
/// identity layer resolves and verifies it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRef {
    pub vc_id: String,
    pub kind: GuardianshipCredential,
}

/// A responsibility a steward holds. Stewardship is **not one unitary "ownership"** — it is a *variety* of
/// distinct responsibilities, which may be held by different stewards (a medical decision-maker who is not
/// the day-to-day carer; a cultural/heritage steward; a financial trustee). Illustrative set — the
/// authoritative roles are curation/legal (Timothy/§9.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StewardRole {
    /// General guardianship (unspecified/full) — the default when roles are not split out.
    Guardian,
    /// Day-to-day care.
    PrimaryCare,
    /// Health decisions.
    Medical,
    /// Legal representation.
    Legal,
    /// Property / finances (in trust).
    Financial,
    Educational,
    /// Cultural / heritage stewardship.
    Cultural,
    Other(String),
}

/// How a steward holds their role — the biological-parent default, or an official credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StewardBasis {
    /// The biological-parent default (guardians are by default the biological parents).
    BiologicalDefault,
    /// Established/assigned/overridden by an official credential (adoption, surrogacy order, grant).
    Credentialed,
}

/// One steward in the guardianship commons — a guardian holding one or more [`StewardRole`]s. Stewardship
/// is held **in trust, in the child's interest** — it is *never* ownership of the person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Steward {
    pub principal: PrincipalRef,
    pub roles: Vec<StewardRole>,
    pub basis: StewardBasis,
}

/// Guardianship as a **permissive-commons stewardship** among [`Steward`]s — the correct model for an
/// infant/child, and the illustration of why "self-sovereign" is wrong (an infant is not sovereign; it is
/// *stewarded*, between guardians, in a commons). Not a single owner: a set of stewards holding distributed
/// responsibilities, in the child's interest. By **default the biological parents**, **subject to official
/// credentials** (`br:hasCredential`) that confirm, add, or override stewards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Guardianship {
    /// The stewardship commons.
    pub stewards: Vec<Steward>,
    /// Official credentials bearing on stewardship (`br:hasCredential`).
    pub credentials: Vec<CredentialRef>,
}

impl Guardianship {
    /// The principals who steward the child (`br:isGuardianOf` → the subject).
    pub fn guardians(&self) -> Vec<&PrincipalRef> {
        self.stewards.iter().map(|s| &s.principal).collect()
    }

    /// Whether any steward holds their role by official credential (vs the biological default alone).
    pub fn is_credentialed(&self) -> bool {
        self.stewards
            .iter()
            .any(|s| s.basis == StewardBasis::Credentialed)
            || !self.credentials.is_empty()
    }

    /// Stewards holding a particular responsibility — supports the "different roles held by different
    /// guardians" reality (e.g. who is the medical decision-maker).
    pub fn stewards_with(&self, role: &StewardRole) -> Vec<&Steward> {
        self.stewards
            .iter()
            .filter(|s| s.roles.contains(role))
            .collect()
    }
}

/// Where a person is on the **developmental accrual of agency and personhood** — a *gradient*, not a binary
/// "sovereign self" switch. Agency accrues gradually and relationally; the endpoint is the capacity to
/// maintain adult personhood **if healthy**; continued stewardship for those who need it is
/// [`SupportedAdult`] — **full personhood with supports, never a lesser status**.
///
/// The stage *boundaries* (and how they map to any legal age of majority / capacity assessment) are
/// deliberately **not asserted** here — that is values/legal content (Timothy/§9.4). The type encodes the
/// *shape* (a monotone gradient of self-determination) so downstream layers cannot model personhood as an
/// on/off sovereign property.
///
/// [`SupportedAdult`]: AgencyStage::SupportedAdult
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgencyStage {
    /// Newborn — the fourth trimester. Fully stewarded.
    Neonate,
    /// Infancy — fully stewarded via the guardianship commons.
    Infant,
    /// Childhood — developing agency, a growing say alongside stewardship.
    Child,
    /// Adolescence — substantial, evolving capacity (a large and increasing say).
    Adolescent,
    /// Adulthood — maintains personhood independently, if healthy.
    Adult,
    /// Adulthood with **supported** decision-making where needed — full personhood, with stewardship
    /// supports. Not a diminished status; the CRPD "supported, not substituted" model.
    SupportedAdult,
}

impl AgencyStage {
    /// A coarse, monotone indication (0..=100) of the person's own self-determination at this stage — it
    /// **increases** across the developmental gradient. Illustrative shape only (the authoritative model is
    /// Timothy's/§9.4); its purpose is to make "agency is graduated, not binary" structural. `SupportedAdult`
    /// is full self-determination *with supports* — not reduced.
    pub fn self_determination(self) -> u8 {
        match self {
            AgencyStage::Neonate => 0,
            AgencyStage::Infant => 10,
            AgencyStage::Child => 40,
            AgencyStage::Adolescent => 75,
            AgencyStage::Adult => 100,
            AgencyStage::SupportedAdult => 100,
        }
    }

    /// Whether stewardship is still active at this stage (everything up to independent/ supported adulthood
    /// carries stewardship; `SupportedAdult` carries *supports*, which are stewardship in service of the
    /// person's own will).
    pub fn stewardship_active(self) -> bool {
        !matches!(self, AgencyStage::Adult)
    }
}

/// A **Digital Birth Record** — the foundational, self-owned, biometric-extended record produced at birth.
/// An *inalienable prosthetic extension of the person*: they own it; the guardianship commons stewards it
/// (and the data store) as their agency develops.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigitalBirthRecord {
    /// The born person — the **subject who owns this record** (their own pairwise fabric, carried from the
    /// emerging child; never collapsed into a guardian or parent).
    pub subject: PrincipalRef,
    /// Which biometrics are attached (structure only; data held Sanctuary-class in the person's store).
    pub biometrics: Vec<BiometricClass>,
    /// The parentage carried forward from gestation (genetic + social roles), the basis of the biological
    /// default and of ancestry use-cases.
    pub parentage: Parentage,
    /// The stewardship commons (default biological parents, subject to official credentials).
    pub guardianship: Guardianship,
    /// The subject's stage on the developmental agency gradient (at birth, [`AgencyStage::Neonate`]).
    pub agency_stage: AgencyStage,
    /// The issuing/attesting authority's principal (`br:issuedBy`), if any. It *attests*; it does not own.
    pub issued_by: Option<PrincipalRef>,
    /// Date of birth as an ISO-8601 string, if recorded (kept as a string so this pure crate takes no date
    /// dependency; the caller supplies it).
    pub birth_date: Option<String>,
}

/// A reason a birth record is structurally invalid — a **rights** violation or an impossibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BirthRecordInvalid {
    /// The subject (the born person) is missing.
    NoSubject,
    /// The subject is **collapsed into a steward or parent** — the person is their own principal, never a
    /// field of an adult's identity.
    SubjectCollapsed,
    /// No steward — a child is never left unstewarded (an infant is not self-sovereign).
    NoSteward,
}

impl DigitalBirthRecord {
    /// A birth record is **forum internum**, `Sanctuary`-class (biometrics + a person's foundational
    /// record — the most sensitive class).
    pub const fn forum_class(&self) -> ForumClass {
        ForumClass::Internum
    }
    pub const fn sensitivity_class(&self) -> &'static str {
        "Sanctuary"
    }

    /// The effective guardians / stewards (`br:isGuardianOf` → this subject).
    pub fn guardians(&self) -> Vec<&PrincipalRef> {
        self.guardianship.guardians()
    }

    /// Enforce the rights invariants.
    pub fn validate(&self) -> Result<(), BirthRecordInvalid> {
        let subject = self.subject.did.trim();
        if subject.is_empty() {
            return Err(BirthRecordInvalid::NoSubject);
        }
        if self.guardianship.stewards.is_empty() {
            return Err(BirthRecordInvalid::NoSteward);
        }
        // The subject is distinct from every steward and every known adult in the parentage.
        let collapses_into_steward = self
            .guardianship
            .stewards
            .iter()
            .any(|s| s.principal.did.trim() == subject);
        let collapses_into_parent = self
            .parentage
            .known_genetic_parents()
            .into_iter()
            .chain(self.parentage.intended_parents.iter())
            .any(|p| p.did.trim() == subject);
        if collapses_into_steward || collapses_into_parent {
            return Err(BirthRecordInvalid::SubjectCollapsed);
        }
        Ok(())
    }
}

impl MaternalFetalDyad {
    /// Resolve the gestational dyad into a [`DigitalBirthRecord`] **at birth**. The emerging child becomes
    /// the born **subject** (owner of the record) at [`AgencyStage::Neonate`]; the guardianship **commons**
    /// defaults to the known biological parents (each a [`Steward`] via [`StewardBasis::BiologicalDefault`]),
    /// **subject to** `credentials` + `credentialed_guardians` (an adoption/surrogacy order or a guardianship
    /// grant — those become the stewards via [`StewardBasis::Credentialed`], overriding the biological
    /// default). Each default steward is assigned the general [`StewardRole::Guardian`]; splitting roles
    /// across different stewards is a later, legal/curation act. Only valid from a valid dyad.
    pub fn give_birth(
        &self,
        biometrics: Vec<BiometricClass>,
        credentials: Vec<CredentialRef>,
        credentialed_guardians: Vec<PrincipalRef>,
        issued_by: Option<PrincipalRef>,
        birth_date: Option<String>,
    ) -> Result<DigitalBirthRecord, DyadInvalid> {
        self.validate()?;
        let stewards: Vec<Steward> = if credentialed_guardians.is_empty() {
            // Default: the known biological parents steward the child.
            self.child
                .parentage
                .known_genetic_parents()
                .into_iter()
                .cloned()
                .map(|principal| Steward {
                    principal,
                    roles: vec![StewardRole::Guardian],
                    basis: StewardBasis::BiologicalDefault,
                })
                .collect()
        } else {
            // An official credential assigns/overrides the stewards (adoption, surrogacy order, grant).
            credentialed_guardians
                .into_iter()
                .map(|principal| Steward {
                    principal,
                    roles: vec![StewardRole::Guardian],
                    basis: StewardBasis::Credentialed,
                })
                .collect()
        };
        Ok(DigitalBirthRecord {
            subject: PrincipalRef::new(self.child.pairwise_did.clone()),
            biometrics,
            parentage: self.child.parentage.clone(),
            guardianship: Guardianship {
                stewards,
                credentials,
            },
            agency_stage: AgencyStage::Neonate,
            issued_by,
            birth_date,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::dyad::InterfaceKind;
    use super::super::dyad::{
        EmergingChild, MaternalBody, Progenitor, RightsStage, SocialRightsThreshold,
    };
    use super::super::physiology::{PhysiologicalState, ReproductiveState, Trimester};
    use super::*;

    /// A dyad with two KNOWN biological parents (the common case).
    fn dyad_two_known_parents() -> MaternalFetalDyad {
        MaternalFetalDyad {
            maternal: MaternalBody {
                principal: PrincipalRef::new("did:wf:mother"),
                state: PhysiologicalState::Reproductive(ReproductiveState::Pregnant(
                    Trimester::Third,
                )),
            },
            child: EmergingChild {
                pairwise_did: "did:wf:child-1".into(),
                parentage: Parentage {
                    ovum_source: Progenitor::Known(PrincipalRef::new("did:wf:mother")),
                    sperm_source: Progenitor::Known(PrincipalRef::new("did:wf:father")),
                    intended_parents: vec![],
                },
                rights_stage: RightsStage::EmergingInGestation,
            },
            gestational_age_days: 38 * 7,
            carnegie_stage: None,
            interface: InterfaceKind::Placenta,
            social_rights_threshold: SocialRightsThreshold::Undecided,
        }
    }

    #[test]
    fn birth_default_stewards_are_the_biological_parents_in_a_commons() {
        let rec = dyad_two_known_parents()
            .give_birth(
                vec![BiometricClass::Dna, BiometricClass::BloodType],
                vec![],
                vec![],
                None,
                None,
            )
            .unwrap();
        assert_eq!(rec.validate(), Ok(()));
        // The born person owns their record; starts as a neonate on the agency gradient.
        assert_eq!(rec.subject.did, "did:wf:child-1");
        assert_eq!(rec.agency_stage, AgencyStage::Neonate);
        // Guardianship is a COMMONS of the two biological-parent stewards (not one sovereign owner).
        assert_eq!(rec.guardianship.stewards.len(), 2);
        let guardian_dids: Vec<&str> = rec.guardians().iter().map(|g| g.did.as_str()).collect();
        assert!(guardian_dids.contains(&"did:wf:mother"));
        assert!(guardian_dids.contains(&"did:wf:father"));
        assert!(
            rec.guardianship
                .stewards
                .iter()
                .all(|s| s.basis == StewardBasis::BiologicalDefault)
        );
        assert!(!rec.guardianship.is_credentialed());
        assert_eq!(rec.forum_class(), ForumClass::Internum);
    }

    #[test]
    fn an_official_credential_overrides_the_biological_default() {
        // Surrogacy: the carrier gives birth, but a surrogacy parentage order assigns the intended parents.
        let mut dyad = dyad_two_known_parents();
        dyad.maternal.principal = PrincipalRef::new("did:wf:surrogate");
        dyad.child.parentage = Parentage {
            ovum_source: Progenitor::Donor,
            sperm_source: Progenitor::Known(PrincipalRef::new("did:wf:genetic-father")),
            intended_parents: vec![
                PrincipalRef::new("did:wf:parent-a"),
                PrincipalRef::new("did:wf:parent-b"),
            ],
        };
        let rec = dyad
            .give_birth(
                vec![BiometricClass::Dna],
                vec![CredentialRef {
                    vc_id: "vc:surrogacy-order-1".into(),
                    kind: GuardianshipCredential::SurrogacyParentageOrder,
                }],
                vec![
                    PrincipalRef::new("did:wf:parent-a"),
                    PrincipalRef::new("did:wf:parent-b"),
                ],
                Some(PrincipalRef::new("did:wf:registry")),
                Some("2026-07-06".into()),
            )
            .unwrap();
        assert_eq!(rec.validate(), Ok(()));
        assert!(rec.guardianship.is_credentialed());
        assert!(
            rec.guardianship
                .stewards
                .iter()
                .all(|s| s.basis == StewardBasis::Credentialed)
        );
        let guardian_dids: Vec<&str> = rec.guardians().iter().map(|g| g.did.as_str()).collect();
        assert_eq!(guardian_dids, vec!["did:wf:parent-a", "did:wf:parent-b"]);
        assert!(!guardian_dids.contains(&"did:wf:surrogate"));
    }

    #[test]
    fn agency_is_a_gradient_not_a_binary_and_stewardship_yields_over_development() {
        // Self-determination increases monotonically across the developmental gradient.
        assert!(
            AgencyStage::Neonate.self_determination() < AgencyStage::Child.self_determination()
        );
        assert!(
            AgencyStage::Child.self_determination() < AgencyStage::Adolescent.self_determination()
        );
        assert!(
            AgencyStage::Adolescent.self_determination() < AgencyStage::Adult.self_determination()
        );
        // A supported adult has FULL self-determination (with supports), not a reduced status.
        assert_eq!(
            AgencyStage::SupportedAdult.self_determination(),
            AgencyStage::Adult.self_determination()
        );
        // Stewardship is active through development; an independent adult carries none; a supported adult
        // carries supports (stewardship in service of the person's own will), so it stays "active".
        assert!(AgencyStage::Neonate.stewardship_active());
        assert!(!AgencyStage::Adult.stewardship_active());
        assert!(AgencyStage::SupportedAdult.stewardship_active());
    }

    #[test]
    fn roles_can_be_split_across_different_stewards() {
        // The medical decision-maker need not be the day-to-day carer — the commons holds distributed roles.
        let g = Guardianship {
            stewards: vec![
                Steward {
                    principal: PrincipalRef::new("did:wf:carer"),
                    roles: vec![StewardRole::PrimaryCare],
                    basis: StewardBasis::BiologicalDefault,
                },
                Steward {
                    principal: PrincipalRef::new("did:wf:medical-guardian"),
                    roles: vec![StewardRole::Medical, StewardRole::Legal],
                    basis: StewardBasis::Credentialed,
                },
            ],
            credentials: vec![],
        };
        let medical = g.stewards_with(&StewardRole::Medical);
        assert_eq!(medical.len(), 1);
        assert_eq!(medical[0].principal.did, "did:wf:medical-guardian");
        assert_eq!(
            g.stewards_with(&StewardRole::PrimaryCare)[0].principal.did,
            "did:wf:carer"
        );
    }

    #[test]
    fn a_child_is_never_left_unstewarded_or_collapsed_into_a_steward() {
        // No known biological parent and no credentialed steward ⇒ no steward ⇒ invalid.
        let mut dyad = dyad_two_known_parents();
        dyad.child.parentage = Parentage {
            ovum_source: Progenitor::Donor,
            sperm_source: Progenitor::Unknown,
            intended_parents: vec![],
        };
        let rec = dyad.give_birth(vec![], vec![], vec![], None, None).unwrap();
        assert_eq!(rec.validate(), Err(BirthRecordInvalid::NoSteward));

        // Subject collapsed into a steward ⇒ invalid.
        let mut rec2 = dyad_two_known_parents()
            .give_birth(vec![], vec![], vec![], None, None)
            .unwrap();
        rec2.subject = PrincipalRef::new("did:wf:mother");
        assert_eq!(rec2.validate(), Err(BirthRecordInvalid::SubjectCollapsed));
    }

    #[test]
    fn birth_requires_a_valid_dyad() {
        let mut dyad = dyad_two_known_parents();
        dyad.maternal.state = PhysiologicalState::Baseline;
        assert_eq!(
            dyad.give_birth(vec![], vec![], vec![], None, None)
                .unwrap_err(),
            DyadInvalid::NotPregnant
        );
    }

    #[test]
    fn serde_round_trips() {
        let rec = dyad_two_known_parents()
            .give_birth(vec![BiometricClass::Dna], vec![], vec![], None, None)
            .unwrap();
        let json = serde_json::to_string(&rec).unwrap();
        let back: DigitalBirthRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(rec, back);
    }
}
