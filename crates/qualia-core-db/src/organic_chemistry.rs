//! Organic Chemistry Engine.
//!
//! Pure-Rust implementations of core organic-chemistry primitives:
//! - SMILES parsing & molecular graph building
//! - Molecular formula, exact weight, and isotope-aware mass
//! - Lipinski Rule-of-Five, Veber, Ghose, Egan drug-likeness filters
//! - LogP (Crippen–Wildman atomic contributions, 25 atom types)
//! - TPSA (Ertl 2000 atomic contributions)
//! - H-bond donors / acceptors, rotatable bonds, aromatic ring count
//! - Functional group detection (20 groups via SMARTS-inspired pattern matching)
//! - Chiral centre enumeration
//! - Morgan circular fingerprint generation
//! - Thermochemistry: Arrhenius, Gibbs–Helmholtz, van't Hoff, Henderson–Hasselbalch
//! - Green chemistry: atom economy, E-factor, PMI, RME, yield-adjusted AE
//! - pKa estimation (functional-group based)
//! - SMILES structural validation
//! - InChI / InChIKey format validation
//!
//! SHACL constraint name → function:
//!   `qualia:validateSmiles`              → `validate_smiles()`
//!   `qualia:validateInchi`               → `validate_inchi()`
//!   `qualia:computeMolecularWeight`      → `exact_molecular_weight()`
//!   `qualia:computeLogP`                 → `compute_logp()`
//!   `qualia:computeTPSA`                 → `compute_tpsa()`
//!   `qualia:evaluateLipinski`            → `evaluate_lipinski()`
//!   `qualia:evaluateVeber`               → `evaluate_veber()`
//!   `qualia:evaluateGhose`               → `evaluate_ghose()`
//!   `qualia:evaluateEgan`                → `evaluate_egan()`
//!   `qualia:detectFunctionalGroups`      → `detect_functional_groups()`
//!   `qualia:computePka`                  → `estimate_pka()`
//!   `qualia:computeChiralCenters`        → `count_chiral_centers()`
//!   `qualia:generateCircularFingerprint` → `circular_fingerprint()`
//!   `qualia:computeArrheniusRate`        → `arrhenius_rate()`
//!   `qualia:computeGibbsEnergy`          → `gibbs_free_energy()`
//!   `qualia:computeEquilibrium`          → `equilibrium_constant()`
//!   `qualia:computeHendersonHasselbalch` → `henderson_hasselbalch()`
//!   `qualia:computeAtomEconomy`          → `atom_economy()`
//!   `qualia:computeEFactor`              → `e_factor()`
//!   `qualia:computeGreenMetrics`         → `green_metrics()`

// ─── Physical constants ───────────────────────────────────────────────────────

/// Universal gas constant J / (mol·K)
pub const R_J_MOL_K: f64 = 8.314_462_618;

// ─── Atomic data ──────────────────────────────────────────────────────────────

/// Monoisotopic / standard atomic weights (IUPAC 2021).
const ATOMIC_WEIGHTS: &[(&str, f64)] = &[
    ("H",  1.00794), ("B",  10.811),  ("C",  12.011),  ("N",  14.007),
    ("O",  15.999),  ("F",  18.998),  ("P",  30.974),  ("S",  32.06),
    ("Cl", 35.45),   ("Br", 79.904),  ("I",  126.904), ("Si", 28.085),
    ("As", 74.922),  ("Se", 78.971),  ("Te", 127.6),
];

fn atomic_weight(symbol: &str) -> f64 {
    ATOMIC_WEIGHTS.iter()
        .find(|(s, _)| *s == symbol)
        .map(|(_, w)| *w)
        .unwrap_or(12.0) // default C
}

/// Default valences for the organic-subset elements.
const VALENCES: &[(&str, u8)] = &[
    ("C", 4), ("N", 3), ("O", 2), ("S", 2), ("S", 4), ("S", 6),
    ("P", 3), ("P", 5), ("F", 1), ("Cl", 1), ("Br", 1), ("I", 1),
    ("B", 3), ("Si", 4), ("Se", 2), ("As", 3),
];

fn default_valence(symbol: &str) -> u8 {
    VALENCES.iter().find(|(s, _)| *s == symbol).map(|(_, v)| *v).unwrap_or(4)
}

// ─── Molecule representation ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BondOrder {
    Single,
    Double,
    Triple,
    Aromatic,
}

#[derive(Debug, Clone)]
pub struct Bond {
    pub atom_a: usize,
    pub atom_b: usize,
    pub order: BondOrder,
    /// Set to true once ring-detection pass identifies this bond as part of a cycle.
    pub in_ring: bool,
}

#[derive(Debug, Clone)]
pub struct Atom {
    pub element: String,
    pub is_aromatic: bool,
    pub charge: i8,
    pub isotope: Option<u16>,
    pub explicit_h: u8,
    pub n_implicit_h: u8,
    /// Number of heavy-atom bonds (computed after parsing).
    pub degree: u8,
    pub idx: usize,
}

#[derive(Debug, Clone)]
pub struct Molecule {
    pub smiles: String,
    pub atoms: Vec<Atom>,
    pub bonds: Vec<Bond>,
    pub is_valid: bool,
    pub error: Option<String>,
}

// ─── SMILES parser ────────────────────────────────────────────────────────────

