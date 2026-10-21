//! Canonical Registry of all Model Context Protocol (MCP) tools for QualiaDB.
//!
//! Exposes the complete suite of capabilities required to build software,
//! run simulations, perform formal reasoning, and query the codebase audit.
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::types::McpToolDescriptor;
use serde_json::{json, Value};

/// Returns all stable MCP tools exposed by the Qualia MCP subsystem.
pub fn stable_mcp_tools() -> &'static [McpToolDescriptor] {
    &[
        // ── Graph & Codebase Audit ──────────────────────────────────────────
        McpToolDescriptor {
            name: "query_sparql",
            description: "Execute a SPARQL query against the in-process daemon graph or a .q42 vault (pass vault_path: 'audit' for codebase.q42).",
            input_schema: r#"{"type":"object","required":["query"],"properties":{"query":{"type":"string"},"limit":{"type":"integer"},"vault_path":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "query_graph",
            description: "Run bare-metal graph traversal against the in-memory daemon graph.",
            input_schema: r#"{"type":"object","properties":{"sanctuary_override":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "codebase_audit",
            description: "Fast lookup of functions, signatures, methods, and files across the entire QualiaDB codebase audit (C:\\github\\qualiaDB\\.audit\\codebase.q42).",
            input_schema: r#"{"type":"object","properties":{"search":{"type":"string"},"limit":{"type":"integer"},"query":{"type":"string"},"vault_path":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "get_graph_stats",
            description: "Return quin count, capacity, and utilization for the resident daemon graph.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "list_ontologies",
            description: "List startup ontology catalog entries and on-disk presence.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "ingest_ontology",
            description: "Parse TTL/N3/q42 ontology file and extend the daemon graph.",
            input_schema: r#"{"type":"object","required":["path"],"properties":{"path":{"type":"string"},"context_hash":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "get_did_info",
            description: "Parse a did:q42 identifier into its topological pointer hash.",
            input_schema: r#"{"type":"object","required":["did"],"properties":{"did":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "get_wallet_status",
            description: "Inspect queued ILP micropayments from pending_payments.ndjson.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },

        // ── VibeScript DSL & Software Building ──────────────────────────────
        McpToolDescriptor {
            name: "vibe_check",
            description: "Parse and typecheck VibeScript 0.1 source code. Returns diagnostics, AST validation, and evidential typing.",
            input_schema: r#"{"type":"object","required":["source"],"properties":{"source":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "vibe_compile",
            description: "Compile VibeScript source into stack-based bytecode Chunk for Webizen VM.",
            input_schema: r#"{"type":"object","required":["source"],"properties":{"source":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "vibe_format",
            description: "Canonical pretty-printer formatting for VibeScript source declarations.",
            input_schema: r#"{"type":"object","required":["source"],"properties":{"source":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "vibe_invoke",
            description: "Invoke any VibeScript capability by ID across logic, math, physics, audio, and studio domains.",
            input_schema: r#"{"type":"object","required":["id"],"properties":{"id":{"type":"string"},"args":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "vibe_eval",
            description: "Evaluate VibeScript source code against the live snapshot and return exported bindings and quin count.",
            input_schema: r#"{"type":"object","required":["source"],"properties":{"source":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "vibe_catalog",
            description: "List and query all bound VibeScript capability IDs filtered by prefix and/or architectural seam.",
            input_schema: r#"{"type":"object","properties":{"prefix":{"type":"string"},"seam":{"type":"string"},"limit":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "wgsl_forge",
            description: "Deterministic WGSL generation, Naga validation, Roofline arithmetic intensity, schedule pruning, and attention synthesis.",
            input_schema: r#"{"type":"object","properties":{"op":{"type":"string","enum":["list_kernels","generate","validate","roofline","schedule_candidates","attention_synthesize"]},"kernel":{"type":"string"},"target":{"type":"string"},"workgroup_size":{"type":"integer"},"items_per_invocation":{"type":"integer"},"vector_width":{"type":"integer"},"source":{"type":"string"},"n":{"type":"integer"},"head_dim":{"type":"integer"},"head_count":{"type":"integer"},"context_tokens":{"type":"integer"},"batch_tokens":{"type":"integer"},"mode":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "poet_authoring",
            description: "POET shell authoring and Shed catalog (shed_catalog, manifold_create, container_place, nested_link, toolchest_list, manifold_inspect, etc.).",
            input_schema: r#"{"type":"object","properties":{"op":{"type":"string","enum":["shed_catalog","manifold_create","container_place","nested_link","subject_declare","participant_invite","toolchest_list","manifold_inspect","toolchain_execute","studio_configure","ide_evaluate","qapp_scaffold","theme_configure","inspector_bind","timeline_scrub","viewport_configure","export_bundle"]},"args":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "poet_studio_domain",
            description: "Cross-cutting Shed capability runner: apply any toolbox (ComputationalGeometry, Logic, Semantics, Economics, Science, ML, Studio) across containers with target_container.",
            input_schema: r#"{"type":"object","properties":{"capability":{"type":"string"},"tool":{"type":"string"},"domain":{"type":"string"},"op":{"type":"string"},"target_container":{"type":"string"},"artifact_id":{"type":"string"},"args":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "poet_artifact_lifecycle",
            description: "Full MCP lifecycle (create, edit, inspect, import, export, undo, redo, list) for POET Studio artifacts and QualiaDB/industry formats.",
            input_schema: r#"{"type":"object","properties":{"action":{"type":"string","enum":["create","edit","inspect","import","export","undo","redo","list_artifacts"]},"artifact_id":{"type":"string"},"domain":{"type":"string"},"format":{"type":"string"},"mutation":{"type":"string"},"args":{"type":"object"}}}"#,
        },

        // ── Simulation: Geomechanics, Space Weather, Fire Ecology ───────────
        McpToolDescriptor {
            name: "sim_soil_compaction",
            description: "Evaluate soil compaction, void ratio, and moisture-density Proctor curve for geotechnical engineering.",
            input_schema: r#"{"type":"object","required":["moisture_ratio"],"properties":{"moisture_ratio":{"type":"number"},"compaction_energy_knm_m3":{"type":"number"},"specific_gravity":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "sim_trench_excavation",
            description: "Evaluate trench stability, active earth pressure, and cutting force using Mohr-Coulomb failure criteria.",
            input_schema: r#"{"type":"object","required":["trench_depth_m"],"properties":{"trench_depth_m":{"type":"number"},"cohesion_kpa":{"type":"number"},"friction_angle_deg":{"type":"number"},"soil_unit_weight_kn_m3":{"type":"number"},"water_table_depth_m":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "sim_space_weather_eval",
            description: "Evaluate Schwabe/Hale solar cycles, solar irradiance (EUV/TSI), cosmic ray modulation, and magnetic reconnection.",
            input_schema: r#"{"type":"object","required":["year_ce"],"properties":{"year_ce":{"type":"number"},"imf_bz_nt":{"type":"number"},"solar_wind_speed_km_s":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "sim_fire_ecology_eval",
            description: "Evaluate Olson fuel accumulation, dead fuel moisture (DFMC), Byram fireline intensity, and Rothermel rate of spread.",
            input_schema: r#"{"type":"object","required":["air_temp_c","relative_humidity_pct"],"properties":{"air_temp_c":{"type":"number"},"relative_humidity_pct":{"type":"number"},"wind_speed_kmh":{"type":"number"},"stand_age_years":{"type":"number"}}}"#,
        },

        // ── Geospatial & Planetary Coordinates ──────────────────────────────
        McpToolDescriptor {
            name: "geospatial_transform",
            description: "Transform 3D geodetic coordinates between WGS84, ECEF, ENU, and MGRS frames.",
            input_schema: r#"{"type":"object","required":["op"],"properties":{"op":{"type":"string","enum":["wgs84_to_ecef","ecef_to_wgs84","ecef_to_enu","wgs84_to_mgrs"]},"lat_deg":{"type":"number"},"lon_deg":{"type":"number"},"alt_m":{"type":"number"},"x":{"type":"number"},"y":{"type":"number"},"z":{"type":"number"},"ref_lat_deg":{"type":"number"},"ref_lon_deg":{"type":"number"},"ref_alt_m":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "geospatial_hilbert",
            description: "Encode and decode 2D and 3D Hilbert curve spatial indices for zero-heap spatial hashing.",
            input_schema: r#"{"type":"object","required":["op"],"properties":{"op":{"type":"string","enum":["encode_2d","decode_2d","encode_3d"]},"order":{"type":"integer"},"x":{"type":"integer"},"y":{"type":"integer"},"z":{"type":"integer"},"index":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "geospatial_strata",
            description: "Query geological strata, lithology, and hydrogeological properties from WFS catalogs.",
            input_schema: r#"{"type":"object","required":["unit_code"],"properties":{"unit_code":{"type":"string"},"easting":{"type":"number"},"northing":{"type":"number"}}}"#,
        },

        // ── Vision, Biosense & Media ────────────────────────────────────────
        McpToolDescriptor {
            name: "computer_vision",
            description: "Native computer_vision specialized library: super-resolution, rgb_to_gray, mesh quality cleanup, class scoring.",
            input_schema: r#"{"type":"object","required":["op"],"properties":{"op":{"type":"string","enum":["list","capability_summary","rgb_to_gray","super_resolve","mesh_quality_cleanup","class_score_to_sigma"]},"width":{"type":"integer"},"height":{"type":"integer"},"scale":{"type":"integer"},"kernel":{"type":"string","enum":["nearest","bilinear","bicubic","lanczos3"]},"rgb":{"type":"array","items":{"type":"integer"}},"class_hash":{"type":"integer"},"score":{"type":"number"},"positions":{"type":"array","items":{"type":"array","items":{"type":"number"}}},"indices":{"type":"array","items":{"type":"integer"}},"weld_epsilon":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "vision_rppg",
            description: "Remote photoplethysmography pulse estimation from raw RGB video channel traces.",
            input_schema: r#"{"type":"object","required":["r","g","b"],"properties":{"r":{"type":"array","items":{"type":"number"}},"g":{"type":"array","items":{"type":"number"}},"b":{"type":"array","items":{"type":"number"}},"fps":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "vision_respiration",
            description: "Estimate respiration rate (breaths per minute) from motion and optical biosensing traces.",
            input_schema: r#"{"type":"object","required":["trace"],"properties":{"trace":{"type":"array","items":{"type":"number"}},"fps":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "vision_liveness_pad",
            description: "Presentation Attack Detection (liveness analysis) on biometric facial tracking.",
            input_schema: r#"{"type":"object","properties":{"landmark_jitter":{"type":"number"},"profile_asymmetry":{"type":"number"},"non_rigid_depth_var":{"type":"number"}}}"#,
        },

        // ── Auditory Intelligence ───────────────────────────────────────────
        McpToolDescriptor {
            name: "audio_features",
            description: "Native qualia-audio features: log_mel spectrogram, pitch_yin (YIN fundamental frequency), loudness_r128 (EBU R128 LUFS).",
            input_schema: r#"{"type":"object","required":["op"],"properties":{"op":{"type":"string","enum":["list","capability_summary","log_mel","pitch_yin","loudness_r128"]},"samples":{"type":"array","items":{"type":"number"}},"sample_rate":{"type":"integer"},"n_mel":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "audio_spatial_hrtf",
            description: "Zero-heap 3D spatial audio positioning, distance attenuation, and binaural stereo panning.",
            input_schema: r#"{"type":"object","required":["emitter_x","emitter_y","emitter_z"],"properties":{"emitter_x":{"type":"number"},"emitter_y":{"type":"number"},"emitter_z":{"type":"number"},"listener_x":{"type":"number"},"listener_y":{"type":"number"},"listener_z":{"type":"number"}}}"#,
        },

        // ── Logic Modalities & N3 Reasoning ─────────────────────────────────
        McpToolDescriptor {
            name: "evaluate_modality",
            description: "Evaluate logic modalities: LTL temporal traces, ASP stable models, deontic norms, epistemic belief/knowledge, paraconsistent merge.",
            input_schema: r#"{"type":"object","required":["modality"],"properties":{"modality":{"type":"string"},"quins":{"type":"array"},"trace":{"type":"array"},"formula":{"type":"object"},"now_unix":{"type":"integer"},"agent_did_hash":{"type":"integer"},"world_hash":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "evaluate_logic_rules",
            description: "Load N3 rules into RuleEngine and fire them in the Webizen VM.",
            input_schema: r#"{"type":"object","required":["n3_source","quin"],"properties":{"n3_source":{"type":"string"},"quin":{"type":"object"},"ruleset_name":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "symbolic_logic_infer",
            description: "Defeasible forward-chaining or bounded SAT with caller facts, rules, and clauses.",
            input_schema: r#"{"type":"object","properties":{"solver":{"type":"string","enum":["defeasible","sat"]},"facts":{"type":"array"},"rules":{"type":"array"},"clauses":{"type":"array"},"max_iterations":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "inject_test_quin",
            description: "Inject a deterministic test Quin through the paraconsistent router.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },

        // ── SHACL Constraints & Verification ────────────────────────────────
        McpToolDescriptor {
            name: "validate_shacl",
            description: "Validate quins for target subject/property against SHACL constraints.",
            input_schema: r#"{"type":"object","required":["quins","target_subject","target_property","constraints"],"properties":{"quins":{"type":"array"},"target_subject":{"type":"integer"},"target_property":{"type":"integer"},"constraints":{"type":"array"}}}"#,
        },
        McpToolDescriptor {
            name: "validate_enumerated_identity",
            description: "SHACL identity extension: validate identity as enumerated multi-attested state.",
            input_schema: r#"{"type":"object","required":["bindings"],"properties":{"bindings":{"type":"array"},"min_distinct":{"type":"integer"},"min_attested":{"type":"integer"}}}"#,
        },
        McpToolDescriptor {
            name: "shacl_credential_gate",
            description: "Decide whether a verified W3C Verifiable Credential gates a SHACL target node.",
            input_schema: r#"{"type":"object","required":["focus_node","gate","credential"],"properties":{"focus_node":{"type":"integer"},"gate":{"type":"object"},"credential":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "shacl_degrade_violations",
            description: "SHACL identity extension: real-time severity degradation off-grid.",
            input_schema: r#"{"type":"object","required":["violations"],"properties":{"violations":{"type":"array"},"mode":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "shacl_route",
            description: "Decentralized shape-target routing between shapes and loci.",
            input_schema: r#"{"type":"object","required":["routes"],"properties":{"routes":{"type":"array"},"query_locus":{"type":"integer"},"query_shape":{"type":"integer"}}}"#,
        },

        // ── QApps Host Runtime ──────────────────────────────────────────────
        McpToolDescriptor {
            name: "list_qapps",
            description: "List installed qapps from storage Qapps directory.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "get_qapp_manifest",
            description: "Load qapp.json manifest for an installed or bundled qapp.",
            input_schema: r#"{"type":"object","required":["qapp_name"],"properties":{"qapp_name":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "inspect_qapp_readiness",
            description: "Check manifest, entrypoints, and ontology readiness for a qapp.",
            input_schema: r#"{"type":"object","required":["qapp_name"],"properties":{"qapp_name":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "list_qapp_updates",
            description: "Compare installed qapp versions against bundled update offers.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "describe_qapp_surface_schema",
            description: "Describe the current QApp host surface schema exposed by Qualia.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },

        // ── Scientific & Engineering Compute ────────────────────────────────
        McpToolDescriptor {
            name: "matrix_operation",
            description: "Linear algebra: multiply, transpose, solve, or inverse matrices.",
            input_schema: r#"{"type":"object","required":["op"],"properties":{"op":{"type":"string","enum":["multiply","transpose","solve","inverse"]},"left":{"type":"object"},"right":{"type":"object"},"matrices":{"type":"array"},"result_id":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "algebra_solve_polynomial",
            description: "Find all roots of a polynomial given descending coefficients.",
            input_schema: r#"{"type":"object","required":["coeffs"],"properties":{"coeffs":{"type":"array","items":{"type":"number"}}}}"#,
        },
        McpToolDescriptor {
            name: "algebra_matrix_analyze",
            description: "Determinant, eigenvalues, symmetric eigensystem, or SVD of a matrix.",
            input_schema: r#"{"type":"object","required":["op","rows","data"],"properties":{"op":{"type":"string","enum":["determinant","eigenvalues","eigen_symmetric","svd"]},"rows":{"type":"integer"},"cols":{"type":"integer"},"data":{"type":"array","items":{"type":"number"}}}}"#,
        },
        McpToolDescriptor {
            name: "cas",
            description: "Symbolic algebra: differentiate, simplify, expand, evaluate, solve quadratic, factor.",
            input_schema: r#"{"type":"object","required":["op"],"properties":{"op":{"type":"string","enum":["differentiate","simplify","expand","evaluate","solve_quadratic","factor"]},"expr":{"type":"string"},"var":{"type":"string"},"env":{"type":"object"},"a":{"type":"number"},"b":{"type":"number"},"c":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "ode_solve",
            description: "Run configurable CFD or molecular-dynamics simulation step.",
            input_schema: r#"{"type":"object","properties":{"type":{"type":"string","enum":["cfd","distributed","molecular_dynamics"]},"nx":{"type":"integer"},"ny":{"type":"integer"},"dx":{"type":"number"},"time_step":{"type":"number"},"total_time":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "chemical_analysis",
            description: "Predict molecular properties from SMILES or chemical formula.",
            input_schema: r#"{"type":"object","properties":{"smiles":{"type":"string"},"formula":{"type":"string"},"molecular_weight":{"type":"number"},"prop":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "chemical_descriptors",
            description: "Compute Lipinski and Veber descriptors from a SMILES string.",
            input_schema: r#"{"type":"object","required":["smiles"],"properties":{"smiles":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "statistical_analysis",
            description: "Descriptive statistics (mean, variance, correlation) on tabular data.",
            input_schema: r#"{"type":"object","required":["rows"],"properties":{"stat":{"type":"string","enum":["mean","variance","correlation"]},"rows":{"type":"array"},"columns":{"type":"array","items":{"type":"string"}},"column":{"type":"string"},"column_y":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "financial_model",
            description: "Black-Scholes option pricing or portfolio risk analysis.",
            input_schema: r#"{"type":"object","properties":{"op":{"type":"string","enum":["option","risk"]},"underlying_price":{"type":"number"},"strike":{"type":"number"},"volatility":{"type":"number"},"assets":{"type":"array"},"cash_balance":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "medical_score",
            description: "Clinical diagnostic and prognostic score analysis.",
            input_schema: r#"{"type":"object","properties":{"patient_id":{"type":"string"},"score":{"type":"string","enum":["diagnosis","treatment","prognosis","prevention"]},"patient":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "clinical_risk",
            description: "Clinical risk scores: framingham, cha2ds2_vasc, sofa, egfr.",
            input_schema: r#"{"type":"object","properties":{"score":{"type":"string","enum":["framingham","cha2ds2","cha2ds2_vasc","sofa","egfr","renal"]},"age":{"type":"integer"},"input":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "engineering_analysis_op",
            description: "Structural, thermal, or dynamic FEA analysis.",
            input_schema: r#"{"type":"object","properties":{"analysis":{"type":"string","enum":["structural","thermal","dynamic"]},"model":{"type":"object"},"youngs_modulus":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "computational_geometry",
            description: "2D convex hull, Delaunay, Voronoi, nearest site, 3D primitive mesh generation.",
            input_schema: r#"{"type":"object","required":["op"],"properties":{"op":{"type":"string","enum":["orientation_2","convex_hull_2","triangle_topology","delaunay_2","voronoi_2","nearest_site","mesh_topology","create_box","create_sphere","create_cylinder","create_plane"]},"points":{"type":"array"},"width":{"type":"number"},"height":{"type":"number"},"depth":{"type":"number"},"radius":{"type":"number"}}}"#,
        },
        McpToolDescriptor {
            name: "geometry_manifests",
            description: "Per-op computational geometry capability manifests and resource budgets.",
            input_schema: r#"{"type":"object","properties":{"op":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "geometric_algebra_op",
            description: "3D vector cross product, dot product, or angle.",
            input_schema: r#"{"type":"object","required":["a","b"],"properties":{"op":{"type":"string","enum":["cross","angle","dot"]},"a":{"type":"array","items":{"type":"number"}},"b":{"type":"array","items":{"type":"number"}}}}"#,
        },
        McpToolDescriptor {
            name: "bioinformatics_align",
            description: "Pairwise nucleotide or protein sequence alignment (Smith-Waterman).",
            input_schema: r#"{"type":"object","required":["query","target"],"properties":{"query":{"type":"string"},"target":{"type":"string"},"mode":{"type":"string","enum":["dna","protein"]}}}"#,
        },
        McpToolDescriptor {
            name: "conditioning_compile",
            description: "Compile conditioning profile into execution contract and budget plan.",
            input_schema: r#"{"type":"object","properties":{"profile":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "conditioning_validate",
            description: "Validate conditioning profile specification against schema rules.",
            input_schema: r#"{"type":"object","properties":{"profile":{"type":"object"}}}"#,
        },

        // ── Data Formats: Zero-Heap RDF, JSON, CSV ──────────────────────────
        McpToolDescriptor {
            name: "parse_rdf",
            description: "Stream RDF formats (nt, turtle, nquads, trig, n3, jsonld, cbor) into quins.",
            input_schema: r#"{"type":"object","required":["format"],"properties":{"format":{"type":"string"},"rdf_data":{"type":"string"},"file_path":{"type":"string"},"ingest_to_graph":{"type":"boolean"}}}"#,
        },
        McpToolDescriptor {
            name: "parse_json",
            description: "Stream JSON objects into Quins via zero-heap field mappings.",
            input_schema: r#"{"type":"object","required":["field_mappings"],"properties":{"json_data":{"type":"string"},"file_path":{"type":"string"},"field_mappings":{"type":"array"}}}"#,
        },
        McpToolDescriptor {
            name: "parse_csv",
            description: "Stream CSV tabular data into Quins via zero-heap field mappings.",
            input_schema: r#"{"type":"object","required":["field_mappings"],"properties":{"csv_data":{"type":"string"},"file_path":{"type":"string"},"field_mappings":{"type":"array"}}}"#,
        },
        McpToolDescriptor {
            name: "serialize_rdf",
            description: "Serialize quins or graph slice to RDF/RDF-Star formats.",
            input_schema: r#"{"type":"object","properties":{"quins":{"type":"array"},"format":{"type":"string"},"output":{"type":"string","enum":["inline","file"]}}}"#,
        },
        McpToolDescriptor {
            name: "serialize_json",
            description: "Serialize quins to JSON array.",
            input_schema: r#"{"type":"object","required":["field_names","predicate_hashes"],"properties":{"quins":{"type":"array"},"field_names":{"type":"array"},"predicate_hashes":{"type":"array"}}}"#,
        },
        McpToolDescriptor {
            name: "serialize_csv",
            description: "Serialize quins to CSV tabular output.",
            input_schema: r#"{"type":"object","required":["headers","predicate_hashes"],"properties":{"quins":{"type":"array"},"headers":{"type":"array"},"predicate_hashes":{"type":"array"}}}"#,
        },

        // ── Cooperative Governance ──────────────────────────────────────────
        McpToolDescriptor {
            name: "values_check",
            description: "Values abuse-check (human-rights dignity guard).",
            input_schema: r#"{"type":"object","required":["agentType"],"properties":{"agentType":{"type":"string"},"claimsDignityRight":{"type":"boolean"}}}"#,
        },
        McpToolDescriptor {
            name: "values_evaluate",
            description: "Deontic-contract reasoner in values terms.",
            input_schema: r#"{"type":"object","required":["modality","party","action"],"properties":{"modality":{"type":"string","enum":["forbid","oblige","permit"]},"party":{"type":"string"},"action":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "jural_correlate",
            description: "Hohfeldian jural correlation and opposites for 8 positions.",
            input_schema: r#"{"type":"object","required":["position"],"properties":{"position":{"type":"string","enum":["claim","duty","privilege","no-right","power","liability","immunity","disability"]}}}"#,
        },
        McpToolDescriptor {
            name: "deontic_govern",
            description: "Interaction governance runtime PolicyMode mapping.",
            input_schema: r#"{"type":"object","required":["status"],"properties":{"status":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "mcp_cooperate",
            description: "Track-M agent cooperation gate authorization check.",
            input_schema: r#"{"type":"object","required":["caller","verified"],"properties":{"caller":{"type":"string"},"verified":{"type":"boolean"},"grounded":{"type":"boolean"}}}"#,
        },

        // ── Quantum Computing & Optimization ────────────────────────────────
        McpToolDescriptor {
            name: "qpu_optimize",
            description: "Formulate a QUBO/circuit job from problem description and classical solve.",
            input_schema: r#"{"type":"object","required":["problem"],"properties":{"problem":{"type":"object"}}}"#,
        },
        McpToolDescriptor {
            name: "qpu_dft",
            description: "Bounded Thomas-Fermi DFT ground-state energy evaluation.",
            input_schema: r#"{"type":"object","properties":{"grid_resolution":{"type":"integer"},"quins":{"type":"array"}}}"#,
        },
        McpToolDescriptor {
            name: "qpu_status",
            description: "Return QPU bridge connection and job-queue status.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },

        // ── Local LLM Intelligence ─────────────────────────────────────────
        McpToolDescriptor {
            name: "llm_infer",
            description: "Run local GGUF autoregressive inference loop.",
            input_schema: r#"{"type":"object","required":["prompt"],"properties":{"prompt":{"type":"string"},"model_path":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "llm_chat",
            description: "Multi-turn chat completion via local GGUF inference stack.",
            input_schema: r#"{"type":"object","required":["messages"],"properties":{"messages":{"type":"array"}}}"#,
        },
        McpToolDescriptor {
            name: "list_models",
            description: "Discover GGUF models under storage and resident mounted models.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },

        // ── System Status & Tasks ───────────────────────────────────────────
        McpToolDescriptor {
            name: "get_system_status",
            description: "Return runtime status for the Qualia MCP surface.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "list_capabilities",
            description: "List the canonical Qualia capability catalogue.",
            input_schema: r#"{"type":"object","properties":{"domain":{"type":"string"}}}"#,
        },
        McpToolDescriptor {
            name: "get_pending_tasks",
            description: "Get tasks pending in the ambient orchestrator.",
            input_schema: r#"{"type":"object","properties":{}}"#,
        },
        McpToolDescriptor {
            name: "run_docs_tests",
            description: "Run headless test suites (logic, wasm, native).",
            input_schema: r#"{"type":"object","properties":{"mode":{"type":"string","enum":["logic","wasm","native","both"]}}}"#,
        },
    ]
}

/// Generates the JSON-RPC `tools/list` response object.
pub fn tool_list_json() -> Value {
    let tools = stable_mcp_tools();
    let mut mapped = Vec::with_capacity(tools.len());

    for tool in tools {
        let schema: Value = serde_json::from_str(tool.input_schema).unwrap_or_else(|_| json!({"type": "object"}));
        mapped.push(json!({
            "name": tool.name,
            "description": tool.description,
            "inputSchema": schema,
        }));
    }

    json!({
        "tools": mapped
    })
}
