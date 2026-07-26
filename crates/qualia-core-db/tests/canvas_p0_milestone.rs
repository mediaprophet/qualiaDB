//! P0 milestone: offline world config + sealed `.10d` with provenance passes validate-before-render.

use qualia_core_db::container_10d::header::Container10dHeader;
use qualia_core_db::container_10d::provenance_section::{
    decode_provenance_section, validate_provenance,
};
use qualia_core_db::container_10d::section::{parse_section_table, SectionType};
use qualia_core_db::indexing::QuinIndex;
use qualia_core_db::modalities::logic::geometry_asset_shacl::{
    GeometryAssetConfiguration, GeometryManifestFacts,
};
use qualia_core_db::render::barrier::validate_before_render;
use qualia_core_db::render::compile_10d::compiled_digest;
use qualia_core_db::render::derivation::run_derivation_job;

const TRI_OBJ: &[u8] = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

#[test]
fn p0_offline_tile_passes_validate_before_render() {
    let container = run_derivation_job(TRI_OBJ, Some("obj"), "model/obj", "CC0", None).unwrap();
    let crc = compiled_digest(&container);

    let header = Container10dHeader::parse(&container).unwrap();
    let descs = parse_section_table(&container, &header).unwrap();
    let prov = descs
        .iter()
        .find(|d| d.section_type == SectionType::ProvenanceSidecar as u8)
        .expect("provenance sidecar");
    let payload = &container[prov.byte_offset as usize..][..prov.byte_length as usize];
    let view = decode_provenance_section(payload).unwrap();
    validate_provenance(&view).unwrap();

    let manifest = GeometryManifestFacts {
        vertex_count: 3,
        triangle_count: 1,
        source_format: "obj",
        unit: "metre",
        bbox_min: [0.0, 0.0, 0.0],
        bbox_max: [1.0, 1.0, 0.0],
        max_triangle_index: Some(2),
        claimed_compiled_digest: crc,
        actual_container_crc32c: crc,
        input_sensitivities: &[],
        declared_sensitivity: Some("Public"),
        licence: "CC0",
        creator: None,
        valid_from: None,
        valid_until: None,
    };
    let cfg = GeometryAssetConfiguration::default();
    let index = QuinIndex::from_slice(&[]);

    let mesh = validate_before_render(&container, &manifest, &cfg, &index, u32::MAX, |_| None);
    assert!(
        mesh.is_ok(),
        "barrier should accept CC0 provenance tile: {:?}",
        mesh.err()
    );
}
