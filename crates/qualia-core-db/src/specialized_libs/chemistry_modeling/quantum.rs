use super::integrals::{GtoPrimitive, IntegralEngine};
use super::scf::solve_rhf_scf_4index;
use super::*;
use crate::specialized_libs::shared::zero_heap_algebra::ZeroHeapMatrix;
use core::f64::consts::PI;

/// A contracted s-type Gaussian basis function (STO-3G): three primitives sharing
/// a center, each carrying its fully-normalized effective coefficient.
#[derive(Debug, Clone, Copy)]
struct ContractedS {
    /// The three STO-3G primitives; each carries the shell center in `origin`.
    prims: [GtoPrimitive; 3],
}

/// Build the STO-3G 1s contracted basis function for a supported element
/// (H, Z=1; He, Z=2). The three-Gaussian expansion of a Slater 1s uses the
/// published STO-3G contraction coefficients (identical for the 1s shell of every
/// element) with element-specific exponents. Each primitive coefficient is
/// premultiplied by the primitive s normalization `(2α/π)^{3/4}`, and the whole
/// contraction is renormalized so `<φ|φ> = 1`. Returns `None` for unsupported
/// elements rather than inventing parameters.
fn build_sto3g_s(z: usize, center: [f64; 3]) -> Option<ContractedS> {
    // Published STO-3G data (Basis Set Exchange / EMSL).
    let (exps, coefs): ([f64; 3], [f64; 3]) = match z {
        1 => (
            [3.425_250_914, 0.623_913_730, 0.168_855_400],
            [0.154_328_967, 0.535_328_142, 0.444_634_542],
        ),
        2 => (
            [6.362_421_394, 1.158_922_999, 0.313_649_790],
            [0.154_328_967, 0.535_328_142, 0.444_634_542],
        ),
        _ => return None,
    };
    // Primitive normalization folded into the coefficient: d_i = c_i · (2α_i/π)^{3/4}.
    let mut d = [0.0_f64; 3];
    for i in 0..3 {
        d[i] = coefs[i] * (2.0 * exps[i] / PI).powf(0.75);
    }
    // Contracted self-overlap  S = Σ_ij d_i d_j (π/(α_i+α_j))^{3/2}, then renormalize.
    let mut s_cc = 0.0;
    for i in 0..3 {
        for j in 0..3 {
            s_cc += d[i] * d[j] * (PI / (exps[i] + exps[j])).powf(1.5);
        }
    }
    let renorm = 1.0 / s_cc.sqrt();
    let mut prims = [GtoPrimitive {
        origin: center,
        exponent: 0.0,
        l: [0, 0, 0],
        coefficient: 0.0,
    }; 3];
    for i in 0..3 {
        prims[i] = GtoPrimitive {
            origin: center,
            exponent: exps[i],
            l: [0, 0, 0],
            coefficient: d[i] * renorm,
        };
    }
    Some(ContractedS { prims })
}

/// Contracted overlap `<a|b>` = Σ_ij `<g_i|g_j>` over the primitive pairs.
fn c_overlap(a: &ContractedS, b: &ContractedS) -> f64 {
    let mut v = 0.0;
    for i in 0..3 {
        for j in 0..3 {
            v += IntegralEngine::overlap_s(&a.prims[i], &b.prims[j]);
        }
    }
    v
}

/// Contracted kinetic energy `<a|−½∇²|b>`.
fn c_kinetic(a: &ContractedS, b: &ContractedS) -> f64 {
    let mut v = 0.0;
    for i in 0..3 {
        for j in 0..3 {
            v += IntegralEngine::kinetic_s(&a.prims[i], &b.prims[j]);
        }
    }
    v
}

/// Contracted nuclear attraction, summed over every nucleus `(center, Z)`.
fn c_nuclear(a: &ContractedS, b: &ContractedS, nuclei: &[([f64; 3], f64)]) -> f64 {
    let mut v = 0.0;
    for &(center, z) in nuclei {
        for i in 0..3 {
            for j in 0..3 {
                v += IntegralEngine::nuclear_s(&a.prims[i], &b.prims[j], center, z);
            }
        }
    }
    v
}

/// Contracted two-electron integral `(ab|cd)` in chemists' notation.
fn c_eri(a: &ContractedS, b: &ContractedS, c: &ContractedS, d: &ContractedS) -> f64 {
    let mut v = 0.0;
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                for l in 0..3 {
                    v += IntegralEngine::evaluate_eri(
                        &a.prims[i],
                        &b.prims[j],
                        &c.prims[k],
                        &d.prims[l],
                    );
                }
            }
        }
    }
    v
}

