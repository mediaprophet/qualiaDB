use super::{super::StudioArtifact, CmlLimits};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use std::collections::BTreeMap;

fn attributes(
    event: &BytesStart<'_>,
    reader: &Reader<&[u8]>,
) -> Result<BTreeMap<String, String>, String> {
    let mut result = BTreeMap::new();
    for attr in event.attributes() {
        if result.len() >= 4 {
            return Err("CML XML attribute count exceeds profile bounds".into());
        }
        let attr = attr.map_err(|e| format!("CML attribute: {e}"))?;
        let key = std::str::from_utf8(attr.key.as_ref())
            .map_err(|e| e.to_string())?
            .to_owned();
        let value = attr
            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
            .map_err(|e| e.to_string())?
            .into_owned();
        if result.insert(key, value).is_some() {
            return Err("duplicate CML XML attribute".into());
        }
    }
    Ok(result)
}
fn required(attrs: &BTreeMap<String, String>, key: &str) -> Result<String, String> {
    attrs
        .get(key)
        .cloned()
        .ok_or_else(|| format!("CML lacks '{key}'"))
}
fn admitted_attributes(attrs: &BTreeMap<String, String>, allowed: &[&str]) -> Result<(), String> {
    if attrs.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err("unsupported CML XML attribute; source was not imported".into());
    }
    Ok(())
}

/// Parse a detached candidate completely before changing the supplied source.
/// The caller chooses local record identifiers/domain. File did/clock fields
/// never replace that authority/revision envelope; the editable title and all
/// property/node/attribute values are preserved, including XML escapes.
pub fn import_into(
    artifact: &mut StudioArtifact,
    payload: &str,
    limits: CmlLimits,
) -> Result<(), String> {
    if payload.len() > limits.bytes || limits.fields == 0 || limits.depth == 0 {
        return Err("CML input exceeds caller bounds".into());
    }
    let mut reader = Reader::from_str(payload);
    let mut depth = 0usize;
    let mut fields = 0usize;
    let mut root_seen = false;
    let mut root_closed = false;
    let mut current_node: Option<String> = None;
    let mut title = artifact.title.clone();
    let mut properties = BTreeMap::new();
    let mut nodes: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    loop {
        match reader.read_event().map_err(|e| format!("CML XML: {e}"))? {
            event @ (Event::Start(_) | Event::Empty(_)) => {
                let empty = matches!(event, Event::Empty(_));
                let start = match event { Event::Start(start) | Event::Empty(start) => start, _ => unreachable!() };
                let next_depth = depth.checked_add(1).filter(|depth| *depth <= limits.depth).ok_or("CML depth budget exceeded")?;
                let attrs = attributes(&start, &reader)?;
                match start.name().as_ref() {
                    b"q-artifact" if depth == 0 && !root_seen && !root_closed => {
                        admitted_attributes(&attrs, &["domain", "did", "clock", "title"])?;
                        if attrs.get("domain").is_some_and(|domain| domain != &artifact.domain) { return Err("CML domain differs from native caller binding".into()); }
                        if let Some(value) = attrs.get("title") { title = value.clone(); }
                        root_seen = true; root_closed = empty;
                    }
                    b"q-prop" if depth == 1 && root_seen && !root_closed => {
                        admitted_attributes(&attrs, &["key", "value"])?;
                        if properties.insert(required(&attrs, "key")?, required(&attrs, "value")?).is_some() { return Err("duplicate CML source property".into()); }
                        fields = fields.saturating_add(1);
                    }
                    b"q-node" if depth == 1 && root_seen && !root_closed => {
                        admitted_attributes(&attrs, &["id"])?;
                        let id = required(&attrs, "id")?;
                        if id.is_empty() || nodes.insert(id.clone(), BTreeMap::new()).is_some() { return Err("empty or duplicate CML node identifier".into()); }
                        if !empty { current_node = Some(id); }
                        fields = fields.saturating_add(1);
                    }
                    b"q-attr" if depth == 2 && current_node.is_some() => {
                        admitted_attributes(&attrs, &["key", "value"])?;
                        let target = nodes.get_mut(current_node.as_ref().ok_or("CML node absent")?).ok_or("CML node absent")?;
                        if target.insert(required(&attrs, "key")?, required(&attrs, "value")?).is_some() { return Err("duplicate CML node attribute".into()); }
                        fields = fields.saturating_add(1);
                    }
                    _ => return Err("unsupported or misplaced CML element; source was not imported".into()),
                }
                if fields > limits.fields { return Err("CML field budget exceeded".into()); }
                if !empty { depth = next_depth; }
            }
            Event::End(end) => {
                if depth == 0 { return Err("unmatched CML closing element".into()); }
                match end.name().as_ref() { b"q-node" => current_node = None, b"q-artifact" => root_closed = true, _ => {} }
                depth -= 1;
            }
            Event::Text(text) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Event::Comment(_) => {}
            Event::Decl(_) if !root_seen => {}
            Event::Eof => break,
            _ => return Err("CML supports source attributes only; DTD, external entities and text bodies are refused".into()),
        }
    }
    if !root_seen || !root_closed || depth != 0 {
        return Err("incomplete CML document".into());
    }
    artifact.title = title;
    artifact.properties = properties;
    artifact.nodes = nodes;
    Ok(())
}
