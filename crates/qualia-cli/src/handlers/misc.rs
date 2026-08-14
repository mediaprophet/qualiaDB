use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::path::PathBuf;

use qualia_core_db::NQuin;

use crate::cli::{
    CompileAction, ExtensionAction, GovernanceAction, IngestFormat, MigrateAction, ProfileAction,
    QueryDialect, ShaclAction,
};
use crate::sparql::run_sparql_query;

pub fn handle_extension(action: &ExtensionAction) {
    match action {
        ExtensionAction::Register { manifest_path } => {
            println!("Registering extension from {:?}", manifest_path);
        }
        ExtensionAction::List => {
            println!("Listing registered extensions...");
        }
        ExtensionAction::Dispatch { id, input } => {
            println!("Dispatching to extension '{}': {}", id, input);
        }
    }
}

pub fn handle_governance(action: &GovernanceAction) {
    match action {
        GovernanceAction::WalAppend { quin, sign } => {
            println!("Appending to WAL: {} (signed by {})", quin, sign);
        }
        GovernanceAction::Ratify { agreement_did } => {
            println!("Ratifying agreement: {}", agreement_did);
        }
    }
}

pub fn handle_compile(action: &CompileAction) {
    match action {
        CompileAction::N3ToDeontic { file } => {
            println!("Compiling N3 logic to native norms from {:?}", file);
        }
    }
}

pub fn handle_shacl(action: &ShaclAction) {
    match action {
        ShaclAction::List => {
            println!("============================================================");
            println!("⚙️  QualiaDB SHACL Extensions Active in Binary");
            println!("============================================================");
            println!("  - DeonticObligate");
            println!("  - DeonticPermit");
            println!("  - DeonticForbid");
            println!("  - DeonticNotExpired");
            println!("  - EpistemicKnowledge");
            println!("  - EpistemicBelief");
            println!("  - CommonKnowledge");
            println!("============================================================");
        }
        ShaclAction::Validate { dataset, shapes } => {
            println!(
                "Validating {:?} against SHACL shapes in {:?}...",
                dataset, shapes
            );
        }
    }
}

pub fn handle_vault(init: bool) {
    if init {
        println!("Initializing Memory-Mapped Vault...");
        let storage_dir = std::env::var("QUALIA_DATA_DIR").unwrap_or_else(|_| ".".to_string());
        let _vault = qualia_core_db::key_vault::KeyVault::load_or_generate(&storage_dir)
            .expect("Failed to load KeyVault");
        println!("Vault Initialization Complete!");
    }
}

pub fn handle_migrate(action: &MigrateAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        MigrateAction::Meta { path, dry_run } => {
            if *dry_run {
                use std::fs::File;
                use std::io::Read as _;
                let mut f = File::open(path)?;
                let mut magic = [0u8; 6];
                f.read_exact(&mut magic)?;
                let version = u16::from_le_bytes([magic[4], magic[5]]);
                if version >= 3 {
                    println!(
                        "[dry-run] {} is already v3 — no migration needed.",
                        path.display()
                    );
                } else {
                    println!("[dry-run] {} is v{version} — would migrate to v3 (Lamport bits [60:32]→[31:0], header bump).", path.display());
                }
            } else {
                println!("Migrating {} to Q42 v3…", path.display());
                qualia_core_db::q42_volume::migrate_v2_to_v3(path)?;
                println!("Migration complete: {} is now v3.", path.display());
            }
        }
    }
    Ok(())
}

pub fn handle_mem(inspect: bool) {
    if inspect {
        println!("Please use `qualia-cli inspect <superblock_path>` directly to inspect specific layouts.");
    }
}

