//! Strict opt-in policy evaluation; legacy policy checkers retain their semantics.
use super::{AuthenticationMethod, KeyAccessControl, KeyOperation};

/// Context supplied by the trusted local host AFTER authenticating the actor.
/// These declarations are not authentication evidence on their own. Never fill
/// them from an untrusted request's claimed methods/IP. No login system is added.
#[derive(Clone, Copy)]
pub struct SigningAccess<'a> {
    pub authenticated_methods: &'a [AuthenticationMethod],
    pub ip_address: &'a str,
}

impl KeyAccessControl {
    pub(crate) fn permits_managed_signing(
        &self,
        policy_id: &str,
        key_id: &str,
        context: SigningAccess<'_>,
        now: u64,
    ) -> bool {
        self.permits_managed_operation(policy_id, key_id, KeyOperation::Sign, context, now)
    }
    pub(crate) fn permits_managed_operation(
        &self,
        policy_id: &str,
        key_id: &str,
        operation: KeyOperation,
        context: SigningAccess<'_>,
        now: u64,
    ) -> bool {
        let Some(policy) = self.access_policies.get(policy_id) else {
            return false;
        };
        let time = &policy.time_restrictions;
        // UTC weekday: Sunday=0, Unix epoch Thursday=4.
        let hour = ((now / 3600) % 24) as u8;
        let day = ((now / 86400 + 4) % 7) as u8;
        policy.key_id == key_id
            && policy.allowed_operations.contains(&operation)
            && policy
                .required_auth
                .iter()
                .all(|m| context.authenticated_methods.contains(m))
            && time.start_date.is_none_or(|start| now >= start)
            && time.end_date.is_none_or(|end| now < end)
            && (time.allowed_hours.is_empty() || time.allowed_hours.contains(&hour))
            && (time.allowed_days.is_empty() || time.allowed_days.contains(&day))
            && (policy.ip_restrictions.is_empty()
                || policy
                    .ip_restrictions
                    .iter()
                    .any(|ip| ip == context.ip_address))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::cryptographic_library::{AccessPolicy, TimeRestrictions};
    #[test]
    fn exact_policy_requires_auth_dates_ip_and_operation() {
        let mut access = KeyAccessControl::new();
        let context = SigningAccess {
            authenticated_methods: &[AuthenticationMethod::HardwareToken],
            ip_address: "local",
        };
        assert!(!access.permits_managed_signing("p", "key", context, 10));
        let policy = AccessPolicy {
            policy_id: "p".into(),
            key_id: "key".into(),
            allowed_operations: vec![KeyOperation::Sign],
            required_auth: vec![AuthenticationMethod::HardwareToken],
            time_restrictions: TimeRestrictions {
                allowed_hours: vec![0],
                allowed_days: vec![4],
                start_date: Some(10),
                end_date: Some(20),
            },
            ip_restrictions: vec!["local".into()],
        };
        access.add_policy(policy.clone());
        assert!(access.permits_managed_signing("p", "key", context, 10));
        assert!(!access.permits_managed_signing("p", "other", context, 10));
        assert!(!access.permits_managed_signing("p", "key", context, 9));
        assert!(!access.permits_managed_signing("p", "key", context, 20));
        assert!(!access.permits_managed_signing(
            "p",
            "key",
            SigningAccess {
                authenticated_methods: &[],
                ..context
            },
            10
        ));
        assert!(!access.permits_managed_signing(
            "p",
            "key",
            SigningAccess {
                ip_address: "remote",
                ..context
            },
            10
        ));
        let mut denied = policy;
        denied.allowed_operations = vec![KeyOperation::Read];
        access.add_policy(denied);
        assert!(!access.permits_managed_signing("p", "key", context, 10));
    }
}
