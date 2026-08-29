//! P2 domain computational panels: clinical risk, DICOM viewer, comorbidity,
//! chemistry modeler, physics simulator, ODE solver, bioinformatics lab,
//! GBM/VaR simulator, diffusion controller.

use super::helpers::{
    make_button, make_results_area, make_section_label, make_select, make_text_input,
    make_textarea, make_tool_panel, show_logic_notification, show_mock_results,
};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlSelectElement, MouseEvent};

pub(super) fn append_panels(document: &Document, content: &Element) {
    content
        .append_child(&build_clinical_risk_panel(document))
        .unwrap();
    content
        .append_child(&build_dicom_viewer_panel(document))
        .unwrap();
    content
        .append_child(&build_comorbidity_panel(document))
        .unwrap();
    content
        .append_child(&build_chemistry_panel(document))
        .unwrap();
    content
        .append_child(&build_physics_panel(document))
        .unwrap();
    content
        .append_child(&build_ode_solver_panel(document))
        .unwrap();
    content
        .append_child(&build_bioinformatics_panel(document))
        .unwrap();
    content
        .append_child(&build_gbm_var_panel(document))
        .unwrap();
    content
        .append_child(&build_diffusion_panel(document))
        .unwrap();
}

pub(super) fn wire_all(document: &Document) {
    wire_clinical_risk_panel(document);
    wire_dicom_viewer_panel(document);
    wire_comorbidity_panel(document);
    wire_chemistry_panel(document);
    wire_physics_panel(document);
    wire_ode_solver_panel(document);
    wire_bioinformatics_panel(document);
    wire_gbm_var_panel(document);
    wire_diffusion_panel(document);
}

