use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ScienceAction {
    Chem { #[command(subcommand)] action: ChemAction },
    Bio { #[command(subcommand)] action: BioAction },
    Geo { #[command(subcommand)] action: GeoAction },
    Thermo { #[command(subcommand)] action: ThermoAction },
    Geometric { #[command(subcommand)] action: GeometricAction },
    Clinical { #[command(subcommand)] action: ClinicalAction },
    Economics { #[command(subcommand)] action: EconomicsAction },
}

#[derive(Subcommand, Debug)]
pub enum ChemAction {
    Smiles { smiles: String },
    Thermo {
        reaction: String,
        #[arg(long)] a: f64,
        #[arg(long)] b: f64,
        #[arg(long)] c: f64,
    },
    DrugLike { smiles: String },
    Pka {
        #[arg(long)] pka: f64,
        #[arg(long)] conc_base: f64,
        #[arg(long)] conc_acid: f64,
    },
}

#[derive(Subcommand, Debug)]
pub enum BioAction {
    Align {
        query: String,
        target: String,
        #[arg(long, default_value = "dna")] mode: String,
    },
    Kmer { sequence: String, #[arg(long, default_value = "4")] k: usize },
    Translate { dna: String },
    Isoelectric { protein: String },
    Jaccard { sketch_a: String, sketch_b: String },
    Minhash {
        sequence: String,
        #[arg(long, default_value = "4")] k: usize,
        #[arg(long, default_value = "128")] size: usize,
    },
}

#[derive(Subcommand, Debug)]
pub enum GeoAction {
    EmbedH3 { #[arg(long)] index: u64 },
}

#[derive(Subcommand, Debug)]
pub enum ThermoAction {
    Gibbs {
        #[arg(long)] enthalpy: f64,
        #[arg(long)] entropy: f64,
        #[arg(long)] temp: f64,
    },
    Anneal {
        #[arg(long, default_value = "300.0")] initial_temp: f64,
        #[arg(long, default_value = "1")] particles: usize,
        #[arg(long)] proposed_energy: f64,
        #[arg(long, default_value = "0.5")] random: f64,
    },
}

#[derive(Subcommand, Debug)]
pub enum GeometricAction {
    Cross { #[arg(long)] a: String, #[arg(long)] b: String },
    Angle { #[arg(long)] a: String, #[arg(long)] b: String },
}

#[derive(Subcommand, Debug)]
pub enum ClinicalAction {
    Framingham {
        #[arg(long)] age: u8,
        #[arg(long)] sex_male: bool,
        #[arg(long)] total_chol: f64,
        #[arg(long)] hdl_chol: f64,
        #[arg(long)] systolic_bp: f64,
        #[arg(long)] bp_treated: bool,
        #[arg(long)] smoker: bool,
        #[arg(long)] diabetic: bool,
    },
    Sofa {
        #[arg(long)] pao2_fio2: f64,
        #[arg(long)] platelets: f64,
        #[arg(long)] bilirubin: f64,
        #[arg(long)] map: f64,
        #[arg(long)] gcs: u8,
        #[arg(long)] creatinine: f64,
    },
    Ckd {
        #[arg(long)] age: u8,
        #[arg(long)] sex_male: bool,
        #[arg(long)] weight_kg: f64,
        #[arg(long)] creatinine: f64,
    },
    Pk {
        #[arg(long)] dose_mg: f64,
        #[arg(long)] vd_l: f64,
        #[arg(long)] cl_l_hr: f64,
        #[arg(long)] time_hr: f64,
    },
    DrugInteractions { drug_names: String },
}

#[derive(Subcommand, Debug)]
pub enum EconomicsAction {
    Gbm {
        #[arg(long, default_value = "100.0")] price: f64,
        #[arg(long, default_value = "0.05")] drift: f64,
        #[arg(long, default_value = "0.2")] vol: f64,
        #[arg(long, default_value = "1.0")] horizon: f64,
        #[arg(long, default_value = "252")] steps: usize,
    },
    Var {
        #[arg(long, default_value = "100.0")] price: f64,
        #[arg(long, default_value = "0.05")] drift: f64,
        #[arg(long, default_value = "0.2")] vol: f64,
        #[arg(long, default_value = "1.0")] horizon: f64,
        #[arg(long, default_value = "252")] steps: usize,
        #[arg(long, default_value = "1000")] paths: usize,
    },
    Macro {
        #[arg(long, default_value = "1000.0")] m0: f64,
        #[arg(long, default_value = "1.0")] p0: f64,
        #[arg(long, default_value = "2.0")] velocity: f64,
        #[arg(long, default_value = "500.0")] real_gdp: f64,
        #[arg(long, default_value = "10.0")] horizon: f64,
        #[arg(long, default_value = "100")] steps: usize,
    },
    Bond {
        #[arg(long, default_value = "100.0")] face: f64,
        #[arg(long, default_value = "0.05")] coupon_rate: f64,
        #[arg(long, default_value = "0.06")] yield_rate: f64,
        #[arg(long, default_value = "5")] periods: u32,
    },
    Paper {
        #[arg(long, default_value = "10.0")] qty: f64,
        #[arg(long, default_value = "100.5")] last_price: f64,
    },
    Welfare {
        #[arg(long, default_value = "1,2,3,10")] incomes: String,
    },
    Game {
        #[arg(long, default_value = "100")] market_a: f64,
    },
}
