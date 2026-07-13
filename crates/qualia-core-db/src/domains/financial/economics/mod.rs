//! Financial economics kernels.
//!
//! This module is intentionally a short re-export barrel. Keep concrete
//! functionality in purpose-defined submodules so finance/economics does not
//! collapse back into one oversized file.

pub mod input_output;
pub mod macro_flows;
pub mod node_pricing;
pub mod resilience;
pub mod stochastic;

pub use input_output::{propagate_supply_shock, MAX_SECTORS};
pub use macro_flows::simulate_macroeconomic_flow;
pub use node_pricing::{calculate_bandwidth_liability, get_current_system_context, SystemContext};
pub use resilience::resilience_resource_pricing;
pub use stochastic::{
    run_monte_carlo_var, run_monte_carlo_var_seeded_into, simulate_gbm_path,
    simulate_gbm_path_seeded, simulate_gbm_steps_into, StochasticError, DEFAULT_MONTE_CARLO_SEED,
};