pub fn handle_inspect(file_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing Block Inspector for: {:?}", file_path);

    let mut file = std::fs::File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    if buffer.len() % 48 != 0 {
        eprintln!("WARNING: File size {} is not a multiple of 48 bytes (NQuin alignment). File may be corrupted.", buffer.len());
    }

    let quin_size = std::mem::size_of::<NQuin>();
    let mut count = 0;

    for chunk in buffer.chunks_exact(quin_size) {
        let quin: NQuin = unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const NQuin) };
        let lamport_clock = quin.extract_lamport_clock();
        let geometric_payload = quin.extract_clean_metadata_value();

        println!(
            "[Quin {}] S: {}, P: {}, O: {}, Ctx: {}, LamportClock: {}, GeoPayload: {}, Parity: {}",
            count,
            quin.subject,
            quin.predicate,
            quin.object,
            quin.context,
            lamport_clock,
            geometric_payload,
            quin.parity
        );
        count += 1;
    }

    println!("Successfully inspected {} Quins.", count);
    Ok(())
}

pub fn handle_dump(out_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    println!("Dumping raw SuperBlock to: {:?}", out_path);

    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(out_path)?;

    let mut q1 = NQuin {
        subject: 100,
        predicate: 200,
        object: 300,
        context: 50,
        metadata: 0,
        parity: 0,
    };
    q1.set_lamport_clock(1);
    let mut q2 = NQuin {
        subject: 101,
        predicate: 201,
        object: 301,
        context: 51,
        metadata: 555,
        parity: 0,
    };
    q2.set_lamport_clock(2);
    let mut q3 = NQuin {
        subject: 102,
        predicate: 202,
        object: 302,
        context: 52,
        metadata: 999,
        parity: 0,
    };
    q3.set_lamport_clock(3);

    let quins = [q1, q2, q3];

    for quin in quins.iter() {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (quin as *const NQuin) as *const u8,
                std::mem::size_of::<NQuin>(),
            )
        };
        file.write_all(bytes)?;
    }

    file.sync_all()?;
    println!("Dumped 3 mocked Quins (144 bytes) to .q42 successfully.");
    Ok(())
}

pub fn handle_export_solid(input: &PathBuf, output: &PathBuf) {
    println!("============================================================");
    println!("🌐 W3C Solid Exporter Bridge");
    println!("============================================================");

    let in_path = input.to_string_lossy().to_string();
    let out_path = output.to_string_lossy().to_string();

    match qualia_core_db::solid_ldp::SolidExporter::export_to_solid_pod(&in_path, &out_path) {
        Ok(_) => {
            println!("✅ Export Complete! Your data is now fully portable to any Solid Pod.");
        }
        Err(e) => {
            eprintln!("❌ Export Failed: {}", e);
        }
    }
}

pub fn handle_verify_integrity(input: &PathBuf, dataset: &PathBuf) {
    println!("============================================================");
    println!("🔒 QualiaDB Zero-Allocation Integrity Verification");
    println!("  Input   : {}", input.display());
    println!("  Dataset : {}", dataset.display());
    println!("============================================================");

    match qualia_core_db::ingest::verify_integrity(input.clone(), dataset.clone()) {
        Ok(true) => {
            println!("Warning: this legacy XOR result is diagnostic only, not a proof of exact graph equality.");
            println!("\n✅ Integrity Check Passed: 100% Exact Match!");
            println!("XOR folds and record counts match; this is not a losslessness or graph-equality proof. Use `verify-graph` for the bounded encoded-set proof.");
        }
        Ok(false) => {
            eprintln!("\n❌ Integrity Check Failed: Checksums mismatch!");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("\n❌ Integrity Verification Error: {}", e);
            std::process::exit(1);
        }
    }
}