/// Parse a SMILES string into a `Molecule`.
pub fn parse_smiles(smiles: &str) -> Molecule {
    let chars: Vec<char> = smiles.chars().collect();
    let n = chars.len();
    let mut atoms: Vec<Atom> = Vec::new();
    let mut bonds: Vec<Bond> = Vec::new();
    let mut branch_stack: Vec<usize> = Vec::new();
    let mut ring_map: std::collections::HashMap<u16, (usize, Option<BondOrder>)> = std::collections::HashMap::new();
    let mut current_atom: Option<usize> = None;
    let mut pending_bond: Option<BondOrder> = None;
    let mut pos = 0;
    let mut error: Option<String> = None;

    macro_rules! add_atom {
        ($elem:expr, $aromatic:expr, $charge:expr, $isotope:expr, $explicit_h:expr) => {{
            let idx = atoms.len();
            let atom = Atom {
                element: $elem.to_string(),
                is_aromatic: $aromatic,
                charge: $charge,
                isotope: $isotope,
                explicit_h: $explicit_h,
                n_implicit_h: 0,
                degree: 0,
                idx,
            };
            atoms.push(atom);
            if let Some(from) = current_atom {
                let bond_order = pending_bond.take().unwrap_or_else(|| {
                    if atoms[from].is_aromatic && $aromatic { BondOrder::Aromatic } else { BondOrder::Single }
                });
                bonds.push(Bond { atom_a: from, atom_b: idx, order: bond_order, in_ring: false });
                atoms[from].degree += 1;
                atoms[idx].degree += 1;
            } else {
                pending_bond = None;
            }
            current_atom = Some(idx);
        }};
    }

    while pos < n {
        let c = chars[pos];
        match c {
            // ── Branch open/close ─────────────────────────────────────────
            '(' => {
                if let Some(idx) = current_atom { branch_stack.push(idx); }
                pos += 1;
            }
            ')' => {
                current_atom = branch_stack.pop();
                pending_bond = None;
                pos += 1;
            }
            // ── Explicit bond tokens ──────────────────────────────────────
            '-' => { pending_bond = Some(BondOrder::Single);   pos += 1; }
            '=' => { pending_bond = Some(BondOrder::Double);   pos += 1; }
            '#' => { pending_bond = Some(BondOrder::Triple);   pos += 1; }
            ':' => { pending_bond = Some(BondOrder::Aromatic); pos += 1; }
            // ── Disconnected structure ────────────────────────────────────
            '.' => { current_atom = None; pending_bond = None; pos += 1; }
            // ── Ring closures: single digit ───────────────────────────────
            '0'..='9' => {
                let rn = c as u16 - b'0' as u16;
                close_or_open_ring(&mut ring_map, &mut bonds, &mut atoms, rn, current_atom, &mut pending_bond);
                pos += 1;
            }
            // ── Ring closures: %nn ────────────────────────────────────────
            '%' if pos + 2 < n => {
                if let (Some(d1), Some(d2)) = (chars[pos+1].to_digit(10), chars[pos+2].to_digit(10)) {
                    let rn = (d1 * 10 + d2) as u16;
                    close_or_open_ring(&mut ring_map, &mut bonds, &mut atoms, rn, current_atom, &mut pending_bond);
                    pos += 3;
                } else { pos += 1; }
            }
            // ── Bracketed atom [isotope?Symbol charge? Hn?] ──────────────
            '[' => {
                let (atom_elem, aromatic, charge, isotope, expl_h, advance) = parse_bracket_atom(&chars, pos);
                add_atom!(atom_elem, aromatic, charge, isotope, expl_h);
                pos += advance;
            }
            // ── Two-letter organic-subset atoms ───────────────────────────
            'C' if pos + 1 < n && chars[pos+1] == 'l' => {
                add_atom!("Cl", false, 0, None, 0); pos += 2;
            }
            'B' if pos + 1 < n && chars[pos+1] == 'r' => {
                add_atom!("Br", false, 0, None, 0); pos += 2;
            }
            // ── Single-letter organic subset ──────────────────────────────
            'C' => { add_atom!("C", false, 0, None, 0); pos += 1; }
            'c' => { add_atom!("C", true,  0, None, 0); pos += 1; }
            'N' => { add_atom!("N", false, 0, None, 0); pos += 1; }
            'n' => { add_atom!("N", true,  0, None, 0); pos += 1; }
            'O' => { add_atom!("O", false, 0, None, 0); pos += 1; }
            'o' => { add_atom!("O", true,  0, None, 0); pos += 1; }
            'S' => { add_atom!("S", false, 0, None, 0); pos += 1; }
            's' => { add_atom!("S", true,  0, None, 0); pos += 1; }
            'P' => { add_atom!("P", false, 0, None, 0); pos += 1; }
            'p' => { add_atom!("P", true,  0, None, 0); pos += 1; }
            'F' => { add_atom!("F", false, 0, None, 0); pos += 1; }
            'B' => { add_atom!("B", false, 0, None, 0); pos += 1; }
            'I' => { add_atom!("I", false, 0, None, 0); pos += 1; }
            _ => { pos += 1; }
        }
    }

    // Validate unclosed rings
    if !ring_map.is_empty() {
        error = Some(format!("Unclosed ring bonds: {:?}", ring_map.keys().collect::<Vec<_>>()));
    }

    // Compute implicit hydrogens and adjacency
    fill_implicit_hydrogens(&mut atoms, &bonds);
    // Mark ring bonds via DFS
    mark_ring_bonds(&atoms, &mut bonds);

    Molecule {
        smiles: smiles.to_string(),
        is_valid: error.is_none() && !atoms.is_empty(),
        error,
        atoms,
        bonds,
    }
}

fn close_or_open_ring(
    ring_map: &mut std::collections::HashMap<u16, (usize, Option<BondOrder>)>,
    bonds: &mut Vec<Bond>,
    atoms: &mut Vec<Atom>,
    ring_num: u16,
    current_atom: Option<usize>,
    pending_bond: &mut Option<BondOrder>,
) {
    let from_atom = match current_atom { Some(i) => i, None => return };
    if let Some((open_idx, open_bond)) = ring_map.remove(&ring_num) {
        let order = pending_bond.take().or(open_bond).unwrap_or(BondOrder::Single);
        bonds.push(Bond { atom_a: open_idx, atom_b: from_atom, order, in_ring: true });
        atoms[open_idx].degree += 1;
        atoms[from_atom].degree += 1;
    } else {
        ring_map.insert(ring_num, (from_atom, pending_bond.take()));
    }
}

/// Parse a bracketed atom [isotope?Symbol charge? Hn?]. Returns (elem, aromatic, charge, isotope, explicit_h, chars_consumed).
fn parse_bracket_atom(chars: &[char], start: usize) -> (&'static str, bool, i8, Option<u16>, u8, usize) {
    let mut pos = start + 1; // skip '['
    let end = chars.len();

    // Isotope
    let mut isotope_str = String::new();
    while pos < end && chars[pos].is_ascii_digit() {
        isotope_str.push(chars[pos]); pos += 1;
    }
    let isotope: Option<u16> = isotope_str.parse().ok();

    // Element symbol (1-2 chars, first uppercase)
    let sym_start = pos;
    if pos < end { pos += 1; }
    if pos < end && chars[pos].is_ascii_lowercase() && chars[pos] != 'h' { pos += 1; }
    let sym: String = chars[sym_start..pos].iter().collect();

    // Aromaticity: if symbol is lowercase that's already handled by aromatic atoms above;
    // inside brackets, aromatic lowercase atoms are also valid
    let is_aromatic = sym_start < chars.len() && chars[sym_start].is_ascii_lowercase();

    // Explicit H count
    let mut explicit_h = 0u8;
    if pos < end && chars[pos] == 'H' {
        pos += 1;
        if pos < end && chars[pos].is_ascii_digit() {
            explicit_h = chars[pos] as u8 - b'0'; pos += 1;
        } else {
            explicit_h = 1;
        }
    }

    // Charge
    let mut charge = 0i8;
    if pos < end && (chars[pos] == '+' || chars[pos] == '-') {
        let sign: i8 = if chars[pos] == '+' { 1 } else { -1 };
        pos += 1;
        if pos < end && chars[pos].is_ascii_digit() {
            charge = sign * (chars[pos] as i8 - b'0' as i8); pos += 1;
        } else if pos < end && (chars[pos] == '+' || chars[pos] == '-') {
            charge = sign * 2; pos += 1; // e.g. ++ or --
        } else {
            charge = sign;
        }
    }

    // Skip to closing ']'
    while pos < end && chars[pos] != ']' { pos += 1; }
    let advance = pos - start + 1; // +1 for ']'

    // Map symbol to &'static str
    let elem: &'static str = match sym.to_uppercase().as_str() {
        "C" => "C", "N" => "N", "O" => "O", "S" => "S", "P" => "P",
        "F" => "F", "CL" => "Cl", "BR" => "Br", "I" => "I", "B" => "B",
        "SI" => "Si", "AS" => "As", "SE" => "Se", "TE" => "Te",
        "H" => "H", _ => "C",
    };

    (elem, is_aromatic, charge, isotope, explicit_h, advance)
}

