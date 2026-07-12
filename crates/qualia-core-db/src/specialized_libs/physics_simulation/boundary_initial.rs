use super::*;

/// Boundary conditions
pub struct BoundaryConditions {
    boundary_types: HashMap<String, BoundaryType>,
    boundary_values: HashMap<String, Vec<f64>>,
    time_dependent_boundaries: HashMap<String, TimeDependentBoundary>,
}

/// Boundary types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BoundaryType {
    /// Dirichlet boundary
    Dirichlet,
    /// Neumann boundary
    Neumann,
    /// Robin boundary
    Robin,
    /// Periodic boundary
    Periodic,
    /// Symmetry boundary
    Symmetry,
    /// Wall boundary
    Wall,
    /// Inflow boundary
    Inflow,
    /// Outflow boundary
    Outflow,
    /// Far-field boundary
    FarField,
}

/// Time-dependent boundary
#[derive(Debug, Clone)]
pub struct TimeDependentBoundary {
    pub boundary_id: String,
    pub time_function: TimeFunction,
    pub spatial_function: Option<SpatialFunction>,
}

/// Time functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TimeFunction {
    Constant(f64),
    Linear(f64, f64),
    Sinusoidal(f64, f64, f64),
    Exponential(f64, f64),
    Piecewise(Vec<(f64, f64, TimeFunction)>),
    Custom(String),
}

/// Spatial functions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SpatialFunction {
    Constant(f64),
    Linear(Vec<f64>),
    Quadratic(Vec<f64>),
    Polynomial(Vec<f64>),
    Trigonometric(String, Vec<f64>),
    Custom(String),
}

/// Initial conditions
pub struct InitialConditions {
    condition_types: HashMap<String, InitialConditionType>,
    condition_values: HashMap<String, Vec<f64>>,
    perturbations: HashMap<String, Perturbation>,
}

/// Initial condition types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InitialConditionType {
    /// Uniform initial condition
    Uniform,
    /// Gaussian initial condition
    Gaussian,
    /// Sinusoidal initial condition
    Sinusoidal,
    /// Random initial condition
    Random,
    /// Analytical solution
    Analytical,
    /// User-defined
    UserDefined,
}

/// Perturbation
#[derive(Debug, Clone)]
pub struct Perturbation {
    pub perturbation_id: String,
    pub perturbation_type: PerturbationType,
    pub amplitude: f64,
    pub wavelength: Option<f64>,
    pub frequency: Option<f64>,
    pub phase: Option<f64>,
}

/// Perturbation types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PerturbationType {
    /// Sinusoidal perturbation
    Sinusoidal,
    /// Random perturbation
    Random,
    /// Gaussian perturbation
    Gaussian,
    /// Wave packet
    WavePacket,
    /// Soliton
    Soliton,
}

impl BoundaryConditions {
    pub fn new() -> Self {
        Self {
            boundary_types: HashMap::new(),
            boundary_values: HashMap::new(),
            time_dependent_boundaries: HashMap::new(),
        }
    }

    /// Register a boundary condition for a field.
    ///
    /// The `value` is interpreted according to the boundary type:
    /// - Dirichlet: the fixed value at the boundary
    /// - Neumann: the gradient (du/dn) at the boundary
    /// - Robin: the target value for the combined condition
    /// - Periodic: ignored (periodic copies from the opposite edge)
    pub fn set_boundary(&mut self, field_id: &str, boundary_type: BoundaryType, value: f64) {
        self.boundary_types
            .insert(field_id.to_string(), boundary_type.clone());
        // Store the value for both edges (left, right) of a 1-D field.
        self.boundary_values
            .insert(field_id.to_string(), vec![value, value]);
    }

    /// Register a time-dependent boundary condition for a field.
    pub fn set_time_dependent_boundary(
        &mut self,
        field_id: &str,
        boundary_type: BoundaryType,
        time_fn: TimeFunction,
    ) {
        self.boundary_types
            .insert(field_id.to_string(), boundary_type);
        self.time_dependent_boundaries.insert(
            field_id.to_string(),
            TimeDependentBoundary {
                boundary_id: field_id.to_string(),
                time_function: time_fn,
                spatial_function: None,
            },
        );
    }

    /// Evaluate a `TimeFunction` at the given time, returning the scalar value.
    fn evaluate_time_function(time_fn: &TimeFunction, time: f64) -> f64 {
        match time_fn {
            TimeFunction::Constant(v) => *v,
            TimeFunction::Linear(a, b) => a + b * time,
            TimeFunction::Sinusoidal(amplitude, frequency, phase) => {
                amplitude * (2.0 * std::f64::consts::PI * frequency * time + phase).sin()
            }
            TimeFunction::Exponential(amplitude, rate) => amplitude * (rate * time).exp(),
            TimeFunction::Piecewise(segments) => {
                for (start, end, fn_in_segment) in segments {
                    if *start <= time && time < *end {
                        return Self::evaluate_time_function(fn_in_segment, time);
                    }
                }
                0.0
            }
            TimeFunction::Custom(_) => 0.0,
        }
    }