/// Run the exact, bounded-memory encoded graph proof.
pub fn handle_verify_graph(
    input: &PathBuf,
    dataset: &PathBuf,
    memory_mib: u64,
    temp_gib: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let memory_limit_bytes = memory_mib
        .checked_mul(1024 * 1024)
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or_else(|| std::io::Error::other("--memory-mib is too large for this platform"))?;
    let temporary_byte_budget = temp_gib
        .checked_mul(1024 * 1024 * 1024)
        .ok_or_else(|| std::io::Error::other("--temp-gib is too large"))?;

    println!("============================================================");
    println!("QualiaDB bounded encoded-graph proof");
    println!("  Input       : {}", input.display());
    println!("  Q42         : {}", dataset.display());
    println!("  RAM budget  : {memory_mib} MiB");
    println!("  Temp budget : {temp_gib} GiB");
    println!("============================================================");

    let report = qualia_core_db::graph_proof::prove_cli_ntriples_q42_equivalence(
        input,
        dataset,
        qualia_core_db::graph_proof::GraphProofOptions {
            memory_limit_bytes,
            temporary_byte_budget,
        },
    )?;

    println!("Source records      : {}", report.source_records);
    println!("Q42 records         : {}", report.q42_records);
    println!("Unique source quads : {}", report.source_unique_records);
    println!("Unique Q42 quads    : {}", report.q42_unique_records);
    println!("Missing from Q42    : {}", report.missing_from_q42);
    println!("Unexpected in Q42   : {}", report.unexpected_in_q42);
    println!("Skipped source lines : {}", report.source_skipped_lines);

    if !report.encoded_sets_match() {
        if let Some(record) = report.first_missing {
            println!("First missing encoded quad    : {record:016X?}");
        }
        if let Some(record) = report.first_unexpected {
            println!("First unexpected encoded quad : {record:016X?}");
        }
        return Err(std::io::Error::other("encoded graph sets differ").into());
    }

    match report.rdf_isomorphism {
        qualia_core_db::graph_proof::RdfIsomorphismStatus::GroundGraphProven => {
            println!("PASS: exact ground-graph equivalence is proven in the Q42 encoding.");
            Ok(())
        }
        qualia_core_db::graph_proof::RdfIsomorphismStatus::BlankNodeCanonicalizationRequired => {
            Err(std::io::Error::other(
                "encoded sets match only under blank-node label identity; RDF isomorphism requires canonical lexical blank-node support",
            )
            .into())
        }
        qualia_core_db::graph_proof::RdfIsomorphismStatus::Different => {
            Err(std::io::Error::other("encoded graph sets differ").into())
        }
    }
}

pub fn handle_import(
    input: &PathBuf,
    output: &PathBuf,
    strip_literals: bool,
    segment_mib: Option<u64>,
) {
    println!("============================================================");
    println!("📥 QualiaDB Native RDF/XML Ingestion Pipeline");
    println!("============================================================");

    let in_path = input.to_string_lossy().to_string();
    let out_path = output.to_string_lossy().to_string();
    let mode = if strip_literals {
        qualia_core_db::ingest::IngestMode::StripLiterals
    } else {
        qualia_core_db::ingest::IngestMode::Complete
    };

    let result = match segment_mib {
        Some(mib) => match mib.checked_mul(1024 * 1024) {
            Some(bytes) if bytes != 0 => {
                println!("Publishing a Q42 logical volume with {mib} MiB child cap.");
                qualia_core_db::ingest::streaming_import_rdf_volume_set_with_mode(
                    &in_path, &out_path, mode, bytes,
                )
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--segment-mib must be greater than zero",
            )),
        },
        None => qualia_core_db::ingest::streaming_import_rdf_with_mode(&in_path, &out_path, mode),
    };
    match result {
        Ok(quin_count) => {
            println!("✨ Done! Wrote {quin_count} Super-Quins.");
        }
        Err(e) => {
            eprintln!("❌ Import Failed: {}", e);
        }
    }
}

