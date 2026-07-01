//! Basis Set & Spatial Discretization (Task H)
//!
//! Implements contracted Gaussian-Type Orbitals (GTOs) in both Cartesian and
//! real-spherical forms, with a native deserialization module for Basis Set
//! Exchange (BSE) JSON data (STO-3G, def2-SVP, cc-pVXZ, etc.).
//!
//! All orbital structures implement `category_theory::Object` so that integral
//! evaluation (Task I) and SCF iteration (Task J) can treat basis functions as
//! categorical objects with morphisms between them.
//!
//! # BSE JSON Format
//!
//! The MolSSI BSE schema (v0.1) is a JSON object keyed by element Z-number:
//! ```json
//! {
//!   "name": "STO-3G",
//!   "elements": {
//!     "1": {
//!       "electron_shells": [
//!         {
//!           "function_type": "gto",
//!           "angular_momentum": [0],
//!           "exponents": ["3.425250914E+00", ...],
//!           "coefficients": [["0.1543289673E+00", ...]]
//!         }
//!       ]
//!     }
//!   }
//! }
//! ```
//!
//! For heavy elements, `ecp_potentials` replace core electrons:
//! ```json
//! {
//!   "ecp_electrons": 28,
//!   "ecp_potentials": [
//!     {
//!       "ecp_type": "scalar_ecp",
//!       "angular_momentum": [0],
//!       "r_exponents": [2, 2, ...],
//!       "gaussian_exponents": ["40.00518400", ...],
//!       "coefficients": [["49.99796200", ...]]
//!     }
//!   ]
//! }
//! ```

use super::super::category_theory::Object;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ─── Angular Momentum ─────────────────────────────────────────────────────

/// Angular momentum quantum number (l = 0, 1, 2, 3, 4, 5, 6, 7, 8).
///
/// Maps to the standard spectroscopic letters:
///   0=s, 1=p, 2=d, 3=f, 4=g, 5=h, 6=i, 7=k, 8=l
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AngularMomentum(pub u8);

impl AngularMomentum {
    /// Spectroscopic letter for this angular momentum.
    pub fn letter(&self) -> char {
        const LETTERS: [char; 9] = ['s', 'p', 'd', 'f', 'g', 'h', 'i', 'k', 'l'];
        LETTERS[(self.0 as usize).min(8)]
    }

    /// Number of Cartesian Gaussian functions for this angular momentum:
    ///   (l+1)(l+2)/2
    pub fn n_cartesian(&self) -> usize {
        let l = self.0 as usize;
        (l + 1) * (l + 2) / 2
    }

    /// Number of real-spherical Gaussian functions for this angular momentum:
    ///   2l+1
    pub fn n_spherical(&self) -> usize {
        2 * self.0 as usize + 1
    }

    /// Parse from spectroscopic letter (case-insensitive).
    pub fn from_letter(c: char) -> Option<Self> {
        match c.to_ascii_lowercase() {
            's' => Some(Self(0)),
            'p' => Some(Self(1)),
            'd' => Some(Self(2)),
            'f' => Some(Self(3)),
            'g' => Some(Self(4)),
            'h' => Some(Self(5)),
            'i' => Some(Self(6)),
            'k' => Some(Self(7)),
            'l' => Some(Self(8)),
            _ => None,
        }
    }
}


impl std::fmt::Display for AngularMomentum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.letter())
    }
}

impl From<u8> for AngularMomentum {
    fn from(v: u8) -> Self {
        Self(v)
    }
}

// ─── Coordinate Type ──────────────────────────────────────────────────────

/// Cartesian coordinate for an atom center (Bohr or Angstrom — caller's
/// responsibility to track units consistently).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Squared distance to another point.
    pub fn dist_sq(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    /// Euclidean distance to another point.
    pub fn dist(&self, other: &Self) -> f64 {
        self.dist_sq(other).sqrt()
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Self;
    fn mul(self, s: f64) -> Self::Output {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

// ─── Primitive Gaussian ───────────────────────────────────────────────────

/// A single primitive Gaussian function:
///   g(r) = (x − x₀)^lx (y − y₀)^ly (z − z₀)^lz · exp(−α · |r − r₀|²)
///
/// where (lx, ly, lz) are the Cartesian powers, α is the exponent, and
/// r₀ is the nuclear center.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimitiveGaussian {
    /// Exponent α (controls how tightly the Gaussian is localized).
    pub exponent: f64,
    /// Nuclear center coordinates.
    pub center: Vec3,
    /// Cartesian powers (lx, ly, lz). Their sum = angular momentum l.
    pub l: [u8; 3],
}

impl PrimitiveGaussian {
    /// Create an s-type primitive (l=0) at the given center.
    pub fn s(exponent: f64, center: Vec3) -> Self {
        Self { exponent, center, l: [0, 0, 0] }
    }

    /// Angular momentum quantum number l = lx + ly + lz.
    pub fn angular_momentum(&self) -> AngularMomentum {
        AngularMomentum(self.l[0] + self.l[1] + self.l[2])
    }

    /// Evaluate the primitive at a point r (unnormalized).
    pub fn evaluate(&self, r: &Vec3) -> f64 {
        let dx = r.x - self.center.x;
        let dy = r.y - self.center.y;
        let dz = r.z - self.center.z;
        let r2 = dx * dx + dy * dy + dz * dz;
        let poly = dx.powi(self.l[0] as i32)
            * dy.powi(self.l[1] as i32)
            * dz.powi(self.l[2] as i32);
        poly * (-self.exponent * r2).exp()
    }

    /// Gaussian normalization constant for a Cartesian primitive:
    ///   N = (2α/π)^(3/4) · (4α)^(l/2) / [(2l−1)!!]^(1/2)
    /// where l = lx+ly+lz and the double factorial is per-component.
    ///
    /// For a full Cartesian Gaussian with individual powers (lx, ly, lz):
    ///   N_x = (2α/π)^(1/4) · (4α)^lx / (2lx−1)!!
    ///   (similarly for y, z), and N = N_x · N_y · N_z
    pub fn normalization(&self) -> f64 {
        let alpha = self.exponent;
        let prefactor = (2.0 * alpha / std::f64::consts::PI).powf(0.25);
        let nx = Self::norm_component(alpha, self.l[0]);
        let ny = Self::norm_component(alpha, self.l[1]);
        let nz = Self::norm_component(alpha, self.l[2]);
        prefactor.powi(3) * nx * ny * nz
    }

    fn norm_component(alpha: f64, l: u8) -> f64 {
        // (4α)^l / (2l−1)!!
        let numerator = (4.0 * alpha).powi(l as i32);
        let denom = Self::double_factorial(2 * l as i64 - 1);
        numerator / denom
    }

    fn double_factorial(n: i64) -> f64 {
        // (−1)!! = 1, 0!! = 1, 1!! = 1
        if n <= 0 {
            return 1.0;
        }
        let mut result = 1.0;
        let mut k = n;
        while k > 0 {
            result *= k as f64;
            k -= 2;
        }
        result
    }
}

// ─── Contracted Shell ─────────────────────────────────────────────────────

/// Whether a contracted shell uses Cartesian or real-spherical angular
/// functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ShellType {
    /// Cartesian Gaussians: (l+1)(l+2)/2 functions per shell.
    Cartesian,
    /// Real spherical harmonics: 2l+1 functions per shell.
    Spherical,
}

impl ShellType {
    /// Number of basis functions in a shell with this type and angular momentum.
    pub fn n_functions(&self, am: AngularMomentum) -> usize {
        match self {
            ShellType::Cartesian => am.n_cartesian(),
            ShellType::Spherical => am.n_spherical(),
        }
    }
}

/// A contracted Gaussian shell: a linear combination of primitive Gaussians
/// sharing the same center and angular momentum.
///
///   χ(r) = Σ_i d_i · g_i(r)
///
/// where each primitive g_i has exponent `exponents[i]` and the contraction
/// coefficient for the j-th angular momentum component is
/// `coefficients[j][i]`.
///
/// For segmented contractions (the common case), `coefficients` has one row
/// per angular momentum in `angular_momentum`. For general contractions
/// (e.g. SP shells in STO-3G), `angular_momentum` has multiple entries and
/// `coefficients` has one row per entry.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractedShell {
    /// Angular momentum quantum number(s). For a simple shell, this is [l].
    /// For a combined shell (e.g. STO-3G carbon SP), this is [0, 1].
    pub angular_momentum: Vec<AngularMomentum>,
    /// Shell type (Cartesian or spherical).
    pub shell_type: ShellType,
    /// Nuclear center coordinates.
    pub center: Vec3,
    /// Primitive exponents (one per primitive).
    pub exponents: Vec<f64>,
    /// Contraction coefficients: `coefficients[am_index][primitive_index]`.
    /// For a simple shell with one angular momentum, this is a 1×N matrix.
    pub coefficients: Vec<Vec<f64>>,
    /// Maximum angular momentum in this shell (convenience).
    pub max_am: AngularMomentum,
}

