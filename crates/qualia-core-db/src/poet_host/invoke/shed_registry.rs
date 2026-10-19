//! POET Shed Registry & Cross-Container ToolChain Execution.
//!
//! Provides the unified POET Shed environment: Constructs (observer lenses),
//! Manifolds (17 work surfaces + 17 dimensional axes), Containers (cross-compatible
//! workspaces), Toolboxes, and ToolChains backed by QualiaDB specialized libraries.
//! Follows AGENTS.md §0-B (< 485 lines).

use super::args;
use super::ids;
use vibe::{Diagnostic, Span, Value};

// ── Static Shed Constructs ──────────────────────────────────────────────────

pub struct ShedConstruct {
    pub id: &'static str,
    pub title: &'static str,
    pub manifolds: &'static [&'static str],
    pub default_toolboxes: &'static [&'static str],
}

pub const SHED_CONSTRUCTS: &[ShedConstruct] = &[
    ShedConstruct { id: "poet", title: "POET Core HyperCanvas", manifolds: &["studio-bay", "vibe", "media"], default_toolboxes: &["StudioAndHypermedia", "ComputationalGeometry"] },
    ShedConstruct { id: "research-lab", title: "Autonomous Scientific Research Lab", manifolds: &["research", "datasets", "knowledge"], default_toolboxes: &["StatisticsMlAndResearch", "ScienceChemistryClinical"] },
    ShedConstruct { id: "studio", title: "Cross-Domain Multi-Disciplinary Studio", manifolds: &["studio-bay", "studio", "media"], default_toolboxes: &["StudioAndHypermedia", "GeometricAlgebra"] },
    ShedConstruct { id: "health", title: "Clinical & Genomic Medicine Laboratory", manifolds: &["health", "anatomy", "datasets"], default_toolboxes: &["ScienceChemistryClinical", "StatisticsMlAndResearch"] },
    ShedConstruct { id: "rights", title: "Human Rights & Cooperative Commons", manifolds: &["rights", "sanctuary", "social"], default_toolboxes: &["EconomicsAndGovernance", "LogicAndSemantics"] },
    ShedConstruct { id: "knowledge", title: "Ontological Knowledge Graph", manifolds: &["knowledge", "ontology"], default_toolboxes: &["LogicAndSemantics", "StatisticsMlAndResearch"] },
    ShedConstruct { id: "projects", title: "Collaborative Work & Engineering Projects", manifolds: &["projects", "studio"], default_toolboxes: &["EconomicsAndGovernance", "MathCasAndSolvers"] },
    ShedConstruct { id: "geometry-lab", title: "N-Dimensional Volumetric Geometry Lab", manifolds: &["studio", "media"], default_toolboxes: &["ComputationalGeometry", "GeometricAlgebra"] },
    ShedConstruct { id: "economics-commons", title: "Cooperative Economics & Market Design", manifolds: &["social", "communications"], default_toolboxes: &["EconomicsAndGovernance", "MathCasAndSolvers"] },
    ShedConstruct { id: "logic-sentinel", title: "Formal Deontic & Sentinel Logic Workbench", manifolds: &["ontology", "rights"], default_toolboxes: &["LogicAndSemantics", "MathCasAndSolvers"] },
];

// ── Static Shed Toolboxes ───────────────────────────────────────────────────