/// Contracted Cartesian dipole integrals `[<a|x|b>, <a|y|b>, <a|z|b>]`.
fn c_dipole(a: &ContractedS, b: &ContractedS) -> [f64; 3] {
    let mut v = [0.0; 3];
    for i in 0..3 {
        for j in 0..3 {
            let d = IntegralEngine::dipole_s(&a.prims[i], &b.prims[j]);
            v[0] += d[0];
            v[1] += d[1];
            v[2] += d[2];
        }
    }
    v
}

/// Run closed-shell RHF over `N` contracted s-type basis functions and assemble
/// the electronic-structure observables. Real integrals, real 4-index Fock build,
/// real diagonalization — no fabricated numbers.
fn run_rhf<const N: usize>(
    shells: &[ContractedS; N],
    nuclei: &[([f64; 3], f64)],
    n_elec: usize,
) -> Result<QuantumProperties, ChemistryError> {
    // Assemble overlap S, core Hamiltonian H = T + V_nuc, and the 4-index ERI.
    let mut s = ZeroHeapMatrix::<f64, N, N>::zeros();
    let mut h = ZeroHeapMatrix::<f64, N, N>::zeros();
    for i in 0..N {
        for j in 0..N {
            s.set(i, j, c_overlap(&shells[i], &shells[j]));
            h.set(
                i,
                j,
                c_kinetic(&shells[i], &shells[j]) + c_nuclear(&shells[i], &shells[j], nuclei),
            );
        }
    }
    let mut eri = [[[[0.0_f64; N]; N]; N]; N];
    for i in 0..N {
        for j in 0..N {
            for k in 0..N {
                for l in 0..N {
                    eri[i][j][k][l] = c_eri(&shells[i], &shells[j], &shells[k], &shells[l]);
                }
            }
        }
    }

    let res = solve_rhf_scf_4index(&h, &s, &eri, n_elec).map_err(|e| {
        ChemistryError::ConvergenceError(format!("RHF SCF did not converge: {:?}", e))
    })?;

    // Nuclear repulsion (coordinates assumed in bohr) → total energy.
    let mut e_nn = 0.0;
    for i in 0..nuclei.len() {
        for j in (i + 1)..nuclei.len() {
            let (ci, zi) = nuclei[i];
            let (cj, zj) = nuclei[j];
            let r = ((ci[0] - cj[0]).powi(2) + (ci[1] - cj[1]).powi(2) + (ci[2] - cj[2]).powi(2))
                .sqrt();
            e_nn += zi * zj / r;
        }
    }
    let total_energy = res.electronic_energy + e_nn;

    // HOMO / LUMO / gap from the orbital energies.
    let num_occ = res.num_occ;
    let homo = if num_occ >= 1 {
        res.orbital_energies[num_occ - 1]
    } else {
        0.0
    };
    let (lumo, gap) = if N > num_occ {
        let l = res.orbital_energies[num_occ];
        (l, l - homo)
    } else {
        // Minimal basis with no virtual orbital (e.g. He/STO-3G): there is no
        // LUMO. Report lumo = homo and gap = 0 to signal "no virtual in basis".
        (homo, 0.0)
    };

    // Mulliken charges q_A = Z_A − (P·S)_AA (one basis function per atom here).
    let ps = res.density * s;
    let mut mulliken = Vec::with_capacity(N);
    for i in 0..N {
        mulliken.push(nuclei[i].1 - ps.get(i, i));
    }

    // Dipole μ = Σ_A Z_A R_A − Σ_μν P_μν <μ|r|ν>  (electron charge −1).
    let mut dip = [0.0_f64; 3];
    for &(center, z) in nuclei {
        for w in 0..3 {
            dip[w] += z * center[w];
        }
    }
    for i in 0..N {
        for j in 0..N {
            let dij = c_dipole(&shells[i], &shells[j]);
            let p = res.density.get(i, j);
            for w in 0..3 {
                dip[w] -= p * dij[w];
            }
        }
    }
    let dipole_magnitude = (dip[0] * dip[0] + dip[1] * dip[1] + dip[2] * dip[2]).sqrt();

    Ok(QuantumProperties {
        total_energy,
        homo_energy: homo,
        lumo_energy: lumo,
        gap,
        dipole_moment: dipole_magnitude,
        // Polarizability is a response property (needs CPHF / finite-field
        // perturbation) and is NOT one of the observables this RHF path computes;
        // it is left at 0.0 and must not be read as a computed value.
        polarizability: 0.0,
        mulliken_charges: mulliken,
    })
}

