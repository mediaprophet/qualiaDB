use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Serialization formats supported by the ingest pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticFormat {
    NTriples,
    NTriplesStar,
    NQuads,
    NQuadsStar,
    Turtle,
    TurtleStar,
    TriG,
    TriGStar,
    N3,
    RdfXml,
    JsonLd,
    JsonLdStar,
    CborLd,
    Kml,
    Chk,
    Q42,
}

impl SemanticFormat {
    pub fn label(self) -> &'static str {
        match self {
            SemanticFormat::NTriples     => "N-Triples",
            SemanticFormat::NTriplesStar => "N-Triples-Star",
            SemanticFormat::NQuads       => "N-Quads",
            SemanticFormat::NQuadsStar   => "N-Quads-Star",
            SemanticFormat::Turtle       => "Turtle",
            SemanticFormat::TurtleStar   => "Turtle-Star",
            SemanticFormat::TriG         => "TriG",
            SemanticFormat::TriGStar     => "TriG-Star",
            SemanticFormat::N3           => "N3",
            SemanticFormat::RdfXml       => "RDF/XML",
            SemanticFormat::JsonLd       => "JSON-LD",
            SemanticFormat::JsonLdStar   => "JSON-LD-Star",
            SemanticFormat::CborLd       => "CBOR-LD",
            SemanticFormat::Kml          => "KML",
            SemanticFormat::Chk          => "CHK",
            SemanticFormat::Q42          => "Q42",
        }
    }
}

