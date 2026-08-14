pub mod range_bind;
pub mod range_hash_join;
pub mod range_join_select;
pub mod range_optional;
pub mod range_select_apply;
pub mod range_union;
pub use range_bind::{
    apply_bind, execute_range_bind_page_into, execute_range_volume_set_bind_page_into,
    Q42RangeBindPlan, Q42RangeBindState, Q42RangeVolumeSetBindState,
};
pub use range_hash_join::{
    execute_range_hash_join_page_into, execute_range_volume_set_hash_join_page_into,
    Q42RangeHashJoinPlan, Q42RangeHashJoinSlot, Q42RangeHashJoinState,
    Q42RangeVolumeSetHashJoinState,
};
pub use range_join_select::{
    execute_range_join_select_page_into, execute_range_volume_set_join_select_page_into,
    Q42RangeJoinSelectPlan, Q42RangeJoinSelectState, Q42RangeVolumeSetJoinSelectState,
};
pub use range_optional::{
    execute_range_optional_page_into, execute_range_volume_set_optional_page_into,
    Q42RangeOptionalPlan, Q42RangeOptionalState, Q42RangeVolumeSetOptionalState,
};
pub use range_union::{
    execute_range_union_page_into, execute_range_volume_set_union_page_into, Q42RangeUnionPlan,
    Q42RangeUnionState, Q42RangeVolumeSetUnionState,
};
pub mod external_sort;
pub mod geosparql;
/// Immersive SPARQL (QISP) profile — Phase 2 typed values, dense-asset registry,
/// and (integrated here) the typed function descriptor registry. See
/// `docs/plans/immersive-sparql-hypermedia-profile.md`.
pub mod immersive;
pub mod parsers;
pub mod quin_sink;
pub mod rdf_formats;
pub mod serialisers;
pub mod sparql_aggregates;
pub mod sparql_ast;
pub mod sparql_did;
pub mod sparql_endpoint;
pub mod sparql_executor;
pub mod sparql_federated;
pub mod sparql_filter;
pub mod sparql_grammar;
pub mod sparql_mm;
pub mod sparql_parser;
pub mod sparql_planner;
pub mod sparql_shacl;
#[cfg(test)]
pub mod sparql_tests;
pub mod sparql_update;
pub mod sparql_websocket;
pub mod vision_shacl;

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
