use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: RpcParams,
    pub id: u64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct RpcParams {
    pub query: Option<String>,
    pub token: Option<String>,
}

#[derive(Serialize)]
#[allow(dead_code)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub id: u64,
}

/// Execute a SPARQL (or SPARQL-Star) query string against a `.q42` vault.
///
/// Uses the core-db native pipeline:
///   `parse_sparql` → `QueryPlanner::plan` → memmap vault → `QueryExecutor::execute`
///
/// Results are printed as variable-slot → hash-value pairs. A reverse lexicon
/// lookup is not yet implemented; values are the FNV-1a hashes stored in the
/// vault and can be cross-referenced with the vault's embedded `.lex` section.
pub fn run_sparql_query(vault: &std::path::Path, query_str: &str) {
    use qualia_core_db::sparql_executor::QueryExecutor;
    use qualia_core_db::sparql_planner::QueryPlanner;

    let (query, ctx) = match qualia_core_db::sparql_parser::parse_sparql(query_str) {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("SPARQL parse error: {e}");
            return;
        }
    };

    let plan = match QueryPlanner::plan(&query, &ctx) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("SPARQL plan error: {e}");
            return;
        }
    };

    let quins = match qualia_core_db::q42_reader::read_q42_quins(vault) {
        Ok(quins) => quins,
        Err(e) => {
            eprintln!("Warning: could not open vault '{}': {e}", vault.display());
            Vec::new()
        }
    };
    let lex = qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(vault).ok();

    if quins.is_empty() {
        eprintln!(
            "Warning: vault '{}' is empty or could not be opened.",
            vault.display()
        );
    }

    let executor = QueryExecutor::new(&quins);
    match executor.execute(&plan, &ctx) {
        Err(e) => eprintln!("SPARQL execution error: {e}"),
        Ok(rows) => {
            println!("{} result(s) from {}", rows.len(), vault.display());
            for (i, row) in rows.iter().enumerate() {
                print!("[{i:>4}]");
                for (slot_idx, slot) in row.slots.iter().enumerate() {
                    if let Some(v) = slot {
                        match lex.as_ref().and_then(|l| l.lookup(*v)) {
                            Some(s) => print!("  ?v{slot_idx}={s}"),
                            None => print!("  ?v{slot_idx}=0x{v:016x}"),
                        }
                    }
                }
                println!();
            }
        }
    }
}