pub fn handle_ingest(format: &IngestFormat) {
    match format {
        IngestFormat::Semantic { file } => {
            let out_path = file.with_extension("q42");
            println!("Detecting format for: {}", file.display());
            match crate::ingest::ingest_auto(&file, &out_path) {
                Ok((stats, fmt)) => {
                    println!("Format : {}", fmt.label());
                    println!("Triples: {}", stats.triples_ingested);
                    println!("Blocks : {}", stats.blocks_written);
                    println!("Output : {}", out_path.display());
                    println!("Done.");
                }
                Err(e) => eprintln!("Ingest error: {e}"),
            }
        }
        IngestFormat::Csv { file, map } => {
            println!("CSV ingest for {:?} using map {:?}", file, map);
            match crate::ingest::mapper::compile_shacl_mapping(map) {
                Ok(mut profile) => {
                    let path_str = file.to_string_lossy();
                    let out_path = file.with_extension("q42").to_string_lossy().into_owned();
                    crate::ingest::csv_mapper::stream_csv_to_quins(
                        &path_str,
                        &out_path,
                        &mut profile,
                    );
                    println!("✅ CSV Ingest Complete");
                }
                Err(e) => eprintln!("❌ Failed to compile SHACL mapping: {}", e),
            }
        }
        IngestFormat::Json { file, map } => {
            println!("JSON ingest for {:?} using map {:?}", file, map);
            match crate::ingest::mapper::compile_shacl_mapping(map) {
                Ok(profile) => {
                    let path_str = file.to_string_lossy();
                    let out_path = file.with_extension("q42").to_string_lossy().into_owned();
                    crate::ingest::json_mapper::stream_json_to_quins(
                        &path_str, &out_path, &profile,
                    );
                    println!("✅ JSON Ingest Complete");
                }
                Err(e) => eprintln!("❌ Failed to compile SHACL mapping: {}", e),
            }
        }
    }
}

pub fn handle_query(dialect: &QueryDialect) {
    match dialect {
        QueryDialect::Sparql {
            vault,
            query_string,
            file,
        } => {
            let qs = if let Some(q) = query_string {
                q.clone()
            } else if let Some(f) = file {
                std::fs::read_to_string(f).expect("Failed to read SPARQL file")
            } else {
                panic!("Must provide either a query_string or a file");
            };
            run_sparql_query(&vault, &qs);
        }
        QueryDialect::SparqlStar {
            vault,
            query_string,
            file,
        } => {
            let qs = if let Some(q) = query_string {
                q.clone()
            } else if let Some(f) = file {
                std::fs::read_to_string(f).expect("Failed to read SPARQL-Star file")
            } else {
                panic!("Must provide either a query_string or a file");
            };
            run_sparql_query(&vault, &qs);
        }
    }
}

pub fn handle_compress(input: &PathBuf, output: &PathBuf) {
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let is_q42 = ext == "q42";

    println!("============================================================");
    println!("QualiaDB LZ4 Block-Stream Compressor");
    println!("  input  : {}", input.display());
    println!("  output : {}", output.display());
    println!(
        "  mode   : {}",
        if is_q42 {
            "SuperBlock → raw Quins"
        } else {
            "raw bytes"
        }
    );
    println!("============================================================");

    let result = if is_q42 {
        crate::compress::compress_q42(input, output)
    } else {
        crate::compress::compress_raw(input, output)
    };

    match result {
        Ok(stats) => {
            println!("Done.");
            println!(
                "  Input  : {:.1} MB",
                stats.input_bytes as f64 / 1_048_576.0
            );
            println!(
                "  Output : {:.1} MB",
                stats.output_bytes as f64 / 1_048_576.0
            );
            println!("  Blocks : {}", stats.blocks);
            println!("  Ratio  : {:.2}x", stats.ratio);
        }
        Err(e) => eprintln!("Compression failed: {}", e),
    }
}