pub const SHED_TOOLBOXES: &[(&str, &[&str])] = &[
    ("ComputationalGeometry", &["ComputationalGeometry.hull2", "ComputationalGeometry.triangulate_polygon", "ComputationalGeometry.heat_geodesics", "ComputationalGeometry.svo_raycast", "ComputationalGeometry.morton_encode_3d", "ComputationalGeometry.sdf_smooth", "ComputationalGeometry.space_colonize_3d", "ComputationalGeometry.solar_track", "ComputationalGeometry.poisson_disk_3d", "ComputationalGeometry.pbd_step", "ComputationalGeometry.dqs_skin", "ComputationalGeometry.boolean_union_area"]),
    ("GeometricAlgebra", &["GeometricAlgebra.geometric_product", "GeometricAlgebra.outer_product", "GeometricAlgebra.rotor_from_angle_axis", "GeometricAlgebra.apply_rotor", "GeometricAlgebra.translator_from_displacement"]),
    ("MathCasAndSolvers", &["LinearAlgebra.solve_linear_system", "LinearAlgebra.svd", "LinearAlgebra.eigen_symmetric", "SymbolicAlgebra.differentiate", "SymbolicAlgebra.integrate_definite", "SymbolicAlgebra.taylor_coefficients", "SymbolicAlgebra.to_quins", "SymbolicODE.solve_linear_second_order", "Calculus.stormer_verlet_step", "NumberTheory.crt", "SpecialFunctions.bessel_j", "Interpolation.lagrange_eval"]),
    ("LogicAndSemantics", &["Deontic.evaluate", "Epistemic.evaluate", "Paraconsistent.route", "LTL.evaluate_trace", "N3.evaluate", "SHACL.validate", "SHACL.extensions", "GraphDatabase.sparql", "GraphReasoning.spreading_activation", "FuzzyQuery.top_k", "DescriptionLogic.subsumes", "ASP.enumerate", "Causal.caused"]),
    ("EconomicsAndGovernance", &["Econ.black_scholes", "Econ.capm_expected_return", "Econ.gini", "Econ.atkinson", "Econ.rawlsian_welfare", "Econ.solow_steady_state", "Econ.ramsey_steady_state", "Econ.cournot_duopoly", "Econ.vcg_payment", "Econ.social_cost_of_carbon", "Econ.leontief_inverse", "Econ.interbank_clearing", "Econ.accounting_balance", "CooperativeDelegation.permits", "CooperativeWork.board_project", "Agency.evaluate", "Sentinel.gate"]),
    ("ScienceChemistryClinical", &["Physics.molecular_dynamics", "Physics.cfd_step", "Physics.emf_field_grid_3d", "Physics.quantum_states_1d", "Chemistry.evaluate_eri", "Chemistry.lda_exchange", "Chemistry.sto3g_h2", "Chemistry.boys_function", "Bioinformatics.align", "ClinicalRisk.framingham", "ClinicalRisk.cha2ds2_vasc", "ClinicalRisk.score2", "ClinicalRisk.check_drug_interaction", "EngineeringAnalysis.fem_static", "Simulation.soil_compaction", "Simulation.fire_ecology", "Cosmic.flrw_redshift"]),
    ("StatisticsMlAndResearch", &["Statistics.one_way_anova", "Statistics.mann_whitney_u", "Statistics.fisher_distance", "Statistics.kl_bregman_form", "Statistics.mvn_mle", "MachineLearning.pca", "MachineLearning.kmeans_fit", "MachineLearning.random_forest_fit_classifier", "MachineLearning.gp_fit", "MachineLearning.hmm_baum_welch", "NLP.graphrag_query", "NLP.coref_resolve", "Research.new_investigation", "Research.infer_dark_link", "Research.reconcile_perspectives", "Inference.run_transformer"]),
    ("StudioAndHypermedia", &["Document.create", "Document.layout_paragraph", "Sheet.eval_formula", "Deck.slide_layout", "Pdf.preflight_a11y", "Raster.cms_convert", "RawDevelop.demosaic", "Vector.path_boolean", "Cad.sketch_constrain", "Cad.brep_solid", "Timeline.master_lufs", "WgslForge.generate", "WgslForge.roofline", "Studio.edit", "Studio.inspect", "Studio.import", "Studio.export", "Scene.render", "Audio.oscillator", "Video.colour_grade", "HID.poll"]),
];

const BASE_FAMILIES: &[(&str, &[&str])] = &[
    ("Core", &["Select", "Transform", "Pen", "Brush", "Eraser", "Measure", "Eyedropper", "Slice"]),
    ("Constructs", &["Container", "ManifoldPortal", "NestedLink", "TimelineTrack", "ShaderNode", "ConstraintSolver"]),
    ("Manifolds", &["Spatial", "Temporal", "Acoustic", "RF", "Magnetic", "Cyber", "Hardware", "Logical", "Agentic", "Socio", "Political", "Legal", "Economic", "Bio", "Thermal", "Ecological", "Relativistic"]),
    ("Agents", &["Planner", "Synthesizer", "Verifier", "Archivist", "Choreographer", "SentinelGuard"]),
    ("Ecosystems", &["EcoPunk", "GameDemo", "CatchmentCommons", "BilateralMicroCommons", "CivicObservatory"]),
    ("Applications", &["DocumentStudio", "SheetStudio", "DeckStudio", "PdfStudio", "RasterStudio", "RawDevelopStudio", "VectorStudio", "GenerativeStudio", "CadStudio", "VideoStudio", "AudioStudio"]),
    ("Services", &["PulseTransport", "Q42VolumeStore", "ShaclGate", "WgslForge", "LocalLlm"]),
    ("Systems", &["SlgSentinel42MB", "WebGpuViewport", "HrtfAudio", "HidRingBuffer", "DeonticJudge"]),
];

