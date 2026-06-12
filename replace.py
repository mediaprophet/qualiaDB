import re
with open('crates/qualia-cli/src/ingest/mod.rs', 'r', encoding='utf-8') as f:
    content = f.read()

new_func = '''pub fn ingest_turtle_star(
    input: &std::path::Path,
    output: &std::path::Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let parent_dir = output.parent().unwrap_or(std::path::Path::new("."));
    let ingestor = pipeline::IncrementalIngestor::new(parent_dir, 256 * 1024 * 1024);
    ingestor.execute_stream_compilation(input, output).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    Ok(IngestStats {
        triples_ingested: 0,
        blocks_written: 0,
        lex_entries: 0,
        lines_skipped: 0,
        bidx_written: true,
    })
}'''

content = re.sub(r'pub fn ingest_turtle_star\([^\{]+?\{.*?\n}', new_func, content, flags=re.DOTALL)

# Also insert pub mod pipeline; near the top
content = content.replace('use std::path::Path;', 'use std::path::Path;\n\npub mod pipeline;\n\n#[derive(Debug)]\npub enum IngestError {\n    Io(std::io::Error),\n    Other(String),\n}\n\nimpl std::fmt::Display for IngestError {\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Formatter<'_> { todo!() }\n}\nimpl std::error::Error for IngestError {}\n')

with open('crates/qualia-cli/src/ingest/mod.rs', 'w', encoding='utf-8') as f:
    f.write(content)