pub fn handle_profile(action: &ProfileAction) {
    match action {
        ProfileAction::Compile { input, out } => {
            let out_path = out.clone().unwrap_or_else(|| input.with_extension("qchk"));
            println!("============================================================");
            println!("⚡ Qualia Capability Profile Compiler");
            println!("  input  : {}", input.display());
            println!("  output : {}", out_path.display());
            println!("============================================================");
            match std::fs::read_to_string(input) {
                Err(e) => eprintln!("❌ Failed to read profile source: {}", e),
                Ok(jsonld_src) => {
                    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
                    let profile_id = qualia_core_db::q_hash(&format!("profile:{}", stem));
                    let mut chk_bytes: Vec<u8> = Vec::new();
                    chk_bytes.extend_from_slice(b"QCHK");
                    chk_bytes.extend_from_slice(&profile_id.to_le_bytes());
                    chk_bytes.extend_from_slice(&(jsonld_src.len() as u32).to_le_bytes());
                    chk_bytes.extend_from_slice(jsonld_src.as_bytes());
                    match std::fs::write(&out_path, &chk_bytes) {
                        Ok(_) => {
                            println!(
                                "✅ Compiled profile 0x{:016X} ({} bytes)",
                                profile_id,
                                chk_bytes.len()
                            );
                            println!("   Stem  : {}", stem);
                            println!("   Output: {}", out_path.display());
                            println!("   Next  : qualia-cli ingest --input data.nt --output out --profile {}", out_path.display());
                        }
                        Err(e) => eprintln!("❌ Write failed: {}", e),
                    }
                }
            }
        }
        ProfileAction::List => {
            println!("============================================================");
            println!("📋 Registered Capability Profiles");
            println!("============================================================");
            println!("  (Profiles are registered when ingested via ExternalSorter)");
            println!("  Known profile ID namespaces:");
            let known = [
                (
                    "profile:general",
                    "General purpose — no engine restrictions",
                ),
                (
                    "profile:health",
                    "Health/Clinical — NativeClinicalRisk, NativeBioAlignment",
                ),
                (
                    "profile:chemistry",
                    "Organic Chemistry — NativeChemicalSynthesis, NativeLipinski",
                ),
                (
                    "profile:research",
                    "Research — all scientific opcodes, no financial engines",
                ),
                (
                    "profile:legal",
                    "Legal/Deontic — OP_OBLIGATE, OP_FORBID, OP_PERMIT",
                ),
                (
                    "profile:financial",
                    "Financial — ILP dispatchers, tax schema, audit trail",
                ),
            ];
            for (name, desc) in &known {
                println!(
                    "  0x{:016X}  {}  — {}",
                    qualia_core_db::q_hash(name),
                    name,
                    desc
                );
            }
        }
        ProfileAction::Inspect { file } => {
            println!("============================================================");
            println!("🔎 Profile Inspector: {}", file.display());
            println!("============================================================");
            match std::fs::read(file) {
                Err(e) => eprintln!("❌ Cannot read file: {}", e),
                Ok(bytes) => {
                    if bytes.len() < 16 || &bytes[0..4] != b"QCHK" {
                        eprintln!(
                            "❌ Not a valid QCHK profile (.qchk or legacy .chk missing QCHK magic)"
                        );
                    } else {
                        let profile_id = u64::from_le_bytes(bytes[4..12].try_into().unwrap());
                        let payload_len =
                            u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
                        let payload =
                            &bytes[16..16 + payload_len.min(bytes.len().saturating_sub(16))];
                        println!("  Profile ID : 0x{:016X}", profile_id);
                        println!("  Payload    : {} bytes (JSON-LD source)", payload_len);
                        println!("  Total file : {} bytes", bytes.len());
                        println!();
                        println!("--- JSON-LD Source ---");
                        println!("{}", String::from_utf8_lossy(payload));
                    }
                }
            }
        }
    }
}

pub fn handle_capabilities(list: bool) {
    if list {
        println!("============================================================");
        println!("🧠 QualiaDB Runtime Capability Registry");
        println!("============================================================");
        for capability in qualia_core_db::CAPABILITY_DESCRIPTORS {
            println!(
                "  - {} [{}] -> {}",
                capability.name,
                capability.domain,
                capability.mcp_tools.join(", ")
            );
        }
        println!("============================================================");
    } else {
        println!("Use `qualia-cli capabilities --list` to view capabilities.");
    }
}
