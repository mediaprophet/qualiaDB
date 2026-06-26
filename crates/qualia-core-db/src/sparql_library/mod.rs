pub mod sparql_ast;
pub mod sparql_parser;
pub mod sparql_planner;
pub mod sparql_executor;
pub mod sparql_filter;
pub mod sparql_aggregates;
pub mod sparql_endpoint;
pub mod sparql_extensions;
pub mod sparql_update;
pub mod sparql_shacl;
pub mod sparql_websocket;
pub mod sparql_federated;
pub mod sparql_mm;
pub mod sparql_did;
#[cfg(test)]
pub mod sparql_tests;
pub mod external_sort;
pub mod quin_sink;
pub mod rdf_formats;
pub mod parsers;
pub mod serialisers;

// Re-export parsers (explicit, not glob: `CsvDatatype` / `JsonDatatype` are *also* defined
// independently in the serializer modules, so glob-flattening both made the bare name
// ambiguous at this level. The parser datatypes keep the top-level name; the serializer
// ones stay reachable via their module path, e.g. `serialisers::csv_serializer::CsvDatatype`.)
pub use parsers::csv_parser::{CsvColumnMapping, CsvDatatype, CsvMappingProfile, parse_csv_to_quins};
pub use parsers::json_parser::{JsonDatatype, JsonFieldMapping, JsonMappingProfile, parse_json_to_quins};

// Re-export serializers (explicit; the serializers' own `CsvDatatype` / `JsonDatatype` are
// intentionally not re-exported here — use the module path for those).
pub use serialisers::csv_serializer::{CsvSerializationProfile, serialize_quins_to_csv};
pub use serialisers::json_serializer::{JsonSerializationProfile, serialize_quins_to_json};
pub use serialisers::rdf_serializers::*;
pub use rdf_formats::{parse_rdf, serialize_rdf, QuinCollector, QuinSink, RdfFormat, RdfParseError, RdfSerializeError, RdfStarMode, MAX_RDF_QUINS};