//! Handler for `qualia-cli entity-verify`.
//!
//! Provides ultra-fast, zero-DOM batch verification for `.10d` living containers
//! and `.hmc` bundles, allowing autonomous software agents to inspect thousands
//! of species/models without browser or Node dependencies.

use std::fs;
use std::path::{Path, PathBuf};

use qualia_core_db::bundle::entity_verifier::{
    verify_entity_10d, verify_entity_bundle, Entity10dVerification, EntityBundleVerification,
};
use serde::{Deserialize, Serialize};

use crate::cli::EntityVerifyArgs;

#[derive(Debug, Serialize, Deserialize)]
pub enum FileVerificationResult {
    Container10d {
        path: String,
        report: Entity10dVerification,
    },
    BundleHmc {
        path: String,
        report: EntityBundleVerification,
    },
    Error {
        path: String,
        error: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchVerificationReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub results: Vec<FileVerificationResult>,
}

pub fn handle(args: &EntityVerifyArgs) -> Result<(), Box<dyn std::error::Error>> {
    let path = &args.path;

    if !path.exists() {
        return Err(format!("path does not exist: {:?}", path).into());
    }

    if path.is_dir() || args.batch {
        handle_batch(path, args)
    } else {
        handle_single_file(path, args)
    }
}

fn handle_single_file(path: &Path, args: &EntityVerifyArgs) -> Result<(), Box<dyn std::error::Error>> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let bytes = fs::read(path)?;

    if ext.eq_ignore_ascii_case("10d") {
        let rep = verify_entity_10d(&bytes);
        if args.json {
            println!("{}", serde_json::to_string_pretty(&rep)?);
        } else if !args.quiet {
            print_10d_report(path, &rep);
        }
        if !rep.valid {
            return Err("entity verification failed".into());
        }
    } else if ext.eq_ignore_ascii_case("hmc") {
        let rep = verify_entity_bundle(&bytes);
        if args.json {
            println!("{}", serde_json::to_string_pretty(&rep)?);
        } else if !args.quiet {
            print_bundle_report(path, &rep);
        }
        if !rep.valid {
            return Err("bundle verification failed".into());
        }
    } else {
        // Sniff by magic bytes
        if bytes.starts_with(b"10D\0") {
            let rep = verify_entity_10d(&bytes);
            if args.json {
                println!("{}", serde_json::to_string_pretty(&rep)?);
            } else if !args.quiet {
                print_10d_report(path, &rep);
            }
            if !rep.valid {
                return Err("entity verification failed".into());
            }
        } else if bytes.starts_with(b"HMC1") {
            let rep = verify_entity_bundle(&bytes);
            if args.json {
                println!("{}", serde_json::to_string_pretty(&rep)?);
            } else if !args.quiet {
                print_bundle_report(path, &rep);
            }
            if !rep.valid {
                return Err("bundle verification failed".into());
            }
        } else {
            return Err(format!("unrecognized file format for {:?}; expected .10d or .hmc", path).into());
        }
    }

    Ok(())
}