impl ContractedShell {
    /// Create a simple contracted shell with a single angular momentum.
    pub fn new(
        am: AngularMomentum,
        shell_type: ShellType,
        center: Vec3,
        exponents: Vec<f64>,
        coefficients: Vec<f64>,
    ) -> Self {
        let max_am = am;
        Self {
            angular_momentum: vec![am],
            shell_type,
            center,
            exponents,
            coefficients: vec![coefficients],
            max_am,
        }
    }

    /// Create a combined contracted shell (e.g. SP shell with [s, p]).
    pub fn combined(
        angular_momentum: Vec<AngularMomentum>,
        shell_type: ShellType,
        center: Vec3,
        exponents: Vec<f64>,
        coefficients: Vec<Vec<f64>>,
    ) -> Self {
        let max_am = angular_momentum.iter().copied().max().unwrap_or(AngularMomentum(0));
        Self { angular_momentum, shell_type, center, exponents, coefficients, max_am }
    }

    /// Number of primitives in this shell.
    pub fn n_primitives(&self) -> usize {
        self.exponents.len()
    }

    /// Number of basis functions produced by this shell.
    /// For a combined shell (e.g. SP), this is the sum over all angular momenta.
    pub fn n_functions(&self) -> usize {
        self.angular_momentum.iter().map(|am| self.shell_type.n_functions(*am)).sum()
    }

    /// Split a combined shell into individual shells (one per angular momentum).
    /// A simple shell (one AM) returns a single-element vec.
    pub fn split(&self) -> Vec<ContractedShell> {
        self.angular_momentum
            .iter()
            .zip(self.coefficients.iter())
            .map(|(am, coeffs)| ContractedShell::new(*am, self.shell_type, self.center, self.exponents.clone(), coeffs.clone()))
            .collect()
    }

    /// Generate the Cartesian power triples (lx, ly, lz) for a given angular
    /// momentum component of this shell.
    ///
    /// For l=0: [(0,0,0)]
    /// For l=1: [(1,0,0), (0,1,0), (0,0,1)]
    /// For l=2: [(2,0,0), (0,2,0), (0,0,2), (1,1,0), (1,0,1), (0,1,1)]
    pub fn cartesian_powers(&self, am: AngularMomentum) -> Vec<[u8; 3]> {
        let l = am.0 as i32;
        let mut powers = Vec::with_capacity(am.n_cartesian());
        // Standard Cartesian ordering: x-first (lx descending), then y, then z.
        // e.g. l=1 → [1,0,0], [0,1,0], [0,0,1]
        for lx in (0..=l).rev() {
            for ly in (0..=(l - lx)).rev() {
                let lz = l - lx - ly;
                powers.push([lx as u8, ly as u8, lz as u8]);
            }
        }
        powers
    }

    /// Evaluate all basis functions in this shell at point r (unnormalized).
    /// Returns one value per basis function (Cartesian or spherical).
    pub fn evaluate(&self, r: &Vec3) -> Vec<f64> {
        let mut result = Vec::with_capacity(self.n_functions());
        for (am_idx, am) in self.angular_momentum.iter().enumerate() {
            let coeffs = &self.coefficients[am_idx];
            match self.shell_type {
                ShellType::Cartesian => {
                    for powers in self.cartesian_powers(*am) {
                        let mut val = 0.0;
                        for (i, exp) in self.exponents.iter().enumerate() {
                            let prim = PrimitiveGaussian {
                                exponent: *exp,
                                center: self.center,
                                l: powers,
                            };
                            val += coeffs[i] * prim.evaluate(r);
                        }
                        result.push(val);
                    }
                }
                ShellType::Spherical => {
                    // For spherical, evaluate the Cartesian components and
                    // transform to real spherical harmonics. For now, we
                    // evaluate the isotropic radial part per m component.
                    // The full spherical transform is done in the integral
                    // engine (Task I).
                    let n = am.n_spherical();
                    for m_idx in 0..n {
                        let m = m_idx as i32 - am.0 as i32; // m = -l..l
                        let mut val = 0.0;
                        for (i, exp) in self.exponents.iter().enumerate() {
                            let radial = PrimitiveGaussian::s(*exp, self.center).evaluate(r);
                            val += coeffs[i] * radial * Self::real_spherical_prefactor(*am, m);
                        }
                        result.push(val);
                    }
                }
            }
        }
        result
    }