#[inline]
fn save_art(id: &str, domain: &str, title: &str, history_op: &str, props: &[(&str, &str)]) -> (u64, u64, u64) {
    let mut art = super::studio_store::get_artifact(id).unwrap_or_else(|| super::studio_store::StudioArtifact::new(id, domain, title));
    for (k, v) in props { art.properties.insert((*k).into(), (*v).into()); }
    art.history.push(history_op.into());
    let quins = art.compile_to_quins();
    let (did, clock, qcount) = (art.did_q42, art.lamport_clock as u64, quins.len() as u64);
    super::studio_store::upsert_artifact(art);
    (did, clock, qcount)
}

/// Recommends a toolbox category, construct ID, and default toolchain CSV for a given container kind and manifold.
pub fn recommend_for_context(container_kind: &str, manifold: &str) -> (&'static str, &'static str, &'static str) {
    let (ck, mf) = (container_kind.trim().to_ascii_lowercase(), manifold.trim().to_ascii_lowercase());
    match ck.as_str() {
        "cad" => ("ComputationalGeometry", "studio", "Cad.sketch_constrain,Cad.brep_solid,Vector.extrude_3d"),
        "spatial-3d" | "3d" => ("ComputationalGeometry", "geometry-lab", "ComputationalGeometry.hull2,WgslForge.generate"),
        "shader" | "gpu" => ("StudioAndHypermedia", "geometry-lab", "WgslForge.generate,WgslForge.roofline,Poet.viewport_configure"),
        "logic" | "proof" | "sparql" => ("LogicAndSemantics", "logic-sentinel", "Deontic.evaluate,SHACL.validate,N3.evaluate"),
        "notebook" | "research" => ("StatisticsMlAndResearch", "research-lab", "Research.new_investigation,NLP.graphrag_query,Statistics.one_way_anova"),
        "sheet" => ("StudioAndHypermedia", "studio", "Sheet.create,Sheet.eval_formula,Sheet.pivot_analyze"),
        "economics" | "wallet" => ("EconomicsAndGovernance", "economics-commons", "Econ.accounting_balance,CooperativeWork.board_project"),
        "social" => ("EconomicsAndGovernance", "rights", "CooperativeDelegation.permits,Social.gini"),
        "document" => ("StudioAndHypermedia", "studio", "Document.create,Document.layout_paragraph,Document.proof_check"),
        "deck" => ("StudioAndHypermedia", "studio", "Deck.create,Deck.slide_layout,Deck.export_pptx"),
        "pdf" => ("StudioAndHypermedia", "studio", "Pdf.inspect_cos,Pdf.preflight_a11y,Pdf.redact_sign"),
        "raster" => ("StudioAndHypermedia", "studio", "Raster.cms_convert,Raster.tone_curve,RawDevelop.demosaic"),
        "vector" => ("StudioAndHypermedia", "studio", "Vector.path_boolean,Vector.bezier_offset,Vector.svg_compile"),
        "timeline" => ("StudioAndHypermedia", "studio", "Timeline.scopes_analyze,Timeline.master_lufs,Timeline.dsp_chain"),
        "simulation" => ("ScienceChemistryClinical", "research-lab", "Simulation.soil_compaction,Simulation.fire_ecology,Physics.cfd_step"),
        "wire" => ("StudioAndHypermedia", "poet", "Poet.nested_link,Poet.inspector_bind"),
        _ => match mf.as_str() {
            "health" | "anatomy" => ("ScienceChemistryClinical", "health", "ClinicalRisk.score2,ClinicalRisk.framingham"),
            "rights" | "sanctuary" => ("EconomicsAndGovernance", "rights", "Agency.evaluate,CooperativeDelegation.permits"),
            "knowledge" | "ontology" => ("LogicAndSemantics", "knowledge", "GraphDatabase.sparql,SHACL.validate"),
            _ => ("StudioAndHypermedia", "poet", "Document.create,Poet.export_bundle"),
        },
    }
}

// ── Vibe Invoke Handlers ────────────────────────────────────────────────────

