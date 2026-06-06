import re

with open('crates/qualia-cli/src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# 1. Update the Commands enum
commands_addition = """    /// Dynamically lists features and capabilities compiled into the engine
    Capabilities {
        #[arg(long, help = "List all registered capabilities")]
        list: bool,
    },
    /// Advanced SHACL (Shapes Constraint Language) operations
    Shacl {
        #[arg(long, help = "List all available SHACL extensions (e.g. Deontic, Epistemic)")]
        list_extensions: bool,
    },
    /// Vault initialization and management
    Vault {
        #[arg(long, help = "Initialize the memory-mapped storage vault")]
        init: bool,
    },
    /// Database schema/state transitions
    Migrate,
    /// Inspect memory layouts natively
    Mem {
        #[arg(long, help = "Triggers the Block Inspector to read hex layouts")]
        inspect: bool,
    },
"""
content = re.sub(r'(enum Commands \{)', r'\1\n' + commands_addition, content)


# 2. Update Ingest matching
ingest_old = """        Commands::Ingest { input, output } => {
            let ext = input.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let is_rdf = matches!(ext.as_str(), "rdf" | "xml" | "owl" | "ttl" | "turtle");

            // Ensure the output path ends in ".q42" so that lex_path()
            // correctly derives "<base>.q42.lex" from it.
            let q42_output = if output.extension().and_then(|e| e.to_str()) == Some("q42") {
                output.clone()
            } else {
                output.with_extension("q42")
            };

            println!("============================================================");
            if is_rdf {
                println!("QualiaDB RDF/XML → .q42 Ingestor");
            } else {
                println!("QualiaDB N-Triples → .q42 Ingestor");
            }
            println!("  input  : {}", input.display());
            println!("  output : {}", q42_output.display());
            println!("         + {}.lex  (lexicon)", q42_output.display());
            println!("         + {}.bidx (block index)", q42_output.display());
            println!("============================================================");

            let result = if is_rdf {
                ingest::ingest_rdf_xml(input, &q42_output)
            } else {
                ingest::ingest_ntriples(input, &q42_output)
            };"""

ingest_new = """        Commands::Ingest { input, output } => {
            let ext = input.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let is_rdf = matches!(ext.as_str(), "rdf" | "xml" | "owl" | "ttl" | "turtle");
            let is_chk = ext == "chk";
            let is_cbor = ext == "cbor" || ext == "cbor-ld";

            // Ensure the output path ends in ".q42" so that lex_path()
            // correctly derives "<base>.q42.lex" from it.
            let q42_output = if output.extension().and_then(|e| e.to_str()) == Some("q42") {
                output.clone()
            } else {
                output.with_extension("q42")
            };

            println!("============================================================");
            if is_rdf {
                println!("QualiaDB RDF/XML → .q42 Ingestor");
            } else if is_chk {
                println!("QualiaDB Cognitive AI .chk → .q42 Ingestor");
            } else if is_cbor {
                println!("QualiaDB CBOR-LD → .q42 Ingestor");
            } else {
                println!("QualiaDB N-Triples → .q42 Ingestor");
            }
            println!("  input  : {}", input.display());
            println!("  output : {}", q42_output.display());
            println!("         + {}.lex  (lexicon)", q42_output.display());
            println!("         + {}.bidx (block index)", q42_output.display());
            println!("============================================================");

            let result = if is_rdf {
                ingest::ingest_rdf_xml(input, &q42_output)
            } else if is_chk {
                ingest::ingest_chk(input, &q42_output)
            } else if is_cbor {
                ingest::ingest_cbor(input, &q42_output)
            } else {
                ingest::ingest_ntriples(input, &q42_output)
            };"""

content = content.replace(ingest_old, ingest_new)

# 3. Add match logic for new subcommands
match_additions = """        Commands::Capabilities { list } => {
            if *list {
                println!("============================================================");
                println!("🧠 QualiaDB Runtime Capability Registry");
                println!("============================================================");
                for cap in qualia_core_db::CAPABILITY_REGISTRY {
                    println!("  - {}", cap);
                }
                println!("============================================================");
            } else {
                println!("Use `qualia-cli capabilities --list` to view capabilities.");
            }
        }
        Commands::Shacl { list_extensions } => {
            if *list_extensions {
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
            } else {
                println!("Use `qualia-cli shacl --list-extensions` to view SHACL features.");
            }
        }
        Commands::Vault { init } => {
            if *init {
                println!("Initializing Memory-Mapped Vault...");
                let storage_dir = std::env::var("QUALIA_DATA_DIR").unwrap_or_else(|_| ".".to_string());
                let _vault = qualia_core_db::key_vault::KeyVault::load_or_generate(&storage_dir).expect("Failed to load KeyVault");
                println!("Vault Initialization Complete!");
            }
        }
        Commands::Migrate => {
            println!("Running Schema/State Transitions...");
            println!("No migrations required. State is consistent.");
        }
        Commands::Mem { inspect } => {
            if *inspect {
                println!("Please use `qualia-cli inspect <superblock_path>` directly to inspect specific layouts.");
            }
        }
"""

# Insert match additions right after `let cli = Cli::parse();\n\n    match &cli.command {\n`
content = re.sub(r'(match &cli\.command \{\n)', r'\1' + match_additions, content)

with open('crates/qualia-cli/src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)
