use super::super::PoetSnapshot;
use vibe::{DiagCode, Diagnostic, Host, Span, Value};

/// Borrow an existing native state owner and explicit host-held grants for one
/// dispatch scope. Construct only from native admission, never `requires` text.
/// This small cold authoring profile does not advertise graph, generic invokes,
/// private-key export or unrestricted host delegation. Other Host defaults are
/// unchanged and are not advertised by this profile. The native owner handles authentication,
/// current revocation and keeping this borrow within its admission transaction.
pub struct ScopedEffects<'a> {
    snapshot: &'a mut PoetSnapshot,
    grants: &'a [&'a str],
}

impl<'a> ScopedEffects<'a> {
    pub fn new(snapshot: &'a mut PoetSnapshot, grants: &'a [&'a str]) -> Result<Self, Diagnostic> {
        const SUPPORTED: &[&str] = &[
            "pulse.publish",
            "hid.poll",
            "crypto.generate_key",
            "crypto.sign",
        ];
        if grants.len() > 32
            || grants
                .iter()
                .enumerate()
                .any(|(index, grant)| !SUPPORTED.contains(grant) || grants[..index].contains(grant))
        {
            return Err(denied(Span::point(0)));
        }
        Ok(Self { snapshot, grants })
    }
    fn require(&self, capability: &str, span: Span) -> Result<(), Diagnostic> {
        if self.grants.contains(&capability) {
            Ok(())
        } else {
            Err(denied(span))
        }
    }
}

fn denied(span: Span) -> Diagnostic {
    Diagnostic::new(
        DiagCode::E300,
        span,
        "operation is outside the current native effect grants",
    )
}

impl Host for ScopedEffects<'_> {
    fn graph_query(&mut self, _: &[Value], _: u64, span: Span) -> Result<Value, Diagnostic> {
        Err(denied(span))
    }
    fn graph_stage(&mut self, _: &Value, span: Span) -> Result<Value, Diagnostic> {
        Err(denied(span))
    }
    fn graph_commit(&mut self, span: Span) -> Result<Value, Diagnostic> {
        Err(denied(span))
    }
    fn aura_validate(&mut self, _: &Value, _: &Value, span: Span) -> Result<Value, Diagnostic> {
        Err(denied(span))
    }
    fn pulse_publish(
        &mut self,
        topic: &str,
        payload: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.require("pulse.publish", span)?;
        self.snapshot.pulse_publish(topic, payload, span)
    }
    fn hid_poll(&mut self, span: Span) -> Result<Value, Diagnostic> {
        self.require("hid.poll", span)?;
        self.snapshot.hid_poll(span)
    }
    fn crypto_generate_key(&mut self, algorithm: &str, span: Span) -> Result<Value, Diagnostic> {
        self.require("crypto.generate_key", span)?;
        self.snapshot.crypto_generate_key(algorithm, span)
    }
    fn crypto_sign(&mut self, key_id: &str, data: &str, span: Span) -> Result<Value, Diagnostic> {
        self.require("crypto.sign", span)?;
        self.snapshot.crypto_sign(key_id, data, span)
    }
    // Verification retains the pinned 0.1 Pure classification. It returns only
    // validity and never exports a native key seed or grants signing authority.
    fn crypto_verify(
        &mut self,
        key_id: &str,
        data: &str,
        signature: &str,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        self.snapshot.crypto_verify(key_id, data, signature, span)
    }
}