fn handle_batch(dir: &Path, args: &EntityVerifyArgs) -> Result<(), Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    collect_files_recursive(dir, &mut files)?;

    if !args.quiet && !args.json {
        println!("============================================================");
        println!("🔍 QualiaDB Batch Entity Verification");
        println!("============================================================");
        println!("  Target Directory: {:?}", dir);
        println!("  Candidate Files:  {}", files.len());
        println!("------------------------------------------------------------");
    }

    let mut results = Vec::new();
    let mut passed = 0;
    let mut failed = 0;

    for file_path in &files {
        let path_str = file_path.to_string_lossy().to_string();
        let bytes = match fs::read(file_path) {
            Ok(b) => b,
            Err(e) => {
                failed += 1;
                results.push(FileVerificationResult::Error {
                    path: path_str,
                    error: e.to_string(),
                });
                continue;
            }
        };

        let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext.eq_ignore_ascii_case("10d") || bytes.starts_with(b"10D\0") {
            let rep = verify_entity_10d(&bytes);
            if rep.valid {
                passed += 1;
                if !args.quiet && !args.json {
                    println!("  ✅ [PASS] 10D: {} (verts: {}, tris: {}, surfels: {})",
                        file_path.file_name().unwrap_or_default().to_string_lossy(),
                        rep.vertices,
                        rep.triangles,
                        rep.surfels.as_ref().map(|s| s.surfel_count).unwrap_or(0)
                    );
                }
            } else {
                failed += 1;
                if !args.quiet && !args.json {
                    println!("  ❌ [FAIL] 10D: {} (violations: {:?})",
                        file_path.file_name().unwrap_or_default().to_string_lossy(),
                        rep.violations
                    );
                }
            }
            results.push(FileVerificationResult::Container10d {
                path: path_str,
                report: rep,
            });
        } else if ext.eq_ignore_ascii_case("hmc") || bytes.starts_with(b"HMC1") {
            let rep = verify_entity_bundle(&bytes);
            if rep.valid {
                passed += 1;
                if !args.quiet && !args.json {
                    println!("  ✅ [PASS] HMC: {} (type: {:?}, entries: {})",
                        file_path.file_name().unwrap_or_default().to_string_lossy(),
                        rep.entity_type.as_deref().unwrap_or("unknown"),
                        rep.entry_count
                    );
                }
            } else {
                failed += 1;
                if !args.quiet && !args.json {
                    println!("  ❌ [FAIL] HMC: {} (violations: {:?})",
                        file_path.file_name().unwrap_or_default().to_string_lossy(),
                        rep.violations
                    );
                }
            }
            results.push(FileVerificationResult::BundleHmc {
                path: path_str,
                report: rep,
            });
        }
    }

    let batch_report = BatchVerificationReport {
        total: files.len(),
        passed,
        failed,
        results,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&batch_report)?);
    } else if !args.quiet {
        println!("------------------------------------------------------------");
        println!("  Summary: Total: {}, Passed: {}, Failed: {}", batch_report.total, batch_report.passed, batch_report.failed);
        println!("============================================================");
    }

    if failed > 0 {
        return Err(format!("batch verification completed with {failed} failure(s)").into());
    }

    Ok(())
}

fn collect_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, out)?;
            } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if ext.eq_ignore_ascii_case("10d") || ext.eq_ignore_ascii_case("hmc") {
                    out.push(path);
                }
            }
        }
    } else {
        out.push(dir.to_path_buf());
    }
    Ok(())
}

fn print_10d_report(path: &Path, rep: &Entity10dVerification) {
    println!("============================================================");
    println!("📦 QualiaDB .10d Living Container Verification");
    println!("============================================================");
    println!("  Path:             {:?}", path);
    println!("  Status:           {}", if rep.valid { "✅ VALID" } else { "❌ INVALID" });
    println!("  Header Version:   {}", rep.header_version);
    println!("  Whole-file CRC:   {}", if rep.crc32c_valid { "Verified" } else { "Mismatch" });
    println!("  Section Count:    {}", rep.section_count);
    println!("  Sections Found:   {}", rep.sections_found.join(", "));
    println!("  Mesh Geometry:    {} vertices, {} triangles", rep.vertices, rep.triangles);
    if let Some(ref s) = rep.surfels {
        println!("  Section 20 SURF1: {} surfels (disc radius: {:.3})", s.surfel_count, s.sphere_radius);
    } else {
        println!("  Section 20 SURF1: None");
    }
    if let Some(ref imp) = rep.impostors {
        println!("  Section 19 IMPO1: {}x{} grid @ {}x{} resolution", imp.grid_dimension, imp.grid_dimension, imp.view_resolution, imp.view_resolution);
    } else {
        println!("  Section 19 IMPO1: None");
    }
    if let Some(ref aff) = rep.affordances {
        println!("  Section 17 EONT1: {} affordances (did hash: 0x{:016x})", aff.affordance_count, aff.entity_did_hash);
    }
    println!("  Reverse-LOD:      {}", if rep.reverse_lod_valid { "Verified (SURF1 -> IMPO1 -> Mesh)" } else { "Violation" });
    println!("  AABB Bounds:      [{:.2}, {:.2}, {:.2}] to [{:.2}, {:.2}, {:.2}]",
        rep.min_bounds[0], rep.min_bounds[1], rep.min_bounds[2],
        rep.max_bounds[0], rep.max_bounds[1], rep.max_bounds[2]
    );
    if !rep.violations.is_empty() {
        println!("  Violations:       {:?}", rep.violations);
    }
    println!("============================================================");
}

