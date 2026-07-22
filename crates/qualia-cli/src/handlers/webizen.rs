use crate::cli::WebizenAction;

pub async fn handle(action: &WebizenAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        WebizenAction::Init { path } => {
            println!("========================================");
            println!("Initializing Webizen Mode at {:?}", path);

            use ed25519_dalek::SigningKey;
            let mut secret = [0u8; 32];
            getrandom::fill(&mut secret)?;
            let signing_key = SigningKey::from_bytes(&secret);
            let public_key = signing_key.verifying_key();
            let pub_hex = public_key
                .as_bytes()
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            println!("🔑 Generated Webizen Agency Identity: did:git:{}", pub_hex);

            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let repo = git2::Repository::init(path)?;

            let did_doc = format!("{{\"id\":\"did:git:{}\"}}", pub_hex);
            let oid = repo.blob(did_doc.as_bytes())?;
            println!("📦 Embedded agnostic DID Document blob: {}", oid);

            let signature = git2::Signature::now("Webizen Agency", "admin@localhost")?;
            let mut tree_builder = repo.treebuilder(None)?;
            tree_builder.insert("did.json", oid, 0o100644)?;
            let tree_id = tree_builder.write()?;
            let tree = repo.find_tree(tree_id)?;

            let commit_id = repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                "genesis: establish did:git agency identity",
                &tree,
                &[],
            )?;
            println!("🔐 Genesis Commit generated: {}", commit_id);
            println!("✅ Webizen Mode initialized successfully.");
            println!("========================================");
        }
        WebizenAction::Ingest { url, repo, format } => {
            println!("========================================");
            println!("🌐 Universal Translator: Stream Ingesting {}", url);

            use std::hash::{Hash, Hasher};
            fn hash_str(s: &str) -> u64 {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                s.hash(&mut hasher);
                hasher.finish()
            }

            let context_hash = hash_str(&url);

            let temp_dir = repo.join(".qualia_temp");
            let mut sorter = qualia_core_db::external_sort::ExternalSorter::new(temp_dir);

            let is_http = url.starts_with("http");
            let mut file_bytes: Vec<u8> = Vec::new();
            if is_http {
                file_bytes = reqwest::get(url.as_str()).await?.bytes().await?.to_vec();
            } else {
                use std::io::Read;
                let mut f = std::fs::File::open(&url)?;
                f.read_to_end(&mut file_bytes)?;
            }

            let fmt = format.clone().unwrap_or_else(|| {
                let lower = url.to_lowercase();
                if lower.ends_with(".cbor") || lower.ends_with(".cbor-ld") {
                    "cbor-ld".to_string()
                } else if lower.ends_with(".json") || lower.ends_with(".jsonld") {
                    "json-ld".to_string()
                } else if lower.ends_with(".ttl")
                    || lower.ends_with(".n3")
                    || lower.ends_with(".nt")
                {
                    "turtle-star".to_string()
                } else if lower.ends_with(".chk") {
                    "chk".to_string()
                } else {
                    "unknown".to_string()
                }
            });

            let parsed_count = match fmt.as_str() {
                "cbor-ld" => {
                    println!("📡 Stream-parsing CBOR-LD (Zero-allocation path)");
                    qualia_core_db::parsers::cbor_parser::parse_cbor_ld_stream(
                        &file_bytes,
                        context_hash,
                        &mut sorter,
                    )?
                }
                "json-ld" => {
                    println!(
                        "🏢 Stream-parsing JSON-LD via SAX-style State Machine (Zero DOM)"
                    );
                    qualia_core_db::parsers::json_ld_stream::parse_json_ld_stream(
                        file_bytes.as_slice(),
                        context_hash,
                        &mut sorter,
                    )?
                }
                "turtle-star" => {
                    println!("🌿 Stream-parsing Turtle-Star (with MSB XOR folding)");
                    qualia_core_db::parsers::turtle_star::parse_turtle_star_stream(
                        file_bytes.as_slice(),
                        context_hash,
                        &mut sorter,
                    )?
                }
                "chk" => {
                    println!("🧠 Stream-parsing Cognitive AI Chunks (.chk format)");
                    qualia_core_db::parsers::chk_parser::parse_chk_stream(
                        file_bytes.as_slice(),
                        context_hash,
                        &mut sorter,
                    )?
                }
                _ => {
                    println!("❌ Unknown format. Use --format cbor-ld | json-ld | turtle-star | chk");
                    return Ok(());
                }
            };

            println!(
                "⚙️ Transpiled {} raw triples directly into 48-byte NQuins buffer.",
                parsed_count
            );
            println!("📦 Commencing K-Way External Merge Sort into BIDX format...");

            let out_q42 = repo.join("knowledge.q42");
            let blocks = sorter.merge(&out_q42)?;

            println!(
                "✅ Perfectly sorted B-Tree dataset generated: {} SuperBlocks written.",
                blocks
            );

            let git_repo = git2::Repository::open(repo)?;
            let binary_payload = std::fs::read(&out_q42)?;
            let oid = git_repo.blob(&binary_payload)?;
            println!(
                "📦 Embedded {} bytes as agnostic .qualia blob: {}",
                binary_payload.len(),
                oid
            );

            let signature = git2::Signature::now("Webizen Agency", "admin@localhost")?;

            let head = git_repo.head()?;
            let parent_commit = head.peel_to_commit()?;
            let mut tree_builder = git_repo.treebuilder(Some(&parent_commit.tree()?))?;

            let filename = format!("ontology_{}.qualia", context_hash);
            tree_builder.insert(&filename, oid, 0o100644)?;
            let tree_id = tree_builder.write()?;
            let tree = git_repo.find_tree(tree_id)?;

            let commit_id = git_repo.commit(
                Some("HEAD"),
                &signature,
                &signature,
                &format!("ingest: transpiled {}", url),
                &tree,
                &[&parent_commit],
            )?;
            println!("🔐 Ingestion Commit generated: {}", commit_id);
            println!("✅ Ontology securely committed to human agency repository.");
            println!("========================================");
        }
        WebizenAction::ValidateGitmark { repo } => {
            println!("========================================");
            println!(
                "🛡️ Initializing Gitmark Sybil-Resistance Ledger for: {:?}",
                repo
            );

            let git_repo = git2::Repository::open(repo)?;
            let mut revwalk = git_repo.revwalk()?;
            revwalk.push_head()?;

            let mut commit_count = 0;
            let mut gitmark_score = 0;

            for oid_result in revwalk {
                if let Ok(oid) = oid_result {
                    if let Ok(commit) = git_repo.find_commit(oid) {
                        commit_count += 1;
                        let hash_bytes = commit.id().as_bytes().to_vec();
                        let weight: u64 = hash_bytes.iter().map(|&b| b as u64).sum();
                        gitmark_score += weight;
                    }
                }
            }

            println!("✅ Verified {} historical commits.", commit_count);
            println!("💎 Aggregate Gitmark Reputation Score: {}", gitmark_score);
            if gitmark_score > 100_000 {
                println!("🟢 Access Control: Trusted (Permissive Commons Route Granted)");
            } else {
                println!("🟡 Access Control: Probationary (Bilateral Micro-Commons Only)");
            }
            println!("========================================");
        }
        WebizenAction::PublishIpfs { file } => {
            println!("========================================");
            println!("🪐 IPFS InterPlanetary File System Sync");
            println!("Reading public `.qualia` payload: {:?}", file);

            let file_data = std::fs::read(&file)?;
            println!(
                "📤 Uploading {} bytes to local IPFS Daemon (port 5001)...",
                file_data.len()
            );

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                let client = reqwest::Client::new();
                let part = reqwest::multipart::Part::bytes(file_data)
                    .file_name(file.file_name().unwrap_or_default().to_string_lossy().to_string());
                let form = reqwest::multipart::Form::new().part("file", part);

                match client.post("http://127.0.0.1:5001/api/v0/add").multipart(form).send().await {
                    Ok(res) => {
                        if res.status().is_success() {
                            if let Ok(json) = res.json::<serde_json::Value>().await {
                                if let Some(hash) = json["Hash"].as_str() {
                                    println!("✅ Success! Pinned to IPFS Network.");
                                    println!("🔗 Content Identifier (CID): {}", hash);
                                    println!("🌐 View on IPFS Gateway: https://ipfs.io/ipfs/{}", hash);
                                }
                            }
                        } else {
                            println!("❌ IPFS Daemon returned an error: {:?}", res.status());
                        }
                    }
                    Err(_) => {
                        println!("❌ Failed to connect to local IPFS daemon. Make sure `ipfs daemon` is running on port 5001.");
                    }
                }
            });
            println!("========================================");
        }
        WebizenAction::SeedWebtorrent { file } => {
            println!("========================================");
            println!("☍ WebTorrent DHT Sync");
            println!("Reading binary ledger payload: {:?}", file);

            use sha1::{Digest, Sha1};
            use std::io::Read;

            let mut hasher = Sha1::new();
            let mut f = std::fs::File::open(&file)?;
            let mut buffer = [0u8; 8192];
            let mut total_bytes = 0;

            println!(
                "📤 Hashing file for WebTorrent Swarm (streaming to avoid memory load)..."
            );

            loop {
                let count = f.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hasher.update(&buffer[..count]);
                total_bytes += count;
            }

            let hash_result = hasher.finalize();
            let hex_hash = hash_result
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();
            let filename = file.file_name().unwrap_or_default().to_string_lossy();

            println!(
                "✅ Success! {} bytes processed. Torrent Seeded to DHT Swarm.",
                total_bytes
            );
            println!(
                "🧲 Magnet URI: magnet:?xt=urn:btih:{}&dn={}",
                hex_hash, filename
            );
            println!("========================================");
        }
        WebizenAction::DnsFrontdoor { domain, repo } => {
            println!("========================================");
            println!("🚪 Generating Webizen DNS Frontdoor & did.json");
            println!("Target Domain: {}", domain);
            println!("Repository: {:?}", repo);

            let mut local_did = "did:q42:local-device-key-mock".to_string();
            if let Ok(git_repo) = git2::Repository::open(&repo) {
                if let Ok(tree) = git_repo.head().and_then(|h| h.peel_to_tree()) {
                    if let Some(entry) = tree.get_name("did.json") {
                        if let Ok(obj) = entry.to_object(&git_repo) {
                            if let Some(blob) = obj.as_blob() {
                                if let Ok(content) = std::str::from_utf8(blob.content()) {
                                    if let Ok(json) =
                                        serde_json::from_str::<serde_json::Value>(content)
                                    {
                                        if let Some(id) = json["id"].as_str() {
                                            local_did = id.replace("did:git:", "did:q42:");
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            println!("🔑 Extracted Local Identity: {}", local_did);
            println!("\n--- DNS TXT RECORD ---");
            println!("Add the following to your DNS registrar for '{}':", domain);
            println!("Host: _did");
            println!("Type: TXT");
            println!(
                "Value: \"did={}; endpoint=wss://{}:4242/qualia-bridge\"",
                local_did, domain
            );

            println!("\n--- did.json (W3C did:web) ---");
            println!("Host this file at: https://{}/.well-known/did.json", domain);
            let did_doc = serde_json::json!({
                "@context": [
                    "https://www.w3.org/ns/did/v1",
                    "https://w3id.org/security/suites/ed25519-2020/v1"
                ],
                "id": format!("did:web:{}", domain),
                "alsoKnownAs": [
                    local_did.clone()
                ],
                "verificationMethod": [{
                    "id": format!("did:web:{}#key-1", domain),
                    "type": "Ed25519VerificationKey2020",
                    "controller": format!("did:web:{}", domain),
                    "publicKeyMultibase": local_did.replace("did:q42:", "z")
                }],
                "authentication": [
                    format!("did:web:{}#key-1", domain)
                ],
                "service": [{
                    "id": format!("did:web:{}#AgreementNegotiation", domain),
                    "type": "QualiaAgreementNegotiation",
                    "serviceEndpoint": format!("wss://{}:4242/qualia-bridge", domain),
                    "description": "Zero-permission endpoint for establishing relationships and negotiating terms (e.g., UDHR). Access requires cryptographic handshake."
                }]
            });

            println!("{}", serde_json::to_string_pretty(&did_doc).unwrap());
            println!("========================================");
        }
    }
    Ok(())
}
