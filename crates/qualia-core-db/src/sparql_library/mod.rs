pub mod external_sort;
pub mod parsers;
pub mod quin_sink;
pub mod rdf_formats;
pub mod serialisers;
pub mod sparql_aggregates;
pub mod sparql_ast;
pub mod sparql_did;
pub mod sparql_endpoint;
pub mod sparql_executor;
pub mod sparql_extensions;
pub mod sparql_federated;
pub mod sparql_filter;
pub mod sparql_mm;
pub mod sparql_parser;
pub mod sparql_planner;
pub mod sparql_shacl;
#[cfg(test)]
pub mod sparql_tests;
pub mod sparql_update;
pub mod sparql_websocket;

// Re-export parsers (explicit, not glob: `CsvDatatype` / `JsonDatatype` are *also* defined
// independently in the serializer modules, so glob-flattening both made the bare name
// ambiguous at this level. The parser datatypes keep the top-level name; the serializer
// ones stay reachable via their module path, e.g. `serialisers::csv_serializer::CsvDatatype`.)
pub use parsers::csv_parser::{
    parse_csv_to_quins, CsvColumnMapping, CsvDatatype, CsvMappingProfile,
};
pub use parsers::json_parser::{
    parse_json_to_quins, JsonDatatype, JsonFieldMapping, JsonMappingProfile,
};

// Re-export serializers (explicit; the serializers' own `CsvDatatype` / `JsonDatatype` are
// intentionally not re-exported here — use the module path for those).
pub use rdf_formats::{
    parse_rdf, serialize_rdf, QuinCollector, QuinSink, RdfFormat, RdfParseError, RdfSerializeError,
    RdfStarMode, MAX_RDF_QUINS,
};
pub use serialisers::csv_serializer::{serialize_quins_to_csv, CsvSerializationProfile};
pub use serialisers::json_serializer::{serialize_quins_to_json, JsonSerializationProfile};
pub use serialisers::rdf_serializers::*;