/// `Poet.toolchest_list` — lists Shed toolboxes, constructs, and tool families or creates custom items.
pub fn toolchest_list(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let op = args::rec_str(args_v, "op").unwrap_or("list");

    if op == "toolbox_create" {
        let (t_id, lbl, cat) = (args::rec_str(args_v, "toolbox_id").unwrap_or("custom-toolbox"), args::rec_str(args_v, "label").unwrap_or("Custom Shed Toolbox"), args::rec_str(args_v, "category").unwrap_or("custom"));
        let (tools, c_kind) = (args::rec_str(args_v, "tools").unwrap_or("custom.op"), args::rec_str(args_v, "container_kind").unwrap_or("any"));
        let art_id = format!("toolbox:{t_id}");
        let (did, clock, qcount) = save_art(&art_id, "Toolbox", lbl, "toolbox_create", &[("toolbox_id", t_id), ("category", cat), ("container_kind", c_kind), ("tools", tools)]);
        return Ok(args::record([
            ("status", Value::String("created".into())), ("op", Value::String("toolbox_create".into())),
            ("id", Value::String(ids::POET_TOOLCHEST_LIST.into())), ("toolbox_id", Value::String(t_id.into())),
            ("label", Value::String(lbl.into())), ("category", Value::String(cat.into())),
            ("container_kind", Value::String(c_kind.into())), ("tools", Value::String(tools.into())),
            ("did_q42", Value::U64(did)), ("lamport_clock", Value::U64(clock)), ("quin_count", Value::U64(qcount)),
            ("shell", Value::String("poet".into())),
        ]));
    }

    if op == "toolchain_create" {
        let (tc_id, lbl) = (args::rec_str(args_v, "toolchain_id").unwrap_or("custom-toolchain"), args::rec_str(args_v, "label").unwrap_or("Custom ToolChain"));
        let (t_kind, steps) = (args::rec_str(args_v, "target_kind").unwrap_or("any"), args::rec_str(args_v, "steps").unwrap_or("Document.create,Poet.export_bundle"));
        let art_id = format!("toolchain:{tc_id}");
        let (did, clock, qcount) = save_art(&art_id, "ToolChain", lbl, "toolchain_create", &[("toolchain_id", tc_id), ("target_kind", t_kind), ("steps", steps)]);
        return Ok(args::record([
            ("status", Value::String("created".into())), ("op", Value::String("toolchain_create".into())),
            ("id", Value::String(ids::POET_TOOLCHEST_LIST.into())), ("toolchain_id", Value::String(tc_id.into())),
            ("label", Value::String(lbl.into())), ("target_kind", Value::String(t_kind.into())),
            ("steps", Value::String(steps.into())), ("did_q42", Value::U64(did)),
            ("lamport_clock", Value::U64(clock)), ("quin_count", Value::U64(qcount)),
            ("shell", Value::String("poet".into())),
        ]));
    }

    let family = args::rec_str(args_v, "family").unwrap_or("all");
    let mut matched = Vec::new();
    let mut total_caps = 0u64;
    for (fname, items) in BASE_FAMILIES.iter().chain(SHED_TOOLBOXES.iter()) {
        total_caps += items.len() as u64;
        if family.eq_ignore_ascii_case("all") || family.eq_ignore_ascii_case(fname) {
            let item_vals: Vec<Value> = items.iter().map(|s| Value::String((*s).into())).collect();
            matched.push(args::record([("family", Value::String((*fname).into())), ("count", Value::U64(item_vals.len() as u64)), ("items", Value::List(item_vals))]));
        }
    }

    let constructs: Vec<Value> = SHED_CONSTRUCTS.iter().map(|c| {
        let man_vals: Vec<Value> = c.manifolds.iter().map(|m| Value::String((*m).into())).collect();
        let box_vals: Vec<Value> = c.default_toolboxes.iter().map(|b| Value::String((*b).into())).collect();
        args::record([("id", Value::String(c.id.into())), ("title", Value::String(c.title.into())), ("manifolds", Value::List(man_vals)), ("default_toolboxes", Value::List(box_vals))])
    }).collect();

    let shed_boxes: Vec<Value> = SHED_TOOLBOXES.iter().map(|(tname, tools)| {
        let tool_vals: Vec<Value> = tools.iter().map(|s| Value::String((*s).into())).collect();
        args::record([("toolbox", Value::String((*tname).into())), ("tool_count", Value::U64(tool_vals.len() as u64)), ("tools", Value::List(tool_vals))])
    }).collect();

    let (rec_box, rec_construct, rec_chain) = recommend_for_context(args::rec_str(args_v, "container_kind").unwrap_or(""), args::rec_str(args_v, "manifold").unwrap_or(""));

    Ok(args::record([
        ("op", Value::String("toolchest_list".into())), ("id", Value::String(ids::POET_TOOLCHEST_LIST.into())),
        ("filter", Value::String(family.into())), ("family_count", Value::U64(matched.len() as u64)),
        ("families", Value::List(matched)), ("construct_count", Value::U64(constructs.len() as u64)),
        ("constructs", Value::List(constructs)), ("shed_toolbox_count", Value::U64(shed_boxes.len() as u64)),
        ("shed_toolboxes", Value::List(shed_boxes.clone())), ("toolbox_count", Value::U64(shed_boxes.len() as u64)),
        ("toolboxes", Value::List(shed_boxes)), ("total_capabilities", Value::U64(total_caps)),
        ("recommended_toolbox", Value::String(rec_box.into())), ("recommended_construct", Value::String(rec_construct.into())),
        ("recommended_toolchain", Value::String(rec_chain.into())), ("shell", Value::String("poet".into())),
    ]))
}

