use super::{super::StudioArtifact, CmlLimits};
use quick_xml::escape::escape;
use std::io::Write;

fn attribute(value: &str) -> Result<String, String> {
    if value.chars().any(|ch| !matches!(ch as u32, 9 | 10 | 13 | 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff)) {
        return Err("CML source contains an unsupported XML character".into());
    }
    // Numeric references preserve attribute whitespace across XML 1.0 readers.
    Ok(escape(value)
        .replace('\r', "&#13;")
        .replace('\n', "&#10;")
        .replace('\t', "&#9;"))
}

struct BoundedXml {
    bytes: Vec<u8>,
    maximum: usize,
}
impl Write for BoundedXml {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("CML output byte budget exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Export the existing CML source profile with native XML escaping and bounded
/// caller-selected output; this does not seal an HMC or attest source rights.
pub fn export(artifact: &StudioArtifact, limits: CmlLimits) -> Result<String, String> {
    let mut raw_bytes = artifact.title.len().saturating_add(artifact.domain.len());
    let mut fields = artifact
        .properties
        .len()
        .saturating_add(artifact.nodes.len());
    for (key, value) in &artifact.properties {
        raw_bytes = raw_bytes
            .saturating_add(key.len())
            .saturating_add(value.len());
    }
    for (id, attrs) in &artifact.nodes {
        raw_bytes = raw_bytes.saturating_add(id.len());
        fields = fields.saturating_add(attrs.len());
        for (key, value) in attrs {
            raw_bytes = raw_bytes
                .saturating_add(key.len())
                .saturating_add(value.len());
        }
    }
    if raw_bytes > limits.bytes || fields > limits.fields || limits.depth < 3 {
        return Err("CML source exceeds caller bounds".into());
    }
    let mut out = BoundedXml {
        bytes: Vec::new(),
        maximum: limits.bytes,
    };
    write!(
        &mut out,
        "<q-artifact domain=\"{}\" did=\"did:q42:0x{:016x}\" clock=\"{}\" title=\"{}\">",
        attribute(&artifact.domain)?,
        artifact.did_q42,
        artifact.lamport_clock,
        attribute(&artifact.title)?
    )
    .map_err(|e| e.to_string())?;
    for (key, value) in &artifact.properties {
        write!(
            &mut out,
            "<q-prop key=\"{}\" value=\"{}\"/>",
            attribute(key)?,
            attribute(value)?
        )
        .map_err(|e| e.to_string())?;
    }
    for (id, attrs) in &artifact.nodes {
        write!(&mut out, "<q-node id=\"{}\">", attribute(id)?).map_err(|e| e.to_string())?;
        for (key, value) in attrs {
            write!(
                &mut out,
                "<q-attr key=\"{}\" value=\"{}\"/>",
                attribute(key)?,
                attribute(value)?
            )
            .map_err(|e| e.to_string())?;
        }
        out.write_all(b"</q-node>").map_err(|e| e.to_string())?;
    }
    out.write_all(b"</q-artifact>").map_err(|e| e.to_string())?;
    String::from_utf8(out.bytes).map_err(|e| e.to_string())
}