/// Quantum calculator for quantum chemistry calculations
pub struct QuantumCalculator {
    wavefunction_calculator: WavefunctionCalculator,
    energy_calculator: QuantumEnergyCalculator,
    property_calculator: QuantumPropertyCalculator,
}

/// Wavefunction calculator
pub struct WavefunctionCalculator {
    method_type: QuantumMethodType,
    basis_set: BasisSet,
    scf_parameters: SCFParameters,
}

/// Quantum method types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QuantumMethodType {
    HartreeFock,
    DFT,
    MP2,
    CCSD,
    CI,
    SemiEmpirical,
    AbInitio,
}

/// Basis sets
#[derive(Debug, Clone)]
pub struct BasisSet {
    pub basis_set_id: String,
    pub basis_set_name: String,
    pub basis_set_type: BasisSetType,
    pub functions: Vec<BasisFunction>,
}

/// Basis set types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BasisSetType {
    Minimal,
    SplitValence,
    TripleZeta,
    Polarization,
    Diffuse,
    Custom,
}

/// Basis functions
#[derive(Debug, Clone)]
pub struct BasisFunction {
    pub function_id: String,
    pub function_type: BasisFunctionType,
    pub center: Vec<f64>,
    pub exponents: Vec<f64>,
    pub coefficients: Vec<f64>,
}

/// Basis function types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BasisFunctionType {
    S,
    P,
    D,
    F,
    G,
    Custom,
}

/// SCF parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SCFParameters {
    pub convergence_threshold: f64,
    pub max_iterations: u32,
    pub damping_factor: f64,
    pub level_shifting: f64,
}

/// Quantum energy calculator
pub struct QuantumEnergyCalculator {
    electronic_energy: ElectronicEnergy,
    nuclear_energy: NuclearEnergy,
    total_energy: QuantumTotalEnergy,
}

/// Electronic energy
#[derive(Debug, Clone)]
pub struct ElectronicEnergy {
    pub kinetic_energy: f64,
    pub electron_nuclear: f64,
    pub electron_electron: f64,
    pub exchange_correlation: f64,
}

/// Nuclear energy
#[derive(Debug, Clone)]
pub struct NuclearEnergy {
    pub nuclear_repulsion: f64,
    pub nuclear_attraction: f64,
}

/// Quantum total energy
#[derive(Debug, Clone)]
pub struct QuantumTotalEnergy {
    pub electronic: f64,
    pub nuclear: f64,
    pub total: f64,
    pub correction_terms: Vec<f64>,
}

/// Quantum property calculator
pub struct QuantumPropertyCalculator {
    dipole_moment: DipoleMoment,
    polarizability: Polarizability,
    mulliken_charges: MullikenCharges,
}

/// Dipole moment
#[derive(Debug, Clone)]
pub struct DipoleMoment {
    pub components: Vec<f64>,
    pub magnitude: f64,
}

/// Polarizability
#[derive(Debug, Clone)]
pub struct Polarizability {
    pub tensor: Vec<Vec<f64>>,
    pub isotropic: f64,
}

/// Mulliken charges
#[derive(Debug, Clone)]
pub struct MullikenCharges {
    pub charges: Vec<f64>,
    pub total_charge: f64,
}

impl QuantumCalculator {
    pub fn new() -> Self {
        Self {
            wavefunction_calculator: WavefunctionCalculator::new(),
            energy_calculator: QuantumEnergyCalculator::new(),
            property_calculator: QuantumPropertyCalculator::new(),
        }
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        self.wavefunction_calculator.initialize()?;
        self.energy_calculator.initialize()?;
        self.property_calculator.initialize()?;
        Ok(())
    }

    pub fn validate_molecule(&self, molecule: &Molecule) -> Result<(), ChemistryError> {
        if molecule.atoms.is_empty() {
            return Err(ChemistryError::ValidationError(
                "Molecule must have at least one atom".to_string(),
            ));
        }
        Ok(())
    }

