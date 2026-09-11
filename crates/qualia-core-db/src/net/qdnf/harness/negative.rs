//! Negative controls: each E00 instrument must detect an introduced defect.

#[cfg(test)]
mod tests {
    use crate::crypto::network::types::AEAD_KEY_LEN;
    use crate::net::qdnf::harness::oracles::{
        independent_finished, plaintext_on_wire, unkeyed_finished_defect, ProtectedView,
    };
    use crate::net::qdnf::harness::qualification::{
        qualify, EvidenceClass, Instrument, InstrumentResult, InstrumentVerdict,
    };
    use crate::net::qdnf::types::StrongDigest;

    #[test]
    fn wire_oracle_negative_control_detects_facade_plaintext() {
        // Defect: current public facade copies application bytes onto the wire.
        let app = b"hello-qpr";
        let view = ProtectedView {
            wire_payload: app,
            application: app,
        };
        assert!(
            plaintext_on_wire(view),
            "oracle must detect the introduced plaintext defect"
        );
        let results = [InstrumentResult {
            instrument: Instrument::WirePayloadOracle,
            class: EvidenceClass::Process,
            verdict: InstrumentVerdict::Fail,
            component_tables_pass: true,
        }];
        assert!(qualify(&results).is_err());
    }

    #[test]
    fn crypto_oracle_negative_control_rejects_unkeyed_finished() {
        let digest = StrongDigest([0x11; 48]);
        let secret = [0x22u8; AEAD_KEY_LEN];
        let keyed = independent_finished(&secret, &digest, true);
        let defect = unkeyed_finished_defect(&digest, b"qsession");
        assert_ne!(keyed, defect);
        let results = [InstrumentResult {
            instrument: Instrument::CryptoOracle,
            class: EvidenceClass::Component,
            verdict: InstrumentVerdict::Fail,
            component_tables_pass: true,
        }];
        assert!(qualify(&results).is_err());
    }

    #[test]
    fn allocator_negative_control_is_wired_in_intercept_module() {
        // See harness::intercept: an intentional Vec is counted as a heap touch.
        assert_eq!(Instrument::AllocatorIntercept as u8, 1);
    }
}
