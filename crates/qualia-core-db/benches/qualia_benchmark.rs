use criterion::{black_box, criterion_group, criterion_main, Criterion};
use qualia_core_db::query_compiler::QueryCompiler;
use qualia_core_db::QualiaQuin;

/// Benchmarks the raw memory allocation limit of our 48-byte mathematical primitives.
/// The engine is explicitly designed to handle hundreds of thousands of these instantaneously.
fn bench_quin_allocation(c: &mut Criterion) {
    c.bench_function("qualia_quin_allocation", |b| {
        b.iter(|| {
            let quin = QualiaQuin {
                subject: black_box(1),
                predicate: black_box(2),
                object: black_box(3),
                context: black_box(4),
                metadata: black_box(5),
                parity: black_box(0),
            };
            black_box(quin);
        })
    });
}

/// Benchmarks the zero-allocation state machine's capability to stream and parse 
/// GeoSPARQL-Star subsets directly into native hardware 64-bit opcodes.
fn bench_query_compiler(c: &mut Criterion) {
    let query = "<<?s qualia:location ?geo>> geof:distance 500";
    c.bench_function("qualia_query_compiler_geosparql", |b| {
        b.iter(|| {
            let quin = QueryCompiler::compile_to_quin(black_box(query));
            black_box(quin);
        })
    });
}

fn bench_ingestion_pipeline(c: &mut Criterion) {
    use qualia_core_db::ingestion::{IngestionPipeline, ZeroCopyStream};

    // Simulate 1,000 lines of mixed RDF-Star and N3Logic
    let mut payload = String::with_capacity(100_000);
    for i in 0..500 {
        payload.push_str(&format!("<< :Agent_{i} :prescribed :Meds_{i} >> :assertedBy :Doctor_0 .\n"));
        payload.push_str(&format!("{{ ?x a :Man_{i} }} => {{ ?x a :Mortal_{i} }} .\n"));
    }

    c.bench_function("qualia_ingestion_pipeline_1k_lines", |b| {
        b.iter(|| {
            let pipeline = IngestionPipeline::new(black_box(&payload));
            // Force iteration to consume and compile every line
            let count = pipeline.stream_parse().count();
            black_box(count);
        })
    });
}

criterion_group!(benches, bench_quin_allocation, bench_query_compiler, bench_ingestion_pipeline);
criterion_main!(benches);
