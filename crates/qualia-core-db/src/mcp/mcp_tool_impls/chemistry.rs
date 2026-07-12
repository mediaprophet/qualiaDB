use super::*;


pub fn chemical_analysis(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::chemistry_modeling::{
        Atom, ChemistryModelingLibrary, MolecularProperties, Molecule, PropertyType,
    };

    let v = parse_tool_args(args)?;
    let mut lib = ChemistryModelingLibrary::new();
    lib.initialize()
        .map_err(|_| McpSystemError::InvalidParameters)?;

    // Real ab-initio RHF/STO-3G electronic structure (H/He, closed-shell) from an
    // explicit atom list with coordinates in BOHR:
    // [{ "element":"H", "atomic_number":1, "x":0.0,"y":0,"z":0 }, ...].
    // Returns real SCF total energy (Hartree), HOMO/LUMO/gap, dipole, Mulliken charges.
    if json_str(&v, "op", "") == "quantum" {
        use crate::specialized_libs::chemistry_modeling::QuantumMethodType;
        let atoms_json = v
            .get("atoms")
            .and_then(Value::as_array)
            .ok_or(McpSystemError::InvalidParameters)?;
        let mut atoms = Vec::with_capacity(atoms_json.len());
        let mut coords = Vec::with_capacity(atoms_json.len());
        for (i, a) in atoms_json.iter().enumerate() {
            let c = vec![json_f64(a, "x", 0.0), json_f64(a, "y", 0.0), json_f64(a, "z", 0.0)];
            atoms.push(Atom {
                atom_id: format!("a{}", i),
                element: a.get("element").and_then(Value::as_str).unwrap_or("X").to_string(),
                atomic_number: a.get("atomic_number").and_then(Value::as_u64).unwrap_or(0) as usize,
                mass: json_f64(a, "mass", 0.0),
                charge: 0.0,
                coordinates: c.clone(),
            });
            coords.push(c);
        }
        let mut m = Molecule::new();
        m.molecule_id = v.get("molecule_id").and_then(Value::as_str).unwrap_or("mcp_mol").to_string();
        m.atoms = atoms;
        m.coordinates = coords;
        let method = match json_str(&v, "method", "hartree_fock") {
            "ab_initio" => QuantumMethodType::AbInitio,
            _ => QuantumMethodType::HartreeFock,
        };
        // A model-dependent / out-of-scope system (heavier than He, open-shell,
        // non-HF method) fails closed as ToolNotReady — never a fabricated energy.
        let r = lib.calculate_quantum_properties(m, method).map_err(|e| {
            use crate::specialized_libs::chemistry_modeling::ChemistryError;
            match e {
                ChemistryError::NotImplemented(_) => McpSystemError::ToolNotReady,
                _ => McpSystemError::InvalidParameters,
            }
        })?;
        let p = r.result;
        return Ok(json!({
            "op": "quantum", "method": "RHF/STO-3G",
            "total_energy_hartree": p.total_energy, "homo_energy": p.homo_energy,
            "lumo_energy": p.lumo_energy, "gap": p.gap, "dipole_moment": p.dipole_moment,
            "mulliken_charges": p.mulliken_charges
        })
        .to_string());
    }

    // Exact structural properties (mass, Hill formula, nuclear-repulsion energy,
    // centre of mass, principal moments of inertia) from an explicit atom list:
    // [{ "element": "O", "atomic_number": 8, "x":.., "y":.., "z":.. }, ...].
    if json_str(&v, "op", "") == "structure" {
        if let Some(atoms_json) = v.get("atoms").and_then(Value::as_array) {
            let mut atoms = Vec::with_capacity(atoms_json.len());
            let mut coords = Vec::with_capacity(atoms_json.len());
            for (i, a) in atoms_json.iter().enumerate() {
                let element = a
                    .get("element")
                    .and_then(Value::as_str)
                    .unwrap_or("X")
                    .to_string();
                let c = vec![json_f64(a, "x", 0.0), json_f64(a, "y", 0.0), json_f64(a, "z", 0.0)];
                atoms.push(Atom {
                    atom_id: format!("a{}", i),
                    element,
                    atomic_number: a.get("atomic_number").and_then(Value::as_u64).unwrap_or(0)
                        as usize,
                    mass: json_f64(a, "mass", 0.0),
                    charge: 0.0,
                    coordinates: c.clone(),
                });
                coords.push(c);
            }
            let mut m = Molecule::new();
            m.molecule_id = v
                .get("molecule_id")
                .and_then(Value::as_str)
                .unwrap_or("mcp_mol")
                .to_string();
            m.atoms = atoms;
            m.coordinates = coords;
            let props = lib
                .structural_properties(&m)
                .map_err(|_| McpSystemError::InvalidParameters)?;
            return Ok(json!({
                "op": "structure",
                "molecular_mass": props.molecular_mass,
                "formula": props.formula,
                "atom_count": props.atom_count,
                "nuclear_repulsion_energy": props.nuclear_repulsion_energy,
                "center_of_mass": props.center_of_mass,
                "principal_moments_of_inertia": props.principal_moments_of_inertia
            })
            .to_string());
        }
        return Err(McpSystemError::InvalidParameters);
    }

    let molecule = if let Some(smiles) = v.get("smiles").and_then(Value::as_str) {
        use crate::domains::chemical::organic_chemistry::{compute_descriptors, parse_smiles};
        let mol = parse_smiles(smiles);
        let desc = compute_descriptors(&mol);
        Molecule {
            molecule_id: v
                .get("molecule_id")
                .and_then(Value::as_str)
                .unwrap_or("mcp_mol")
                .to_string(),
            formula: smiles.to_string(),
            atoms: vec![Atom::new()],
            bonds: vec![],
            coordinates: vec![vec![0.0, 0.0, 0.0]],
            properties: MolecularProperties {
                molecular_weight: desc.molecular_weight,
                dipole_moment: desc.tpsa_ertl,
                polarizability: 0.0,
                energy: 0.0,
            },
        }
    } else {
        let mut m = Molecule::new();
        if let Some(formula) = v.get("formula").and_then(Value::as_str) {
            m.formula = formula.to_string();
        }
        if let Some(mw) = v.get("molecular_weight").and_then(Value::as_f64) {
            m.properties.molecular_weight = mw;
        }
        if let Some(id) = v.get("molecule_id").and_then(Value::as_str) {
            m.molecule_id = id.to_string();
        }
        m
    };

    let props: Vec<PropertyType> = if let Some(arr) = v.get("properties").and_then(Value::as_array)
    {
        arr.iter()
            .filter_map(|p| match p.as_str()? {
                "boiling_point" => Some(PropertyType::BoilingPoint),
                "melting_point" => Some(PropertyType::MeltingPoint),
                "density" => Some(PropertyType::Density),
                "viscosity" => Some(PropertyType::Viscosity),
                _ => None,
            })
            .collect()
    } else {
        match json_str(&v, "prop", "boiling_point") {
            "melting_point" => vec![PropertyType::MeltingPoint],
            "density" => vec![PropertyType::Density],
            _ => vec![PropertyType::BoilingPoint],
        }
    };

    let r = lib
        .predict_properties(molecule, props)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    Ok(json!({
        "properties": r.result.properties,
        "confidence_intervals": r.result.confidence_intervals,
        "execution_time_ms": r.execution_time
    })
    .to_string())
}

pub fn chemical_descriptors(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::domains::chemical::organic_chemistry::{compute_descriptors, parse_smiles};

    let v = parse_tool_args(args)?;
    let smiles = v
        .get("smiles")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let mol = parse_smiles(smiles);
    let desc = compute_descriptors(&mol);

    Ok(json!({
        "smiles": smiles,
        "molecular_weight": desc.molecular_weight,
        "log_p": desc.logp_crippen,
        "tpsa": desc.tpsa_ertl,
        "h_bond_donors": desc.hb_donors,
        "h_bond_acceptors": desc.hb_acceptors,
        "rotatable_bonds": desc.rotatable_bonds
    })
    .to_string())
}
