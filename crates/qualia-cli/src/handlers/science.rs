use crate::cli::{
    BioAction, ChemAction, ClinicalAction, EconomicsAction, GeoAction, GeometricAction,
    ScienceAction, ThermoAction,
};
use crate::science;

pub fn handle(action: &ScienceAction) {
    match action {
        ScienceAction::Chem { action } => match action {
            ChemAction::Smiles { smiles } => science::run_chem_smiles(smiles),
            ChemAction::Thermo { reaction, a, b, c } => {
                science::run_chem_thermo(reaction, *a, *b, *c)
            }
            ChemAction::DrugLike { smiles } => science::run_chem_druglike(smiles),
            ChemAction::Pka {
                pka,
                conc_base,
                conc_acid,
            } => science::run_chem_pka(*pka, *conc_base, *conc_acid),
        },
        ScienceAction::Bio { action } => match action {
            BioAction::Align {
                query,
                target,
                mode,
            } => science::run_bio_align(query, target, mode),
            BioAction::Kmer { sequence, k } => science::run_bio_kmer(sequence, *k),
            BioAction::Translate { dna } => science::run_bio_translate(dna),
            BioAction::Isoelectric { protein } => science::run_bio_isoelectric(protein),
            BioAction::Jaccard { sketch_a, sketch_b } => {
                science::run_bio_jaccard(sketch_a, sketch_b)
            }
            BioAction::Minhash { sequence, k, size } => {
                science::run_bio_minhash(sequence, *k, *size)
            }
        },
        ScienceAction::Geo { action } => match action {
            GeoAction::EmbedH3 { index } => science::run_geo_embed_h3(*index),
        },
        ScienceAction::Thermo { action } => match action {
            ThermoAction::Gibbs {
                enthalpy,
                entropy,
                temp,
            } => science::run_thermo_gibbs(*enthalpy, *entropy, *temp),
            ThermoAction::Anneal {
                initial_temp,
                particles,
                proposed_energy,
                random,
            } => {
                science::run_thermo_anneal(*initial_temp, *particles, *proposed_energy, *random);
            }
        },
        ScienceAction::Geometric { action } => match action {
            GeometricAction::Cross { a, b } => science::run_geometric_cross(a, b),
            GeometricAction::Angle { a, b } => science::run_geometric_angle(a, b),
        },
        ScienceAction::Clinical { action } => match action {
            ClinicalAction::Framingham {
                age,
                sex_male,
                total_chol,
                hdl_chol,
                systolic_bp,
                bp_treated,
                smoker,
                diabetic,
            } => {
                science::run_clinical_framingham(
                    *age,
                    *sex_male,
                    *total_chol,
                    *hdl_chol,
                    *systolic_bp,
                    *bp_treated,
                    *smoker,
                    *diabetic,
                );
            }
            ClinicalAction::Sofa {
                pao2_fio2,
                platelets,
                bilirubin,
                map,
                gcs,
                creatinine,
            } => {
                science::run_clinical_sofa(
                    *pao2_fio2,
                    *platelets,
                    *bilirubin,
                    *map,
                    *gcs,
                    *creatinine,
                );
            }
            ClinicalAction::Ckd {
                age,
                sex_male,
                weight_kg,
                creatinine,
            } => {
                science::run_clinical_ckd(*age, *sex_male, *weight_kg, *creatinine);
            }
            ClinicalAction::Pk {
                dose_mg,
                vd_l,
                cl_l_hr,
                time_hr,
            } => {
                science::run_clinical_pk(*dose_mg, *vd_l, *cl_l_hr, *time_hr);
            }
            ClinicalAction::DrugInteractions { drug_names } => {
                science::run_clinical_drug_interactions(drug_names);
            }
        },
        ScienceAction::Economics { action } => match action {
            EconomicsAction::Gbm {
                price,
                drift,
                vol,
                horizon,
                steps,
            } => {
                science::run_economics_gbm(*price, *drift, *vol, *horizon, *steps);
            }
            EconomicsAction::Var {
                price,
                drift,
                vol,
                horizon,
                steps,
                paths,
            } => {
                science::run_economics_var(*price, *drift, *vol, *horizon, *steps, *paths);
            }
            EconomicsAction::Macro {
                m0,
                p0,
                velocity,
                real_gdp,
                horizon,
                steps,
            } => {
                science::run_economics_macro(*m0, *p0, *velocity, *real_gdp, *horizon, *steps);
            }
            EconomicsAction::Bond {
                face,
                coupon_rate,
                yield_rate,
                periods,
            } => {
                science::run_economics_bond(*face, *coupon_rate, *yield_rate, *periods);
            }
            EconomicsAction::Paper { qty, last_price } => {
                science::run_economics_paper(*qty, *last_price);
            }
            EconomicsAction::Welfare { incomes } => {
                science::run_economics_welfare(&incomes);
            }
            EconomicsAction::Game { market_a } => {
                science::run_economics_game(*market_a);
            }
        },
    }
}