/// Detect the serialization format of a file by inspecting its extension and
/// the first 16 bytes of content (magic bytes).
///
/// Extension check runs first (O(1)); magic-byte read is a fallback disambiguation
/// step that opens the file for at most 16 bytes.  Returns `None` only when
/// neither heuristic yields a conclusive result.
pub fn detect_format(path: &Path) -> Option<SemanticFormat> {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());

    // Read up to 16 magic bytes without allocating more than that.
    let mut magic = [0u8; 16];
    let magic_len = File::open(path)
        .ok()
        .and_then(|mut f| f.read(&mut magic).ok())
        .unwrap_or(0);
    let magic = &magic[..magic_len];

    // ── Magic-byte checks (format-definitive) ────────────────────────────

    // Q42 binary vault
    if magic.len() >= 3 && &magic[..3] == b"Q42" {
        return Some(SemanticFormat::Q42);
    }
    // QCHK / CHK profile blob
    if magic.len() >= 4 && &magic[..4] == b"QCHK" {
        return Some(SemanticFormat::Chk);
    }
    // CBOR-LD: standard CBOR self-describe tag 0xd9 0xd9 0xf7
    if magic.len() >= 3 && magic[0] == 0xd9 && magic[1] == 0xd9 && magic[2] == 0xf7 {
        return Some(SemanticFormat::CborLd);
    }
    // CBOR definite-length map (0xa0–0xb7) — only commit if extension confirms
    if magic.len() >= 1 && (0xa0..=0xb7).contains(&magic[0]) {
        if matches!(ext.as_deref(), Some("cbor") | Some("cborld")) {
            return Some(SemanticFormat::CborLd);
        }
    }

    // XML envelope — distinguish KML from RDF/XML by extension
    let starts_xml = magic.starts_with(b"<?xml")
        || magic.starts_with(b"<rdf:")
        || magic.starts_with(b"<RDF:");
    if starts_xml {
        return match ext.as_deref() {
            Some("kml") => Some(SemanticFormat::Kml),
            _ => Some(SemanticFormat::RdfXml),
        };
    }

    // JSON envelope: { or [ — use extension to pick LD vs LD-Star
    if magic.first().copied() == Some(b'{') || magic.first().copied() == Some(b'[') {
        return match ext.as_deref() {
            Some("jsonld-star") | Some("json-ld-star") => Some(SemanticFormat::JsonLdStar),
            _ => Some(SemanticFormat::JsonLd),
        };
    }

    // ── Extension fallback ────────────────────────────────────────────────
    match ext.as_deref() {
        Some("nt")                                  => Some(SemanticFormat::NTriples),
        Some("nts") | Some("nt-star")               => Some(SemanticFormat::NTriplesStar),
        Some("nq")                                  => Some(SemanticFormat::NQuads),
        Some("nqs") | Some("nq-star")               => Some(SemanticFormat::NQuadsStar),
        Some("ttl")                                 => Some(SemanticFormat::Turtle),
        Some("ttls") | Some("ttl-star")             => Some(SemanticFormat::TurtleStar),
        Some("trig")                                => Some(SemanticFormat::TriG),
        Some("trigs") | Some("trig-star")           => Some(SemanticFormat::TriGStar),
        Some("n3")                                  => Some(SemanticFormat::N3),
        Some("rdf") | Some("owl")                   => Some(SemanticFormat::RdfXml),
        Some("xml")                                 => Some(SemanticFormat::RdfXml),
        Some("jsonld") | Some("json-ld")
            | Some("json")                          => Some(SemanticFormat::JsonLd),
        Some("cbor") | Some("cborld")               => Some(SemanticFormat::CborLd),
        Some("kml")                                 => Some(SemanticFormat::Kml),
        Some("chk") | Some("qchk")                 => Some(SemanticFormat::Chk),
        Some("q42")                                 => Some(SemanticFormat::Q42),
        _                                           => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp_file(name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        let mut f = std::fs::File::create(&path).expect("create tmp file");
        f.write_all(content).expect("write tmp file");
        path
    }

    #[test]
    fn detect_by_extension_nt() {
        let p = tmp_file("test_detect.nt", b"<s> <p> <o> .\n");
        assert_eq!(detect_format(&p), Some(SemanticFormat::NTriples));
    }

    #[test]
    fn detect_by_extension_ttl() {
        let p = tmp_file("test_detect.ttl", b"@prefix ex: <http://ex.org/> .\n");
        assert_eq!(detect_format(&p), Some(SemanticFormat::Turtle));
    }

    #[test]
    fn detect_by_extension_jsonld() {
        let p = tmp_file("test_detect.jsonld", b"{\"@context\": {}}");
        assert_eq!(detect_format(&p), Some(SemanticFormat::JsonLd));
    }

    #[test]
    fn detect_by_magic_q42() {
        let p = tmp_file("test_detect_magic.bin", b"Q42V\x00\x00\x00\x00");
        assert_eq!(detect_format(&p), Some(SemanticFormat::Q42));
    }

    #[test]
    fn detect_by_magic_xml_rdf() {
        let p = tmp_file("test_detect_magic.rdf", b"<?xml version=\"1.0\"?><rdf:RDF");
        assert_eq!(detect_format(&p), Some(SemanticFormat::RdfXml));
    }

    #[test]
    fn detect_by_magic_kml() {
        let p = tmp_file("test_detect_magic.kml", b"<?xml version=\"1.0\"?><kml>");
        assert_eq!(detect_format(&p), Some(SemanticFormat::Kml));
    }

    #[test]
    fn detect_by_magic_cbor_ld() {
        let p = tmp_file("test_detect_magic.cbor", b"\xd9\xd9\xf7\xa1");
        assert_eq!(detect_format(&p), Some(SemanticFormat::CborLd));
    }

    #[test]
    fn detect_json_content_without_ld_extension() {
        let p = tmp_file("test_detect_json.json", b"{\"@context\": {}, \"@id\": \"x\"}");
        assert_eq!(detect_format(&p), Some(SemanticFormat::JsonLd));
    }

    #[test]
    fn detect_unknown_returns_none() {
        let p = tmp_file("test_detect.xyz", b"some unknown data");
        assert_eq!(detect_format(&p), None);
    }

    #[test]
    fn label_round_trips() {
        assert_eq!(SemanticFormat::NTriples.label(), "N-Triples");
        assert_eq!(SemanticFormat::TurtleStar.label(), "Turtle-Star");
        assert_eq!(SemanticFormat::JsonLdStar.label(), "JSON-LD-Star");
    }
}