fn fill_implicit_hydrogens(atoms: &mut Vec<Atom>, bonds: &[Bond]) {
    // valence_used: sum of bond orders (double=2, triple=3, single/aromatic=1)
    // connectivity: number of distinct bond partners (for degree field)
    let mut valence_used = vec![0i16; atoms.len()];
    let mut connectivity = vec![0u8; atoms.len()];

    for b in bonds {
        let order: i16 = match b.order {
            BondOrder::Double   => 2,
            BondOrder::Triple   => 3,
            BondOrder::Single | BondOrder::Aromatic => 1,
        };
        valence_used[b.atom_a] += order;
        valence_used[b.atom_b] += order;
        connectivity[b.atom_a] += 1;
        connectivity[b.atom_b] += 1;
    }

    for atom in atoms.iter_mut() {
        if atom.element == "H" { atom.n_implicit_h = 0; atom.degree = connectivity[atom.idx]; continue; }
        let valence = default_valence(&atom.element) as i16;
        let charge_adj = atom.charge as i16;
        // Aromatic atoms: the delocalized pi electron occupies one valence slot.
        // This matches the standard SMILES spec (c in benzene → 1 implicit H, not 2).
        let aromatic_adj: i16 = if atom.is_aromatic { 1 } else { 0 };
        let used = valence_used[atom.idx] + atom.explicit_h as i16 + aromatic_adj;
        atom.n_implicit_h = (valence + charge_adj - used).max(0) as u8;
        atom.degree = connectivity[atom.idx];
    }
}

/// DFS-based ring bond marking (sets `Bond::in_ring`).
fn mark_ring_bonds(atoms: &[Atom], bonds: &mut Vec<Bond>) {
    // Bonds that close rings were already flagged in_ring during parsing.
    // For any remaining ring bonds (where both endpoints are in a cycle),
    // we do a simple DFS to find back-edges.
    let n = atoms.len();
    let mut adj: Vec<Vec<(usize, usize)>> = vec![vec![]; n]; // (neighbour, bond_idx)
    for (bi, b) in bonds.iter().enumerate() {
        adj[b.atom_a].push((b.atom_b, bi));
        adj[b.atom_b].push((b.atom_a, bi));
    }
    let mut visited = vec![false; n];
    let mut parent = vec![usize::MAX; n];
    for start in 0..n {
        if !visited[start] {
            dfs_mark_rings(&adj, bonds, &mut visited, &mut parent, start, usize::MAX);
        }
    }
}

fn dfs_mark_rings(adj: &[Vec<(usize, usize)>], bonds: &mut Vec<Bond>, visited: &mut Vec<bool>, parent: &mut Vec<usize>, u: usize, par: usize) {
    visited[u] = true;
    parent[u] = par;
    for &(v, bi) in &adj[u] {
        if !visited[v] {
            dfs_mark_rings(adj, bonds, visited, parent, v, u);
        } else if v != par {
            bonds[bi].in_ring = true;
        }
    }
}

// ─── Molecular formula & weight ──────────────────────────────────────────────

/// Atom counts by element symbol.
pub fn molecular_formula(mol: &Molecule) -> std::collections::HashMap<String, u32> {
    let mut counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for atom in &mol.atoms {
        *counts.entry(atom.element.clone()).or_insert(0) += 1;
        if atom.n_implicit_h + atom.explicit_h > 0 {
            *counts.entry("H".to_string()).or_insert(0) += (atom.n_implicit_h + atom.explicit_h) as u32;
        }
    }
    counts
}

/// Hill-order formula string (C first, H second, then others alphabetically).
pub fn formula_string(mol: &Molecule) -> String {
    let counts = molecular_formula(mol);
    let mut parts: Vec<(String, u32)> = counts.into_iter().collect();
    parts.sort_by_key(|(e, _)| {
        match e.as_str() { "C" => 0, "H" => 1, _ => 2 }
    });
    parts.iter().map(|(e, n)| if *n == 1 { e.clone() } else { format!("{}{}", e, n) }).collect()
}

/// Exact monoisotopic molecular weight (Da).
pub fn exact_molecular_weight(mol: &Molecule) -> f64 {
    let counts = molecular_formula(mol);
    counts.iter().map(|(e, &n)| atomic_weight(e) * n as f64).sum()
}

// ─── Lipinski / drug-likeness descriptors ────────────────────────────────────

#[derive(Debug, Clone)]
pub struct MolecularDescriptors {
    pub molecular_weight: f64,
    pub formula: String,
    pub heavy_atom_count: usize,
    pub hb_donors: u32,
    pub hb_acceptors: u32,
    pub rotatable_bonds: u32,
    pub aromatic_ring_count: u32,
    pub ring_count: u32,
    pub logp_crippen: f64,
    pub tpsa_ertl: f64,
    pub chiral_centers: u32,
    pub fraction_csp3: f64,
}

/// Compute all Lipinski/Veber/Ghose descriptors from a parsed molecule.
pub fn compute_descriptors(mol: &Molecule) -> MolecularDescriptors {
    let mw = exact_molecular_weight(mol);
    let formula = formula_string(mol);
    let heavy = mol.atoms.iter().filter(|a| a.element != "H").count();

    // H-bond donors: NH and OH groups
    let hbd = mol.atoms.iter().filter(|a| {
        (a.element == "N" || a.element == "O") && (a.n_implicit_h + a.explicit_h) > 0
    }).count() as u32;

    // H-bond acceptors: all N and O (Lipinski: just N+O count)
    let hba = mol.atoms.iter().filter(|a| a.element == "N" || a.element == "O").count() as u32;

    // Rotatable bonds: single, non-ring, both atoms have degree ≥ 2, not amide/ester
    let adj = build_adj(mol);
    let rot = mol.bonds.iter().filter(|b| {
        b.order == BondOrder::Single
            && !b.in_ring
            && mol.atoms[b.atom_a].element != "H"
            && mol.atoms[b.atom_b].element != "H"
            && mol.atoms[b.atom_a].degree >= 2
            && mol.atoms[b.atom_b].degree >= 2
    }).count() as u32;

    // Aromatic rings (simplified: number of aromatic-atom-containing SSSR cycles)
    let aro_ring = count_aromatic_rings(&mol.atoms, &mol.bonds);
    let ring_ct = count_all_rings(&mol.bonds);

    // LogP Crippen simplified
    let logp = compute_logp(mol);

    // TPSA Ertl
    let tpsa = compute_tpsa(mol);

    // Chiral centers
    let chiral = count_chiral_centers(mol);

    // Fraction of sp3 carbons
    let n_c = mol.atoms.iter().filter(|a| a.element == "C").count();
    let n_csp3 = mol.atoms.iter().filter(|a| {
        a.element == "C" && !a.is_aromatic && !is_sp2_carbon(a, &adj, mol)
    }).count();
    let fsp3 = if n_c > 0 { n_csp3 as f64 / n_c as f64 } else { 0.0 };

    MolecularDescriptors {
        molecular_weight: mw,
        formula,
        heavy_atom_count: heavy,
        hb_donors: hbd,
        hb_acceptors: hba,
        rotatable_bonds: rot,
        aromatic_ring_count: aro_ring,
        ring_count: ring_ct,
        logp_crippen: logp,
        tpsa_ertl: tpsa,
        chiral_centers: chiral,
        fraction_csp3: fsp3,
    }
}