    /// Apply boundary conditions to a field's edge cells based on the registered type.
    ///
    /// For a 1-D field the edge cells are index 0 (left) and index n-1 (right).
    pub fn apply_to_field(&self, field: &mut PhysicsField, time: f64) {
        let field_id = &field.field_id;

        // Look up the boundary type; skip if no boundary is registered for this field.
        let boundary_type = match self.boundary_types.get(field_id) {
            Some(bt) => bt.clone(),
            None => return,
        };

        let n = field.data.len();
        if n < 2 {
            return;
        }

        // Determine the boundary value(s). Time-dependent boundaries override static values.
        let values: Vec<f64> = if let Some(tdb) = self.time_dependent_boundaries.get(field_id) {
            let v = Self::evaluate_time_function(&tdb.time_function, time);
            vec![v, v]
        } else if let Some(vals) = self.boundary_values.get(field_id) {
            vals.clone()
        } else {
            vec![0.0, 0.0]
        };

        let left_val = values.first().copied().unwrap_or(0.0);
        let right_val = values.get(1).copied().unwrap_or(left_val);

        // Estimate dx from the first dimension if available.
        let dx = 1.0; // default grid spacing; callers may normalise beforehand

        match boundary_type {
            BoundaryType::Dirichlet => {
                // Set edge cells to the boundary value.
                field.data[0] = left_val;
                field.data[n - 1] = right_val;
            }
            BoundaryType::Neumann => {
                // du/dn = value at the boundary.
                // Left boundary: outward normal is -x, so du/dn = -du/dx => du/dx = -value
                //   field[0] = field[1] - value * dx
                // Right boundary: outward normal is +x, so du/dn = du/dx = value
                //   field[n-1] = field[n-2] + value * dx
                field.data[0] = field.data[1] - left_val * dx;
                field.data[n - 1] = field.data[n - 2] + right_val * dx;
            }
            BoundaryType::Robin => {
                // Combined Dirichlet + Neumann: blend the fixed value with the Neumann
                // mirror. This approximates a*u + b*du/dn = c by averaging the Dirichlet
                // set and the Neumann correction.
                let dirichlet_left = left_val;
                let neumann_left = field.data[1] - left_val * dx;
                field.data[0] = 0.5 * (dirichlet_left + neumann_left);

                let dirichlet_right = right_val;
                let neumann_right = field.data[n - 2] + right_val * dx;
                field.data[n - 1] = 0.5 * (dirichlet_right + neumann_right);
            }
            BoundaryType::Periodic => {
                // Copy from the opposite edge's inner neighbour to avoid a self-reference.
                let left = field.data[n - 2]; // inner neighbour of the right edge
                let right = field.data[1]; // inner neighbour of the left edge
                field.data[0] = left;
                field.data[n - 1] = right;
            }
            // Other boundary types (Symmetry, Wall, Inflow, Outflow, FarField) are treated
            // as Dirichlet for the generic apply path.
            _ => {
                field.data[0] = left_val;
                field.data[n - 1] = right_val;
            }
        }
    }
}

impl InitialConditions {
    pub fn new() -> Self {
        Self {
            condition_types: HashMap::new(),
            condition_values: HashMap::new(),
            perturbations: HashMap::new(),
        }
    }

    /// Register an initial condition for a field: its type and the initial values.
    pub fn set_condition(
        &mut self,
        field_id: &str,
        cond_type: InitialConditionType,
        values: Vec<f64>,
    ) {
        self.condition_types.insert(field_id.to_string(), cond_type);
        self.condition_values.insert(field_id.to_string(), values);
    }

    /// Get the initial condition type registered for a field, if any.
    pub fn get_condition_type(&self, field_id: &str) -> Option<&InitialConditionType> {
        self.condition_types.get(field_id)
    }

    /// Get the initial condition values registered for a field, if any.
    pub fn get_condition_values(&self, field_id: &str) -> Option<&Vec<f64>> {
        self.condition_values.get(field_id)
    }

    /// Remove a field's initial condition (type and values).
    pub fn remove_condition(&mut self, field_id: &str) {
        self.condition_types.remove(field_id);
        self.condition_values.remove(field_id);
    }

    /// List all field IDs that have a registered initial condition.
    pub fn list_condition_fields(&self) -> Vec<String> {
        self.condition_types.keys().cloned().collect()
    }

    /// Add a perturbation for a field.
    pub fn add_perturbation(&mut self, field_id: &str, perturbation: Perturbation) {
        self.perturbations
            .insert(field_id.to_string(), perturbation);
    }

    /// Get the perturbation registered for a field, if any.
    pub fn get_perturbation(&self, field_id: &str) -> Option<&Perturbation> {
        self.perturbations.get(field_id)
    }

    /// List all field IDs that have a registered perturbation.
    pub fn list_perturbation_fields(&self) -> Vec<String> {
        self.perturbations.keys().cloned().collect()
    }
}

impl Perturbation {
    pub fn new() -> Self {
        Self {
            perturbation_id: "default".to_string(),
            perturbation_type: PerturbationType::Sinusoidal,
            amplitude: 0.01,
            wavelength: Some(1.0),
            frequency: Some(1.0),
            phase: Some(0.0),
        }
    }
}