    /// Real spherical harmonic normalization prefactor (simplified).
    /// The full transform uses associated Legendre polynomials; this gives
    /// the correct normalization constant for the radial × angular product.
    fn real_spherical_prefactor(_am: AngularMomentum, _m: i32) -> f64 {
        // Placeholder: returns 1.0. The integral engine (Task I) will
        // implement the full real-spherical harmonic transform.
        1.0
    }
}

// ─── Effective Core Potential ─────────────────────────────────────────────

/// A single ECP potential term for a given angular momentum.
///
/// ECP potentials are expressed as:
///   U_l(r) = Σ_k d_k · r^(n_k−2) · exp(−β_k · r²)
///
/// where `r_exponents[k]` = n_k, `gaussian_exponents[k]` = β_k,
/// and `coefficients[k]` = d_k.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EcpPotential {
    /// Angular momentum for this potential. [0] = U_s, [1] = U_p, etc.
    /// A special value [−1] (represented as AngularMomentum(255)) means the
    /// "ul" or "sum" potential (the base potential that all others reduce to).
    pub angular_momentum: Vec<AngularMomentum>,
    /// r-exponents n_k for each Gaussian term.
    pub r_exponents: Vec<i32>,
    /// Gaussian exponents β_k.
    pub gaussian_exponents: Vec<f64>,
    /// Coefficients d_k.
    pub coefficients: Vec<f64>,
}

impl EcpPotential {
    /// Number of Gaussian terms in this potential.
    pub fn n_terms(&self) -> usize {
        self.gaussian_exponents.len()
    }
}

/// Effective Core Potential for an element.
///
/// Replaces core electrons with a pseudopotential to include scalar-relativistic
/// effects without explicitly treating core electrons.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectiveCorePotential {
    /// Number of core electrons replaced by the ECP.
    pub n_core_electrons: u32,
    /// List of potentials, one per angular momentum.
    pub potentials: Vec<EcpPotential>,
}

impl EffectiveCorePotential {
    /// Get the potential for a specific angular momentum, or the "ul" (sum)
    /// potential if l is not explicitly defined.
    pub fn potential_for_am(&self, am: AngularMomentum) -> Option<&EcpPotential> {
        // First, try to find an exact match.
        for pot in &self.potentials {
            if pot.angular_momentum.len() == 1 && pot.angular_momentum[0] == am {
                return Some(pot);
            }
        }
        // Fall back to the "ul" potential (angular_momentum contains 255).
        for pot in &self.potentials {
            if pot.angular_momentum.iter().any(|a| a.0 == 255) {
                return Some(pot);
            }
        }
        None
    }
}

// ─── Element Basis Set ────────────────────────────────────────────────────

/// Basis set data for a single element (all shells + optional ECP).
#[derive(Debug, Clone, PartialEq)]
pub struct ElementBasis {
    /// Atomic number Z.
    pub atomic_number: u32,
    /// Element symbol (e.g. "H", "C", "Zn").
    pub symbol: String,
    /// Electron shells (contracted Gaussians).
    pub shells: Vec<ContractedShell>,
    /// Optional effective core potential.
    pub ecp: Option<EffectiveCorePotential>,
}

impl ElementBasis {
    /// Total number of basis functions for this element.
    pub fn n_functions(&self) -> usize {
        self.shells.iter().map(|s| s.n_functions()).sum()
    }

    /// Total number of primitive Gaussians across all shells.
    pub fn n_primitives(&self) -> usize {
        self.shells.iter().map(|s| s.n_primitives()).sum()
    }

    /// Maximum angular momentum present in this element's basis.
    pub fn max_angular_momentum(&self) -> AngularMomentum {
        self.shells
            .iter()
            .map(|s| s.max_am)
            .max()
            .unwrap_or(AngularMomentum(0))
    }

    /// Split all combined shells into simple shells.
    pub fn split_shells(&self) -> Vec<ContractedShell> {
        self.shells.iter().flat_map(|s| s.split()).collect()
    }
}

// ─── Molecular Basis Set ──────────────────────────────────────────────────

/// A complete basis set for a molecule: maps each atom (by index) to its
/// element basis set, with the atom's nuclear coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct MolecularBasis {
    /// Basis set name (e.g. "STO-3G", "def2-SVP").
    pub name: String,
    /// Family (e.g. "sto", "ahlrichs", "cc-pVXZ").
    pub family: String,
    /// Description from BSE.
    pub description: String,
    /// Per-atom basis: (atomic_number, symbol, center, element_basis).
    pub atoms: Vec<(u32, String, Vec3, ElementBasis)>,
}

impl MolecularBasis {
    /// Total number of basis functions in the entire molecule.
    pub fn n_functions(&self) -> usize {
        self.atoms.iter().map(|(_, _, _, eb)| eb.n_functions()).sum()
    }

    /// Total number of primitive Gaussians in the entire molecule.
    pub fn n_primitives(&self) -> usize {
        self.atoms.iter().map(|(_, _, _, eb)| eb.n_primitives()).sum()
    }

    /// Maximum angular momentum across all atoms.
    pub fn max_angular_momentum(&self) -> AngularMomentum {
        self.atoms
            .iter()
            .map(|(_, _, _, eb)| eb.max_angular_momentum())
            .max()
            .unwrap_or(AngularMomentum(0))
    }

    /// Total number of electrons replaced by ECPs.
    pub fn n_ecp_electrons(&self) -> u32 {
        self.atoms
            .iter()
            .filter_map(|(_, _, _, eb)| eb.ecp.as_ref())
            .map(|ecp| ecp.n_core_electrons)
            .sum()
    }

    /// Collect all contracted shells from all atoms, with their centers.
    pub fn all_shells(&self) -> Vec<&ContractedShell> {
        self.atoms.iter().flat_map(|(_, _, _, eb)| eb.shells.iter()).collect()
    }

    /// Collect all shells from all atoms with per-atom nuclear centers assigned.
    ///
    /// Unlike [`ElementBasis::split_shells`], combined contractions (e.g. STO-3G
    /// SP shells) are kept intact; only the molecular → per-atom flattening is
    /// performed.
    pub fn all_split_shells(&self) -> Vec<ContractedShell> {
        self.atoms
            .iter()
            .flat_map(|(_, _, center, eb)| {
                eb.shells.iter().map(|shell| {
                    let mut positioned = shell.clone();
                    positioned.center = *center;
                    positioned
                })
            })
            .collect()
    }
}

// ─── Category Theory Implementation ───────────────────────────────────────

/// Properties of a basis function as a category-theoretic Object.
#[derive(Debug, Clone, PartialEq)]
pub struct BasisObjectProperties {
    /// Angular momentum quantum number.
    pub angular_momentum: AngularMomentum,
    /// Shell type (Cartesian or spherical).
    pub shell_type: ShellType,
    /// Number of basis functions.
    pub n_functions: usize,
    /// Nuclear center.
    pub center: Vec3,
}