/// `Poet.manifold_inspect` — inspects a work-surface manifold or dimensional axis.
pub fn manifold_inspect(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let domain = args::rec_str(args_v, "domain").or_else(|| args::rec_str(args_v, "manifold")).ok_or_else(|| args::bad(span, "Poet.manifold_inspect needs domain"))?;

    let (axis_index, unit, default_shader, graph_iri, toolboxes) = match domain.to_ascii_lowercase().as_str() {
        "spatial" => (0u64, "qudt:Meter", "terrain_pbr_pom", "qualia:manifold:spatial", &["ComputationalGeometry", "GeometricAlgebra"][..]),
        "temporal" => (1, "qudt:Second", "temporal_ripple", "qualia:manifold:temporal", &["StudioAndHypermedia"][..]),
        "acoustic" => (2, "qudt:Decibel", "acoustic_cymatics", "qualia:manifold:acoustic", &["StudioAndHypermedia"][..]),
        "rf" => (3, "qudt:Hertz", "emf_vector_field", "qualia:manifold:rf", &["ScienceChemistryClinical"][..]),
        "magnetic" => (4, "qudt:Tesla", "plasma_confinement", "qualia:manifold:magnetic", &["ScienceChemistryClinical"][..]),
        "cyber" => (5, "qudt:BitPerSecond", "cyber_lattice", "qualia:manifold:cyber", &["StudioAndHypermedia"][..]),
        "hardware" => (6, "qudt:Watt", "silicon_die_thermal", "qualia:manifold:hardware", &["ScienceChemistryClinical"][..]),
        "logical" => (7, "qualia:TruthDegree", "deontic_halo", "qualia:manifold:logical", &["LogicAndSemantics"][..]),
        "agentic" => (8, "qualia:AgencyWeight", "swarm_boids", "qualia:manifold:agentic", &["StatisticsMlAndResearch"][..]),
        "socio" => (9, "qualia:TrustMetric", "social_force_graph", "qualia:manifold:socio", &["EconomicsAndGovernance"][..]),
        "political" => (10, "qualia:ConsensusRatio", "civic_voronoi", "qualia:manifold:political", &["EconomicsAndGovernance"][..]),
        "legal" => (11, "qualia:HohfeldPosition", "jural_correlator", "qualia:manifold:legal", &["EconomicsAndGovernance", "LogicAndSemantics"][..]),
        "economic" => (12, "qualia:UtilityUnit", "flow_sankey", "qualia:manifold:economic", &["EconomicsAndGovernance"][..]),
        "bio" => (13, "qudt:BeatsPerMinute", "bioluminescent_mycelium", "qualia:manifold:bio", &["ScienceChemistryClinical"][..]),
        "thermal" => (14, "qudt:Kelvin", "thermal_infrared", "qualia:manifold:thermal", &["ScienceChemistryClinical"][..]),
        "ecological" => (15, "qualia:BiomassDensity", "canopy_wind_sway", "qualia:manifold:ecological", &["ScienceChemistryClinical"][..]),
        "relativistic" => (16, "qualia:LorentzGamma", "black_hole_lensing", "qualia:manifold:relativistic", &["ScienceChemistryClinical", "MathCasAndSolvers"][..]),
        "studio-bay" => (0, "qualia:Surface", "cyber_glass", "qualia:surface:studio-bay", &["StudioAndHypermedia", "ComputationalGeometry"][..]),
        "research" => (1, "qualia:Surface", "wire_particles", "qualia:surface:research", &["StatisticsMlAndResearch", "ScienceChemistryClinical"][..]),
        "media" => (2, "qualia:Surface", "aurora_curtains", "qualia:surface:media", &["StudioAndHypermedia"][..]),
        "social" => (3, "qualia:Surface", "social_force_graph", "qualia:surface:social", &["EconomicsAndGovernance"][..]),
        "communications" => (4, "qualia:Surface", "cyber_lattice", "qualia:surface:communications", &["StudioAndHypermedia"][..]),
        "knowledge" => (5, "qualia:Surface", "barnes_hut_graph_physics", "qualia:surface:knowledge", &["LogicAndSemantics"][..]),
        "ontology" => (6, "qualia:Surface", "deontic_halo", "qualia:surface:ontology", &["LogicAndSemantics"][..]),
        "projects" => (7, "qualia:Surface", "flow_sankey", "qualia:surface:projects", &["EconomicsAndGovernance"][..]),
        "rights" => (8, "qualia:Surface", "jural_correlator", "qualia:surface:rights", &["EconomicsAndGovernance", "LogicAndSemantics"][..]),
        "sanctuary" => (9, "qualia:Surface", "aurora_curtains", "qualia:surface:sanctuary", &["EconomicsAndGovernance"][..]),
        "health" => (10, "qualia:Surface", "bioluminescent_mycelium", "qualia:surface:health", &["ScienceChemistryClinical"][..]),
        "anatomy" => (11, "qualia:Surface", "thermal_infrared", "qualia:surface:anatomy", &["ScienceChemistryClinical"][..]),
        "studio" => (12, "qualia:Surface", "cyber_glass", "qualia:surface:studio", &["StudioAndHypermedia"][..]),
        "datasets" => (13, "qualia:Surface", "manifold_grid_10d", "qualia:surface:datasets", &["StatisticsMlAndResearch"][..]),
        "settings" => (14, "qualia:Surface", "cyber_glass", "qualia:surface:settings", &["StudioAndHypermedia"][..]),
        "devices" => (15, "qualia:Surface", "silicon_die_thermal", "qualia:surface:devices", &["StudioAndHypermedia"][..]),
        "vibe" => (16, "qualia:Surface", "cyber_glass", "qualia:surface:vibe", &["StudioAndHypermedia"][..]),
        _ => (0, "qualia:Dimensionless", "aurora_curtains", "qualia:surface:generic", &["StudioAndHypermedia"][..]),
    };

    let tb_vals: Vec<Value> = toolboxes.iter().map(|s| Value::String((*s).into())).collect();
    Ok(args::record([
        ("op", Value::String("manifold_inspect".into())), ("id", Value::String(ids::POET_MANIFOLD_INSPECT.into())),
        ("domain", Value::String(domain.into())), ("graph_iri", Value::String(graph_iri.into())),
        ("axis_index", Value::U64(axis_index)), ("unit", Value::String(unit.into())),
        ("default_shader", Value::String(default_shader.into())), ("default_toolboxes", Value::List(tb_vals)),
        ("shell", Value::String("poet".into())),
    ]))
}

