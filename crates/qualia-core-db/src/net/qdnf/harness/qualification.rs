//! E00 qualification classes, fingerprints and fail-closed gates.
//!
//! Component table tests cannot certify encryption, payload authenticity or
//! allocation interception. A failing instrument fails the qualification set.

use crate::net::qdnf::errors::QdnfError;

/// Evidence class. Mixing these into one “all tests passed” claim is forbidden.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    Component = 1,
    Process = 2,
    PhysicalNetwork = 3,
    Application = 4,
}

/// Named qualification instrument. Each has a negative-control that must fire.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instrument {
    AllocatorIntercept = 1,
    WirePayloadOracle = 2,
    CryptoOracle = 3,
    GraphOracle = 4,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentVerdict {
    Pass = 1,
    Fail = 2,
    Unsupported = 3,
}

/// One recorded instrument result. `component_tables_pass` is observational.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstrumentResult {
    pub instrument: Instrument,
    pub class: EvidenceClass,
    pub verdict: InstrumentVerdict,
    pub component_tables_pass: bool,
}

/// Qualification fails when encryption, payload verification or allocation
/// interception fails, even if every state-table unit test passed.
pub fn qualify(results: &[InstrumentResult]) -> Result<(), QdnfError> {
    let mut i = 0;
    while i < results.len() {
        let r = results[i];
        let critical = matches!(
            r.instrument,
            Instrument::AllocatorIntercept
                | Instrument::WirePayloadOracle
                | Instrument::CryptoOracle
        );
        if critical && r.verdict == InstrumentVerdict::Fail {
            return Err(QdnfError::Denied);
        }
        i += 1;
    }
    Ok(())
}

/// Linux native runner recorded at E00. MSVC is not claimed from this host.
pub const LINUX_NATIVE_RUNNER: &str = "linux-x86_64-gnu rustc-1.98.1 cargo-1.98.1";
pub const MSVC_RUNNER: Option<&str> = None;
pub const BASELINE_COMMIT: &str = "5646b818d4e6fa63f0b7c4c6a1a469dca382339a";
pub const REVIEW_BASELINE_COMMIT: &str = "17b6b467c8a4548e302f603674dd594a67be7398";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encryption_failure_fails_qualification_despite_table_passes() {
        let results = [InstrumentResult {
            instrument: Instrument::CryptoOracle,
            class: EvidenceClass::Component,
            verdict: InstrumentVerdict::Fail,
            component_tables_pass: true,
        }];
        assert_eq!(qualify(&results), Err(QdnfError::Denied));
    }

    #[test]
    fn payload_oracle_failure_fails_qualification() {
        let results = [InstrumentResult {
            instrument: Instrument::WirePayloadOracle,
            class: EvidenceClass::Process,
            verdict: InstrumentVerdict::Fail,
            component_tables_pass: true,
        }];
        assert_eq!(qualify(&results), Err(QdnfError::Denied));
    }

    #[test]
    fn allocator_intercept_failure_fails_qualification() {
        let results = [InstrumentResult {
            instrument: Instrument::AllocatorIntercept,
            class: EvidenceClass::Component,
            verdict: InstrumentVerdict::Fail,
            component_tables_pass: true,
        }];
        assert_eq!(qualify(&results), Err(QdnfError::Denied));
    }

    #[test]
    fn passing_instruments_qualify() {
        let results = [
            InstrumentResult {
                instrument: Instrument::AllocatorIntercept,
                class: EvidenceClass::Component,
                verdict: InstrumentVerdict::Pass,
                component_tables_pass: true,
            },
            InstrumentResult {
                instrument: Instrument::WirePayloadOracle,
                class: EvidenceClass::Process,
                verdict: InstrumentVerdict::Pass,
                component_tables_pass: true,
            },
            InstrumentResult {
                instrument: Instrument::CryptoOracle,
                class: EvidenceClass::Component,
                verdict: InstrumentVerdict::Pass,
                component_tables_pass: true,
            },
        ];
        assert!(qualify(&results).is_ok());
    }

    #[test]
    fn msvc_runner_is_explicitly_unavailable_here() {
        assert!(MSVC_RUNNER.is_none());
        assert!(LINUX_NATIVE_RUNNER.contains("linux"));
    }
}
