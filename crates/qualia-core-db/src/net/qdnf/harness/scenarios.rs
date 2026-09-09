//! E21.1 S01–S40 matrix. Fixtures are observational; none are qualified.

/// Blueprint acceptance scenario (sensitive-operations S01–S40).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioId {
    S01 = 1,
    S02 = 2,
    S03 = 3,
    S04 = 4,
    S05 = 5,
    S06 = 6,
    S07 = 7,
    S08 = 8,
    S09 = 9,
    S10 = 10,
    S11 = 11,
    S12 = 12,
    S13 = 13,
    S14 = 14,
    S15 = 15,
    S16 = 16,
    S17 = 17,
    S18 = 18,
    S19 = 19,
    S20 = 20,
    S21 = 21,
    S22 = 22,
    S23 = 23,
    S24 = 24,
    S25 = 25,
    S26 = 26,
    S27 = 27,
    S28 = 28,
    S29 = 29,
    S30 = 30,
    S31 = 31,
    S32 = 32,
    S33 = 33,
    S34 = 34,
    S35 = 35,
    S36 = 36,
    S37 = 37,
    S38 = 38,
    S39 = 39,
    S40 = 40,
}

/// One matrix row. `qualified` stays false until E21 release evidence exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScenarioStatus {
    pub implemented_fixture: bool,
    pub qualified: bool,
}

const fn row(fixture: bool) -> ScenarioStatus {
    ScenarioStatus {
        implemented_fixture: fixture,
        qualified: false,
    }
}

/// Fixture flags cite profiles / clinical / economics owners only.
/// Qualified is false for every row.
const TABLE: [ScenarioStatus; 40] = [
    row(false), // S01 missing label
    row(false), // S02 unsigned confidentiality mutation
    row(false), // S03 inherited restrictions
    row(false), // S04 disjoint audience join
    row(false), // S05 17-in-16 compartments
    row(false), // S06 stale consent cache
    row(true),  // S07 clinical mailbox revoke-before-deliver
    row(true),  // S08 clinical Blocked contact (not-blocked is not a grant)
    row(false), // S09 ordinary bytes as ciphertext
    row(true),  // S10 clinical mailbox key-generation change
    row(false), // S11 child confidential help / guardian
    row(true),  // S12 profiles P3 required / only P1
    row(true),  // S13 profiles mandatory cover
    row(true),  // S14 profiles observer / cover class
    row(false), // S15 worker compartment reuse
    row(false), // S16 restricted agent egress
    row(false), // S17 embedding index
    row(false), // S18 biometric without authority
    row(false), // S19 template cross-domain
    row(false), // S20 biometric alternative
    row(true),  // S21 profiles offline clock-rollback / expiry
    row(false), // S22 process kill before receipt
    row(false), // S23 overlapping preservation holds
    row(true),  // S24 economics payment cannot enlarge consent
    row(false), // S25 gateway missing label mapping (no exact fixture)
    row(false), // S26 independence / same person
    row(false), // S27 public error membership
    row(true),  // S28 clinical stale mailbox generation
    row(false), // S29 restored backup keys
    row(false), // S30 independent evidence verifier
    row(false), // S31 labelled FHIR export
    row(false), // S32 user-edited marking
    row(false), // S33 single redacted release
    row(false), // S34 worker crash biometric/GPU
    row(false), // S35 emergency all-routes-unavailable
    row(false), // S36 combined A/B authority
    row(false), // S37 omitted dependency list
    row(true),  // S38 clinical revoke before consume
    row(false), // S39 revoke after one committed chunk
    row(false), // S40 biometric threshold missing
];

impl ScenarioId {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 40;
    pub const COUNT: usize = 40;

    pub const fn from_u8(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::S01),
            2 => Some(Self::S02),
            3 => Some(Self::S03),
            4 => Some(Self::S04),
            5 => Some(Self::S05),
            6 => Some(Self::S06),
            7 => Some(Self::S07),
            8 => Some(Self::S08),
            9 => Some(Self::S09),
            10 => Some(Self::S10),
            11 => Some(Self::S11),
            12 => Some(Self::S12),
            13 => Some(Self::S13),
            14 => Some(Self::S14),
            15 => Some(Self::S15),
            16 => Some(Self::S16),
            17 => Some(Self::S17),
            18 => Some(Self::S18),
            19 => Some(Self::S19),
            20 => Some(Self::S20),
            21 => Some(Self::S21),
            22 => Some(Self::S22),
            23 => Some(Self::S23),
            24 => Some(Self::S24),
            25 => Some(Self::S25),
            26 => Some(Self::S26),
            27 => Some(Self::S27),
            28 => Some(Self::S28),
            29 => Some(Self::S29),
            30 => Some(Self::S30),
            31 => Some(Self::S31),
            32 => Some(Self::S32),
            33 => Some(Self::S33),
            34 => Some(Self::S34),
            35 => Some(Self::S35),
            36 => Some(Self::S36),
            37 => Some(Self::S37),
            38 => Some(Self::S38),
            39 => Some(Self::S39),
            40 => Some(Self::S40),
            _ => None,
        }
    }

    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

pub fn scenario_status(id: ScenarioId) -> ScenarioStatus {
    TABLE[(id as u8 as usize) - 1]
}

pub fn scenario_table() -> &'static [ScenarioStatus; 40] {
    &TABLE
}

/// Count of rows with an implemented (unqualified) fixture.
pub fn implemented_fixture_count() -> usize {
    let mut n = 0usize;
    let mut i = 0usize;
    while i < TABLE.len() {
        if TABLE[i].implemented_fixture {
            n = n.saturating_add(1);
        }
        i = i.saturating_add(1);
    }
    n
}

/// Always zero. Qualification is not inferred from fixtures or test counts.
pub fn qualified_count() -> usize {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_covers_s01_to_s40_and_none_are_qualified() {
        assert_eq!(ScenarioId::COUNT, 40);
        assert_eq!(TABLE.len(), 40);
        let mut i = 1u8;
        while i <= ScenarioId::MAX {
            let id = ScenarioId::from_u8(i).expect("id");
            assert_eq!(id.as_u8(), i);
            let st = scenario_status(id);
            assert!(!st.qualified);
            assert_eq!(st, TABLE[(i as usize) - 1]);
            i = i.saturating_add(1);
        }
        assert!(ScenarioId::from_u8(0).is_none());
        assert!(ScenarioId::from_u8(41).is_none());
        assert_eq!(qualified_count(), 0);
        assert_eq!(implemented_fixture_count(), 10);
        assert!(scenario_status(ScenarioId::S07).implemented_fixture);
        assert!(scenario_status(ScenarioId::S08).implemented_fixture);
        assert!(scenario_status(ScenarioId::S10).implemented_fixture);
        assert!(scenario_status(ScenarioId::S12).implemented_fixture);
        assert!(scenario_status(ScenarioId::S13).implemented_fixture);
        assert!(scenario_status(ScenarioId::S14).implemented_fixture);
        assert!(scenario_status(ScenarioId::S21).implemented_fixture);
        assert!(scenario_status(ScenarioId::S24).implemented_fixture);
        assert!(scenario_status(ScenarioId::S28).implemented_fixture);
        assert!(scenario_status(ScenarioId::S38).implemented_fixture);
        assert!(!scenario_status(ScenarioId::S01).implemented_fixture);
        assert!(!scenario_status(ScenarioId::S25).implemented_fixture);
        assert!(!scenario_status(ScenarioId::S40).implemented_fixture);
    }
}
