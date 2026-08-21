//! Additional chemistry invoke seams — BSE basis-set parsing.

use super::super::args;
use crate::specialized_libs::chemistry_modeling::basis_set::{parse_bse_json as bse_parse, Vec3};
use poet_vibe::{Diagnostic, Span, Value};

/// `Chemistry.parse_bse_json` — parse a Basis Set Exchange JSON string into a
/// molecular basis set. Returns a summary (name, family, description, atom
/// count, basis-function count).
///
/// Args:
///   {
///     json: String,
///     atoms: [{ z: u64, symbol: String, x: f64, y: f64, z: f64 }]
///   }
pub fn parse_bse_json(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let json = args::rec_str(args, "json")
        .ok_or_else(|| args::bad(span, "Chemistry.parse_bse_json needs json"))?;
    let atoms_val = args::rec(args, "atoms")
        .ok_or_else(|| args::bad(span, "Chemistry.parse_bse_json needs atoms"))?;
    let atom_list = match atoms_val {
        Value::List(l) => l,
        _ => return Err(args::bad(span, "parse_bse_json: atoms must be a list")),
    };

    let mut atoms = Vec::new();
    for a in atom_list {
        let z = args::rec_u64(a, "z")
            .ok_or_else(|| args::bad(span, "parse_bse_json: each atom needs z"))?
            as u32;
        let symbol = args::rec_str(a, "symbol")
            .ok_or_else(|| args::bad(span, "parse_bse_json: each atom needs symbol"))?
            .to_string();
        let x = args::rec_f64(a, "x").unwrap_or(0.0);
        let y = args::rec_f64(a, "y").unwrap_or(0.0);
        let z_coord = args::rec_f64(a, "z_coord").unwrap_or(0.0);
        atoms.push((z, symbol, Vec3 { x, y, z: z_coord }));
    }

    match bse_parse(json, &atoms) {
        Ok(basis) => Ok(args::record([
            ("name", Value::String(basis.name.clone())),
            ("family", Value::String(basis.family.clone())),
            ("description", Value::String(basis.description.clone())),
            ("n_atoms", Value::U64(basis.atoms.len() as u64)),
            ("n_functions", Value::U64(basis.n_functions() as u64)),
        ])),
        Err(e) => Err(args::bad(span, format!("parse_bse_json: {e:?}"))),
    }
}
