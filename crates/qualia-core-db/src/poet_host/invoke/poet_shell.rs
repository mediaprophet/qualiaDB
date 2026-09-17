//! POET shell authoring receipts for Vibe 0.1.
//!
//! These do not mutate the browser DOM. They return a typed receipt the POET
//! HyperCanvas applies as layout. They do not ship canned worlds.

use super::args;
use super::ids;
use vibe::{Diagnostic, Span, Value};

pub fn manifold_create(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let label = args::rec_str(args_v, "label")
        .or_else(|| args::rec_str(args_v, "name"))
        .ok_or_else(|| args::bad(span, "Poet.manifold_create needs label"))?;
    let description = args::rec_str(args_v, "description").unwrap_or("");
    let nest = args::rec_bool(args_v, "nest").unwrap_or(false);
    let social = args::rec_bool(args_v, "social").unwrap_or(false);
    Ok(args::record([
        ("op", Value::String("manifold_create".into())),
        ("id", Value::String(ids::POET_MANIFOLD_CREATE.into())),
        ("label", Value::String(label.into())),
        ("description", Value::String(description.into())),
        ("nest", Value::Bool(nest)),
        ("social", Value::Bool(social)),
        ("shell", Value::String("poet".into())),
    ]))
}

pub fn container_place(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let container_type = args::rec_str(args_v, "container_type")
        .or_else(|| args::rec_str(args_v, "type"))
        .ok_or_else(|| args::bad(span, "Poet.container_place needs container_type"))?;
    let title = args::rec_str(args_v, "title")
        .or_else(|| args::rec_str(args_v, "label"))
        .unwrap_or(container_type);
    Ok(args::record([
        ("op", Value::String("container_place".into())),
        ("id", Value::String(ids::POET_CONTAINER_PLACE.into())),
        ("container_type", Value::String(container_type.into())),
        ("title", Value::String(title.into())),
        ("shell", Value::String("poet".into())),
    ]))
}

pub fn nested_link(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let to = args::rec_str(args_v, "to")
        .or_else(|| args::rec_str(args_v, "target_manifold"))
        .ok_or_else(|| args::bad(span, "Poet.nested_link needs to"))?;
    let title = args::rec_str(args_v, "title").unwrap_or(to);
    Ok(args::record([
        ("op", Value::String("nested_link".into())),
        ("id", Value::String(ids::POET_NESTED_LINK.into())),
        ("to", Value::String(to.into())),
        ("title", Value::String(title.into())),
        ("shell", Value::String("poet".into())),
    ]))
}

pub fn subject_declare(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let label = args::rec_str(args_v, "label")
        .or_else(|| args::rec_str(args_v, "name"))
        .ok_or_else(|| args::bad(span, "Poet.subject_declare needs label"))?;
    let description = args::rec_str(args_v, "description").unwrap_or("");
    Ok(args::record([
        ("op", Value::String("subject_declare".into())),
        ("id", Value::String(ids::POET_SUBJECT_DECLARE.into())),
        ("label", Value::String(label.into())),
        ("description", Value::String(description.into())),
        ("shell", Value::String("poet".into())),
    ]))
}

pub fn participant_invite(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let did = args::rec_str(args_v, "did")
        .or_else(|| args::rec_str(args_v, "participant"))
        .ok_or_else(|| args::bad(span, "Poet.participant_invite needs did"))?;
    let role = args::rec_str(args_v, "role").unwrap_or("member");
    let label = args::rec_str(args_v, "label").unwrap_or(did);
    Ok(args::record([
        ("op", Value::String("participant_invite".into())),
        ("id", Value::String(ids::POET_PARTICIPANT_INVITE.into())),
        ("did", Value::String(did.into())),
        ("role", Value::String(role.into())),
        ("label", Value::String(label.into())),
        ("shell", Value::String("poet".into())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn manifold_create_receipt() {
        let mut m = BTreeMap::new();
        m.insert("label".into(), Value::String("Cellular".into()));
        m.insert("nest".into(), Value::Bool(true));
        let Value::Record(r) =
            manifold_create(&Value::Record(m), Span { start: 0, end: 0 }).unwrap()
        else {
            panic!("record");
        };
        assert_eq!(r.get("op"), Some(&Value::String("manifold_create".into())));
        assert_eq!(r.get("nest"), Some(&Value::Bool(true)));
    }

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    #[test]
    fn container_place_receipt() {
        let Value::Record(r) = container_place(
            &rec(&[("container_type", Value::String("doc".into()))]),
            Span { start: 0, end: 0 },
        )
        .unwrap() else {
            panic!("record");
        };
        assert_eq!(r.get("op"), Some(&Value::String("container_place".into())));
        assert_eq!(r.get("container_type"), Some(&Value::String("doc".into())));
    }

    #[test]
    fn nested_link_needs_target() {
        let err = nested_link(&rec(&[]), Span { start: 0, end: 0 }).unwrap_err();
        assert!(err.message.contains("Poet.nested_link needs to"));
    }

    #[test]
    fn subject_declare_receipt() {
        let Value::Record(r) = subject_declare(
            &rec(&[("label", Value::String("North Spring".into()))]),
            Span { start: 0, end: 0 },
        )
        .unwrap() else {
            panic!("record");
        };
        assert_eq!(r.get("op"), Some(&Value::String("subject_declare".into())));
    }

    #[test]
    fn participant_invite_receipt() {
        let Value::Record(r) = participant_invite(
            &rec(&[("did", Value::String("did:qualia:alice".into()))]),
            Span { start: 0, end: 0 },
        )
        .unwrap() else {
            panic!("record");
        };
        assert_eq!(
            r.get("op"),
            Some(&Value::String("participant_invite".into()))
        );
        assert_eq!(r.get("role"), Some(&Value::String("member".into())));
    }
}