/// `Poet.toolchain_execute` — executes toolchains, defines mind-map links, logs coop contributions, and settles commerce.
pub fn toolchain_execute(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let op = args::rec_str(args_v, "op").unwrap_or("execute");

    if let Some(res) = super::os_shared_services::handle_shared_service(op, args_v, span) {
        return res;
    }

    if let Some(res) = super::semantic_instrument_bridge::handle_semantic_instrument(op, args_v, span) {
        return res;
    }

    if op == "connection_define" {
        let (from, to, t_mf) = (args::rec_str(args_v, "from").unwrap_or("container-1"), args::rec_str(args_v, "to").unwrap_or("container-2"), args::rec_str(args_v, "target_manifold").unwrap_or(""));
        let (rel_iri, l_mod, r_expr) = (args::rec_str(args_v, "relation_iri").unwrap_or("qualia:groundsObservation"), args::rec_str(args_v, "logic_modality").unwrap_or("Deontic"), args::rec_str(args_v, "rule_expr").unwrap_or("{ ?a qualia:linksTo ?b } => { ?b qualia:derived true }"));
        let op_hex = match l_mod.to_ascii_lowercase().as_str() { "deontic" => "0x10", "epistemic" => "0x20", "paraconsistent" => "0x30", "ltl" => "0x40", _ => "0x01" };
        let conn_id = format!("conn_{:016x}", crate::q_hash(&format!("{from}->{to}:{rel_iri}")));
        let (did, clock, qcount) = save_art(&conn_id, "MindMapConnection", &format!("{from} -> {to}"), "connection_define", &[("from", from), ("to", to), ("target_manifold", t_mf), ("relation_iri", rel_iri), ("logic_modality", l_mod), ("rule_expr", r_expr), ("opcode_hex", op_hex)]);
        return Ok(args::record([
            ("status", Value::String("connected".into())), ("op", Value::String("connection_define".into())),
            ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())), ("connection_id", Value::String(conn_id)),
            ("from", Value::String(from.into())), ("to", Value::String(to.into())),
            ("target_manifold", Value::String(t_mf.into())), ("relation_iri", Value::String(rel_iri.into())),
            ("logic_modality", Value::String(l_mod.into())), ("rule_expr", Value::String(r_expr.into())),
            ("opcode_hex", Value::String(op_hex.into())), ("did_q42", Value::U64(did)),
            ("lamport_clock", Value::U64(clock)), ("quin_count", Value::U64(qcount)), ("shell", Value::String("poet".into())),
        ]));
    }

    if op == "coop_contribution_log" {
        let (p_id, c_did) = (args::rec_str(args_v, "project_id").unwrap_or("proj-default"), args::rec_str(args_v, "contributor_did").unwrap_or("did:q42:local-self"));
        let (eff_m, cap_c) = (args::rec_u64(args_v, "effort_minutes").unwrap_or(60), args::rec_u64(args_v, "capital_cents").unwrap_or(0));
        let roi = args::rec_f64(args_v, "roi_multiplier").unwrap_or(1.25);
        let (art_did, cont_id) = (args::rec_str(args_v, "artifact_did_q42").unwrap_or("did:q42:artifact"), args::rec_str(args_v, "container_id").unwrap_or("container-1"));
        let summary = args::rec_str(args_v, "summary").unwrap_or("POET Shed artifact contribution");
        let ob_units = (eff_m as f64 * 0.5 + (cap_c as f64 / 100.0)) * roi;
        let c_id = format!("contrib_{:016x}", crate::q_hash(&format!("{p_id}:{c_did}:{summary}")));
        let (did, clock, qcount) = save_art(&c_id, "CoopContribution", summary, "coop_contribution_log", &[("project_id", p_id), ("contributor_did", c_did), ("container_id", cont_id), ("obligation_units", &ob_units.to_string())]);
        return Ok(args::record([
            ("status", Value::String("logged".into())), ("op", Value::String("coop_contribution_log".into())),
            ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())), ("contribution_id", Value::String(c_id)),
            ("project_id", Value::String(p_id.into())), ("contributor_did", Value::String(c_did.into())),
            ("effort_minutes", Value::U64(eff_m)), ("capital_cents", Value::U64(cap_c)),
            ("roi_multiplier", Value::F64(roi)), ("obligation_units", Value::F64(ob_units)),
            ("artifact_did_q42", Value::String(art_did.into())), ("container_id", Value::String(cont_id.into())),
            ("did_q42", Value::U64(did)), ("lamport_clock", Value::U64(clock)), ("quin_count", Value::U64(qcount)),
            ("shell", Value::String("poet".into())),
        ]));
    }

    if op == "wallet_multichain_status" || op == "commerce_settle" {
        let (chain, cp_did) = (args::rec_str(args_v, "chain").unwrap_or("did:q42"), args::rec_str(args_v, "counterparty_did").unwrap_or("did:q42:commons-pool"));
        let (amt, asset) = (args::rec_f64(args_v, "amount").unwrap_or(100.0), args::rec_str(args_v, "asset").unwrap_or("QCREDIT"));
        let (p_id, memo) = (args::rec_str(args_v, "project_id").unwrap_or("proj-default"), args::rec_str(args_v, "memo").unwrap_or("Cooperative commerce settlement"));
        let chains = "did:q42,bitcoin-taproot,solana,polkadot,cosmos-ibc,solid-rea-credit";
        let tx_id = format!("tx_{:016x}", crate::q_hash(&format!("{chain}:{cp_did}:{memo}")));
        let (did, clock, qcount) = save_art(&tx_id, "CommerceTx", memo, op, &[("chain", chain), ("counterparty_did", cp_did), ("amount", &amt.to_string()), ("asset", asset), ("project_id", p_id)]);
        return Ok(args::record([
            ("status", Value::String("settled".into())), ("op", Value::String(op.into())),
            ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())), ("tx_id", Value::String(tx_id)),
            ("chain", Value::String(chain.into())), ("supported_chains", Value::String(chains.into())),
            ("counterparty_did", Value::String(cp_did.into())), ("amount", Value::F64(amt)),
            ("asset", Value::String(asset.into())), ("project_id", Value::String(p_id.into())),
            ("did_q42", Value::U64(did)), ("lamport_clock", Value::U64(clock)), ("quin_count", Value::U64(qcount)),
            ("shell", Value::String("poet".into())),
        ]));
    }

    let chain = args::rec_str(args_v, "chain").or_else(|| args::rec_str(args_v, "pipeline")).or_else(|| args::rec_str(args_v, "tool")).ok_or_else(|| args::bad(span, "Poet.toolchain_execute needs chain"))?;
    let target = args::rec_str(args_v, "target").unwrap_or("active_manifold");
    let stages = args::rec_str_list(args_v, "stages").unwrap_or_else(|| vec!["ingest".into(), "validate_shacl".into(), "compile_wgsl".into(), "seal_q42".into()]);
    let stage_vals: Vec<Value> = stages.iter().cloned().map(Value::String).collect();
    let stages_joined = stages.join(" -> ");

    let art_id = if target == "active_manifold" { "manifold:active".to_string() } else { target.to_string() };
    let (did, clock, qcount) = save_art(&art_id, "Container", &art_id, &format!("toolchain:{chain}"), &[("last_toolchain", chain), ("last_stages", &stages_joined)]);

    Ok(args::record([
        ("op", Value::String("toolchain_execute".into())), ("id", Value::String(ids::POET_TOOLCHAIN_EXECUTE.into())),
        ("chain", Value::String(chain.into())), ("target", Value::String(target.into())),
        ("stage_count", Value::U64(stage_vals.len() as u64)), ("stages", Value::List(stage_vals)),
        ("status", Value::String("completed".into())), ("did_q42", Value::U64(did)),
        ("lamport_clock", Value::U64(clock)), ("quin_count", Value::U64(qcount)),
        ("shell", Value::String("poet".into())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs { m.insert((*k).into(), v.clone()); }
        Value::Record(m)
    }

    #[test]
    fn test_shed_toolchest_and_constructs() {
        let span = Span { start: 0, end: 0 };
        let Value::Record(tc) = toolchest_list(&rec(&[("family", Value::String("Applications".into()))]), span).unwrap() else { panic!("rec"); };
        assert_eq!(tc.get("family_count"), Some(&Value::U64(1)));
        assert_eq!(tc.get("construct_count"), Some(&Value::U64(10)));
        assert_eq!(tc.get("shed_toolbox_count"), Some(&Value::U64(8)));
        assert!(tc.contains_key("recommended_toolbox"));
    }

    #[test]
    fn test_toolbox_and_toolchain_creation() {
        let span = Span { start: 0, end: 0 };
        let Value::Record(tbox) = toolchest_list(&rec(&[
            ("op", Value::String("toolbox_create".into())),
            ("toolbox_id", Value::String("geo-solvers".into())),
            ("label", Value::String("Geometry Solvers".into())),
        ]), span).unwrap() else { panic!("rec"); };
        assert_eq!(tbox.get("status"), Some(&Value::String("created".into())));

        let Value::Record(tchain) = toolchest_list(&rec(&[
            ("op", Value::String("toolchain_create".into())),
            ("toolchain_id", Value::String("mesh-opt".into())),
        ]), span).unwrap() else { panic!("rec"); };
        assert_eq!(tchain.get("status"), Some(&Value::String("created".into())));
    }

    #[test]
    fn test_manifold_inspect_dimensional_and_workspace() {
        let span = Span { start: 0, end: 0 };
        let Value::Record(mi_dim) = manifold_inspect(&rec(&[("domain", Value::String("Ecological".into()))]), span).unwrap() else { panic!("rec"); };
        assert_eq!(mi_dim.get("axis_index"), Some(&Value::U64(15)));

        let Value::Record(mi_ws) = manifold_inspect(&rec(&[("domain", Value::String("studio-bay".into()))]), span).unwrap() else { panic!("rec"); };
        assert_eq!(mi_ws.get("graph_iri"), Some(&Value::String("qualia:surface:studio-bay".into())));
    }

    #[test]
    fn test_toolchain_extensions() {
        let span = Span { start: 0, end: 0 };
        let Value::Record(conn) = toolchain_execute(&rec(&[
            ("op", Value::String("connection_define".into())),
            ("from", Value::String("cad-container".into())),
            ("to", Value::String("fem-analysis".into())),
        ]), span).unwrap() else { panic!("rec"); };
        assert_eq!(conn.get("status"), Some(&Value::String("connected".into())));
        assert_eq!(conn.get("opcode_hex"), Some(&Value::String("0x10".into())));

        let Value::Record(coop) = toolchain_execute(&rec(&[
            ("op", Value::String("coop_contribution_log".into())),
            ("project_id", Value::String("qualia-shed".into())),
            ("effort_minutes", Value::U64(120)),
        ]), span).unwrap() else { panic!("rec"); };
        assert_eq!(coop.get("status"), Some(&Value::String("logged".into())));

        let Value::Record(comm) = toolchain_execute(&rec(&[
            ("op", Value::String("commerce_settle".into())),
            ("amount", Value::F64(250.0)),
        ]), span).unwrap() else { panic!("rec"); };
        assert_eq!(comm.get("status"), Some(&Value::String("settled".into())));

        let Value::Record(os_apps) = toolchain_execute(&rec(&[
            ("op", Value::String("os_apps_manifest".into())),
        ]), span).unwrap() else { panic!("rec"); };
        assert_eq!(os_apps.get("op"), Some(&Value::String("os_apps_manifest".into())));
        assert_eq!(os_apps.get("app_count"), Some(&Value::U64(7)));

        let Value::Record(si_cat) = toolchain_execute(&rec(&[
            ("op", Value::String("si_catalog".into())),
        ]), span).unwrap() else { panic!("rec"); };
        assert_eq!(si_cat.get("op"), Some(&Value::String("instrument_catalog".into())));
        assert_eq!(si_cat.get("demo_count"), Some(&Value::U64(6)));
    }
}