impl Object for ContractedShell {
    type Properties = BasisObjectProperties;

    fn properties(&self) -> Self::Properties {
        BasisObjectProperties {
            angular_momentum: self.max_am,
            shell_type: self.shell_type,
            n_functions: self.n_functions(),
            center: self.center,
        }
    }
}

impl Object for ElementBasis {
    type Properties = BasisObjectProperties;

    fn properties(&self) -> Self::Properties {
        BasisObjectProperties {
            angular_momentum: self.max_angular_momentum(),
            shell_type: ShellType::Cartesian, // Mixed; default to Cartesian
            n_functions: self.n_functions(),
            center: Vec3::ZERO,
        }
    }
}

// ─── BSE JSON Deserialization ─────────────────────────────────────────────

/// Internal serde structures matching the BSE JSON schema (v0.1).

#[derive(Debug)]
struct BseBasis {
    name: String,
    family: String,
    description: String,
    elements: HashMap<String, BseElement>,
}

#[derive(Debug, Deserialize)]
struct BseElement {
    electron_shells: Option<Vec<BseElectronShell>>,
    ecp_potentials: Option<Vec<BseEcpPotential>>,
    ecp_electrons: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct BseElectronShell {
    function_type: String,
    angular_momentum: Vec<i32>,
    exponents: Vec<String>,
    coefficients: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct BseEcpPotential {
    angular_momentum: Vec<i32>,
    r_exponents: Vec<i32>,
    gaussian_exponents: Vec<String>,
    coefficients: Vec<Vec<String>>,
}

/// Parse a BSE-formatted JSON string into a `MolecularBasis`.
///
/// The `atoms` parameter specifies which elements to extract and their
/// nuclear coordinates: a list of (atomic_number, symbol, center).
///
/// # Errors
///
/// Returns an error if:
/// - The JSON is malformed or doesn't match the BSE schema.
/// - A requested element is not found in the basis set.
/// - A numeric field fails to parse.
pub fn parse_bse_json(
    json: &str,
    atoms: &[(u32, String, Vec3)],
) -> Result<MolecularBasis, BasisSetError> {
    use serde::de::{DeserializeSeed, Deserializer, IgnoredAny, MapAccess, Visitor};
    use std::collections::HashSet;

    struct BseBasisSeed<'a> {
        target_zs: &'a HashSet<String>,
    }

    impl<'de, 'a> DeserializeSeed<'de> for BseBasisSeed<'a> {
        type Value = BseBasis;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct BseBasisVisitor<'a> {
                target_zs: &'a HashSet<String>,
            }

            impl<'de, 'a> Visitor<'de> for BseBasisVisitor<'a> {
                type Value = BseBasis;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("struct BseBasis")
                }

                fn visit_map<V>(self, mut map: V) -> Result<BseBasis, V::Error>
                where
                    V: MapAccess<'de>,
                {
                    let mut name = String::new();
                    let mut family = String::new();
                    let mut description = String::new();
                    let mut elements = HashMap::new();

                    while let Some(key) = map.next_key::<String>()? {
                        match key.as_str() {
                            "name" => name = map.next_value()?,
                            "family" => family = map.next_value()?,
                            "description" => description = map.next_value()?,
                            "elements" => {
                                elements = map.next_value_seed(ElementsSeed { target_zs: self.target_zs })?;
                            }
                            _ => {
                                map.next_value::<IgnoredAny>()?;
                            }
                        }
                    }

                    Ok(BseBasis { name, family, description, elements })
                }
            }

