//! Routing admission separate from fixed hyperparameter and P64 records.
pub const MAX_ADMITTED_EXPERTS: usize = 512;
pub const MAX_ADMITTED_TOPK: usize = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MoeRoutingSettings {
    pub expert_count: Option<u32>,
    pub experts_per_token: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoeRoutingAdmissionError {
    ZeroExperts,
    ExpertCountConflict,
    ExpertCapacityExceeded,
    ZeroTopK,
    InsufficientExperts,
    TopKCapacityExceeded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedMoeRouting {
    expert_count: usize,
    experts_per_token: usize,
}

impl MoeRoutingSettings {
    pub fn admit(
        self,
        actual_experts: usize,
    ) -> Result<AdmittedMoeRouting, MoeRoutingAdmissionError> {
        use MoeRoutingAdmissionError::*;
        if actual_experts == 0 || self.expert_count == Some(0) {
            return Err(ZeroExperts);
        }
        if self
            .expert_count
            .is_some_and(|n| n as usize != actual_experts)
        {
            return Err(ExpertCountConflict);
        }
        if actual_experts > MAX_ADMITTED_EXPERTS {
            return Err(ExpertCapacityExceeded);
        }
        let k = self
            .experts_per_token
            .map_or(8.min(actual_experts), |n| n as usize);
        if k == 0 {
            return Err(ZeroTopK);
        }
        if k > MAX_ADMITTED_TOPK {
            return Err(TopKCapacityExceeded);
        }
        if k > actual_experts {
            return Err(InsufficientExperts);
        }
        Ok(AdmittedMoeRouting {
            expert_count: actual_experts,
            experts_per_token: k,
        })
    }
}

impl AdmittedMoeRouting {
    pub const fn expert_count(self) -> usize {
        self.expert_count
    }
    pub const fn experts_per_token(self) -> usize {
        self.experts_per_token
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_and_legacy_settings() {
        for k in [6, 8, 10] {
            let c = MoeRoutingSettings {
                expert_count: Some(512),
                experts_per_token: Some(k),
            }
            .admit(512)
            .unwrap();
            assert_eq!(c.expert_count(), 512);
            assert_eq!(c.experts_per_token(), k as usize);
        }
        for n in [1, 6, 256, 512] {
            assert_eq!(
                MoeRoutingSettings::default()
                    .admit(n)
                    .unwrap()
                    .experts_per_token(),
                8.min(n)
            );
        }
    }
    #[test]
    fn refuses_invalid_settings() {
        use MoeRoutingAdmissionError::*;
        let defaults = MoeRoutingSettings::default();
        assert_eq!(defaults.admit(0), Err(ZeroExperts));
        assert_eq!(defaults.admit(513), Err(ExpertCapacityExceeded));
        assert_eq!(
            MoeRoutingSettings {
                expert_count: Some(256),
                ..defaults
            }
            .admit(512),
            Err(ExpertCountConflict)
        );
        assert_eq!(
            MoeRoutingSettings {
                expert_count: Some(0),
                ..defaults
            }
            .admit(512),
            Err(ZeroExperts)
        );
        for (k, n, error) in [
            (0, 512, ZeroTopK),
            (17, 512, TopKCapacityExceeded),
            (10, 6, InsufficientExperts),
        ] {
            assert_eq!(
                MoeRoutingSettings {
                    experts_per_token: Some(k),
                    ..defaults
                }
                .admit(n),
                Err(error)
            );
        }
    }
}