fn print_bundle_report(path: &Path, rep: &EntityBundleVerification) {
    println!("============================================================");
    println!("📦 QualiaDB .hmc Entity Bundle Verification");
    println!("============================================================");
    println!("  Path:             {:?}", path);
    println!("  Status:           {}", if rep.valid { "✅ VALID" } else { "❌ INVALID" });
    println!("  Bundle CRC-32C:   {}", if rep.bundle_crc32c_valid { "Verified" } else { "Mismatch" });
    println!("  Entry Count:      {}", rep.entry_count);
    println!("  CBOR-LD Manifest: {}", if rep.manifest_present { "Present" } else { "Missing" });
    if let Some(ref id) = rep.credential_id {
        println!("  Credential ID:    {}", id);
    }
    if let Some(ref iss) = rep.issuer_did {
        println!("  Issuer DID:       {}", iss);
    }
    if let Some(ref sub) = rep.subject_id {
        println!("  Subject ID:       {}", sub);
    }
    if let Some(ref cls) = rep.entity_type {
        println!("  Entity Class:     {}", cls);
    }
    if let Some(ref anc) = rep.acoustic_anchors {
        println!("  Acoustic Anchors: [{} Hz, {} Hz, {} Hz]", anc.low_hz, anc.mid_hz, anc.high_hz);
    }
    println!("  Anti-Commodif.:   {}", if rep.anti_commodification_passed { "Passed" } else { "Violated" });
    println!("  Permissive Commons:{}", if rep.permissive_commons_passed { "Passed (Permissive / SPC)" } else { "Violated" });
    if let Some(ref cob) = rep.commons_obligation {
        println!("  Commons Licence:  {}", cob.licence_class);
        println!(
            "  Production Cost:  ${:.2} (Accumulated Pool: ${:.2}, Discharged: {})",
            cob.production_cost_cents as f64 / 100.0,
            cob.accumulated_pool_cents as f64 / 100.0,
            if cob.is_discharged { "Yes" } else { "No" }
        );
        if !cob.is_discharged {
            println!(
                "  Outstanding Cost: ${:.2} (ROI Cap: {}%)",
                cob.outstanding_cents as f64 / 100.0,
                cob.roi_cap_percent
            );
        }
        if cob.transaction_cost_per_mib_cents > 0 {
            println!(
                "  Carrier Recovery: ${:.4}/MiB (At-cost bandwidth/transport only)",
                cob.transaction_cost_per_mib_cents as f64 / 100.0
            );
        } else {
            println!("  Carrier Recovery: $0.00/MiB (Zero transaction surcharge)");
        }
        println!(
            "  Perpetual Fees:   {}",
            if cob.perpetual_fees_prohibited {
                "Prohibited (Humanitarian ICT Compliance)"
            } else {
                "Warning: Not prohibited!"
            }
        );
    }
    println!("  Geometry Digests: {}", if rep.geometry_digests_match { "Verified" } else { "Mismatch" });
    println!("  Contained .10d:   {} entity/entities", rep.contained_10d_entities.len());
    if !rep.violations.is_empty() {
        println!("  Violations:       {:?}", rep.violations);
    }
    println!("============================================================");
}
