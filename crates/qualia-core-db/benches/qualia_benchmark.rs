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

fn bench_cbor_compiler(c: &mut Criterion) {
    use qualia_core_db::cbor_compiler::parse_cbor_ld_to_quin;

    // CBOR Array of 4 integers: [1000, 2000, 3000, 4000]
    let cbor_payload: [u8; 13] = [
        0x84, 0x19, 0x03, 0xE8, 0x19, 0x07, 0xD0, 0x19, 0x0B, 0xB8, 0x19, 0x0F, 0xA0
    ];

    c.bench_function("qualia_cbor_ld_ingestion", |b| {
        b.iter(|| {
            let quin = parse_cbor_ld_to_quin(black_box(&cbor_payload)).unwrap();
            black_box(quin);
        })
    });
}

criterion_group!(benches, bench_quin_allocation, bench_query_compiler, bench_ingestion_pipeline, bench_cbor_compiler);
criterion_main!(benches);
