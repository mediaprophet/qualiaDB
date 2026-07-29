/// IUPAC standard atomic weight (amu) for common elements. Conventional values
/// (IUPAC 2021). Returns `None` for elements outside the table so callers can
/// fall back to a declared per-atom mass rather than receive a fabricated one.
pub fn standard_atomic_weight(element: &str) -> Option<f64> {
    let w = match element {
        "H" => 1.008,
        "He" => 4.002602,
        "Li" => 6.94,
        "Be" => 9.0121831,
        "B" => 10.81,
        "C" => 12.011,
        "N" => 14.007,
        "O" => 15.999,
        "F" => 18.998403163,
        "Ne" => 20.1797,
        "Na" => 22.98976928,
        "Mg" => 24.305,
        "Al" => 26.9815385,
        "Si" => 28.085,
        "P" => 30.973761998,
        "S" => 32.06,
        "Cl" => 35.45,
        "Ar" => 39.948,
        "K" => 39.0983,
        "Ca" => 40.078,
        "Fe" => 55.845,
        "Cu" => 63.546,
        "Zn" => 65.38,
        "Br" => 79.904,
        "I" => 126.90447,
        _ => return None,
    };
    Some(w)
}

/// Exact structural / mass properties of a molecule (see the methods on
/// [`ChemistryModelingLibrary`]). Every field is computed from a closed-form
/// definition over atomic data and geometry, not an approximation or fit.
#[derive(Debug, Clone)]
pub struct StructuralProperties {
    /// Total molecular mass (amu), from standard atomic weights.
    pub molecular_mass: f64,
    /// Molecular formula in Hill notation.
    pub formula: String,
    /// Number of atoms.
    pub atom_count: usize,
    /// Nuclear repulsion energy Σ Z_i Z_j / r_ij (Hartree when coords are in
    /// bohr); `None` when the geometry/charges cannot support it.
    pub nuclear_repulsion_energy: Option<f64>,
    /// Center of mass (same length unit as the coordinates).
    pub center_of_mass: [f64; 3],
    /// Principal moments of inertia, ascending (amu·length²).
    pub principal_moments_of_inertia: [f64; 3],
}
