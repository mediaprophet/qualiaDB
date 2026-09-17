//! Produce a curated `.hmc` anatomy asset pack from the live HRA endpoints.
//!
//! Usage:
//!   cargo run -p qualia-client-core --example build_anatomy_pack -- list [male|female|both]
//!   cargo run -p qualia-client-core --example build_anatomy_pack -- build [male|female|both] [out_dir]
//!   cargo run -p qualia-client-core --example build_anatomy_pack -- workshop [male|female] <export_dir> [out_path]
//!   cargo run -p qualia-client-core --example build_anatomy_pack -- systems
//!
//! `list`  — discover every reference organ for the model(s) and print each
//!           filename + normalised token (use this to curate the token set).
//! `build` — fetch the curated organs, compile to `.10d`, and write
//!           `<out_dir>/anatomy-<model>.hmc` (default out_dir: current dir).

use qualia_client_core::wellfair::anatomy_pack::{build_anatomy_pack, discover_model_organs};
use qualia_core_db::bundle::BundleMmap;
use qualia_core_db::render::anatomy_pack::AnatomyOrganMeta;
use wellfare_core::anatomy::AnatomyModel;

fn models_arg(arg: Option<&String>) -> Vec<AnatomyModel> {
    match arg.map(|s| s.as_str()) {
        Some("male") => vec![AnatomyModel::Male],
        Some("female") => vec![AnatomyModel::Female],
        _ => vec![AnatomyModel::Male, AnatomyModel::Female],
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("list");

    match cmd {
        "list" => {
            for model in models_arg(args.get(2)) {
                match discover_model_organs(model) {
                    Ok(organs) => {
                        println!(
                            "== {} : {} reference organs ==",
                            model.as_str(),
                            organs.len()
                        );
                        for o in &organs {
                            println!("  {:<40} {}", o.token, o.filename);
                        }
                    }
                    Err(e) => eprintln!("discover {} failed: {e}", model.as_str()),
                }
            }
        }
        "systems" => {
            print!(
                "{}",
                wellfare_core::anatomy::seed_system_coverage_markdown()
            );
            println!(
                "\nAuthority is Qualia (`wellfare_core::anatomy::seed_system_coverage`), not the workshop Python CLI."
            );
        }
        "workshop" => {
            let model = match args.get(2).map(|s| s.as_str()) {
                Some("female") => AnatomyModel::Female,
                _ => AnatomyModel::Male,
            };
            let dir = args
                .get(3)
                .cloned()
                .unwrap_or_else(|| r"C:\Projects\anatomy\export".to_string());
            let out = args
                .get(4)
                .cloned()
                .unwrap_or_else(|| format!("target/anatomy-pack/workshop-{}.hmc", model.as_str()));
            match qualia_client_core::wellfair::workshop_ingest::build_workshop_pack(
                &dir, model, &out,
            ) {
                Ok(r) => {
                    println!("== workshop packed {} ==", r.model);
                    println!("  file        : {}", r.out_path);
                    println!("  organs      : {}", r.organs_packed);
                    println!("  .10d bytes  : {}", r.total_10d_bytes);
                    println!("  keys        : {}", r.packed_keys.join(", "));
                    if !r.failed.is_empty() {
                        for (k, e) in &r.failed {
                            eprintln!("  FAILED {k}: {e}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("workshop ingest failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        "build" => {
            let out_dir = args.get(3).cloned().unwrap_or_else(|| ".".to_string());
            let mut had_err = false;
            for model in models_arg(args.get(2)) {
                let out = format!("{out_dir}/anatomy-{}.hmc", model.as_str());
                match build_anatomy_pack(model, &out, None) {
                    Ok(r) => {
                        println!("== packed {} ==", r.model);
                        println!("  file        : {}", r.out_path);
                        println!("  organs      : {}", r.organs_packed);
                        println!("  .10d bytes  : {}", r.total_10d_bytes);
                        println!(
                            "  body.q42    : {} B · {} quins (provenance + organ→system graph)",
                            r.q42_graph_bytes, r.q42_quins
                        );
                        println!("  q42 sidecar : {}", r.q42_sidecar_path);
                        println!("  bundle bytes: {}", r.bundle_bytes);
                        println!("  keys        : {}", r.packed_keys.join(", "));
                        if !r.curated_not_found.is_empty() {
                            println!("  not found   : {}", r.curated_not_found.join(", "));
                        }
                        if !r.failed.is_empty() {
                            for (k, e) in &r.failed {
                                eprintln!("  FAILED {k}: {e}");
                            }
                            if r.organs_packed == 0 {
                                had_err = true;
                            }
                        }
                    }
                    Err(e) => {
                        had_err = true;
                        eprintln!("build {} failed: {e}", model.as_str());
                    }
                }
            }
            if had_err {
                std::process::exit(1);
            }
        }
        "verify" => {
            let out_dir = args.get(3).cloned().unwrap_or_else(|| ".".to_string());
            let mut had_err = false;
            for model in models_arg(args.get(2)) {
                let path = format!("{out_dir}/anatomy-{}.hmc", model.as_str());
                let m = match BundleMmap::open(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        had_err = true;
                        eprintln!("open {path} failed: {e}");
                        continue;
                    }
                };
                let r = match m.reader() {
                    Ok(r) => r,
                    Err(e) => {
                        had_err = true;
                        eprintln!("parse {path} failed: {e}");
                        continue;
                    }
                };
                println!(
                    "== {} : {} entries, {} bytes ==",
                    path,
                    r.entries().len(),
                    m.as_bytes().len()
                );
                for e in r.entries() {
                    let sha_ok = r.verify_entry(&e.key);
                    if !sha_ok {
                        had_err = true;
                    }
                    // The pack-level provenance/semantics graph — round-trip it and report the facts.
                    if e.kind == "q42" {
                        let quins_licence = r.get(&e.key).and_then(|bytes| {
                            let tmp = tempfile::NamedTempFile::new().ok()?;
                            std::fs::write(tmp.path(), bytes).ok()?;
                            let vol =
                                qualia_core_db::q42_volume::Q42Volume::open(tmp.path()).ok()?;
                            let quins = vol.read_all_quins().ok()?;
                            let lex = vol.lex_view().ok()?;
                            let lic = quins
                                .iter()
                                .filter(|q| lex.lookup_hash(q.object) == Some("CC-BY-4.0"))
                                .count();
                            Some((quins.len(), lic))
                        });
                        match quins_licence {
                            Some((n, lic)) => println!(
                                "  [{}] {:<34} {:>9} B  q42 · {} quins · {} CC-BY-4.0 facts",
                                if sha_ok { "ok" } else { "BAD-SHA" },
                                e.key,
                                e.length,
                                n,
                                lic
                            ),
                            None => {
                                had_err = true;
                                println!(
                                    "  [{}] {:<34} {:>9} B  q42 UNREADABLE",
                                    if sha_ok { "ok" } else { "BAD-SHA" },
                                    e.key,
                                    e.length
                                );
                            }
                        }
                        continue;
                    }
                    let meta = e.meta.as_deref().and_then(AnatomyOrganMeta::from_cbor);
                    match meta {
                        Some(md) => println!(
                            "  [{}] {:<34} {:>9} B  sys={:<15} pos={:?} rgba={:?}",
                            if sha_ok { "ok" } else { "BAD-SHA" },
                            e.key,
                            e.length,
                            md.system,
                            md.position,
                            md.rgba
                        ),
                        None => {
                            had_err = true;
                            println!(
                                "  [{}] {:<34} {:>9} B  META MISSING/BAD",
                                if sha_ok { "ok" } else { "BAD-SHA" },
                                e.key,
                                e.length
                            );
                        }
                    }
                }
            }
            if had_err {
                std::process::exit(1);
            }
        }
        "bounds" => {
            // Diagnose the coordinate space: print each organ's centroid + bbox. If centroids vary
            // anatomically (brain high, bladder low) the meshes are in a SHARED body space and the
            // renderer should use one global scale + true positions; if every centroid is ~origin
            // they are local-framed and need CCF placement transforms.
            let out_dir = args.get(3).cloned().unwrap_or_else(|| ".".to_string());
            for model in models_arg(args.get(2)) {
                let path = format!("{out_dir}/anatomy-{}.hmc", model.as_str());
                let m = match BundleMmap::open(&path) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("open {path} failed: {e}");
                        continue;
                    }
                };
                let r = match m.reader() {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("parse {path} failed: {e}");
                        continue;
                    }
                };
                println!("== {} ==", path);
                for e in r.entries() {
                    if e.kind != "10d" {
                        continue;
                    }
                    let Some(bytes) = r.get(&e.key) else { continue };
                    match qualia_core_db::render::compile_10d::decode_10d_mesh(bytes) {
                        Ok(mesh) => {
                            let c = mesh.centroid();
                            let sx = mesh.max[0] - mesh.min[0];
                            let sy = mesh.max[1] - mesh.min[1];
                            let sz = mesh.max[2] - mesh.min[2];
                            println!(
                                "  {:<32} centroid=[{:7.3},{:7.3},{:7.3}]  size=[{:7.3},{:7.3},{:7.3}]",
                                e.key, c[0], c[1], c[2], sx, sy, sz
                            );
                        }
                        Err(err) => println!("  {:<32} decode error: {err}", e.key),
                    }
                }
            }
        }
        "bodyparts" => {
            // Build a SEPARATE, CC-BY-SA BodyParts3D pack — the muscles/bones/glands/nerves that complete
            // the body CCF (viscera-only) cannot. Bandwidth-controlled: pick systems + caps (the full set
            // is ~1.3 GB / 937 files). Usage:
            //   bodyparts [out_dir] [systems_csv|all] [max_structures] [max_stl_mb]
            use qualia_client_core::wellfair::bodyparts3d_resolver::{
                build_bodyparts3d_pack, Bp3dSelection, BP3D_ATTRIBUTION, BP3D_LICENCE,
            };
            let out_dir = args.get(2).cloned().unwrap_or_else(|| ".".to_string());
            // Default: the gap-fill glands + sense/other organs CCF barely covers (small, tens of MB).
            let systems: Vec<String> = match args.get(3).map(String::as_str) {
                Some("all") => Vec::new(),
                Some(csv) => csv
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                None => [
                    "endocrine",
                    "sensory",
                    "urinary",
                    "immune_lymphatic",
                    "reproductive",
                ]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            };
            let max_structures: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
            let max_mb: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(25);
            let sel = Bp3dSelection {
                systems: systems.clone(),
                max_structures,
                max_stl_bytes: max_mb.saturating_mul(1024 * 1024),
            };
            let out = format!("{out_dir}/anatomy-bodyparts3d.hmc");
            println!("BodyParts3D · {BP3D_LICENCE} · {BP3D_ATTRIBUTION}");
            println!(
                "selection: systems={} max_structures={} max_mb={}",
                if systems.is_empty() {
                    "all".to_string()
                } else {
                    systems.join(",")
                },
                max_structures,
                max_mb
            );
            match build_bodyparts3d_pack(&sel, &out) {
                Ok(r) => {
                    println!("== packed BodyParts3D ==");
                    println!("  file        : {}", r.out_path);
                    println!("  structures  : {}", r.structures_packed);
                    println!("  STL bytes   : {}", r.total_stl_bytes);
                    println!(
                        "  ontology    : {} quins · {} B  (OBO FMA IRIs + is-a + part-of + system + geometry)",
                        r.ontology_quins, r.ontology_q42_bytes
                    );
                    println!("  q42 sidecar : {}", r.q42_sidecar_path);
                    println!("  bundle bytes: {}", r.bundle_bytes);
                    println!("  per system  : {:?}", r.per_system);
                    if !r.failed.is_empty() {
                        eprintln!("  {} failed:", r.failed.len());
                        for (k, e) in r.failed.iter().take(10) {
                            eprintln!("    {k}: {e}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("bodyparts build failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!(
                "unknown command {other:?}; use `list`, `build`, `verify`, `bounds`, or `bodyparts`"
            );
            std::process::exit(2);
        }
    }
}
