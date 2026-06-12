use std::path::Path;
use std::time::Instant;
use qualia_core_db::q42_volume::{Q42Volume, SUPERBLOCK_SIZE, SUPERBLOCK_HEADER, QUIN_SIZE};
use qualia_core_db::sparql_library::sparql_parser::parse_sparql;
use qualia_core_db::sparql_library::sparql_planner::QueryPlanner;
use qualia_core_db::sparql_library::sparql_executor::QueryExecutor;
use qualia_core_db::NQuin;

#[derive(Debug)]
pub enum BenchError {
    Format(String),
}

impl std::fmt::Display for BenchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BenchError::Format(s) => write!(f, "{}", s),
        }
    }
}
impl std::error::Error for BenchError {}

pub fn run_sparql_suite(database_volume_path: &Path) -> Result<(), BenchError> {
    println!("Initializing Memory-Mapped Volume: {:?}", database_volume_path);
    let start_init = Instant::now();
    
    // Core engine executes entirely out-of-core via pointers
    // We decompress into RAM for the executor which currently relies on &[NQuin].
    let volume = Q42Volume::open(database_volume_path)
        .map_err(|e| BenchError::Format(format!("Failed to open Q42 volume: {}", e)))?;
    
    let mut all_quins = Vec::new();
    let mut sb_buf = vec![0u8; SUPERBLOCK_SIZE];
    for i in 0..volume.block_count() as usize {
        let _ = volume.read_superblock_into(i, &mut sb_buf)
            .map_err(|e| BenchError::Format(format!("Failed to read superblock {}: {}", i, e)))?;
        let quin_count = u64::from_le_bytes(sb_buf[16..24].try_into().unwrap()) as usize;
        let mut off = SUPERBLOCK_HEADER;
        for _ in 0..quin_count {
            let subject = u64::from_le_bytes(sb_buf[off..off+8].try_into().unwrap());
            let predicate = u64::from_le_bytes(sb_buf[off+8..off+16].try_into().unwrap());
            let object = u64::from_le_bytes(sb_buf[off+16..off+24].try_into().unwrap());
            let context = u64::from_le_bytes(sb_buf[off+24..off+32].try_into().unwrap());
            let metadata = u64::from_le_bytes(sb_buf[off+32..off+40].try_into().unwrap());
            let parity = u64::from_le_bytes(sb_buf[off+40..off+48].try_into().unwrap());
            all_quins.push(NQuin { subject, predicate, object, context, metadata, parity });
            off += QUIN_SIZE;
        }
    }
    
    println!("Volume loaded into memory: {} quins in {} μs", all_quins.len(), start_init.elapsed().as_micros());
    
    let query_point_lookup = "
        PREFIX yago: <http://yago-knowledge.org/resource/>
        SELECT ?birthPlace ?date
        WHERE {
            yago:Albert_Einstein yago:wasBornIn ?birthPlace .
            <<yago:Albert_Einstein yago:wasBornIn ?birthPlace>> yago:occurredOnDate ?date .
        }
    ";

    println!("Running SPARQL-star Point Lookup Benchmark...");
    
    let parse_timer = Instant::now();
    let (ast, ctx) = parse_sparql(query_point_lookup).map_err(|e| BenchError::Format(e))?;
    let plan = QueryPlanner::plan(&ast, &ctx).map_err(|e| BenchError::Format(e))?;
    println!("Query compiled in: {} μs", parse_timer.elapsed().as_micros());
    
    let query_timer = Instant::now();
    let executor = QueryExecutor::new(&all_quins);
    let results = executor.execute(&plan, &ctx).map_err(|e| BenchError::Format(e))?;
    let latency = query_timer.elapsed();

    println!("Query execution completed safely in: {} μs", latency.as_micros());
    println!("Found {} results.", results.len());
    Ok(())
}