    /// Compute the ground-state electronic-structure observables by running a real
    /// RHF/STO-3G calculation: total SCF energy, HOMO/LUMO orbital energies and
    /// gap, Mulliken charges, and the dipole moment.
    ///
    /// The energy falls out of a genuine self-consistent field over analytical
    /// one- and two-electron integrals (kinetic, nuclear attraction, overlap,
    /// four-index ERI) — nothing is hardcoded. Atomic `coordinates` are taken to
    /// be in **bohr** (atomic units); the returned energies are in Hartree.
    ///
    /// Scope of this deterministic path: closed-shell (even electron count)
    /// molecules built from H and He, for which STO-3G is a single s-type shell
    /// per atom with exact closed-form s integrals. Anything outside that scope
    /// (heavier elements needing p/d shells, open-shell/odd-electron systems, or a
    /// requested method other than Hartree-Fock) returns an honest `NotImplemented`
    /// rather than a fabricated number.
    pub fn calculate_properties(
        &mut self,
        molecule: &Molecule,
        method: QuantumMethodType,
    ) -> Result<QuantumProperties, ChemistryError> {
        match method {
            QuantumMethodType::HartreeFock | QuantumMethodType::AbInitio => {}
            other => {
                return Err(ChemistryError::NotImplemented(format!(
                    "electronic structure: only closed-shell RHF (HartreeFock) is implemented; \
                     {:?} would require a correlated/DFT method that is not yet built",
                    other
                )));
            }
        }

        if molecule.atoms.is_empty() {
            return Err(ChemistryError::ValidationError(
                "electronic structure: the molecule has no atoms".to_string(),
            ));
        }

        // Marshal the molecule into an STO-3G contracted basis (one s shell per
        // H/He atom) and collect the nuclei. Refuse unsupported elements.
        let mut shells: Vec<ContractedS> = Vec::with_capacity(molecule.atoms.len());
        let mut nuclei: Vec<([f64; 3], f64)> = Vec::with_capacity(molecule.atoms.len());
        let mut n_elec = 0usize;
        for (idx, a) in molecule.atoms.iter().enumerate() {
            if a.coordinates.len() != 3 {
                return Err(ChemistryError::InsufficientData(format!(
                    "electronic structure: atom {} ('{}') has {} coordinates; 3 (in bohr) required",
                    idx,
                    a.atom_id,
                    a.coordinates.len()
                )));
            }
            let center = [a.coordinates[0], a.coordinates[1], a.coordinates[2]];
            let shell = build_sto3g_s(a.atomic_number, center).ok_or_else(|| {
                ChemistryError::NotImplemented(format!(
                    "electronic structure: STO-3G RHF is implemented for H and He only; atom {} \
                     ('{}', element '{}', Z={}) needs p/d shells that are not yet built",
                    idx, a.atom_id, a.element, a.atomic_number
                ))
            })?;
            shells.push(shell);
            nuclei.push((center, a.atomic_number as f64));
            n_elec += a.atomic_number;
        }

        if n_elec == 0 {
            return Err(ChemistryError::ValidationError(
                "electronic structure: total electron count is zero".to_string(),
            ));
        }
        if n_elec % 2 != 0 {
            return Err(ChemistryError::NotImplemented(format!(
                "electronic structure: {} electrons is open-shell; only closed-shell RHF (even \
                 electron count) is implemented — UHF is not yet built",
                n_elec
            )));
        }

        let n_bf = shells.len();
        // Dispatch to the const-generic RHF driver for the supported basis sizes.
        macro_rules! dispatch {
            ($($n:literal),+ $(,)?) => {
                match n_bf {
                    $(
                        $n => {
                            let mut arr = [shells[0]; $n];
                            for i in 0..$n { arr[i] = shells[i]; }
                            run_rhf::<$n>(&arr, &nuclei, n_elec)
                        }
                    )+
                    _ => Err(ChemistryError::NotImplemented(format!(
                        "electronic structure: STO-3G RHF currently supports 1–8 s-type basis \
                         functions (H/He atoms); this molecule needs {}",
                        n_bf
                    ))),
                }
            };
        }
        dispatch!(1, 2, 3, 4, 5, 6, 7, 8)
    }
}

impl WavefunctionCalculator {
    pub fn new() -> Self {
        Self {
            method_type: QuantumMethodType::HartreeFock,
            basis_set: BasisSet::new(),
            scf_parameters: SCFParameters::new(),
        }
    }

    /// Borrow the quantum method type.
    pub fn method_type(&self) -> &QuantumMethodType {
        &self.method_type
    }

    /// Set the quantum method type.
    pub fn set_method_type(&mut self, method_type: QuantumMethodType) {
        self.method_type = method_type;
    }

    /// Borrow the basis set.
    pub fn basis_set(&self) -> &BasisSet {
        &self.basis_set
    }

    /// Mutably borrow the basis set.
    pub fn basis_set_mut(&mut self) -> &mut BasisSet {
        &mut self.basis_set
    }

    /// Borrow the SCF parameters.
    pub fn scf_parameters(&self) -> &SCFParameters {
        &self.scf_parameters
    }

    /// Mutably borrow the SCF parameters.
    pub fn scf_parameters_mut(&mut self) -> &mut SCFParameters {
        &mut self.scf_parameters
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        Ok(())
    }
}

