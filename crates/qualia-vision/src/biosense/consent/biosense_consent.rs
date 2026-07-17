//! Purpose-bound consent for biosense (fail closed).

use super::purpose::BiosensePurpose;

#[derive(Debug, Clone, Copy)]
pub struct BiosenseConsent {
    pub purpose: BiosensePurpose,
    pub allow_process: bool,
    pub allow_store_template: bool,
    pub allow_graph_observation: bool,
    pub principal_hash: u64,
}

impl BiosenseConsent {
    pub fn denied(purpose: BiosensePurpose) -> Self {
        Self {
            purpose,
            allow_process: false,
            allow_store_template: false,
            allow_graph_observation: false,
            principal_hash: 0,
        }
    }

    pub fn grant_process(purpose: BiosensePurpose, principal_hash: u64) -> Self {
        Self {
            purpose,
            allow_process: true,
            allow_store_template: false,
            allow_graph_observation: true,
            principal_hash,
        }
    }

    pub fn grant_security_template(principal_hash: u64) -> Self {
        Self {
            purpose: BiosensePurpose::Security,
            allow_process: true,
            allow_store_template: true,
            allow_graph_observation: true,
            principal_hash,
        }
    }

    pub fn revoke(&mut self) {
        self.allow_process = false;
        self.allow_store_template = false;
        self.allow_graph_observation = false;
    }

    pub fn may_process(self) -> bool {
        self.allow_process
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fail_closed() {
        assert!(!BiosenseConsent::denied(BiosensePurpose::Research).may_process());
        assert!(BiosenseConsent::grant_process(BiosensePurpose::Research, 1).may_process());
    }
}
