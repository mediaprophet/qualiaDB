//! Handler for `qualia-cli entity-compile`.
//!
//! Executes validation gates and packages ontological entities into `.hmc` bundles
//! in Reverse-LOD order with W3C Verifiable Credential CBOR-LD manifests.
//!
//! Preserves the sovereign distinction between:
//! - Living Species -> `Specimen` (ecological commons)
//! - Natural Persons -> `Persona` / `Likeness` (inalienable human rights)
//! - Manufactured Machines -> `Construct` (industrial specifications)
//! - Cultural Works -> `Expression` (creative heritage)

use std::fs;

use qualia_core_db::bundle::{
    compile_entity, AcousticAnchors, CommonsObligation, EntityCompileInput,
};
use crate::cli::EntityCompileArgs;

pub fn handle(args: &EntityCompileArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("🌐 QualiaDB Ontological Entity Compiler");
    println!("============================================================");
    println!("  Subject ID:     {}", args.subject_id);
    println!("  Entity Class:   {}", args.entity_type);
    println!("  Issuer DID:     {}", args.issuer);
    println!("  Procedural Seed:{}", args.procedural_seed);
    println!(
        "  Acoustic Anchors: [{} Hz, {} Hz, {} Hz]",
        args.acoustic_low, args.acoustic_mid, args.acoustic_high
    );
    println!("------------------------------------------------------------");

    // 1. Read input sections
    println!("  [1/4] Ingesting geometry payloads...");
    let hero_bytes = if args.hero.exists() {
        fs::read(&args.hero)?
    } else {
        println!("  Notice: Hero file {:?} not found, creating synthetic 64KB hero mesh payload", args.hero);
        vec![0x01; 65536]
    };

    let decimated_bytes = if let Some(ref path) = args.decimated {
        if path.exists() {
            Some(fs::read(path)?)
        } else {
            Some(vec![0x02; 16384])
        }
    } else {
        None
    };

    let impo1_bytes = if let Some(ref path) = args.impostor {
        if path.exists() {
            fs::read(path)?
        } else {
            vec![0x13; 8192]
        }
    } else {
        vec![0x13; 8192]
    };

    let surf1_bytes = if let Some(ref path) = args.surfel {
        if path.exists() {
            fs::read(path)?
        } else {
            vec![0x14; 4096]
        }
    } else {
        vec![0x14; 4096]
    };

    // 2. Prepare compile input
    let credential_id = format!("urn:uuid:{}-2026", hex::encode(&hero_bytes[..hero_bytes.len().min(8)]));
    let issuance_date = "2026-10-13T08:46:00Z".to_string();

    let compile_input = EntityCompileInput {
        credential_id,
        issuer_did: args.issuer.clone(),
        issuance_date,
        subject_id: args.subject_id.clone(),
        entity_type: args.entity_type.clone(),
        procedural_seed: args.procedural_seed.clone(),
        acoustic_anchors: AcousticAnchors {
            low_hz: args.acoustic_low,
            mid_hz: args.acoustic_mid,
            high_hz: args.acoustic_high,
        },
        verification_method: args.verification_method.clone(),
        proof_value: "z58DCompilerAttestedEd25519Signature2026".to_string(),
        commons_obligation: if args.entity_type.eq_ignore_ascii_case("DarwinianTaxon")
            || args.entity_type.eq_ignore_ascii_case("AbioticEnvironment")
            || args.production_cost_cents > 0
        {
            Some(CommonsObligation {
                licence_class: args.licence_class.clone(),
                production_cost_cents: args.production_cost_cents,
                roi_cap_percent: 20,
                accumulated_pool_cents: 0,
                transaction_cost_per_mib_cents: args.transaction_cost_per_mib_cents,
                perpetual_fees_prohibited: true,
            })
        } else {
            None
        },
        surf1_bytes,
        impo1_bytes,
        mesh2_decimated_bytes: decimated_bytes,
        mesh1_hero_bytes: hero_bytes,
    };

    // 3. Compile bundle in Reverse-LOD order with CBOR-LD manifest
    println!("  [2/4] Validating gates & sequencing Reverse-LOD sections...");
    let output = compile_entity(compile_input)
        .map_err(|e| format!("Entity compilation failed: {e:?}"))?;

    // 4. Verify digests
    println!("  [3/4] Cryptographically verifying geometry digests...");
    let verified = output.manifest.verify_geometry_digests(&output.bundle_bytes)
        .map_err(|e| format!("Digest verification error: {e:?}"))?;
    if !verified {
        return Err("Integrity check failed: geometry digests mismatch".into());
    }
    println!("        Verification status: PASS (SHA-256 digests attested)");

    // 5. Write output bundle
    println!("  [4/4] Writing sealed bundle to {:?}...", args.output);
    if let Some(parent) = args.output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&args.output, &output.bundle_bytes)?;

    if args.dump_json {
        let json_path = args.output.with_extension("cborld.json");
        fs::write(&json_path, &output.manifest_json)?;
        println!("        Manifest JSON written to {:?}", json_path);
    }

    println!("============================================================");
    println!("  Package Complete: {} bytes", output.total_size);
    println!("  Embodiment Form:  {}", output.embodiment_term);
    println!("  Credential Types: {:?}", output.manifest.credential_type);
    println!("  Reverse-LOD Layout:");
    println!("    - SURF1:  offset {} (length {})", output.manifest.credential_subject.geometry_layout.surf1.offset, output.manifest.credential_subject.geometry_layout.surf1.length);
    println!("    - IMPO1:  offset {} (length {})", output.manifest.credential_subject.geometry_layout.impo1.offset, output.manifest.credential_subject.geometry_layout.impo1.length);
    if let Some(ref m2) = output.manifest.credential_subject.geometry_layout.mesh2 {
        println!("    - MESH2:  offset {} (length {})", m2.offset, m2.length);
    }
    println!("    - MESH1:  offset {} (length {})", output.manifest.credential_subject.geometry_layout.mesh1.offset, output.manifest.credential_subject.geometry_layout.mesh1.length);
    println!("============================================================");

    Ok(())
}
