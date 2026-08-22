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
            "Click \"Compute Risk\" to evaluate clinical risk (mock).",
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
            "# DICOM metadata context\nstudy(uid=\"1.2.840.113619.2.55.3\", patient=\"ANON001\").\nseries(uid=\"1.2.840.113619.2.55.3.1\", modality=\"CT\", slices=128).\n\n# Query: render slice 64 with window/level 400/40",
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
            "Click \"Render Slice\" to view DICOM image (mock).",
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
            show_logic_notification(&doc, "Window/Level: 400/40 applied (mock)");
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
            "# Comorbidity context\ncondition(patient=\"ANON001\", type=\"diabetes\", severity=\"moderate\").\ncondition(patient=\"ANON001\", type=\"hypertension\", severity=\"mild\").\ncondition(patient=\"ANON001\", type=\"ckd\", stage=3).\n\n# Query: comorbidity interaction risk?\n# Query: adjusted risk score?",
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
            "Click \"Analyze\" to evaluate comorbidity interactions (mock).",
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
            "# Additional parameters (operation-dependent)\n# For arrhenius: A=1e10, Ea=50000, T=298\n# For gibbs: dH=-100, dS=50, T=298\n# For equilibrium: dG=-5000, T=298",
            "80px",
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
            "Click \"Compute\" to evaluate chemistry properties (mock).",
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
            "# Physics input (operation-dependent)\n# Metropolis: ensemble_size=1000, temperature=298, steps=10000\n# ODE: dy/dt = -k*y, y0=1.0, k=0.1, dt=0.01, steps=1000\n# DFT: atoms=[H, H], positions=[[0,0,0],[0,0,0.74]]\n# Cell OCV: chemistry=\"LiFePO4\", soc=0.8",
            "120px",
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
            "Click \"Simulate\" to run physics computation (mock).",
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
            "# ODE system definition\n# dy1/dt = -0.5 * y1\n# dy2/dt = y1 - 0.3 * y2\n# y1(0) = 1.0, y2(0) = 0.0\n# dt = 0.01, steps = 1000\n\node(system=\"damped_oscillator\", dt=0.01, steps=1000).\ninitial(y1=1.0, y2=0.0).\n\n# Query: solve and return trajectory at step 500?",
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
            "Click \"Solve ODE\" to integrate the system (mock).",
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
            "# Bioinformatics input (operation-dependent)\n# Nucleotide alignment: seq1=\"ACGTACGT\", seq2=\"ACGTTCGT\"\n# Protein alignment: seq1=\"MKTAYIAKQR\", seq2=\"MKTAYIAKQR\", matrix=\"BLOSUM62\"\n# K-mer: sequence=\"ACGTACGTACGT\", k=3\n# FASTA: record header + sequence\n# Gene expression: control=[...], treatment=[...]\n# Tanimoto: fingerprint1, fingerprint2",
            "120px",
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
            "Click \"Analyze\" to run bioinformatics computation (mock).",
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
            "# GBM / VaR parameters\n# GBM: S0=100, mu=0.05, sigma=0.2, T=1.0, dt=0.01\n# VaR: portfolio_value=1000000, confidence=0.95, horizon=252\n# Seeded: seed=42 for deterministic output\n\ngbm(s0=100, mu=0.05, sigma=0.2, T=1.0, dt=0.01).\nvar(portfolio=1000000, confidence=0.95, horizon=252, seed=42).\n\n# Query: simulate GBM path?\n# Query: compute Monte Carlo VaR?",
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
            "Click \"Simulate\" to run GBM/VaR computation (mock).",
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
    panel
        .append_child(&make_textarea(
            document,
            "diffusion-input",
            "# Diffusion parameters\n# execute_diffusion_pass: input_quins, temperature, steps\n# trigger_diffusion: source_node, target_nodes, mode\n\ndiffusion(temperature=1.0, steps=100, mode=\"gaussian\").\ntrigger(source=\"node_42\", targets=[\"node_43\", \"node_44\"], mode=\"heat_equation\").\n\n# Query: execute diffusion pass?\n# Query: reconfigure diffusion parameters?",
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
            "Click \"Execute Pass\" to run diffusion (mock).",
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
            show_logic_notification(
                &doc,
                "Diffusion reconfigured: temperature=1.0, steps=100 (mock)",
            );
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
