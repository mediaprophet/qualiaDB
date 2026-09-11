//! E21.1 S01–S40 observational fixtures. None are qualified.

mod biometrics;
mod clinical;
mod common;
mod custody;
mod labels;
mod profiles;
mod rest;
mod run;
mod table;

pub use run::run_fixture;
pub use table::{
    implemented_fixture_count, qualified_count, scenario_status, scenario_table, ScenarioId,
    ScenarioStatus,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_covers_s01_to_s40_and_none_are_qualified() {
        assert_eq!(ScenarioId::COUNT, 40);
        assert_eq!(scenario_table().len(), 40);
        let mut i = 1u8;
        while i <= ScenarioId::MAX {
            let id = ScenarioId::from_u8(i).expect("id");
            assert_eq!(id.as_u8(), i);
            let st = scenario_status(id);
            assert!(!st.qualified);
            assert!(st.implemented_fixture);
            i = i.saturating_add(1);
        }
        assert!(ScenarioId::from_u8(0).is_none());
        assert!(ScenarioId::from_u8(41).is_none());
        assert_eq!(qualified_count(), 0);
        assert_eq!(implemented_fixture_count(), 40);
    }

    #[test]
    fn every_fixture_runs_against_real_libraries() {
        let mut i = 1u8;
        while i <= ScenarioId::MAX {
            let id = ScenarioId::from_u8(i).expect("id");
            run_fixture(id).unwrap_or_else(|e| panic!("S{i:02}: {e:?}"));
            i = i.saturating_add(1);
        }
        assert_eq!(qualified_count(), 0);
    }
}
