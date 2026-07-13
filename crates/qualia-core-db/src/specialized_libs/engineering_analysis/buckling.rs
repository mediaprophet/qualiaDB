use super::*;


/// Buckling analysis
pub struct BucklingAnalysis {
    eigenvalue_buckling: EigenvalueBuckling,
    nonlinear_buckling: NonlinearBuckling,
}

/// Eigenvalue buckling
#[derive(Debug, Clone)]
pub struct EigenvalueBuckling {
    pub critical_loads: Vec<f64>,
    pub buckling_modes: Vec<BucklingMode>,
}

/// Buckling modes
#[derive(Debug, Clone)]
pub struct BucklingMode {
    pub mode_number: u32,
    pub critical_load: f64,
    pub mode_shape: Vec<f64>,
}

/// Nonlinear buckling
#[derive(Debug, Clone)]
pub struct NonlinearBuckling {
    pub load_displacement_curve: Vec<(f64, f64)>,
    pub post_buckling_behavior: PostBucklingBehavior,
}

/// Post-buckling behavior
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PostBucklingBehavior {
    Stable,
    Unstable,
    SnapThrough,
}
impl BucklingAnalysis {
    pub fn new() -> Self {
        Self {
            eigenvalue_buckling: EigenvalueBuckling::new(),
            nonlinear_buckling: NonlinearBuckling::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), EngineeringError> {
        Ok(())
    }

    /// Borrow the eigenvalue-buckling sub-component.
    pub fn eigenvalue_buckling(&self) -> &EigenvalueBuckling {
        &self.eigenvalue_buckling
    }

    /// Mutably borrow the eigenvalue-buckling sub-component.
    pub fn eigenvalue_buckling_mut(&mut self) -> &mut EigenvalueBuckling {
        &mut self.eigenvalue_buckling
    }

    /// Borrow the nonlinear-buckling sub-component.
    pub fn nonlinear_buckling(&self) -> &NonlinearBuckling {
        &self.nonlinear_buckling
    }

    /// Mutably borrow the nonlinear-buckling sub-component.
    pub fn nonlinear_buckling_mut(&mut self) -> &mut NonlinearBuckling {
        &mut self.nonlinear_buckling
    }

    /// Euler elastic critical buckling loads of a prismatic column:
    /// `P_cr,n = n²·π²·E·I / (K·L)²` for modes `n = 1..=num_modes`, where `K` is
    /// the effective-length factor (pinned–pinned = 1.0, fixed–free = 2.0,
    /// fixed–fixed = 0.5, fixed–pinned ≈ 0.699). Genuine closed-form column
    /// stability — not a fabricated value. Fills and returns [`EigenvalueBuckling`]:
    /// `critical_loads` ascending, each [`BucklingMode`] carrying the exact
    /// buckling half-wave `sin(nπx/L)` sampled at 11 equally-spaced stations.
    pub fn analyze_euler(
        &mut self,
        youngs_modulus: f64,
        moment_of_inertia: f64,
        length: f64,
        effective_length_factor: f64,
        num_modes: usize,
    ) -> Result<EigenvalueBuckling, EngineeringError> {
        if youngs_modulus <= 0.0 || moment_of_inertia <= 0.0 {
            return Err(EngineeringError::InsufficientData(
                "Young's modulus and moment of inertia must be positive".to_string(),
            ));
        }
        if length <= 0.0 || effective_length_factor <= 0.0 {
            return Err(EngineeringError::ValidationError(
                "length and effective-length factor must be positive".to_string(),
            ));
        }
        if num_modes == 0 {
            return Err(EngineeringError::InsufficientData(
                "num_modes must be at least 1".to_string(),
            ));
        }
        let le = effective_length_factor * length;
        let base = std::f64::consts::PI.powi(2) * youngs_modulus * moment_of_inertia / (le * le);
        const STATIONS: usize = 11;
        let mut critical_loads = Vec::with_capacity(num_modes);
        let mut buckling_modes = Vec::with_capacity(num_modes);
        for n in 1..=num_modes {
            let p_cr = (n as f64).powi(2) * base;
            let shape: Vec<f64> = (0..STATIONS)
                .map(|s| {
                    let x = length * s as f64 / (STATIONS as f64 - 1.0);
                    (n as f64 * std::f64::consts::PI * x / length).sin()
                })
                .collect();
            critical_loads.push(p_cr);
            buckling_modes.push(BucklingMode {
                mode_number: n as u32,
                critical_load: p_cr,
                mode_shape: shape,
            });
        }
        let eb = EigenvalueBuckling {
            critical_loads,
            buckling_modes,
        };
        self.eigenvalue_buckling = eb.clone();
        Ok(eb)
    }

    /// Euler critical buckling of the member described by `model`: uses the first
    /// material's Young's modulus, the weak-axis second moment of area of the
    /// rectangular cross-section `I = min(b·h³, h·b³)/12` from the first two
    /// geometry dimensions `[b, h]`, the third dimension as the member length `L`,
    /// and a pinned–pinned effective-length factor `K = 1.0`. Returns the first
    /// `num_modes` critical loads. Missing/degenerate inputs → `InsufficientData`.
    pub fn analyze_from_model(
        &mut self,
        model: &EngineeringModel,
        num_modes: usize,
    ) -> Result<EigenvalueBuckling, EngineeringError> {
        let material = model.materials.values().next().ok_or_else(|| {
            EngineeringError::InsufficientData(
                "model has no material; cannot compute buckling load".to_string(),
            )
        })?;
        let e = material.material_properties.youngs_modulus;
        let dims = &model.geometry.dimensions;
        if dims.len() < 3 || dims.iter().take(3).any(|&d| !(d > 0.0)) {
            return Err(EngineeringError::InsufficientData(
                "geometry needs three positive dimensions [b, h, L] for column buckling"
                    .to_string(),
            ));
        }
        let (b, h, l) = (dims[0], dims[1], dims[2]);
        let i_weak = (b * h * h * h).min(h * b * b * b) / 12.0;
        self.analyze_euler(e, i_weak, l, 1.0, num_modes)
    }
}

impl EigenvalueBuckling {
    pub fn new() -> Self {
        Self {
            critical_loads: Vec::new(),
            buckling_modes: Vec::new(),
        }
    }
}

impl NonlinearBuckling {
    pub fn new() -> Self {
        Self {
            load_displacement_curve: Vec::new(),
            post_buckling_behavior: PostBucklingBehavior::Stable,
        }
    }
}