pub(super) fn build_clinical_risk_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "clinical_risk", false);
    panel
        .append_child(&make_section_label(
            document,
            "Clinical Risk Scorer \u{2014} Framingham, CHA\u{2082}DS\u{2082}-VASc, SCORE2, drug interactions",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "clinical-risk-model",
        &[
            ("framingham", "Framingham 10yr CVD"),
            ("cha2ds2_vasc", "CHA\u{2082}DS\u{2082}-VASc Stroke"),
            ("score2", "SCORE2 European CVD"),
            ("drug_interaction", "Drug Interaction Check"),
            ("contraindication", "Contraindication Check"),
            ("fhir_observation", "FHIR Observation Validate"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "clinical-risk-input",
            "# Clinical risk input (model-dependent)\n# Framingham: age, sex, total_chol, hdl, sys_bp, smoker, diabetes\n# CHA2DS2-VASc: chf, hypertension, age>=75, diabetes, stroke, vascular, age>=65, female\n# Drug interaction: drug_list with CYP450 pathways\n\npatient(age=65, sex=male, total_chol=240, hdl=45, sys_bp=140, smoker=true, diabetes=true).",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "clinical-risk-evaluate",
            "\u{1F49A} Compute Risk",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "clinical-risk-results",
            "Choose a clinical model, enter its required fields, and run the native validated calculation.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_clinical_risk_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("clinical-risk-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let model = doc
                .get_element_by_id("clinical-risk-model")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|s| s.value())
                .unwrap_or_default();
            show_mock_results(
                &doc,
                "clinical-risk-results",
                &format!("clinical-{}", model),
            );
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_dicom_viewer_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "dicom_viewer", false);
    panel
        .append_child(&make_section_label(
            document,
            "DICOM Viewer \u{2014} diffusion frame proxy, RGBA8 rendering, window/level",
        ))
        .unwrap();
    panel
        .append_child(&make_text_input(document, "dicom-study-uid", "Study UID"))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "dicom-metadata",
            "# De-identified CT slice samples\nwidth=4, height=4\npixels=[-1000,-700,-300,-160,0,20,40,80,120,180,240,400,600,800,1000,1200]\nwindow=400, level=40",
            "100px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "dicom-render",
            "\u{1F5BC} Render Slice",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "dicom-window-level",
            "\u{1F4A1} Window/Level",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "dicom-results",
            "Enter a Study UID and bounded HU slice samples to apply native window/level rendering.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_dicom_viewer_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("dicom-render") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "dicom-results", "dicom-render");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("dicom-window-level") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "dicom-results", "dicom-render");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_comorbidity_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "comorbidity", false);
    panel
        .append_child(&make_section_label(
            document,
            "Comorbidity Analyzer \u{2014} multi-condition interaction, risk adjustment",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "comorbidity-input",
            "# Consent-safe pseudonymous patient context\npatient=did:patient:ANON001\ntarget_organ=Heart\nconditions=[Type 2 Diabetes Mellitus|Hypertension|Heart]\n# Optional single compounding edge\nantecedent=Type 2 Diabetes Mellitus, consequent=Heart, severity=0.8",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "comorbidity-evaluate",
            "\u{1F9EA} Analyze",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "comorbidity-results",
            "Enter patient-scoped conditions to run the zero-allocation comorbidity evaluator.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_comorbidity_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("comorbidity-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "comorbidity-results", "comorbidity");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_chemistry_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "chemistry", false);
    panel
        .append_child(&make_section_label(
            document,
            "Chemistry Modeler \u{2014} SMILES, descriptors, drug-likeness, green metrics",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "chemistry-op",
        &[
            ("smiles_validate", "Validate SMILES"),
            ("mw", "Molecular Weight"),
            ("logp", "Crippen LogP"),
            ("tpsa", "TPSA"),
            ("lipinski", "Lipinski Rule-of-5"),
            ("veber", "Veber Bioavailability"),
            ("ghose", "Ghose Drug-likeness"),
            ("egan", "Egan Absorption"),
            ("functional_groups", "Functional Groups"),
            ("pka", "pKa Estimate"),
            ("chiral", "Chiral Centers"),
            ("fingerprint", "Circular Fingerprint"),
            ("arrhenius", "Arrhenius Rate"),
            ("gibbs", "Gibbs Free Energy"),
            ("equilibrium", "Equilibrium Constant"),
            ("henderson", "Henderson-Hasselbalch"),
            ("atom_economy", "Atom Economy"),
            ("e_factor", "E-Factor"),
            ("green_metrics", "Green Metrics Suite"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_text_input(
            document,
            "chemistry-smiles",
            "SMILES string (e.g. CC(=O)Oc1ccccc1C(=O)O)",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "chemistry-params",
            "# Additional parameters (operation-dependent)\n# arrhenius: A=1e10, Ea=50000, T=298\n# gibbs: dH=-100000, dS=-50, T=298\n# equilibrium: dG=-5000, T=298\n# henderson: pKa=4.8, base=0.1, acid=0.2\n# atom_economy: reactant_mws=[100,80], product_mw=120\n# e_factor: waste_kg=4, product_kg=2\n# green_metrics: reactant_mws=[100,80], byproduct_mws=[20], product_mw=120, yield_fraction=0.8, solvent_kg=1, product_kg=1, reactant_c_atoms=8, product_c_atoms=6",
            "150px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "chemistry-evaluate",
            "\u{1F9EA} Compute",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "chemistry-results",
            "Choose an operation and run it through the bounded native organic-chemistry engine.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_chemistry_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("chemistry-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let op = doc
                .get_element_by_id("chemistry-op")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|s| s.value())
                .unwrap_or_default();
            show_mock_results(&doc, "chemistry-results", &format!("chemistry-{}", op));
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_physics_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "physics", false);
    panel
        .append_child(&make_section_label(
            document,
            "Physics Simulator \u{2014} thermodynamics, MCMC, RK4 ODE, DFT, PINN, off-grid energy",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "physics-op",
        &[
            ("metropolis", "Metropolis MCMC"),
            ("ode_solver", "RK4 ODE Solver"),
            ("dft", "Kohn-Sham DFT"),
            ("pinn", "PINN Receptor Binding"),
            ("gibbs", "Gibbs Free Energy"),
            ("cell_ocv", "Cell OCV"),
            ("pack_ocv", "Pack OCV"),
            ("terminal_voltage", "Terminal Voltage"),
            ("deliverable_power", "Deliverable Power"),
            ("max_power_point", "Max Power Point"),
            ("array_mppt", "Array MPPT Power"),
            ("heat_loss", "Heat Loss Rate"),
            ("phase_change", "Phase Change Energy"),
            ("thermal_efficiency", "Thermal Efficiency"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "physics-input",
            "# Enter one operation's validated parameters\n# Metropolis: temperature=298, ensemble_size=1000, steps=1000, proposal_scale=0.025, seed=42\n# ODE: y1=1.0, y2=0.0, k1=0.5, k2=0.3, coupling=1.0, dt=0.01, steps=1000\n# DFT: electron_count=2, resolution=16\n# PINN: molecule_features=[0.2,0.7], receptor_features=[0.3,0.6]\n# Gibbs: temperature=298, enthalpy=10000, entropy=20\n# Battery: soc=0.8, cells_series=4, cells_parallel=2, cell_resistance=0.005, cell_capacity_ah=100, load_current=50\n# Solar: short_circuit_current=8, open_circuit_voltage=40, fill_factor=0.75, scan_steps=256, panel_count=2\n# Heat: u_value=0.5, area=10, delta_t=20, useful_power=1000\n# Phase: mass=2, latent_heat=334000",
            "190px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "physics-evaluate",
            "\u{269B} Simulate",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "physics-results",
            "Enter the selected operation's parameters, then run the bounded native simulation.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_physics_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("physics-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let op = doc
                .get_element_by_id("physics-op")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|s| s.value())
                .unwrap_or_default();
            show_mock_results(&doc, "physics-results", &format!("physics-{}", op));
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_ode_solver_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "ode_solver", false);
    panel
        .append_child(&make_section_label(
            document,
            "ODE Solver \u{2014} RK4 time-stepper with chaining, SIMD width detection",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "ode-input",
            "# Coupled bounded RK4 system\n# dy1/dt = -k1*y1\n# dy2/dt = coupling*y1 - k2*y2\ny1=1.0, y2=0.0, k1=0.5, k2=0.3, coupling=1.0, dt=0.01, steps=1000",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "ode-evaluate",
            "\u{1F501} Solve ODE",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "ode-results",
            "Enter a coupled system and solve it with the bounded native RK4 integrator.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_ode_solver_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("ode-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "ode-results", "ode-solver");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_bioinformatics_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "bioinformatics", false);
    panel
        .append_child(&make_section_label(
            document,
            "Bioinformatics Lab \u{2014} Smith-Waterman, protein alignment, k-mer, FASTA, Tanimoto",
        ))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center; flex-wrap: wrap;");
    row.append_child(&make_select(
        document,
        "bioinformatics-op",
        &[
            ("nucleotide_align", "Nucleotide Alignment (SW)"),
            ("protein_align", "Protein Alignment (BLOSUM62)"),
            ("kmer_frequency", "K-mer Frequency"),
            ("fasta_validate", "FASTA Validation"),
            ("gene_expression", "Gene Expression Fold-Change"),
            ("metabolite_similarity", "Metabolite Tanimoto"),
            ("needleman_wunsch", "Needleman-Wunsch"),
            ("minhash", "MinHash Sketch"),
            ("upgma_tree", "UPGMA Tree"),
        ],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "bioinformatics-input",
            "# Bioinformatics input (operation-dependent)\n# alignment: seq1=\"ACGTACGT\", seq2=\"ACGTTCGT\"\n# k-mer/minhash: sequence=\"ACGTACGTACGT\", k=3, sketch_size=64\n# FASTA: header=\"sample-1\", sequence=\"ACGTACGT\"\n# gene expression: gene=\"TP53\", baseline=100, treatment=350, threshold=2\n# Tanimoto: fingerprint1=[3,12], fingerprint2=[3,8]\n# UPGMA: n=3, distances=[0,1,2,1,0,1.5,2,1.5,0]",
            "140px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "bioinformatics-evaluate",
            "\u{1F9EC} Analyze",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "bioinformatics-results",
            "Choose an operation and run it through the bounded native bioinformatics engine.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_bioinformatics_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("bioinformatics-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let op = doc
                .get_element_by_id("bioinformatics-op")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|s| s.value())
                .unwrap_or_default();
            show_mock_results(&doc, "bioinformatics-results", &format!("bio-{}", op));
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_gbm_var_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "gbm_var", false);
    panel
        .append_child(&make_section_label(
            document,
            "GBM / VaR Simulator \u{2014} Geometric Brownian Motion, Monte Carlo Value-at-Risk",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "gbm-var-input",
            "# Deterministic GBM path + Monte Carlo VaR\ngbm(s0=100, mu=0.05, sigma=0.2, T=1.0, dt=0.01).\nvar(portfolio=1000000, confidence=0.95, paths=2048, seed=42).",
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "gbm-var-evaluate",
            "\u{1F4B9} Simulate",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "gbm-var-results",
            "Enter GBM and portfolio parameters to run the deterministic bounded native simulation.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_gbm_var_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("gbm-var-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "gbm-var-results", "gbm-var");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_diffusion_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "diffusion", false);
    panel
        .append_child(&make_section_label(
            document,
            "Diffusion Controller \u{2014} diffusion pass execution, reconfiguration",
        ))
        .unwrap();
    let default_config = "# Bounded 1D heat diffusion (insulated ends)\ninitial=[0,0,0,1,1,1,0,0,0]\nalpha=0.1, dx=0.1, total_time=5.0, samples=10";
    let saved_config = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item("poet.diffusion.config").ok().flatten())
        .unwrap_or_else(|| default_config.to_string());
    panel
        .append_child(&make_textarea(
            document,
            "diffusion-input",
            &saved_config,
            "120px",
        ))
        .unwrap();
    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "diffusion-evaluate",
            "\u{1F300} Execute Pass",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "diffusion-reconfigure",
            "\u{2699} Reconfigure",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();
    panel
        .append_child(&make_results_area(
            document,
            "diffusion-results",
            "Execute the configured bounded native diffusion pass, or persist a revised configuration locally.",
        ))
        .unwrap();
    panel
}

pub(super) fn wire_diffusion_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("diffusion-evaluate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "diffusion-results", "diffusion");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("diffusion-reconfigure") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let config = super::helpers::field_value(&doc, "diffusion-input");
            let saved = web_sys::window()
                .and_then(|window| window.local_storage().ok().flatten())
                .is_some_and(|storage| storage.set_item("poet.diffusion.config", &config).is_ok());
            show_logic_notification(
                &doc,
                if saved {
                    "Diffusion configuration validated on execution and saved locally."
                } else {
                    "Unable to persist diffusion configuration in browser storage."
                },
            );
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
