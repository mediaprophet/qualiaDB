use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use qualia_seal_10d::{parse_obj, parse_occ1, seal_mesh};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut occupancy: Option<PathBuf> = None;
    let mut m64_out: Option<PathBuf> = None;
    let mut emf_out: Option<PathBuf> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!(
                    "qualia-seal-10d — seal an OBJ through the official QualiaDB compiler\n\
                     \n\
                     USAGE:\n\
                       qualia-seal-10d MESH.obj MESH.10d [--occupancy MESH.occ] [--m64 MESH.m64] [--emf MESH.emf]\n\
                     \n\
                     Writes QuantizedMesh + SurfaceReading + cardiac SKL1 (6-joint beat clip).\n\
                     With --occupancy, also attaches VOX0/FEMC/MRP1/FIEL/MID2 EMF and writes\n\
                     sibling Matter-64 (.m64) and Q42EMF64 (.emf). Does not change those formats."
                );
                return Ok(());
            }
            "--occupancy" => {
                occupancy = Some(PathBuf::from(
                    args.next().ok_or_else(|| "missing --occupancy PATH".to_string())?,
                ));
            }
            "--m64" => {
                m64_out = Some(PathBuf::from(
                    args.next().ok_or_else(|| "missing --m64 PATH".to_string())?,
                ));
            }
            "--emf" => {
                emf_out = Some(PathBuf::from(
                    args.next().ok_or_else(|| "missing --emf PATH".to_string())?,
                ));
            }
            _ if input.is_none() => input = Some(PathBuf::from(a)),
            _ if output.is_none() => output = Some(PathBuf::from(a)),
            other => return Err(format!("unexpected argument {other}")),
        }
    }
    let input = input.ok_or_else(|| "missing MESH.obj".to_string())?;
    let output = output.ok_or_else(|| "missing MESH.10d".to_string())?;
    let src = fs::read_to_string(&input).map_err(|e| e.to_string())?;
    let mesh = parse_obj(&src).map_err(|e| e.to_string())?;
    let grid = if let Some(path) = occupancy {
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        Some(parse_occ1(&bytes).map_err(|e| e.to_string())?)
    } else {
        None
    };
    if (m64_out.is_some() || emf_out.is_some()) && grid.is_none() {
        return Err("--m64/--emf require --occupancy".into());
    }
    let bundle = seal_mesh(&mesh, grid.as_ref()).map_err(|e| e.to_string())?;
    fs::write(&output, &bundle.ten_d).map_err(|e| e.to_string())?;
    if let Some(bytes) = bundle.m64 {
        let path = m64_out.unwrap_or_else(|| output.with_extension("m64"));
        fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        eprintln!("wrote {} ({} bytes)", path.display(), bytes.len());
    }
    if let Some(bytes) = bundle.emf {
        let path = emf_out.unwrap_or_else(|| output.with_extension("emf"));
        fs::write(&path, &bytes).map_err(|e| e.to_string())?;
        eprintln!("wrote {} ({} bytes)", path.display(), bytes.len());
    }
    eprintln!(
        "sealed {} ({} verts, {} tris, {} bytes)",
        output.display(),
        mesh.positions.len(),
        mesh.indices.len() / 3,
        bundle.ten_d.len()
    );
    Ok(())
}
