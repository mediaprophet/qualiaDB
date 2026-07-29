use crate::cli::EvaluateModality;
use crate::evaluate;

pub fn handle(modality: &EvaluateModality) {
    match modality {
        EvaluateModality::Deontic {
            dataset,
            contract_hash,
        } => {
            evaluate::run_deontic(dataset, *contract_hash);
        }
        EvaluateModality::Epistemic {
            dataset,
            agent_hash,
        } => {
            evaluate::run_epistemic(dataset, *agent_hash);
        }
        EvaluateModality::Paraconsistent { dataset } => {
            evaluate::run_paraconsistent(dataset);
        }
        EvaluateModality::Ltl {
            dataset,
            formula_type,
            hash_a,
            hash_b,
        } => {
            evaluate::run_ltl(dataset, formula_type, *hash_a, *hash_b);
        }
        EvaluateModality::Asp {
            dataset,
            base_index,
        } => {
            evaluate::run_asp(dataset, *base_index);
        }
        EvaluateModality::Dl {
            dataset,
            sub_class,
            super_class,
        } => {
            evaluate::run_dl(dataset, *sub_class, *super_class);
        }
        EvaluateModality::Probabilistic { weight, threshold } => {
            evaluate::run_probabilistic(*weight, *threshold);
        }
        EvaluateModality::LinearLogic {
            dataset,
            quin_index,
        } => {
            evaluate::run_linear_logic(dataset, *quin_index);
        }
        EvaluateModality::Dialectical {
            dataset,
            var1,
            var2,
        } => {
            evaluate::run_dialectical(dataset, *var1, *var2);
        }
        EvaluateModality::Diffusion { graph_id } => {
            evaluate::run_diffusion(graph_id);
        }
        EvaluateModality::SpatioTemporal {
            action,
            ax1,
            ay1,
            ax2,
            ay2,
            bx1,
            by1,
            bx2,
            by2,
        } => {
            evaluate::run_spatio_temporal(action, *ax1, *ay1, *ax2, *ay2, *bx1, *by1, *bx2, *by2);
        }
        EvaluateModality::Interval {
            action,
            start1,
            end1,
            start2,
            end2,
            point,
        } => {
            evaluate::run_interval(action, *start1, *end1, *start2, *end2, *point);
        }
        EvaluateModality::GraphTopology { dataset, context } => {
            evaluate::run_graph_topology(dataset, *context);
        }
        EvaluateModality::Argumentation { demo, dataset } => {
            evaluate::run_argumentation(*demo, dataset.as_deref());
        }
        EvaluateModality::ControlFeedback {
            kp,
            ki,
            kd,
            setpoint,
            measurement,
        } => {
            evaluate::run_control_feedback(*kp, *ki, *kd, *setpoint, *measurement);
        }
        EvaluateModality::NeuroSymbolic { demo: _ } => {
            evaluate::run_neuro_symbolic();
        }
    }
}