            deserializer.deserialize_map(BseBasisVisitor { target_zs: self.target_zs })
        }
    }

    struct ElementsSeed<'a> {
        target_zs: &'a HashSet<String>,
    }

    impl<'de, 'a> DeserializeSeed<'de> for ElementsSeed<'a> {
        type Value = HashMap<String, BseElement>;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct ElementsVisitor<'a> {
                target_zs: &'a HashSet<String>,
            }

            impl<'de, 'a> Visitor<'de> for ElementsVisitor<'a> {
                type Value = HashMap<String, BseElement>;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("map of elements")
                }

                fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
                where
                    V: MapAccess<'de>,
                {
                    let mut elements = HashMap::new();
                    while let Some(key) = map.next_key::<String>()? {
                        if self.target_zs.contains(&key) {
                            elements.insert(key, map.next_value()?);
                        } else {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                    Ok(elements)
                }
            }

            deserializer.deserialize_map(ElementsVisitor { target_zs: self.target_zs })
        }
    }

    let target_zs: HashSet<String> = atoms.iter().map(|(z, _, _)| z.to_string()).collect();
    let mut deserializer = serde_json::Deserializer::from_str(json);
    let bse: BseBasis = BseBasisSeed { target_zs: &target_zs }
        .deserialize(&mut deserializer)
        .map_err(BasisSetError::JsonParse)?;

    let mut atom_basis = Vec::with_capacity(atoms.len());
    for (z, symbol, center) in atoms {
        let key = z.to_string();
        let elem = bse
            .elements
            .get(&key)
            .ok_or_else(|| BasisSetError::ElementNotFound { z: *z, basis: bse.name.clone() })?;

        let shells = elem
            .electron_shells
            .as_ref()
            .map(|shells| shells.iter().map(parse_shell).collect::<Result<Vec<_>, _>>())
            .transpose()?
            .unwrap_or_default();

        let ecp = if let Some(pots) = &elem.ecp_potentials {
            Some(EffectiveCorePotential {
                n_core_electrons: elem.ecp_electrons.unwrap_or(0),
                potentials: pots
                    .iter()
                    .map(parse_ecp_potential)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        } else {
            None
        };

        atom_basis.push((*z, symbol.clone(), *center, ElementBasis {
            atomic_number: *z,
            symbol: symbol.clone(),
            shells,
            ecp,
        }));
    }

    Ok(MolecularBasis {
        name: bse.name,
        family: bse.family,
        description: bse.description,
        atoms: atom_basis,
    })
}

/// Parse a single BSE electron shell into a `ContractedShell`.
fn parse_shell(shell: &BseElectronShell) -> Result<ContractedShell, BasisSetError> {
    let shell_type = match shell.function_type.as_str() {
        "gto" => ShellType::Cartesian,
        "gto_spherical" => ShellType::Spherical,
        other => return Err(BasisSetError::UnknownFunctionType(other.to_string())),
    };

    let angular_momentum: Vec<AngularMomentum> = shell
        .angular_momentum
        .iter()
        .map(|am| {
            if *am < 0 || *am > 255 {
                Err(BasisSetError::InvalidAngularMomentum(*am))
            } else {
                Ok(AngularMomentum(*am as u8))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let exponents: Vec<f64> = shell
        .exponents
        .iter()
        .map(|s| parse_fortran_float(s))
        .collect::<Result<Vec<_>, _>>()?;

    let coefficients: Vec<Vec<f64>> = shell
        .coefficients
        .iter()
        .map(|row| {
            row.iter()
                .map(|s| parse_fortran_float(s))
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;

    // Validate dimensions.
    if angular_momentum.len() != coefficients.len() {
        return Err(BasisSetError::DimensionMismatch {
            expected: angular_momentum.len(),
            got: coefficients.len(),
            context: "angular_momentum vs coefficients rows".to_string(),
        });
    }
    for (i, row) in coefficients.iter().enumerate() {
        if row.len() != exponents.len() {
            return Err(BasisSetError::DimensionMismatch {
                expected: exponents.len(),
                got: row.len(),
                context: format!("coefficients row {} vs exponents", i),
            });
        }
    }

    Ok(ContractedShell::combined(
        angular_momentum,
        shell_type,
        Vec3::ZERO, // Center set later when assigned to an atom
        exponents,
        coefficients,
    ))
}

/// Parse a single BSE ECP potential into an `EcpPotential`.
fn parse_ecp_potential(pot: &BseEcpPotential) -> Result<EcpPotential, BasisSetError> {
    let angular_momentum: Vec<AngularMomentum> = pot
        .angular_momentum
        .iter()
        .map(|am| {
            // BSE uses -1 for the "ul" (sum) potential.
            if *am == -1 {
                Ok(AngularMomentum(255))
            } else if *am >= 0 && *am <= 255 {
                Ok(AngularMomentum(*am as u8))
            } else {
                Err(BasisSetError::InvalidAngularMomentum(*am))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    let gaussian_exponents: Vec<f64> = pot
        .gaussian_exponents
        .iter()
        .map(|s| parse_fortran_float(s))
        .collect::<Result<Vec<_>, _>>()?;

    let coefficients: Vec<f64> = pot
        .coefficients
        .first()
        .ok_or_else(|| BasisSetError::DimensionMismatch {
            expected: 1,
            got: 0,
            context: "ECP coefficients rows".to_string(),
        })?
        .iter()
        .map(|s| parse_fortran_float(s))
        .collect::<Result<Vec<_>, _>>()?;

    if gaussian_exponents.len() != coefficients.len()
        || gaussian_exponents.len() != pot.r_exponents.len()
    {
        return Err(BasisSetError::DimensionMismatch {
            expected: gaussian_exponents.len(),
            got: coefficients.len(),
            context: "ECP exponents vs coefficients vs r_exponents".to_string(),
        });
    }

    Ok(EcpPotential {
        angular_momentum,
        r_exponents: pot.r_exponents.clone(),
        gaussian_exponents,
        coefficients,
    })
}

/// Parse a Fortran-style floating-point string (e.g. "0.3425250914E+01",
/// "0.1543289673E+00", ".309071490776").
///
/// BSE JSON stores all numeric values as strings in Fortran scientific
/// notation. Rust's `f64::from_str` handles standard `e` notation but
/// may fail on leading-dot or `D` exponent formats.
fn parse_fortran_float(s: &str) -> Result<f64, BasisSetError> {
    let trimmed = s.trim();
    // Replace Fortran 'D'/'d' exponents with 'e'.
    let normalized = trimmed.replace(['D', 'd'], "e");
    // Handle leading dot: ".5" → "0.5"
    let normalized = if normalized.starts_with('.') {
        format!("0{}", normalized)
    } else if normalized.starts_with("-.") {
        format!("-0{}", &normalized[1..])
    } else if normalized.starts_with("+.") {
        format!("+0{}", &normalized[1..])
    } else {
        normalized
    };
    normalized
        .parse::<f64>()
        .map_err(|e| BasisSetError::FloatParse { input: s.to_string(), source: e })
}

// ─── Error Type ───────────────────────────────────────────────────────────

/// Errors that can occur during basis set parsing or operations.
#[derive(Debug)]
pub enum BasisSetError {
    JsonParse(serde_json::Error),
    ElementNotFound { z: u32, basis: String },
    UnknownFunctionType(String),
    InvalidAngularMomentum(i32),
    DimensionMismatch { expected: usize, got: usize, context: String },
    FloatParse { input: String, source: std::num::ParseFloatError },
    EcpNotFound(AngularMomentum),
}

impl std::fmt::Display for BasisSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::JsonParse(e) => write!(f, "JSON parse error: {}", e),
            Self::ElementNotFound { z, basis } => write!(f, "element Z={} not found in basis set '{}'", z, basis),
            Self::UnknownFunctionType(s) => write!(f, "unknown function type: '{}' (expected 'gto' or 'gto_spherical')", s),
            Self::InvalidAngularMomentum(am) => write!(f, "invalid angular momentum value: {} (must be 0–255)", am),
            Self::DimensionMismatch { expected, got, context } => write!(f, "dimension mismatch: expected {}, got {} ({})", expected, got, context),
            Self::FloatParse { input, source } => write!(f, "failed to parse float from '{}': {}", input, source),
            Self::EcpNotFound(am) => write!(f, "ECP potential not found for angular momentum {}", am.0),
        }
    }
}

impl std::error::Error for BasisSetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::JsonParse(e) => Some(e),
            Self::FloatParse { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for BasisSetError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonParse(err)
    }
}

// ─── Element Symbol Lookup ────────────────────────────────────────────────

/// Atomic number → element symbol lookup (H through Lw, Z=1–103).
pub fn element_symbol(z: u32) -> Option<&'static str> {
    const SYMBOLS: [&str; 104] = [
        "n", "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne",
        "Na", "Mg", "Al", "Si", "P", "S", "Cl", "Ar", "K", "Ca",
        "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn",
        "Ga", "Ge", "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr",
        "Nb", "Mo", "Tc", "Ru", "Rh", "Pd", "Ag", "Cd", "In", "Sn",
        "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd",
        "Pm", "Sm", "Eu", "Gd", "Tb", "Dy", "Ho", "Er", "Tm", "Yb",
        "Lu", "Hf", "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg",
        "Tl", "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th",
        "Pa", "U", "Np", "Pu", "Am", "Cm", "Bk", "Cf", "Es", "Fm",
        "Md", "No", "Lr",
    ];
    if (z as usize) < SYMBOLS.len() {
        Some(SYMBOLS[z as usize])
    } else {
        None
    }
}

/// Element symbol → atomic number lookup.
pub fn atomic_number(symbol: &str) -> Option<u32> {
    for z in 1..=103u32 {
        if let Some(s) = element_symbol(z) {
            if s.eq_ignore_ascii_case(symbol) {
                return Some(z);
            }
        }
    }
    None
}

// ─── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── STO-3G for H and C (real BSE data) ──
    const STO3G_HC_JSON: &str = r#"{
        "molssi_bse_schema": {"schema_type": "complete", "schema_version": "0.1"},
        "name": "STO-3G",
        "family": "sto",
        "description": "STO-3G Minimal Basis (3 functions/AO)",
        "elements": {
            "1": {
                "electron_shells": [
                    {
                        "function_type": "gto",
                        "region": "",
                        "angular_momentum": [0],
                        "exponents": ["0.3425250914E+01", "0.6239137298E+00", "0.1688554040E+00"],
                        "coefficients": [["0.1543289673E+00", "0.5353281423E+00", "0.4446345422E+00"]]
                    }
                ],
                "references": []
            },
            "6": {
                "electron_shells": [
                    {
                        "function_type": "gto",
                        "region": "",
                        "angular_momentum": [0],
                        "exponents": ["0.7161683735E+02", "0.1304509632E+02", "0.3530512160E+01"],
                        "coefficients": [["0.1543289673E+00", "0.5353281423E+00", "0.4446345422E+00"]]
                    },
                    {
                        "function_type": "gto",
                        "region": "",
                        "angular_momentum": [0, 1],
                        "exponents": ["0.2941249355E+01", "0.6834830964E+00", "0.2222899159E+00"],
                        "coefficients": [
                            ["-0.9996722919E-01", "0.3995128261E+00", "0.7001154689E+00"],
                            ["0.1559162750E+00", "0.6076837186E+00", "0.3919573931E+00"]
                        ]
                    }
                ],
                "references": []
            }
        },
        "version": "1",
        "function_types": ["gto"],
        "names": ["STO-3G"],
        "tags": [],
        "role": "orbital",
        "auxiliaries": {}
    }"#;

    // ── def2-SVP for H (real BSE data) ──
    const DEF2_SVP_H_JSON: &str = r#"{
        "molssi_bse_schema": {"schema_type": "complete", "schema_version": "0.1"},
        "name": "def2-SVP",
        "family": "ahlrichs",
        "description": "def2-SVP",
        "elements": {
            "1": {
                "electron_shells": [
                    {
                        "function_type": "gto",
                        "region": "",
                        "angular_momentum": [0],
                        "exponents": ["13.0107010", "1.9622572", "0.44453796"],
                        "coefficients": [["0.19682158E-01", "0.13796524", "0.47831935"]]
                    },
                    {
                        "function_type": "gto",
                        "region": "",
                        "angular_momentum": [0],
                        "exponents": ["0.12194962"],
                        "coefficients": [["1.0000000"]]
                    },
                    {
                        "function_type": "gto",
                        "region": "",
                        "angular_momentum": [1],
                        "exponents": ["0.8000000"],
                        "coefficients": [["1.0000000"]]
                    }
                ],
                "references": []
            }
        },
        "version": "1",
        "function_types": ["gto"],
        "names": ["def2-SVP"],
        "tags": [],
        "role": "orbital",
        "auxiliaries": {},
        "auxiliaries": {}
    }"#;

    // ── def2-ECP for Xe (Z=54) (real BSE data, truncated) ──
    const DEF2_ECP_XE_JSON: &str = r#"{
        "molssi_bse_schema": {"schema_type": "complete", "schema_version": "0.1"},
        "name": "def2-ECP",
        "family": "ahlrichs",
        "description": "ECP for use with Ahlrichs def2 basis sets",
        "elements": {
            "54": {
                "ecp_potentials": [
                    {
                        "angular_momentum": [3],
                        "coefficients": [["-23.08929500", "-30.07447500", "-0.28822700", "-0.38692400"]],
                        "ecp_type": "scalar_ecp",
                        "r_exponents": [2, 2, 2, 2],
                        "gaussian_exponents": ["20.88155700", "20.78344300", "5.25338900", "5.36118800"]
                    },
                    {
                        "angular_momentum": [0],
                        "coefficients": [["49.99796200", "281.01330300", "61.53825500", "23.08929500", "30.07447500", "0.28822700", "0.38692400"]],
                        "ecp_type": "scalar_ecp",
                        "r_exponents": [2, 2, 2, 2, 2, 2, 2],
                        "gaussian_exponents": ["40.00518400", "17.81221400", "9.30415000", "20.88155700", "20.78344300", "5.25338900", "5.36118800"]
                    },
                    {
                        "angular_momentum": [1],
                        "coefficients": [["67.43914200", "134.87471100", "14.66330000", "29.35473000", "23.08929500", "30.07447500", "0.28822700", "0.38692400"]],
                        "ecp_type": "scalar_ecp",
                        "r_exponents": [2, 2, 2, 2, 2, 2, 2, 2],
                        "gaussian_exponents": ["15.70177200", "15.25860800", "9.29218400", "8.55900300", "20.88155700", "20.78344300", "5.25338900", "5.36118800"]
                    },
                    {
                        "angular_momentum": [2],
                        "coefficients": [["35.43690800", "53.19577200", "9.04623200", "13.22368100", "0.08485300", "0.04415500", "23.08929500", "30.07447500", "0.28822700", "0.38692400"]],
                        "ecp_type": "scalar_ecp",
                        "r_exponents": [2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
                        "gaussian_exponents": ["15.18560000", "14.28450000", "7.12188900", "6.99196300", "0.62394600", "0.64728400", "20.88155700", "20.78344300", "5.25338900", "5.36118800"]
                    }
                ],
                "ecp_electrons": 28,
                "references": []
            }
        },
        "version": "1",
        "function_types": ["scalar_ecp"],
        "names": ["def2-ECP"],
        "tags": [],
        "role": "orbital",
        "auxiliaries": {}
    }"#;

    // ── AngularMomentum tests ──

    #[test]
    fn angular_momentum_letters() {
        assert_eq!(AngularMomentum(0).letter(), 's');
        assert_eq!(AngularMomentum(1).letter(), 'p');
        assert_eq!(AngularMomentum(2).letter(), 'd');
        assert_eq!(AngularMomentum(3).letter(), 'f');
        assert_eq!(AngularMomentum(4).letter(), 'g');
    }

    #[test]
    fn angular_momentum_from_letter() {
        assert_eq!(AngularMomentum::from_letter('s'), Some(AngularMomentum(0)));
        assert_eq!(AngularMomentum::from_letter('P'), Some(AngularMomentum(1)));
        assert_eq!(AngularMomentum::from_letter('d'), Some(AngularMomentum(2)));
        assert_eq!(AngularMomentum::from_letter('z'), None);
    }

    #[test]
    fn angular_momentum_function_counts() {
        assert_eq!(AngularMomentum(0).n_cartesian(), 1);   // s
        assert_eq!(AngularMomentum(1).n_cartesian(), 3);   // p
        assert_eq!(AngularMomentum(2).n_cartesian(), 6);   // d
        assert_eq!(AngularMomentum(3).n_cartesian(), 10);  // f
        assert_eq!(AngularMomentum(0).n_spherical(), 1);   // s
        assert_eq!(AngularMomentum(1).n_spherical(), 3);   // p
        assert_eq!(AngularMomentum(2).n_spherical(), 5);   // d
        assert_eq!(AngularMomentum(3).n_spherical(), 7);   // f
    }

    // ── PrimitiveGaussian tests ──

    #[test]
    fn primitive_s_type_evaluate() {
        let prim = PrimitiveGaussian::s(1.0, Vec3::ZERO);
        // At the center, exp(0) = 1.
        let val = prim.evaluate(&Vec3::ZERO);
        assert!((val - 1.0).abs() < 1e-12);
        // At distance 1, exp(-1) ≈ 0.3679.
        let val = prim.evaluate(&Vec3::new(1.0, 0.0, 0.0));
        assert!((val - (-1.0f64).exp()).abs() < 1e-12);
    }

    #[test]
    fn primitive_p_type_evaluate() {
        let prim = PrimitiveGaussian {
            exponent: 1.0,
            center: Vec3::ZERO,
            l: [1, 0, 0], // px
        };
        // px at (1,0,0) = 1 * exp(-1).
        let val = prim.evaluate(&Vec3::new(1.0, 0.0, 0.0));
        assert!((val - (-1.0f64).exp()).abs() < 1e-12);
        // px at (0,1,0) = 0 (x-component is 0).
        let val = prim.evaluate(&Vec3::new(0.0, 1.0, 0.0));
        assert!(val.abs() < 1e-15);
    }

    #[test]
    fn primitive_normalization_s() {
        // s-type normalization: N = (2α/π)^(3/4)
        let prim = PrimitiveGaussian::s(1.0, Vec3::ZERO);
        let expected = (2.0 / std::f64::consts::PI).powf(0.75);
        assert!((prim.normalization() - expected).abs() < 1e-12);
    }

    // ── ContractedShell tests ──

    #[test]
    fn contracted_shell_n_primitives() {
        let shell = ContractedShell::new(
            AngularMomentum(0),
            ShellType::Cartesian,
            Vec3::ZERO,
            vec![3.425, 0.624, 0.169],
            vec![0.154, 0.535, 0.445],
        );
        assert_eq!(shell.n_primitives(), 3);
        assert_eq!(shell.n_functions(), 1); // s-type: 1 function
    }

    #[test]
    fn contracted_shell_split_sp() {
        let shell = ContractedShell::combined(
            vec![AngularMomentum(0), AngularMomentum(1)],
            ShellType::Cartesian,
            Vec3::ZERO,
            vec![2.941, 0.683, 0.222],
            vec![
                vec![-0.100, 0.400, 0.700],
                vec![0.156, 0.608, 0.392],
            ],
        );
        assert_eq!(shell.n_functions(), 4); // 1 (s) + 3 (p)
        let split = shell.split();
        assert_eq!(split.len(), 2);
        assert_eq!(split[0].max_am, AngularMomentum(0));
        assert_eq!(split[1].max_am, AngularMomentum(1));
        assert_eq!(split[0].n_functions(), 1);
        assert_eq!(split[1].n_functions(), 3);
    }

    #[test]
    fn cartesian_powers_s() {
        let shell = ContractedShell::new(
            AngularMomentum(0),
            ShellType::Cartesian,
            Vec3::ZERO,
            vec![1.0],
            vec![1.0],
        );
        let powers = shell.cartesian_powers(AngularMomentum(0));
        assert_eq!(powers, vec![[0, 0, 0]]);
    }

    #[test]
    fn cartesian_powers_p() {
        let shell = ContractedShell::new(
            AngularMomentum(1),
            ShellType::Cartesian,
            Vec3::ZERO,
            vec![1.0],
            vec![1.0],
        );
        let powers = shell.cartesian_powers(AngularMomentum(1));
        assert_eq!(powers, vec![[1, 0, 0], [0, 1, 0], [0, 0, 1]]);
    }

    #[test]
    fn cartesian_powers_d() {
        let shell = ContractedShell::new(
            AngularMomentum(2),
            ShellType::Cartesian,
            Vec3::ZERO,
            vec![1.0],
            vec![1.0],
        );
        let powers = shell.cartesian_powers(AngularMomentum(2));
        assert_eq!(powers.len(), 6);
        assert!(powers.contains(&[2, 0, 0]));
        assert!(powers.contains(&[0, 2, 0]));
        assert!(powers.contains(&[0, 0, 2]));
        assert!(powers.contains(&[1, 1, 0]));
        assert!(powers.contains(&[1, 0, 1]));
        assert!(powers.contains(&[0, 1, 1]));
    }

    // ── BSE JSON parsing tests ──

    #[test]
    fn parse_sto3g_hydrogen() {
        let atoms = vec![(1u32, "H".to_string(), Vec3::ZERO)];
        let basis = parse_bse_json(STO3G_HC_JSON, &atoms).expect("parse STO-3G H");

        assert_eq!(basis.name, "STO-3G");
        assert_eq!(basis.family, "sto");
        assert_eq!(basis.atoms.len(), 1);

        let (_, _, _, elem) = &basis.atoms[0];
        assert_eq!(elem.atomic_number, 1);
        assert_eq!(elem.shells.len(), 1);
        assert_eq!(elem.shells[0].n_primitives(), 3);
        assert_eq!(elem.shells[0].max_am, AngularMomentum(0));
        assert_eq!(elem.n_functions(), 1); // 1 s function
    }

    #[test]
    fn parse_sto3g_carbon() {
        let atoms = vec![(6u32, "C".to_string(), Vec3::ZERO)];
        let basis = parse_bse_json(STO3G_HC_JSON, &atoms).expect("parse STO-3G C");

        let (_, _, _, elem) = &basis.atoms[0];
        assert_eq!(elem.shells.len(), 2);
        // First shell: 1s (3 primitives).
        assert_eq!(elem.shells[0].max_am, AngularMomentum(0));
        assert_eq!(elem.shells[0].n_primitives(), 3);
        // Second shell: SP combined (3 primitives, 4 functions).
        assert_eq!(elem.shells[1].angular_momentum, vec![AngularMomentum(0), AngularMomentum(1)]);
        assert_eq!(elem.shells[1].n_primitives(), 3);
        assert_eq!(elem.shells[1].n_functions(), 4); // 1s + 3p
        // Total: 1 + 4 = 5 functions.
        assert_eq!(elem.n_functions(), 5);
    }

    #[test]
    fn parse_sto3g_methane() {
        // CH4: C at origin, 4 H atoms.
        let ch_dist = 1.089; // Bohr
        let atoms = vec![
            (6u32, "C".to_string(), Vec3::ZERO),
            (1u32, "H".to_string(), Vec3::new(ch_dist, ch_dist, ch_dist)),
            (1u32, "H".to_string(), Vec3::new(-ch_dist, -ch_dist, ch_dist)),
            (1u32, "H".to_string(), Vec3::new(-ch_dist, ch_dist, -ch_dist)),
            (1u32, "H".to_string(), Vec3::new(ch_dist, -ch_dist, -ch_dist)),
        ];
        let basis = parse_bse_json(STO3G_HC_JSON, &atoms).expect("parse STO-3G CH4");

        assert_eq!(basis.atoms.len(), 5);
        // C: 5 functions, 4×H: 4 functions each = 9 total.
        assert_eq!(basis.n_functions(), 9);
        assert_eq!(basis.max_angular_momentum(), AngularMomentum(1)); // p
    }

    #[test]
    fn parse_def2_svp_hydrogen() {
        let atoms = vec![(1u32, "H".to_string(), Vec3::ZERO)];
        let basis = parse_bse_json(DEF2_SVP_H_JSON, &atoms).expect("parse def2-SVP H");

        let (_, _, _, elem) = &basis.atoms[0];
        assert_eq!(elem.shells.len(), 3);
        // Shell 1: contracted s (3 primitives).
        assert_eq!(elem.shells[0].n_primitives(), 3);
        // Shell 2: single primitive s.
        assert_eq!(elem.shells[1].n_primitives(), 1);
        // Shell 3: single primitive p.
        assert_eq!(elem.shells[2].max_am, AngularMomentum(1));
        // Total: 1 + 1 + 3 = 5 functions.
        assert_eq!(elem.n_functions(), 5);
    }

    #[test]
    fn parse_def2_ecp_xenon() {
        let atoms = vec![(54u32, "Xe".to_string(), Vec3::ZERO)];
        let basis = parse_bse_json(DEF2_ECP_XE_JSON, &atoms).expect("parse def2-ECP Xe");

        let (_, _, _, elem) = &basis.atoms[0];
        let ecp = elem.ecp.as_ref().expect("Xe should have ECP");
        assert_eq!(ecp.n_core_electrons, 28);
        assert_eq!(ecp.potentials.len(), 4); // s, p, d, f

        // Check that we can look up each angular momentum.
        assert!(ecp.potential_for_am(AngularMomentum(0)).is_some()); // U_s
        assert!(ecp.potential_for_am(AngularMomentum(1)).is_some()); // U_p
        assert!(ecp.potential_for_am(AngularMomentum(2)).is_some()); // U_d
        assert!(ecp.potential_for_am(AngularMomentum(3)).is_some()); // U_f
    }

    #[test]
    fn parse_fortran_float_formats() {
        assert!((parse_fortran_float("0.3425250914E+01").unwrap() - 3.425250914).abs() < 1e-12);
        assert!((parse_fortran_float("0.1543289673E+00").unwrap() - 0.1543289673).abs() < 1e-12);
        assert!((parse_fortran_float("13.0107010").unwrap() - 13.0107010).abs() < 1e-12);
        assert!((parse_fortran_float("1.0000000").unwrap() - 1.0).abs() < 1e-12);
        assert!((parse_fortran_float(".309071490776").unwrap() - 0.309071490776).abs() < 1e-12);
        assert!((parse_fortran_float("0.19682158E-01").unwrap() - 0.019682158).abs() < 1e-12);
    }

    #[test]
    fn element_not_found_error() {
        let atoms = vec![(99u32, "Es".to_string(), Vec3::ZERO)];
        let result = parse_bse_json(STO3G_HC_JSON, &atoms);
        assert!(matches!(result, Err(BasisSetError::ElementNotFound { z: 99, .. })));
    }

    // ── Category theory tests ──

    #[test]
    fn contracted_shell_implements_object() {
        use super::super::super::category_theory::Object;
        let shell = ContractedShell::new(
            AngularMomentum(1),
            ShellType::Cartesian,
            Vec3::new(1.0, 2.0, 3.0),
            vec![1.0, 2.0],
            vec![0.5, 0.5],
        );
        let props = shell.properties();
        assert_eq!(props.angular_momentum, AngularMomentum(1));
        assert_eq!(props.shell_type, ShellType::Cartesian);
        assert_eq!(props.n_functions, 3); // p: 3 Cartesian
        assert_eq!(props.center, Vec3::new(1.0, 2.0, 3.0));
    }

    // ── Element lookup tests ──

    #[test]
    fn element_symbol_lookup() {
        assert_eq!(element_symbol(1), Some("H"));
        assert_eq!(element_symbol(6), Some("C"));
        assert_eq!(element_symbol(8), Some("O"));
        assert_eq!(element_symbol(54), Some("Xe"));
        assert_eq!(element_symbol(0), Some("n"));
        assert_eq!(element_symbol(200), None);
    }

    #[test]
    fn atomic_number_lookup() {
        assert_eq!(atomic_number("H"), Some(1));
        assert_eq!(atomic_number("C"), Some(6));
        assert_eq!(atomic_number("xe"), Some(54));
        assert_eq!(atomic_number("Xe"), Some(54));
        assert_eq!(atomic_number("ZZ"), None);
    }

    // ── Shell evaluation tests ──

    #[test]
    fn shell_evaluate_sto3g_h_at_center() {
        let atoms = vec![(1u32, "H".to_string(), Vec3::ZERO)];
        let basis = parse_bse_json(STO3G_HC_JSON, &atoms).unwrap();
        let (_, _, _, elem) = &basis.atoms[0];
        let shell = &elem.shells[0];
        let vals = shell.evaluate(&Vec3::ZERO);
        assert_eq!(vals.len(), 1);
        // At center, each primitive evaluates to 1.0, so the value is
        // the sum of coefficients.
        let expected: f64 = 0.1543289673 + 0.5353281423 + 0.4446345422;
        assert!((vals[0] - expected).abs() < 1e-6);
    }

    #[test]
    fn molecular_basis_all_shells() {
        let atoms = vec![
            (1u32, "H".to_string(), Vec3::ZERO),
            (6u32, "C".to_string(), Vec3::new(1.0, 0.0, 0.0)),
        ];
        let basis = parse_bse_json(STO3G_HC_JSON, &atoms).unwrap();
        // H: 1 shell, C: 2 shells → 3 total.
        assert_eq!(basis.all_shells().len(), 3);
        // Split: H: 1, C: 1 (s) + 2 (s, p from SP) → 3 split shells.
        assert_eq!(basis.all_split_shells().len(), 3);
    }
}