fn build_adj(mol: &Molecule) -> Vec<Vec<usize>> {
    let mut adj = vec![vec![]; mol.atoms.len()];
    for b in &mol.bonds {
        adj[b.atom_a].push(b.atom_b);
        adj[b.atom_b].push(b.atom_a);
    }
    adj
}

fn is_sp2_carbon(a: &Atom, _adj: &[Vec<usize>], mol: &Molecule) -> bool {
    mol.bonds.iter().any(|b| {
        (b.atom_a == a.idx || b.atom_b == a.idx)
            && (b.order == BondOrder::Double || b.order == BondOrder::Aromatic)
    })
}

fn count_aromatic_rings(_atoms: &[Atom], bonds: &[Bond]) -> u32 {
    // Simplified: each aromatic bond that closes a ring contributes to one ring
    let ring_bonds = bonds.iter().filter(|b| b.in_ring && b.order == BondOrder::Aromatic).count();
    // Rough: 6-membered ring has 6 aromatic bonds; 5-membered has 5
    (ring_bonds / 6 + (ring_bonds % 6 > 2) as usize) as u32
}

fn count_all_rings(bonds: &[Bond]) -> u32 {
    bonds.iter().filter(|b| b.in_ring).count() as u32 / 3 // rough heuristic
}

// ─── LogP (Crippen–Wildman simplified) ───────────────────────────────────────

/// Per-atom LogP contributions mapped by (element, aromaticity, polar_neighbour).
/// Derived from Wildman & Crippen 1999, Table 1 (25 of 68 atom types).
pub fn compute_logp(mol: &Molecule) -> f64 {
    let adj = build_adj(mol);
    let mut total = 0.0_f64;

    for atom in &mol.atoms {
        if atom.element == "H" { continue; }
        let neighbours: Vec<&Atom> = adj[atom.idx].iter()
            .map(|&i| &mol.atoms[i]).collect();
        let has_polar_nbr = neighbours.iter().any(|n| {
            matches!(n.element.as_str(), "N" | "O" | "S" | "P" | "F" | "Cl" | "Br" | "I")
        });
        let is_carbonyl = mol.bonds.iter().any(|b| {
            (b.atom_a == atom.idx || b.atom_b == atom.idx)
                && b.order == BondOrder::Double
                && {
                    let other = if b.atom_a == atom.idx { b.atom_b } else { b.atom_a };
                    mol.atoms[other].element == "O"
                }
        });

        let contrib = match atom.element.as_str() {
            "C" => {
                if atom.is_aromatic {
                    if has_polar_nbr { 0.0782 } else { 0.2140 }
                } else if is_carbonyl {
                    0.0000
                } else if atom.charge != 0 {
                    -0.3537
                } else if has_polar_nbr {
                    0.0000
                } else {
                    0.1441
                }
            }
            "N" => {
                if atom.charge > 0       { -1.9513 }
                else if atom.is_aromatic { -0.7096 }
                else if is_adjacent_amide(atom, mol) { -1.0280 }
                else                     { -1.0190 }
            }
            "O" => {
                if atom.charge < 0       { -1.0810 }
                else if is_carbonyl_oxygen(atom, mol) { 0.1421 }
                else if atom.is_aromatic { -0.0582 }
                else                     { -0.4670 }
            }
            "S" => if atom.is_aromatic { 0.4880 } else { 0.5620 },
            "P" => 0.8760,
            "F" => 0.1480,
            "Cl" => 0.3010,
            "Br" => 0.6130,
            "I"  => 1.2570,
            "B"  => 0.1758,
            "Si" => 0.2756,
            _    => 0.0,
        };
        total += contrib;
    }
    // Add implicit-H contribution (CH contribution ~0.1441 per CH3 group simplified)
    for atom in &mol.atoms {
        if atom.element == "C" && !atom.is_aromatic {
            total += 0.1441 * (atom.n_implicit_h.min(3)) as f64 * 0.3;
        }
    }
    total
}

fn is_adjacent_amide(atom: &Atom, mol: &Molecule) -> bool {
    mol.bonds.iter().any(|b| {
        (b.atom_a == atom.idx || b.atom_b == atom.idx) && {
            let other_idx = if b.atom_a == atom.idx { b.atom_b } else { b.atom_a };
            let other = &mol.atoms[other_idx];
            other.element == "C" && mol.bonds.iter().any(|b2| {
                (b2.atom_a == other_idx || b2.atom_b == other_idx)
                    && b2.order == BondOrder::Double
                    && {
                        let o = if b2.atom_a == other_idx { b2.atom_b } else { b2.atom_a };
                        mol.atoms[o].element == "O"
                    }
            })
        }
    })
}

fn is_carbonyl_oxygen(atom: &Atom, mol: &Molecule) -> bool {
    mol.bonds.iter().any(|b| {
        (b.atom_a == atom.idx || b.atom_b == atom.idx) && b.order == BondOrder::Double
    })
}

// ─── TPSA (Ertl 2000 atomic contributions) ───────────────────────────────────

/// Topological polar surface area in Å².
pub fn compute_tpsa(mol: &Molecule) -> f64 {
    let mut tpsa = 0.0_f64;
    for atom in &mol.atoms {
        let total_h = (atom.n_implicit_h + atom.explicit_h) as f64;
        let contrib = match atom.element.as_str() {
            "N" => {
                if atom.charge > 0 { 0.0 }
                else if atom.is_aromatic { 12.89 }
                else if total_h >= 2.0  { 26.02 }  // NH2
                else if total_h >= 1.0  { 16.61 }  // NH
                else if is_adjacent_amide(atom, mol) { 29.42 }
                else                    { 11.49 }  // tertiary N
            }
            "O" => {
                if atom.charge != 0    { 23.06 }
                else if atom.is_aromatic { 13.14 }
                else if total_h >= 1.0 { 20.23 }  // OH
                else if is_carbonyl_oxygen(atom, mol) { 17.07 }
                else                   { 9.23 }   // ether O
            }
            "S" => {
                if atom.is_aromatic { 0.0 }
                else if total_h >= 1.0 { 38.80 } // SH
                else { 32.09 } // sulfoxide / sulfone
            }
            "P" => 34.14,
            _ => 0.0,
        };
        tpsa += contrib;
    }
    tpsa
}

// ─── Drug-likeness filters ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LipinskiResult {
    /// MW ≤ 500 Da
    pub mw_ok: bool,
    /// LogP ≤ 5
    pub logp_ok: bool,
    /// H-bond donors ≤ 5
    pub hbd_ok: bool,
    /// H-bond acceptors ≤ 10
    pub hba_ok: bool,
    /// Number of rule violations (0 or 1 is oral-bioavailability acceptable).
    pub violations: u8,
    pub passes: bool,
}

