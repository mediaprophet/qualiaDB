use std::fs::File;
use qualia_core_db::sparql_library::parsers::csv_parser::{CsvMappingProfile, CsvColumnMapping, CsvDatatype, parse_csv_to_quins};

pub fn stream_csv_to_quins(csv_path: &str, output_path: &str, profile: &mut super::mapper::MappingProfile) {
    let mut writer = super::writer::SuperBlockWriter::new(std::path::Path::new(output_path)).expect("Failed to create SuperBlockWriter");
    let file = File::open(csv_path).expect("Failed to open CSV");
    
    // Convert CLI profile to core-db profile
    let mut core_profile = CsvMappingProfile {
        base_class_hash: profile.base_class_hash,
        fields: profile.fields.iter().map(|f| CsvColumnMapping {
            source_key: f.source_key.clone(),
            column_index: f.column_index,
            predicate_hash: f.predicate_hash,
            datatype: match f.datatype {
                super::mapper::TargetDatatype::Integer => CsvDatatype::Integer,
                super::mapper::TargetDatatype::Float => CsvDatatype::Float,
                super::mapper::TargetDatatype::DateTime => CsvDatatype::DateTime,
                super::mapper::TargetDatatype::StringRef => CsvDatatype::StringRef,
            },
        }).collect(),
    };
    
    // Use core-db parser
    parse_csv_to_quins(file, &mut core_profile, |quin| {
        writer.push(quin).expect("Failed to write to SuperBlock");
    }).expect("Failed to parse CSV");
    
    // Update column indices back to CLI profile
    for (i, field) in profile.fields.iter_mut().enumerate() {
        if let Some(core_field) = core_profile.fields.get(i) {
            field.column_index = core_field.column_index;
        }
    }
}