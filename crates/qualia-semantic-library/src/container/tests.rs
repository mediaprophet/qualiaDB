use super::*;

fn sample_source() -> (SourceInfo, Vec<u8>) {
    let bytes = b"%PDF-1.7 fake source bytes for testing".to_vec();
    let info = SourceInfo {
        filename: "Some Paper (2023).pdf".to_string(),
        mime: "application/pdf".to_string(),
        size_bytes: 0, // recomputed by the writer
        blake3: String::new(),
        title: "Some Paper".to_string(),
        page_count: 12,
    };
    (info, bytes)
}

#[test]
fn round_trips_manifest_and_assets() {
    let (info, src) = sample_source();
    let mut w = HmcWriter::new(info, &src);
    w.add_derived(AssetKind::Html, "document.html", "text/html", b"<html>hi</html>".to_vec());
    w.add_derived(AssetKind::Text, "document.txt", "text/plain", b"[[page 1]]\nhi".to_vec());
    w.manifest_mut().status.extracted = true;
    w.manifest_mut().tags.push("test".to_string());

    let bytes = w.to_bytes().unwrap();
    let mut c = HmcContainer::from_bytes(bytes).unwrap();

    // doc_id == source hash
    assert_eq!(c.manifest().doc_id, blake3_hex(&src));
    assert_eq!(c.manifest().source.size_bytes, src.len() as u64);
    assert!(c.manifest().status.extracted);
    assert_eq!(c.manifest().tags, vec!["test".to_string()]);

    // source survives verbatim
    let back = c.read_kind(AssetKind::Source).unwrap();
    assert_eq!(back, src);

    // derived html readable
    let html = c.read_kind(AssetKind::Html).unwrap();
    assert_eq!(html, b"<html>hi</html>");

    // integrity holds
    c.verify().unwrap();
}

#[test]
fn add_derived_is_idempotent() {
    let (info, src) = sample_source();
    let mut w = HmcWriter::new(info, &src);
    w.add_derived(AssetKind::Html, "document.html", "text/html", b"v1".to_vec());
    w.add_derived(AssetKind::Html, "document.html", "text/html", b"v2".to_vec());

    // only one html asset, latest wins
    let html_assets: Vec<_> = w
        .manifest()
        .assets
        .iter()
        .filter(|a| a.kind == AssetKind::Html)
        .collect();
    assert_eq!(html_assets.len(), 1);

    let mut c = HmcContainer::from_bytes(w.to_bytes().unwrap()).unwrap();
    assert_eq!(c.read_kind(AssetKind::Html).unwrap(), b"v2");
}

#[test]
fn filename_is_sanitized_no_traversal() {
    let mut info = sample_source().0;
    info.filename = "../../etc/evil.pdf".to_string();
    let w = HmcWriter::new(info, b"x");
    let src_asset = w.manifest().asset_of(AssetKind::Source).unwrap();
    assert_eq!(src_asset.path, "source/evil.pdf");
}

#[test]
fn rejects_non_hmc_archive() {
    // A bare zip with no manifest must fail cleanly.
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut buf);
        zw.start_file("foo.txt", zip::write::SimpleFileOptions::default()).unwrap();
        use std::io::Write;
        zw.write_all(b"hi").unwrap();
        zw.finish().unwrap();
    }
    match HmcContainer::from_bytes(buf.into_inner()) {
        Err(HmcError::MissingManifest) => {}
        Ok(_) => panic!("expected MissingManifest, got a valid container"),
        Err(e) => panic!("expected MissingManifest, got {e:?}"),
    }
}
