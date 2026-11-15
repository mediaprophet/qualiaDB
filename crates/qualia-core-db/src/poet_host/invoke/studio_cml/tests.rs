use super::super::StudioArtifact;
use super::*;

#[test]
fn compact_xml_preserves_explicit_style_and_tracks_the_current_node() {
    let source = r##"<q-artifact domain="Vector" did="did:q42:foreign" clock="999" title="Logo"><q-prop key="stroke" value="#000"/><q-prop key="fill" value="#000"/><q-node id="z"><q-attr key="d" value="first"/></q-node><q-node id="a"><q-attr key="d" value="second"/></q-node></q-artifact>"##;
    let mut art = StudioArtifact::new("caller-selected", "Vector", "Before");
    let envelope = (art.id.clone(), art.did_q42, art.lamport_clock);
    import_into(&mut art, source, Default::default()).unwrap();
    assert_eq!(art.properties["stroke"], "#000");
    assert_eq!(art.properties["fill"], "#000");
    assert_eq!(art.nodes["z"]["d"], "first");
    assert_eq!(art.nodes["a"]["d"], "second");
    assert_eq!(art.title, "Logo");
    assert_eq!((art.id, art.did_q42, art.lamport_clock), envelope);
}

#[test]
fn native_cml_roundtrip_preserves_unicode_escapes_whitespace_and_provenance_properties() {
    let mut art = StudioArtifact::new("source-a", "Vector", "A \"quoted\" <tree> & café");
    art.properties.insert(
        "rights/source".into(),
        "unreviewed & unpaid; source <A>".into(),
    );
    art.nodes
        .entry("part & leaf".into())
        .or_default()
        .insert("text".into(), "line 1\nline 2\t\"<>&'\"\r".into());
    let encoded = export(&art, Default::default()).unwrap();
    assert!(encoded.contains("&amp;") && encoded.contains("&#10;") && encoded.contains("&#9;"));
    let mut imported = StudioArtifact::new("source-b", "Vector", "Before");
    import_into(&mut imported, &encoded, Default::default()).unwrap();
    assert_eq!(imported.title, art.title);
    assert_eq!(imported.nodes, art.nodes);
    assert_eq!(imported.properties, art.properties);
    assert_eq!(imported.id, "source-b");
}

#[test]
fn malformed_unknown_duplicate_and_budgeted_xml_never_partially_changes_source() {
    let original = StudioArtifact::new("preserved", "Vector", "Original");
    for source in [
        "<q-artifact><q-node id=\"x\"></q-artifact>",
        "<!DOCTYPE q-artifact SYSTEM \"file:///private\"><q-artifact/>",
        "<q-artifact><unknown/></q-artifact>",
        "<q-artifact><q-prop key=\"x\" value=\"1\"/><q-prop key=\"x\" value=\"2\"/></q-artifact>",
        "<q-artifact><q-attr key=\"x\" value=\"orphan\"/></q-artifact>",
        "<q-artifact domain=\"Foreign\"/>",
    ] {
        let mut candidate = original.clone();
        assert!(
            import_into(&mut candidate, source, Default::default()).is_err(),
            "{source}"
        );
        assert_eq!(candidate, original);
    }
    let mut candidate = original.clone();
    assert!(import_into(
        &mut candidate,
        "<q-artifact/>",
        CmlLimits {
            bytes: 2,
            ..Default::default()
        }
    )
    .is_err());
    assert_eq!(candidate, original);
    assert!(export(
        &original,
        CmlLimits {
            bytes: 2,
            ..Default::default()
        }
    )
    .is_err());
}
