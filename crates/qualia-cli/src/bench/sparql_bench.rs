use qualia_core_db::sparql_library::sparql_executor::QueryExecutor;
use qualia_core_db::sparql_library::sparql_parser::parse_sparql;
use qualia_core_db::sparql_library::sparql_planner::QueryPlanner;
use qualia_core_db::NQuin;
use std::path::Path;
use std::time::Instant;

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
    println!(
        "Initializing Memory-Mapped Volume: {:?}",
        database_volume_path
    );
    let start_init = Instant::now();

    // The resident executor remains a benchmark compatibility path, but it
    // opens a front-manifested logical root as one graph.
    let all_quins: Vec<NQuin> = qualia_core_db::q42_reader::read_q42_quins(database_volume_path)
        .map_err(|e| BenchError::Format(format!("Failed to open Q42 volume: {e}")))?;

    println!(
        "Volume loaded into memory: {} quins in {} μs",
        all_quins.len(),
        start_init.elapsed().as_micros()
    );

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
    println!(
        "Query compiled in: {} μs",
        parse_timer.elapsed().as_micros()
    );

    let query_timer = Instant::now();
    let executor = QueryExecutor::new(&all_quins);
    let results = executor
        .execute(&plan, &ctx)
        .map_err(|e| BenchError::Format(e))?;
    let latency = query_timer.elapsed();

    println!(
        "Query execution completed safely in: {} μs",
        latency.as_micros()
    );
    println!("Found {} results.", results.len());
    Ok(())
}