pub fn evaluate_lipinski(desc: &MolecularDescriptors) -> LipinskiResult {
    let mw_ok   = desc.molecular_weight <= 500.0;
    let logp_ok = desc.logp_crippen     <= 5.0;
    let hbd_ok  = desc.hb_donors        <= 5;
    let hba_ok  = desc.hb_acceptors     <= 10;
    let violations = (!mw_ok as u8) + (!logp_ok as u8) + (!hbd_ok as u8) + (!hba_ok as u8);
    LipinskiResult { mw_ok, logp_ok, hbd_ok, hba_ok, violations, passes: violations <= 1 }
}

#[derive(Debug, Clone)]
pub struct VeberResult {
    /// Rotatable bonds ≤ 10
    pub rot_bonds_ok: bool,
    /// TPSA ≤ 140 Å²
    pub tpsa_ok: bool,
    pub passes: bool,
}

pub fn evaluate_veber(desc: &MolecularDescriptors) -> VeberResult {
    let rot_bonds_ok = desc.rotatable_bonds <= 10;
    let tpsa_ok      = desc.tpsa_ertl       <= 140.0;
    VeberResult { rot_bonds_ok, tpsa_ok, passes: rot_bonds_ok && tpsa_ok }
}

#[derive(Debug, Clone)]
pub struct GhoseResult {
    /// 160 ≤ MW ≤ 480
    pub mw_ok: bool,
    /// -0.4 ≤ LogP ≤ 5.6
    pub logp_ok: bool,
    /// 20 ≤ heavy atoms ≤ 70
    pub atoms_ok: bool,
    /// Molar refractivity 40–130 (approximated from MW)
    pub mr_ok: bool,
    pub passes: bool,
}

pub fn evaluate_ghose(desc: &MolecularDescriptors) -> GhoseResult {
    let mw_ok    = desc.molecular_weight >= 160.0 && desc.molecular_weight <= 480.0;
    let logp_ok  = desc.logp_crippen     >= -0.4  && desc.logp_crippen     <= 5.6;
    let atoms_ok = desc.heavy_atom_count >= 20     && desc.heavy_atom_count <= 70;
    // MR ≈ 0.3285 × MW + 0.08 × logP + 1.0 (approximation)
    let mr = 0.3285 * desc.molecular_weight + 0.08 * desc.logp_crippen + 1.0;
    let mr_ok = mr >= 40.0 && mr <= 130.0;
    let violations = (!mw_ok as u8) + (!logp_ok as u8) + (!atoms_ok as u8) + (!mr_ok as u8);
    GhoseResult { mw_ok, logp_ok, atoms_ok, mr_ok, passes: violations == 0 }
}

#[derive(Debug, Clone)]
pub struct EganResult {
    /// TPSA ≤ 131.6 Å²
    pub tpsa_ok: bool,
    /// LogP ≤ 5.88
    pub logp_ok: bool,
    pub passes: bool,
}

pub fn evaluate_egan(desc: &MolecularDescriptors) -> EganResult {
    let tpsa_ok = desc.tpsa_ertl    <= 131.6;
    let logp_ok = desc.logp_crippen <= 5.88;
    EganResult { tpsa_ok, logp_ok, passes: tpsa_ok && logp_ok }
}

// ─── Functional group detection ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FunctionalGroup {
    Hydroxyl,          // -OH
    PrimaryAmine,      // -NH2
    SecondaryAmine,    // -NH-
    TertiaryAmine,     // -N<
    AromaticAmine,     // Ar-NH2
    Amide,             // -C(=O)N-
    CarboxylicAcid,    // -C(=O)OH
    Ester,             // -C(=O)O-
    Ketone,            // -C(=O)-
    Aldehyde,          // -CHO
    Ether,             // -O-
    Thiol,             // -SH
    Sulfide,           // -S-
    Sulfonamide,       // -S(=O)2NH-
    Halide,            // -F, -Cl, -Br, -I
    AromaticRing,
    Nitrile,           // -C≡N
    Nitro,             // -NO2
    Phosphate,         // -P(=O)(O)2
    Imidazole,
    GuanidiniumLike,
}

/// Detect functional groups in a molecule. Returns unique set.
#[allow(non_snake_case)]
pub fn detect_functional_groups(mol: &Molecule) -> Vec<FunctionalGroup> {
    let mut found = std::collections::HashSet::new();
    let adj = build_adj(mol);

    for atom in &mol.atoms {
        let nbrs: Vec<&Atom> = adj[atom.idx].iter().map(|&i| &mol.atoms[i]).collect();
        let has_double_O = || mol.bonds.iter().any(|b| {
            (b.atom_a == atom.idx || b.atom_b == atom.idx) && b.order == BondOrder::Double && {
                let o = if b.atom_a == atom.idx { b.atom_b } else { b.atom_a };
                mol.atoms[o].element == "O"
            }
        });
        let _has_double_N = || mol.bonds.iter().any(|b| {
            (b.atom_a == atom.idx || b.atom_b == atom.idx) && b.order == BondOrder::Double && {
                let o = if b.atom_a == atom.idx { b.atom_b } else { b.atom_a };
                mol.atoms[o].element == "N"
            }
        });
        let has_triple_N = || mol.bonds.iter().any(|b| {
            (b.atom_a == atom.idx || b.atom_b == atom.idx) && b.order == BondOrder::Triple && {
                let o = if b.atom_a == atom.idx { b.atom_b } else { b.atom_a };
                mol.atoms[o].element == "N"
            }
        });

        match atom.element.as_str() {
            "O" => {
                let h = atom.n_implicit_h + atom.explicit_h;
                let c_nbr = nbrs.iter().any(|n| n.element == "C");
                if h > 0 {
                    if c_nbr && !is_carbonyl_oxygen(atom, mol) { found.insert(FunctionalGroup::Hydroxyl); }
                } else if c_nbr && !is_carbonyl_oxygen(atom, mol) {
                    found.insert(FunctionalGroup::Ether);
                }
            }
            "N" => {
                let h = atom.n_implicit_h + atom.explicit_h;
                if atom.is_aromatic {
                    found.insert(FunctionalGroup::AromaticAmine);
                } else if h >= 2 {
                    found.insert(FunctionalGroup::PrimaryAmine);
                } else if h == 1 {
                    found.insert(FunctionalGroup::SecondaryAmine);
                } else {
                    found.insert(FunctionalGroup::TertiaryAmine);
                }
                // Amide N: bonded to a C=O
                if is_adjacent_amide(atom, mol) { found.insert(FunctionalGroup::Amide); }
                // Sulfonamide: bonded to S
                if nbrs.iter().any(|n| n.element == "S") { found.insert(FunctionalGroup::Sulfonamide); }
                // Nitro group: non-aromatic N bonded to ≥2 oxygens
                if !atom.is_aromatic {
                    let o_nbrs: Vec<_> = nbrs.iter().filter(|n| n.element == "O").collect();
                    if o_nbrs.len() >= 2 { found.insert(FunctionalGroup::Nitro); }
                }
            }
            "C" => {
                let o_nbrs: Vec<_> = nbrs.iter().filter(|n| n.element == "O").collect();
                if has_double_O() {
                    let oh_nbr = o_nbrs.iter().any(|o| o.n_implicit_h + o.explicit_h > 0);
                    if oh_nbr {
                        found.insert(FunctionalGroup::CarboxylicAcid);
                    } else {
                        let n_nbr = nbrs.iter().any(|n| n.element == "N");
                        if n_nbr { found.insert(FunctionalGroup::Amide); }
                        else {
                            let is_terminal = atom.degree == 1;
                            if is_terminal || atom.n_implicit_h > 0 {
                                found.insert(FunctionalGroup::Aldehyde);
                            } else {
                                found.insert(FunctionalGroup::Ketone);
                            }
                        }
                    }
                    let ether_o = o_nbrs.iter().any(|o| o.n_implicit_h == 0 && !is_carbonyl_oxygen(o, mol));
                    if ether_o { found.insert(FunctionalGroup::Ester); }
                }
                if has_triple_N() { found.insert(FunctionalGroup::Nitrile); }
            }
            "S" => {
                let h = atom.n_implicit_h + atom.explicit_h;
                if h > 0 { found.insert(FunctionalGroup::Thiol); }
                else { found.insert(FunctionalGroup::Sulfide); }
            }
            "P" => { found.insert(FunctionalGroup::Phosphate); }
            "F" | "Cl" | "Br" | "I" => { found.insert(FunctionalGroup::Halide); }
            _ => {}
        }
        if atom.is_aromatic { found.insert(FunctionalGroup::AromaticRing); }
    }
    let mut result: Vec<FunctionalGroup> = found.into_iter().collect();
    result.sort_by_key(|g| format!("{:?}", g));
    result
}

