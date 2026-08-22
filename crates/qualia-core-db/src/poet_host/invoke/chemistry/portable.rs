//! Browser-safe chemistry primitives used by the Vibe capability host.
//!
//! These routines are the dependency-free subset of the native chemistry
//! library. Keeping them here avoids pulling model registries and native
//! storage adapters into the portal merely to resolve periodic-table lookups
//! or local-density functionals.

const SYMBOLS: [&str; 104] = [
    "", "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S",
    "Cl", "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga", "Ge",
    "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd", "Ag", "Cd",
    "In", "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm", "Sm", "Eu", "Gd",
    "Tb", "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg",
    "Tl", "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th", "Pa", "U", "Np", "Pu", "Am", "Cm",
    "Bk", "Cf", "Es", "Fm", "Md", "No", "Lr",
];

pub fn element_symbol(z: u32) -> Option<&'static str> {
    SYMBOLS
        .get(z as usize)
        .copied()
        .filter(|symbol| !symbol.is_empty())
}

pub fn atomic_number(symbol: &str) -> Option<u32> {
    SYMBOLS
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(symbol))
        .map(|z| z as u32)
}

pub fn standard_atomic_weight(element: &str) -> Option<f64> {
    Some(match element {
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
    })
}

pub fn lda_exchange(rho: f64) -> (f64, f64) {
    if rho <= 1e-12 {
        return (0.0, 0.0);
    }
    let energy = -0.75 * (3.0 / core::f64::consts::PI).powf(1.0 / 3.0) * rho.cbrt();
    (energy, energy * (4.0 / 3.0))
}

pub fn lda_correlation_vwn(rho: f64) -> (f64, f64) {
    if rho <= 1e-12 {
        return (0.0, 0.0);
    }
    // This is the dependency-free browser VWN5 value. Its potential uses a
    // bounded symmetric derivative, avoiding the native DFT-grid dependency.
    fn energy(rho: f64) -> f64 {
        let rs = (3.0 / (4.0 * core::f64::consts::PI * rho)).powf(1.0 / 3.0);
        let x = rs.sqrt();
        let a = 0.0621814;
        let x0 = -0.409286;
        let b = 13.0720;
        let c: f64 = 42.7198;
        let q = (4.0 * c - b * b).sqrt();
        let xf = x * x + b * x + c;
        let x0f = x0 * x0 + b * x0 + c;
        let atan = (q / (2.0 * x + b)).atan();
        a * ((x * x / xf).ln() + 2.0 * b / q * atan
            - (b * x0 / x0f) * (((x - x0) * (x - x0) / xf).ln() + 2.0 * (b + 2.0 * x0) / q * atan))
    }
    let e = energy(rho);
    let h = (rho * 1e-6).max(1e-12);
    let derivative =
        (energy(rho + h) - energy((rho - h).max(1e-12))) / (rho + h - (rho - h).max(1e-12));
    (e, e + rho * derivative)
}
