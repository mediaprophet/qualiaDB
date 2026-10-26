//! Known native dependency refusal for explicit resource edits. Unknown schemas
//! remain inert and unassessed; this is not full embodiment-completeness proof.
use super::{codec, ResourceData};
use crate::container_10d::{
    material_section::{decode_material_section_into, material_section_counts},
    mesh_section::parse_mesh_header,
    section::{parse_section_table, SectionDescriptor, SectionType},
    Container10dHeader, MaterialRecord, SubmeshRange,
};

fn payload<'a>(bytes: &'a [u8], descriptor: &SectionDescriptor) -> Result<&'a [u8], String> {
    let start =
        usize::try_from(descriptor.byte_offset).map_err(|_| "native section offset overflow")?;
    let length =
        usize::try_from(descriptor.byte_length).map_err(|_| "native section length overflow")?;
    bytes
        .get(
            start
                ..start
                    .checked_add(length)
                    .ok_or("native section end overflow")?,
        )
        .ok_or_else(|| "native section exceeds resource bytes".into())
}

pub(super) fn validate(
    before: &[ResourceData],
    after: &[ResourceData],
    guarded_replacements: &std::collections::BTreeSet<String>,
    budget: usize,
) -> Result<(), String> {
    // These pairs are emitted together by the native compiled-asset checkpoint
    // writer. Removing one requires explicit removal of its companion; changing
    // compiled geometry requires an explicit old-digest-guarded graph edit in
    // the same plan. Its valid payload may remain identical: this intent guard
    // does not pretend to prove semantic geometry/graph agreement.
    for old in before.iter().filter(|entry| entry.kind == "10d") {
        let graph_key = super::graph::key(&old.key);
        if before
            .iter()
            .any(|entry| entry.key == graph_key && entry.kind == "q42")
        {
            let geometry = after.iter().find(|entry| entry.key == old.key);
            let graph = after.iter().find(|entry| entry.key == graph_key);
            if geometry.is_some() != graph.is_some() {
                return Err(format!("native geometry/graph dependency invalidation: {} and {graph_key} must remain together or be explicitly removed together", old.key));
            }
            if let (Some(geometry), Some(graph)) = (geometry, graph) {
                if geometry.kind != "10d"
                    || graph.kind != "q42"
                    || (geometry.bytes != old.bytes && !guarded_replacements.contains(&graph_key))
                {
                    return Err("native compiled geometry replacement requires an explicit guarded companion graph edit in the same plan; existing package preserved".into());
                }
            }
        }
    }
    for resource in after {
        if resource.key.starts_with("textures/sha256/") {
            let actual =
                crate::render::asset_package::texture_hmc_key(&codec::digest(&resource.bytes));
            if resource.key != actual {
                return Err("content-addressed texture key differs from payload digest; use an explicit new digest/key and update references".into());
            }
        }
        if resource.kind != "10d" {
            continue;
        }
        let bytes = &resource.bytes;
        let header = Container10dHeader::parse(bytes)
            .map_err(|e| format!("native resource dependency header: {e:?}"))?;
        if crate::container_10d::compute_whole_file_crc32c(bytes) != header.header_crc32c {
            return Err("native resource dependency CRC failed".into());
        }
        let sections = parse_section_table(bytes, &header)
            .map_err(|e| format!("native dependency sections: {e:?}"))?;
        let Some(materials) = sections
            .iter()
            .find(|entry| entry.typ() == Some(SectionType::Materials))
        else {
            continue;
        };
        let mesh = sections
            .iter()
            .find(|entry| entry.typ() == Some(SectionType::QuantizedMesh))
            .ok_or("material dependency has no native mesh")?;
        let (mesh, _) = parse_mesh_header(payload(bytes, mesh)?)
            .map_err(|e| format!("native dependency mesh: {e:?}"))?;
        let index_count = mesh
            .triangle_count
            .checked_mul(3)
            .ok_or("native mesh index count overflow")?;
        let material_bytes = payload(bytes, materials)?;
        let (count, range_count) = material_section_counts(material_bytes)
            .map_err(|e| format!("native dependency material counts: {e:?}"))?;
        let charge = count
            .checked_mul(std::mem::size_of::<MaterialRecord>())
            .and_then(|n| {
                range_count
                    .checked_mul(std::mem::size_of::<SubmeshRange>())
                    .and_then(|m| n.checked_add(m))
            })
            .ok_or("native dependency scratch overflow")?;
        if charge > budget {
            return Err("native dependency scratch exceeds caller cold-work budget".into());
        }
        let mut records = vec![MaterialRecord::legacy_default(); count];
        let mut ranges = vec![
            SubmeshRange {
                first_index: 0,
                index_count: 0,
                material_id: 0,
                semantic_id: 0
            };
            range_count
        ];
        decode_material_section_into(material_bytes, index_count, &mut records, &mut ranges)
            .map_err(|e| format!("native dependency materials: {e:?}"))?;
        for record in records {
            for digest in [
                record.base_color_texture,
                record.normal_texture,
                record.metallic_roughness_texture,
                record.occlusion_texture,
                record.emissive_texture,
                record.stylized_ramp_texture,
            ] {
                if digest == [0; 32] {
                    continue;
                }
                let key = crate::render::asset_package::texture_hmc_key(&digest);
                let texture = after.iter().find(|entry| entry.key == key).ok_or_else(|| {
                    format!("native material dependency missing: {key}; package edit refused")
                })?;
                if codec::digest(&texture.bytes) != digest {
                    return Err(
                        "native material dependency digest mismatch; package edit refused".into(),
                    );
                }
            }
        }
    }
    Ok(())
}