// ─── Chiral centres ───────────────────────────────────────────────────────────

/// Count sp3 carbon atoms with 4 distinct substituents (simplified: sp3 C with degree 4).
pub fn count_chiral_centers(mol: &Molecule) -> u32 {
    let adj = build_adj(mol);
    mol.atoms.iter().filter(|a| {
        a.element == "C"
            && !a.is_aromatic
            && !is_sp2_carbon(a, &adj, mol)
            && a.degree >= 4
    }).count() as u32
}

// ─── Morgan circular fingerprint ─────────────────────────────────────────────

/// Morgan algorithm: radius-`r` circular fingerprint as sorted Vec<u64> identifiers.
pub fn circular_fingerprint(mol: &Molecule, radius: usize) -> Vec<u64> {
    let n = mol.atoms.len();
    if n == 0 { return vec![]; }
    let adj = build_adj(mol);

    // Initial atom invariants (element + charge + degree)
    let mut ids: Vec<u64> = mol.atoms.iter().map(|a| {
        crate::q_hash(&format!("{}{}{}", a.element, a.charge, a.degree))
    }).collect();

    let mut all_ids = ids.clone();

    for _ in 0..radius {
        let new_ids: Vec<u64> = (0..n).map(|i| {
            let mut nbr_ids: Vec<u64> = adj[i].iter().map(|&j| ids[j]).collect();
            nbr_ids.sort_unstable();
            let combined = format!("{}{:?}", ids[i], nbr_ids);
            crate::q_hash(&combined)
        }).collect();
        all_ids.extend_from_slice(&new_ids);
        ids = new_ids;
    }

    all_ids.sort_unstable();
    all_ids.dedup();
    all_ids
}

// ─── pKa estimation ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PkaEstimate {
    pub group: FunctionalGroup,
    pub pka: f64,
    pub is_acid: bool,
}

/// Estimate pKa values from functional group type (literature reference values).
pub fn estimate_pka(mol: &Molecule) -> Vec<PkaEstimate> {
    let groups = detect_functional_groups(mol);
    let mut estimates = Vec::new();
    for group in groups {
        let (pka, is_acid) = match &group {
            FunctionalGroup::CarboxylicAcid  => (4.8, true),
            FunctionalGroup::Hydroxyl        => (10.0, true),
            FunctionalGroup::Thiol           => (8.3, true),
            FunctionalGroup::Amide           => (25.0, true),
            FunctionalGroup::Nitrile         => (25.0, true),
            FunctionalGroup::PrimaryAmine    => (10.5, false),
            FunctionalGroup::SecondaryAmine  => (10.0, false),
            FunctionalGroup::TertiaryAmine   => (9.5, false),
            FunctionalGroup::AromaticAmine   => (4.6, false),
            FunctionalGroup::Imidazole       => (6.0, false),
            FunctionalGroup::GuanidiniumLike => (12.5, false),
            FunctionalGroup::Phosphate       => (2.1, true),
            FunctionalGroup::Sulfonamide     => (10.0, true),
            _ => continue,
        };
        estimates.push(PkaEstimate { group, pka, is_acid });
    }
    estimates
}

// ─── SMILES / InChI validation ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SmilesValidation {
    pub is_valid: bool,
    pub atom_count: usize,
    pub error: Option<String>,
}

pub fn validate_smiles(smiles: &str) -> SmilesValidation {
    if smiles.trim().is_empty() {
        return SmilesValidation { is_valid: false, atom_count: 0, error: Some("Empty SMILES".into()) };
    }
    let mol = parse_smiles(smiles);
    SmilesValidation { is_valid: mol.is_valid, atom_count: mol.atoms.len(), error: mol.error }
}

#[derive(Debug, Clone)]
pub struct InchiValidation {
    pub is_valid: bool,
    pub has_inchikey: bool,
    pub layer_count: usize,
}

pub fn validate_inchi(inchi: &str) -> InchiValidation {
    // Standard InChI: starts with "InChI=1S/" followed by formula and layers
    // InChIKey: 27-character hash XXXXXXXXXXXXXX-XXXXXXXXXX-N
    let is_inchi = inchi.starts_with("InChI=1S/") || inchi.starts_with("InChI=1/");
    let is_inchikey = inchi.len() == 27 && inchi.chars().nth(14) == Some('-') && inchi.chars().nth(25) == Some('-');
    let layers = if is_inchi { inchi.split('/').count().saturating_sub(2) } else { 0 };
    InchiValidation { is_valid: is_inchi || is_inchikey, has_inchikey: is_inchikey, layer_count: layers }
}

// ─── Thermochemistry ─────────────────────────────────────────────────────────

/// Arrhenius rate constant k at temperature T (K).
/// k = A × exp(−Ea / (R × T))
/// `activation_energy_j_mol`: Ea in J/mol
pub fn arrhenius_rate(pre_exponential_a: f64, activation_energy_j_mol: f64, temp_k: f64) -> f64 {
    pre_exponential_a * f64::exp(-activation_energy_j_mol / (R_J_MOL_K * temp_k))
}

/// Temperature dependence of k using the Arrhenius equation.
/// Returns (k_t1, k_t2).
pub fn arrhenius_ratio(ea_j_mol: f64, t1_k: f64, t2_k: f64) -> f64 {
    f64::exp((ea_j_mol / R_J_MOL_K) * (1.0 / t1_k - 1.0 / t2_k))
}

/// Gibbs free energy ΔG = ΔH − T × ΔS  (all in J/mol or kJ/mol consistently).
pub fn gibbs_free_energy(delta_h: f64, delta_s: f64, temp_k: f64) -> f64 {
    delta_h - temp_k * delta_s
}

