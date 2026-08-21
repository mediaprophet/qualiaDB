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
/// Range operators are tried first. Vaults larger than
/// [`RESIDENT_QUERY_MAX_BYTES`] never call `read_q42_quins` / `read_all_quins`;
/// they fail if no range plan applies. Smaller files may still use the
/// resident executor.
pub fn run_sparql_query(vault: &std::path::Path, query_str: &str) {
    use qualia_core_db::q42_volume::{Q42QueryMode, RESIDENT_QUERY_MAX_BYTES};
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

    let file_bytes = std::fs::metadata(vault).map(|m| m.len()).unwrap_or(0);
    let mode = Q42QueryMode::for_file_bytes(file_bytes);

    match try_range_query(vault, &plan, &ctx) {
        Ok(()) => return,
        Err(RangeDispatch::NoPlan(reason)) => {
            if !mode.allows_read_all_quins() {
                eprintln!(
                    "Range SPARQL required: vault is {file_bytes} bytes (limit {RESIDENT_QUERY_MAX_BYTES}); no range plan applies ({reason}). Resident fallback to read_q42_quins is forbidden."
                );
                return;
            }
            eprintln!("Range SPARQL unavailable; using resident compatibility executor: {reason}");
        }
        Err(RangeDispatch::Failed(error)) => {
            if !mode.allows_read_all_quins() {
                eprintln!(
                    "Range SPARQL failed; resident fallback forbidden ({file_bytes} bytes > {RESIDENT_QUERY_MAX_BYTES}): {error}"
                );
                return;
            }
            eprintln!("Range SPARQL unavailable; using resident compatibility executor: {error}");
        }
    }

    if !mode.allows_read_all_quins() {
        eprintln!(
            "Range SPARQL required: vault is {file_bytes} bytes (limit {RESIDENT_QUERY_MAX_BYTES}); refusing read_q42_quins / read_all_quins."
        );
        return;
    }

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

enum RangeDispatch {
    NoPlan(String),
    Failed(String),
}

fn try_range_query(
    vault: &std::path::Path,
    plan: &qualia_core_db::sparql_planner::ExecutionPlan,
    ctx: &qualia_core_db::sparql_ast::SparqlQueryContext,
) -> Result<(), RangeDispatch> {
    if let Ok(range_plan) = qualia_core_db::Q42RangeOptionalPlan::from_execution_plan(plan) {
        return run_range_optional_query(vault, range_plan, ctx).map_err(RangeDispatch::Failed);
    }
    if let Ok(range_plan) = qualia_core_db::Q42RangeUnionPlan::from_execution_plan(plan) {
        return run_range_union_query(vault, range_plan, ctx).map_err(RangeDispatch::Failed);
    }
    if let Ok(range_plan) = qualia_core_db::Q42RangeBindPlan::from_execution_plan(plan) {
        return run_range_bind_query(vault, range_plan, ctx).map_err(RangeDispatch::Failed);
    }
    if let Ok(range_plan) = qualia_core_db::Q42RangeHashJoinPlan::from_execution_plan(plan) {
        match run_range_hash_join_query(vault, range_plan, ctx) {
            Ok(()) => return Ok(()),
            Err(error) if error.contains("planner must keep NestedLoopJoin") => {
                return run_range_join_select_query(vault, range_plan.as_join_select(), ctx)
                    .map_err(RangeDispatch::Failed);
            }
            Err(error) => return Err(RangeDispatch::Failed(error)),
        }
    }
    if let Ok(range_plan) = qualia_core_db::Q42RangeJoinSelectPlan::from_execution_plan(plan) {
        return run_range_join_select_query(vault, range_plan, ctx).map_err(RangeDispatch::Failed);
    }
    if let Ok(range_plan) =
        qualia_core_db::sparql_executor::Q42RangeTripleSelectPlan::from_execution_plan(plan)
    {
        return run_range_triple_query(vault, range_plan, ctx).map_err(RangeDispatch::Failed);
    }
    if let Ok(range_plan) =
        qualia_core_db::sparql_executor::Q42RangeNestedLoopJoinPlan::from_execution_plan(plan)
    {
        return run_range_join_query(vault, range_plan, ctx).map_err(RangeDispatch::Failed);
    }
    Err(RangeDispatch::NoPlan(
        "query is not a range-executable TripleScan, join, OPTIONAL, UNION, or BIND".into(),
    ))
}

fn run_range_triple_query(
    vault: &std::path::Path,
    plan: qualia_core_db::sparql_executor::Q42RangeTripleSelectPlan,
    ctx: &qualia_core_db::sparql_ast::SparqlQueryContext,
) -> Result<(), String> {
    use qualia_core_db::q42_volume::{
        LocalFileRangeSource, Q42RangeVolume, Q42RangeVolumeSet, Q42VolumeSegment,
    };
    use qualia_core_db::sparql_ast::BindingRow;
    use qualia_core_db::sparql_executor::{
        execute_range_triple_select_page_into, execute_range_volume_set_triple_select_page_into,
        Q42RangeTripleSelectCursor, Q42RangeVolumeSetTripleSelectCursor,
    };

    const PAGE_ROWS: usize = 128;
    let source = LocalFileRangeSource::open(vault).map_err(|error| error.to_string())?;
    let root = Q42RangeVolume::open(source).map_err(|error| error.to_string())?;
    let lexicon = qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(vault).ok();
    let mut compressed = [0u8; qualia_core_db::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = [0u8; qualia_core_db::q42_volume::SUPERBLOCK_SIZE];
    let mut quins = [qualia_core_db::NQuin::default(); PAGE_ROWS];
    let mut rows = [BindingRow::default(); PAGE_ROWS];
    let mut total = 0usize;

    if root
        .volume_manifest_length()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        let parent = vault.parent().unwrap_or_else(|| std::path::Path::new("."));
        let factory =
            |entry: &Q42VolumeSegment| LocalFileRangeSource::open(&parent.join(&entry.locator));
        let mut set =
            Q42RangeVolumeSet::open_root(&root, &factory).map_err(|error| error.to_string())?;
        set.attach_lexicon_segments(&|entry| {
            LocalFileRangeSource::open(&parent.join(&entry.locator))
        })
        .map_err(|error| error.to_string())?;
        let mut cursor = Q42RangeVolumeSetTripleSelectCursor::default();
        loop {
            let page = execute_range_volume_set_triple_select_page_into(
                &set,
                plan,
                ctx,
                cursor,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            let Some(next) = page.next_cursor else {
                break;
            };
            cursor = next;
        }
    } else {
        let mut cursor = Q42RangeTripleSelectCursor::default();
        loop {
            let page = execute_range_triple_select_page_into(
                &root,
                plan,
                ctx,
                cursor,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            let Some(next) = page.next_cursor else {
                break;
            };
            cursor = next;
        }
    }
    println!("{total} result(s) from {} (range-backed)", vault.display());
    Ok(())
}

/// Execute a two-pattern nested-loop join directly from physical SuperBlocks.
/// The supported plan deliberately excludes project/filter/limit wrappers: the
/// resident executor remains authoritative for those until their range-native
/// equivalents can preserve every SPARQL semantic.
fn run_range_join_query(
    vault: &std::path::Path,
    plan: qualia_core_db::sparql_executor::Q42RangeNestedLoopJoinPlan,
    ctx: &qualia_core_db::sparql_ast::SparqlQueryContext,
) -> Result<(), String> {
    use qualia_core_db::q42_volume::{
        LocalFileRangeSource, Q42RangeVolume, Q42RangeVolumeSet, Q42VolumeSegment,
    };
    use qualia_core_db::sparql_ast::BindingRow;
    use qualia_core_db::sparql_executor::{
        execute_range_nested_loop_join_page_into,
        execute_range_volume_set_nested_loop_join_page_into, Q42RangeNestedLoopJoinState,
        Q42RangeVolumeSetNestedLoopJoinState,
    };

    const PAGE_ROWS: usize = 128;
    let source = LocalFileRangeSource::open(vault).map_err(|error| error.to_string())?;
    let root = Q42RangeVolume::open(source).map_err(|error| error.to_string())?;
    let lexicon = qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(vault).ok();
    let mut compressed = [0u8; qualia_core_db::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = [0u8; qualia_core_db::q42_volume::SUPERBLOCK_SIZE];
    let mut quins = [qualia_core_db::NQuin::default(); PAGE_ROWS];
    let mut left_rows = [BindingRow::default(); PAGE_ROWS];
    let mut right_rows = [BindingRow::default(); PAGE_ROWS];
    let mut rows = [BindingRow::default(); PAGE_ROWS];
    let mut total = 0usize;

    if root
        .volume_manifest_length()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        let parent = vault.parent().unwrap_or_else(|| std::path::Path::new("."));
        let factory =
            |entry: &Q42VolumeSegment| LocalFileRangeSource::open(&parent.join(&entry.locator));
        let volumes =
            Q42RangeVolumeSet::open_root(&root, &factory).map_err(|error| error.to_string())?;
        let mut state = Q42RangeVolumeSetNestedLoopJoinState::default();
        loop {
            let page = execute_range_volume_set_nested_loop_join_page_into(
                &volumes,
                plan,
                ctx,
                &mut state,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut left_rows,
                &mut right_rows,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    } else {
        let mut state = Q42RangeNestedLoopJoinState::default();
        loop {
            let page = execute_range_nested_loop_join_page_into(
                &root,
                plan,
                ctx,
                &mut state,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut left_rows,
                &mut right_rows,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    }
    println!(
        "{total} result(s) from {} (range-backed join)",
        vault.display()
    );
    Ok(())
}

fn run_range_join_select_query(
    vault: &std::path::Path,
    plan: qualia_core_db::Q42RangeJoinSelectPlan,
    ctx: &qualia_core_db::sparql_ast::SparqlQueryContext,
) -> Result<(), String> {
    use qualia_core_db::q42_volume::{
        LocalFileRangeSource, Q42RangeVolume, Q42RangeVolumeSet, Q42VolumeSegment,
    };
    use qualia_core_db::sparql_ast::BindingRow;
    use qualia_core_db::{
        execute_range_join_select_page_into, execute_range_volume_set_join_select_page_into,
        Q42RangeJoinSelectState, Q42RangeVolumeSetJoinSelectState,
    };

    const PAGE_ROWS: usize = 128;
    let source = LocalFileRangeSource::open(vault).map_err(|error| error.to_string())?;
    let root = Q42RangeVolume::open(source).map_err(|error| error.to_string())?;
    let lexicon = qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(vault).ok();
    let mut compressed = [0u8; qualia_core_db::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = [0u8; qualia_core_db::q42_volume::SUPERBLOCK_SIZE];
    let mut quins = [qualia_core_db::NQuin::default(); PAGE_ROWS];
    let mut left_rows = [BindingRow::default(); PAGE_ROWS];
    let mut right_rows = [BindingRow::default(); PAGE_ROWS];
    let mut join_out = [BindingRow::default(); PAGE_ROWS];
    let mut rows = [BindingRow::default(); PAGE_ROWS];
    let mut total = 0usize;

    if root
        .volume_manifest_length()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        let parent = vault.parent().unwrap_or_else(|| std::path::Path::new("."));
        let factory =
            |entry: &Q42VolumeSegment| LocalFileRangeSource::open(&parent.join(&entry.locator));
        let volumes =
            Q42RangeVolumeSet::open_root(&root, &factory).map_err(|error| error.to_string())?;
        let mut state = Q42RangeVolumeSetJoinSelectState::default();
        loop {
            let page = execute_range_volume_set_join_select_page_into(
                &volumes,
                plan,
                ctx,
                &mut state,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut left_rows,
                &mut right_rows,
                &mut join_out,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    } else {
        let mut state = Q42RangeJoinSelectState::default();
        loop {
            let page = execute_range_join_select_page_into(
                &root,
                plan,
                ctx,
                &mut state,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut left_rows,
                &mut right_rows,
                &mut join_out,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    }
    println!(
        "{total} result(s) from {} (range-backed join-select)",
        vault.display()
    );
    Ok(())
}

fn run_range_optional_query(
    vault: &std::path::Path,
    plan: qualia_core_db::Q42RangeOptionalPlan,
    ctx: &qualia_core_db::sparql_ast::SparqlQueryContext,
) -> Result<(), String> {
    use qualia_core_db::q42_volume::{
        LocalFileRangeSource, Q42RangeVolume, Q42RangeVolumeSet, Q42VolumeSegment,
    };
    use qualia_core_db::sparql_ast::BindingRow;
    use qualia_core_db::{
        execute_range_optional_page_into, execute_range_volume_set_optional_page_into,
        Q42RangeOptionalState, Q42RangeVolumeSetOptionalState,
    };

    const PAGE_ROWS: usize = 128;
    let source = LocalFileRangeSource::open(vault).map_err(|error| error.to_string())?;
    let root = Q42RangeVolume::open(source).map_err(|error| error.to_string())?;
    let lexicon = qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(vault).ok();
    let mut compressed = [0u8; qualia_core_db::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = [0u8; qualia_core_db::q42_volume::SUPERBLOCK_SIZE];
    let mut quins = [qualia_core_db::NQuin::default(); PAGE_ROWS];
    let mut left_rows = [BindingRow::default(); PAGE_ROWS];
    let mut right_rows = [BindingRow::default(); PAGE_ROWS];
    let mut join_out = [BindingRow::default(); PAGE_ROWS];
    let mut rows = [BindingRow::default(); PAGE_ROWS];
    let mut total = 0usize;

    if root
        .volume_manifest_length()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        let parent = vault.parent().unwrap_or_else(|| std::path::Path::new("."));
        let factory =
            |entry: &Q42VolumeSegment| LocalFileRangeSource::open(&parent.join(&entry.locator));
        let volumes =
            Q42RangeVolumeSet::open_root(&root, &factory).map_err(|error| error.to_string())?;
        let mut state = Q42RangeVolumeSetOptionalState::default();
        loop {
            let page = execute_range_volume_set_optional_page_into(
                &volumes,
                plan,
                ctx,
                &mut state,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut left_rows,
                &mut right_rows,
                &mut join_out,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    } else {
        let mut state = Q42RangeOptionalState::default();
        loop {
            let page = execute_range_optional_page_into(
                &root,
                plan,
                ctx,
                &mut state,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut left_rows,
                &mut right_rows,
                &mut join_out,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    }
    println!(
        "{total} result(s) from {} (range-backed optional)",
        vault.display()
    );
    Ok(())
}

fn run_range_union_query(
    vault: &std::path::Path,
    plan: qualia_core_db::Q42RangeUnionPlan,
    ctx: &qualia_core_db::sparql_ast::SparqlQueryContext,
) -> Result<(), String> {
    use qualia_core_db::q42_volume::{
        LocalFileRangeSource, Q42RangeVolume, Q42RangeVolumeSet, Q42VolumeSegment,
    };
    use qualia_core_db::sparql_ast::BindingRow;
    use qualia_core_db::{
        execute_range_union_page_into, execute_range_volume_set_union_page_into,
        Q42RangeUnionState, Q42RangeVolumeSetUnionState,
    };

    const PAGE_ROWS: usize = 128;
    let source = LocalFileRangeSource::open(vault).map_err(|error| error.to_string())?;
    let root = Q42RangeVolume::open(source).map_err(|error| error.to_string())?;
    let lexicon = qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(vault).ok();
    let mut compressed = [0u8; qualia_core_db::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = [0u8; qualia_core_db::q42_volume::SUPERBLOCK_SIZE];
    let mut quins = [qualia_core_db::NQuin::default(); PAGE_ROWS];
    let mut raw = [BindingRow::default(); PAGE_ROWS];
    let mut rows = [BindingRow::default(); PAGE_ROWS];
    let mut total = 0usize;

    if root
        .volume_manifest_length()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        let parent = vault.parent().unwrap_or_else(|| std::path::Path::new("."));
        let factory =
            |entry: &Q42VolumeSegment| LocalFileRangeSource::open(&parent.join(&entry.locator));
        let volumes =
            Q42RangeVolumeSet::open_root(&root, &factory).map_err(|error| error.to_string())?;
        let mut state = Q42RangeVolumeSetUnionState::default();
        loop {
            let page = execute_range_volume_set_union_page_into(
                &volumes,
                plan,
                ctx,
                &mut state,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut raw,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    } else {
        let mut state = Q42RangeUnionState::default();
        loop {
            let page = execute_range_union_page_into(
                &root,
                plan,
                ctx,
                &mut state,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut raw,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    }
    println!(
        "{total} result(s) from {} (range-backed union)",
        vault.display()
    );
    Ok(())
}

fn run_range_bind_query(
    vault: &std::path::Path,
    plan: qualia_core_db::Q42RangeBindPlan,
    ctx: &qualia_core_db::sparql_ast::SparqlQueryContext,
) -> Result<(), String> {
    use qualia_core_db::q42_volume::{
        LocalFileRangeSource, Q42RangeVolume, Q42RangeVolumeSet, Q42VolumeSegment,
    };
    use qualia_core_db::sparql_ast::BindingRow;
    use qualia_core_db::{
        execute_range_bind_page_into, execute_range_volume_set_bind_page_into, Q42RangeBindState,
        Q42RangeVolumeSetBindState,
    };

    const PAGE_ROWS: usize = 128;
    let source = LocalFileRangeSource::open(vault).map_err(|error| error.to_string())?;
    let root = Q42RangeVolume::open(source).map_err(|error| error.to_string())?;
    let lexicon = qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(vault).ok();
    let mut compressed = [0u8; qualia_core_db::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = [0u8; qualia_core_db::q42_volume::SUPERBLOCK_SIZE];
    let mut quins = [qualia_core_db::NQuin::default(); PAGE_ROWS];
    let mut raw = [BindingRow::default(); PAGE_ROWS];
    let mut rows = [BindingRow::default(); PAGE_ROWS];
    let mut total = 0usize;

    if root
        .volume_manifest_length()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        let parent = vault.parent().unwrap_or_else(|| std::path::Path::new("."));
        let factory =
            |entry: &Q42VolumeSegment| LocalFileRangeSource::open(&parent.join(&entry.locator));
        let volumes =
            Q42RangeVolumeSet::open_root(&root, &factory).map_err(|error| error.to_string())?;
        let mut state = Q42RangeVolumeSetBindState::default();
        loop {
            let page = execute_range_volume_set_bind_page_into(
                &volumes,
                plan,
                ctx,
                &mut state,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut raw,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    } else {
        let mut state = Q42RangeBindState::default();
        loop {
            let page = execute_range_bind_page_into(
                &root,
                plan,
                ctx,
                &mut state,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut raw,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    }
    println!(
        "{total} result(s) from {} (range-backed bind)",
        vault.display()
    );
    Ok(())
}

fn run_range_hash_join_query(
    vault: &std::path::Path,
    plan: qualia_core_db::Q42RangeHashJoinPlan,
    ctx: &qualia_core_db::sparql_ast::SparqlQueryContext,
) -> Result<(), String> {
    use qualia_core_db::q42_volume::{
        LocalFileRangeSource, Q42RangeVolume, Q42RangeVolumeSet, Q42VolumeSegment,
    };
    use qualia_core_db::sparql_ast::BindingRow;
    use qualia_core_db::{
        execute_range_hash_join_page_into, execute_range_volume_set_hash_join_page_into,
        Q42RangeHashJoinSlot, Q42RangeHashJoinState, Q42RangeVolumeSetHashJoinState,
    };

    const PAGE_ROWS: usize = 128;
    const HASH_CAP: usize = 256;
    let source = LocalFileRangeSource::open(vault).map_err(|error| error.to_string())?;
    let root = Q42RangeVolume::open(source).map_err(|error| error.to_string())?;
    let lexicon = qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(vault).ok();
    let mut compressed = [0u8; qualia_core_db::q42_volume::MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = [0u8; qualia_core_db::q42_volume::SUPERBLOCK_SIZE];
    let mut quins = [qualia_core_db::NQuin::default(); PAGE_ROWS];
    let mut probe_rows = [BindingRow::default(); PAGE_ROWS];
    let mut join_out = [BindingRow::default(); PAGE_ROWS];
    let mut rows = [BindingRow::default(); PAGE_ROWS];
    let mut table = [Q42RangeHashJoinSlot::default(); HASH_CAP];
    let mut total = 0usize;

    if root
        .volume_manifest_length()
        .map_err(|error| error.to_string())?
        .is_some()
    {
        let parent = vault.parent().unwrap_or_else(|| std::path::Path::new("."));
        let factory =
            |entry: &Q42VolumeSegment| LocalFileRangeSource::open(&parent.join(&entry.locator));
        let volumes =
            Q42RangeVolumeSet::open_root(&root, &factory).map_err(|error| error.to_string())?;
        let mut state = Q42RangeVolumeSetHashJoinState::default();
        loop {
            let page = execute_range_volume_set_hash_join_page_into(
                &volumes,
                plan,
                ctx,
                &mut state,
                &mut table,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut probe_rows,
                &mut join_out,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    } else {
        let mut state = Q42RangeHashJoinState::default();
        loop {
            let page = execute_range_hash_join_page_into(
                &root,
                plan,
                ctx,
                &mut state,
                &mut table,
                &mut compressed,
                &mut decoded,
                &mut quins,
                &mut probe_rows,
                &mut join_out,
                &mut rows,
            )?;
            print_range_rows(&rows[..page.returned], lexicon.as_ref());
            total += page.returned;
            if page.done {
                break;
            }
        }
    }
    println!(
        "{total} result(s) from {} (range-backed hash-join)",
        vault.display()
    );
    Ok(())
}

fn print_range_rows(
    rows: &[qualia_core_db::sparql_ast::BindingRow],
    lexicon: Option<&qualia_core_db::q42_lex::Q42Lexicon>,
) {
    for row in rows {
        print!("[range]");
        for (slot, value) in row.slots.iter().enumerate() {
            if let Some(value) = value {
                match lexicon.and_then(|lexicon| lexicon.lookup(*value)) {
                    Some(text) => print!("  ?v{slot}={text}"),
                    None => print!("  ?v{slot}=0x{value:016x}"),
                }
            }
        }
        println!();
    }
}
