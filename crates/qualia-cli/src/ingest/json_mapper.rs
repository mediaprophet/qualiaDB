use qualia_core_db::sparql_library::parsers::json_parser::{
    parse_json_to_quins, JsonDatatype, JsonFieldMapping, JsonMappingProfile,
};
use std::fs::File;
use std::io::BufReader;

pub fn stream_json_to_quins(
    json_path: &str,
    output_path: &str,
    profile: &super::mapper::MappingProfile,
) {
    let mut writer = super::writer::SuperBlockWriter::new(std::path::Path::new(output_path))
        .expect("Failed to create SuperBlockWriter");
    let file = File::open(json_path).expect("Failed to open JSON file");
    let reader = BufReader::new(file);

    // Convert CLI profile to core-db profile
    let core_profile = JsonMappingProfile {
        base_class_hash: profile.base_class_hash,
        fields: profile
            .fields
            .iter()
            .map(|f| JsonFieldMapping {
                source_key: f.source_key.clone(),
                predicate_hash: f.predicate_hash,
                datatype: match f.datatype {
                    super::mapper::TargetDatatype::Integer => JsonDatatype::Integer,
                    super::mapper::TargetDatatype::Float => JsonDatatype::Float,
                    super::mapper::TargetDatatype::DateTime => JsonDatatype::DateTime,
                    super::mapper::TargetDatatype::StringRef => JsonDatatype::StringRef,
                },
            })
            .collect(),
    };

    // Use core-db parser
    parse_json_to_quins(reader, &core_profile, |quin| {
        writer.push(quin).expect("Failed to write to SuperBlock");
    })
    .expect("Failed to parse JSON");
}