impl BasisSet {
    pub fn new() -> Self {
        Self {
            basis_set_id: "basis_1".to_string(),
            basis_set_name: "6-31G".to_string(),
            basis_set_type: BasisSetType::SplitValence,
            functions: vec![BasisFunction::new()],
        }
    }
}

impl BasisFunction {
    pub fn new() -> Self {
        Self {
            function_id: "func_1".to_string(),
            function_type: BasisFunctionType::S,
            center: vec![0.0, 0.0, 0.0],
            exponents: vec![0.5],
            coefficients: vec![1.0],
        }
    }
}

impl SCFParameters {
    pub fn new() -> Self {
        Self {
            convergence_threshold: 1e-8,
            max_iterations: 100,
            damping_factor: 0.5,
            level_shifting: 0.3,
        }
    }
}

impl QuantumEnergyCalculator {
    pub fn new() -> Self {
        Self {
            electronic_energy: ElectronicEnergy::new(),
            nuclear_energy: NuclearEnergy::new(),
            total_energy: QuantumTotalEnergy::new(),
        }
    }

    /// Borrow the electronic-energy breakdown.
    pub fn electronic_energy(&self) -> &ElectronicEnergy {
        &self.electronic_energy
    }

    /// Mutably borrow the electronic-energy breakdown.
    pub fn electronic_energy_mut(&mut self) -> &mut ElectronicEnergy {
        &mut self.electronic_energy
    }

    /// Borrow the nuclear-energy breakdown.
    pub fn nuclear_energy(&self) -> &NuclearEnergy {
        &self.nuclear_energy
    }

    /// Mutably borrow the nuclear-energy breakdown.
    pub fn nuclear_energy_mut(&mut self) -> &mut NuclearEnergy {
        &mut self.nuclear_energy
    }

    /// Borrow the total-energy breakdown.
    pub fn total_energy(&self) -> &QuantumTotalEnergy {
        &self.total_energy
    }

    /// Mutably borrow the total-energy breakdown.
    pub fn total_energy_mut(&mut self) -> &mut QuantumTotalEnergy {
        &mut self.total_energy
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        Ok(())
    }
}

impl ElectronicEnergy {
    pub fn new() -> Self {
        Self {
            kinetic_energy: 0.0,
            electron_nuclear: 0.0,
            electron_electron: 0.0,
            exchange_correlation: 0.0,
        }
    }
}

impl NuclearEnergy {
    pub fn new() -> Self {
        Self {
            nuclear_repulsion: 0.0,
            nuclear_attraction: 0.0,
        }
    }
}

impl QuantumTotalEnergy {
    pub fn new() -> Self {
        Self {
            electronic: 0.0,
            nuclear: 0.0,
            total: 0.0,
            correction_terms: Vec::new(),
        }
    }
}

impl QuantumPropertyCalculator {
    pub fn new() -> Self {
        Self {
            dipole_moment: DipoleMoment::new(),
            polarizability: Polarizability::new(),
            mulliken_charges: MullikenCharges::new(),
        }
    }

    /// Borrow the dipole-moment result.
    pub fn dipole_moment(&self) -> &DipoleMoment {
        &self.dipole_moment
    }

    /// Mutably borrow the dipole-moment result.
    pub fn dipole_moment_mut(&mut self) -> &mut DipoleMoment {
        &mut self.dipole_moment
    }

    /// Borrow the polarizability result.
    pub fn polarizability(&self) -> &Polarizability {
        &self.polarizability
    }

    /// Mutably borrow the polarizability result.
    pub fn polarizability_mut(&mut self) -> &mut Polarizability {
        &mut self.polarizability
    }

    /// Borrow the Mulliken-charges result.
    pub fn mulliken_charges(&self) -> &MullikenCharges {
        &self.mulliken_charges
    }

    /// Mutably borrow the Mulliken-charges result.
    pub fn mulliken_charges_mut(&mut self) -> &mut MullikenCharges {
        &mut self.mulliken_charges
    }

    pub fn initialize(&mut self) -> Result<(), ChemistryError> {
        Ok(())
    }
}

impl DipoleMoment {
    pub fn new() -> Self {
        Self {
            components: vec![0.0, 0.0, 0.0],
            magnitude: 0.0,
        }
    }
}

impl Polarizability {
    pub fn new() -> Self {
        Self {
            tensor: vec![vec![0.0; 3]; 3],
            isotropic: 0.0,
        }
    }
}

impl MullikenCharges {
    pub fn new() -> Self {
        Self {
            charges: Vec::new(),
            total_charge: 0.0,
        }
    }
}