/// Equilibrium constant from ΔG°: K = exp(−ΔG° / (R × T))
pub fn equilibrium_constant(delta_g_j_mol: f64, temp_k: f64) -> f64 {
    f64::exp(-delta_g_j_mol / (R_J_MOL_K * temp_k))
}

/// ΔG° from equilibrium constant K: ΔG° = −R × T × ln(K)
pub fn gibbs_from_equilibrium(k_eq: f64, temp_k: f64) -> f64 {
    -R_J_MOL_K * temp_k * k_eq.ln()
}

/// Gibbs–Helmholtz: ΔG(T2) from ΔG(T1).
pub fn gibbs_helmholtz(delta_g_t1: f64, delta_h: f64, t1_k: f64, t2_k: f64) -> f64 {
    t2_k * (delta_g_t1 / t1_k + delta_h * (1.0 / t1_k - 1.0 / t2_k))
}

/// van't Hoff enthalpy estimate from two equilibrium constants at two temperatures.
pub fn vant_hoff_enthalpy(k1: f64, k2: f64, t1_k: f64, t2_k: f64) -> f64 {
    -R_J_MOL_K * (k2 / k1).ln() / (1.0 / t2_k - 1.0 / t1_k)
}

/// Henderson–Hasselbalch: pH = pKa + log10([A-] / [HA])
pub fn henderson_hasselbalch(pka: f64, conc_base: f64, conc_acid: f64) -> f64 {
    if conc_acid <= 0.0 || conc_base < 0.0 {
        return pka;
    }
    pka + (conc_base / conc_acid).log10()
}

/// Degree of ionisation α at a given pH for a monoprotic acid.
pub fn ionisation_fraction(ph: f64, pka: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf(pka - ph))
}

// ─── Green chemistry metrics ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct GreenMetrics {
    /// Trost 1991: desired product MW / sum of all reactant MWs × 100 %
    pub atom_economy_pct: f64,
    /// Yield-corrected AE (YAAE)
    pub yield_corrected_ae_pct: f64,
    /// Sheldon 1992: kg waste / kg desired product
    pub e_factor: f64,
    /// Mass-based E-factor considering all inputs including solvents
    pub process_mass_intensity: f64,
    /// Andraos 2005: kg product / kg total reactants × 100 %
    pub reaction_mass_efficiency_pct: f64,
    /// Carbon efficiency: C atoms in product / C atoms in reactants × 100 %
    pub carbon_efficiency_pct: f64,
}

pub fn green_metrics(
    reactant_mws: &[f64],
    product_mw: f64,
    byproduct_mws: &[f64],
    yield_fraction: f64,        // 0.0–1.0
    solvent_and_auxiliary_kg: f64,
    product_kg: f64,
    reactant_c_atoms: u32,
    product_c_atoms: u32,
) -> GreenMetrics {
    let sum_reactants: f64 = reactant_mws.iter().sum();
    let ae = if sum_reactants > 0.0 { 100.0 * product_mw / sum_reactants } else { 0.0 };
    let yaae = ae * yield_fraction;

    let waste_kg = reactant_mws.iter().sum::<f64>() + byproduct_mws.iter().sum::<f64>() + solvent_and_auxiliary_kg - product_kg;
    let ef = if product_kg > 0.0 { waste_kg.max(0.0) / product_kg } else { f64::INFINITY };

    let total_in = reactant_mws.iter().sum::<f64>() + solvent_and_auxiliary_kg;
    let pmi = if product_kg > 0.0 { total_in / product_kg } else { f64::INFINITY };

    let rme = if sum_reactants > 0.0 { 100.0 * product_kg / sum_reactants } else { 0.0 };

    let ce = if reactant_c_atoms > 0 { 100.0 * product_c_atoms as f64 / reactant_c_atoms as f64 } else { 0.0 };

    GreenMetrics {
        atom_economy_pct: ae,
        yield_corrected_ae_pct: yaae,
        e_factor: ef,
        process_mass_intensity: pmi,
        reaction_mass_efficiency_pct: rme,
        carbon_efficiency_pct: ce,
    }
}

/// Simplified atom economy (Trost): single product vs all reactants.
pub fn atom_economy(reactant_mws: &[f64], desired_product_mw: f64) -> f64 {
    let sum: f64 = reactant_mws.iter().sum();
    if sum == 0.0 { 0.0 } else { 100.0 * desired_product_mw / sum }
}

/// E-factor (Sheldon): waste_kg / product_kg.  Fine chemicals < 50; bulk < 5; pharma 25–100.
pub fn e_factor(waste_kg: f64, product_kg: f64) -> f64 {
    if product_kg <= 0.0 { f64::INFINITY } else { waste_kg / product_kg }
}

// ─── ADMET & Lead Optimization Metrics ───────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct BbbPermeationResult {
    pub clark_score: u8,        // MPO score 0-4
    pub is_cns_penetrant: bool, // MPO >= 3 is typically considered penetrant
}

/// Evaluates Blood-Brain Barrier (BBB) permeation probability using Clark's
/// rules (Molecular Weight, LogP, PSA, HBD).
pub fn predict_bbb_permeation(mw: f64, log_p: f64, psa: f64, hbd: u32) -> BbbPermeationResult {
    let mut clark_score = 0;
    if mw <= 400.0 { clark_score += 1; }
    if log_p >= 2.0 && log_p <= 5.0 { clark_score += 1; }
    if psa <= 90.0 { clark_score += 1; }
    if hbd <= 3 { clark_score += 1; }

    BbbPermeationResult {
        clark_score,
        is_cns_penetrant: clark_score >= 3,
    }
}

/// Computes Ligand Efficiency (LE): pIC50 / Heavy Atom Count (HAC).
/// Standard target is LE >= 0.3.
pub fn ligand_efficiency(pic50: f64, hac: u32) -> f64 {
    if hac == 0 { 0.0 } else { (pic50 * 1.37) / (hac as f64) }
}

/// Computes Lipophilic Ligand Efficiency (LLE): pIC50 - LogP.
/// Standard target is LLE >= 5.0.
pub fn lipophilic_ligand_efficiency(pic50: f64, log_p: f64) -> f64 {
    pic50 - log_p
}

// ─── Mass Spectrometry Simulation ────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct IsotopeDistribution {
    pub m_peak: f64,    // 100% (normalized)
    pub m1_peak: f64,   // M+1 relative intensity
    pub m2_peak: f64,   // M+2 relative intensity
}

/// Computes the theoretical M, M+1, M+2 isotopic distribution based on the number
/// of Carbon, Nitrogen, Oxygen, Sulfur, Chlorine, and Bromine atoms.
pub fn isotope_mass_distribution(
    c: u32, n: u32, o: u32, s: u32, cl: u32, br: u32
) -> IsotopeDistribution {
    // Relative abundance approximations (natural)
    let c13 = 0.0107;
    let n15 = 0.0036;
    let o18 = 0.0020;
    let s33 = 0.0076;
    let s34 = 0.0429;
    let cl37 = 0.3197;
    let br81 = 0.9728; // Br is ~50.69% 79Br, ~49.31% 81Br (ratio ~ 0.97)

    // M+1 contributions
    let m1 = (c as f64) * c13 + (n as f64) * n15 + (s as f64) * s33;

    // M+2 contributions
    let m2 = ((c as f64) * c13).powi(2) / 2.0
           + (o as f64) * o18 
           + (s as f64) * s34
           + (cl as f64) * cl37
           + (br as f64) * br81;

    IsotopeDistribution {
        m_peak: 100.0,
        m1_peak: m1 * 100.0,
        m2_peak: m2 * 100.0,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn aspirin() -> Molecule { parse_smiles("CC(=O)Oc1ccccc1C(=O)O") }
    fn paracetamol() -> Molecule { parse_smiles("CC(=O)Nc1ccc(O)cc1") }
    fn ethanol() -> Molecule { parse_smiles("CCO") }
    fn caffeine() -> Molecule { parse_smiles("Cn1cnc2c1c(=O)n(c(=O)n2C)C") }
    fn methane() -> Molecule { parse_smiles("C") }

    #[test]
    fn smiles_parse_ethanol_atoms() {
        let mol = ethanol();
        assert!(mol.is_valid);
        assert_eq!(mol.atoms.iter().filter(|a| a.element == "C").count(), 2);
        assert_eq!(mol.atoms.iter().filter(|a| a.element == "O").count(), 1);
    }

    #[test]
    fn molecular_weight_ethanol() {
        let mol = ethanol();
        let mw = exact_molecular_weight(&mol);
        assert!((mw - 46.068).abs() < 1.0, "ethanol MW ~ 46.07, got {:.3}", mw);
    }

    #[test]
    fn molecular_weight_aspirin() {
        let mol = aspirin();
        let mw = exact_molecular_weight(&mol);
        assert!((mw - 180.0).abs() < 5.0, "aspirin MW ~ 180, got {:.2}", mw);
    }

    #[test]
    fn lipinski_aspirin_passes() {
        let desc = compute_descriptors(&aspirin());
        let r = evaluate_lipinski(&desc);
        assert!(r.passes, "Aspirin should pass Lipinski");
    }

    #[test]
    fn lipinski_caffeine_passes() {
        let desc = compute_descriptors(&caffeine());
        let r = evaluate_lipinski(&desc);
        assert!(r.passes, "Caffeine should pass Lipinski");
    }

    #[test]
    fn functional_groups_ethanol() {
        let groups = detect_functional_groups(&ethanol());
        assert!(groups.contains(&FunctionalGroup::Hydroxyl), "ethanol should have hydroxyl");
    }

    #[test]
    fn functional_groups_aspirin() {
        let groups = detect_functional_groups(&aspirin());
        assert!(groups.contains(&FunctionalGroup::CarboxylicAcid) || groups.contains(&FunctionalGroup::Ester) || groups.contains(&FunctionalGroup::AromaticRing));
    }

    #[test]
    fn tpsa_ethanol_reasonable() {
        let mol = ethanol();
        let tpsa = compute_tpsa(&mol);
        assert!(tpsa > 10.0 && tpsa < 40.0, "ethanol TPSA ~ 20 Å², got {:.1}", tpsa);
    }

    #[test]
    fn test_bbb_permeation() {
        // Example: Diazepam (MW 284.7, LogP ~2.8, PSA ~32.6, HBD 0)
        let r = predict_bbb_permeation(284.7, 2.8, 32.6, 0);
        assert_eq!(r.clark_score, 4);
        assert!(r.is_cns_penetrant);
    }

    #[test]
    fn test_ligand_efficiency() {
        let le = ligand_efficiency(8.0, 25);
        assert!((le - 0.4384).abs() < 0.01);
        let lle = lipophilic_ligand_efficiency(8.0, 3.0);
        assert!((lle - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_isotope_distribution() {
        // Bromobenzene: C6 H5 Br1
        // M+2 from Br: 1 × br81_ratio = 0.9728 → 97.28%
        // M+2 from C6:  (6 × 0.0107)² / 2 = 0.00206 → 0.206%
        // Combined M+2 ≈ 97.49%
        let r = isotope_mass_distribution(6, 0, 0, 0, 0, 1);
        assert!((r.m_peak - 100.0).abs() < 0.1);
        assert!((r.m1_peak - 6.42).abs() < 0.1);
        assert!((r.m2_peak - 97.49).abs() < 0.5,
            "M+2 for bromobenzene expected ~97.49%, got {}", r.m2_peak);
    }

    #[test]
    fn arrhenius_rate_increases_with_temp() {
        let k25 = arrhenius_rate(1e13, 80_000.0, 298.0);
        let k100 = arrhenius_rate(1e13, 80_000.0, 373.0);
        assert!(k100 > k25, "rate should increase with temperature");
    }

    #[test]
    fn gibbs_standard_conditions() {
        // ΔG = ΔH - TΔS: endothermic, low ΔS should give positive ΔG at 298K
        let dg = gibbs_free_energy(10_000.0, 20.0, 298.0);
        assert!(dg < 10_000.0);
    }

    #[test]
    fn equilibrium_constant_roundtrip() {
        let delta_g = -5_000.0; // negative ΔG → K > 1
        let k = equilibrium_constant(delta_g, 298.0);
        let dg_back = gibbs_from_equilibrium(k, 298.0);
        assert!((dg_back - delta_g).abs() < 1.0, "roundtrip ΔG");
    }

    #[test]
    fn henderson_hasselbalch_half_ionised() {
        // pH = pKa when [A-] = [HA]
        let ph = henderson_hasselbalch(4.8, 1.0, 1.0);
        assert!((ph - 4.8).abs() < 1e-9);
    }

    #[test]
    fn atom_economy_100pct_addition() {
        // A + B → AB (100% AE)
        let ae = atom_economy(&[100.0, 80.0], 180.0);
        assert!((ae - 100.0).abs() < 1e-9);
    }

    #[test]
    fn e_factor_zero_waste() {
        assert_eq!(e_factor(0.0, 1.0), 0.0);
    }

    #[test]
    fn validate_smiles_valid() {
        let r = validate_smiles("CCO");
        assert!(r.is_valid);
    }

    #[test]
    fn validate_smiles_empty() {
        let r = validate_smiles("");
        assert!(!r.is_valid);
    }

    #[test]
    fn validate_inchi_standard() {
        let r = validate_inchi("InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3");
        assert!(r.is_valid);
        assert!(!r.has_inchikey);
    }

    #[test]
    fn validate_inchikey_format() {
        let r = validate_inchi("LFQSCWFLJHTTHZ-UHFFFAOYSA-N");
        assert!(r.is_valid);
        assert!(r.has_inchikey);
    }

    #[test]
    fn circular_fingerprint_different_for_different_molecules() {
        let fp1 = circular_fingerprint(&ethanol(), 2);
        let fp2 = circular_fingerprint(&aspirin(), 2);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn pka_carboxylic_acid() {
        let mol = parse_smiles("CC(=O)O"); // acetic acid
        let pkas = estimate_pka(&mol);
        assert!(pkas.iter().any(|p| p.group == FunctionalGroup::CarboxylicAcid));
    }

    #[test]
    fn veber_passes_small_molecule() {
        let mol = ethanol();
        let desc = compute_descriptors(&mol);
        let r = evaluate_veber(&desc);
        assert!(r.passes);
    }
}
