# Getting Started with QualiaDB

**Version:** 0.0.33
**Last Updated:** 2026-08-15

This guide will help you get up and running with QualiaDB, the human-centric semantic engine designed for personal devices and edge computing.

---

## Quick Start

### Installation

#### Option 1: Download Pre-built Binaries

1. Visit the [GitHub Releases page](https://github.com/mediaprophet/qualiaDB/releases)
2. Download the appropriate binary for your platform:
   - Windows: `qualia-windows-x64.exe`
   - macOS (Intel): `qualia-macos-x64`
   - macOS (Apple Silicon): `qualia-macos-arm64`
   - Linux: `qualia-linux-x64`
3. Make the binary executable (Linux/macOS):
   ```bash
   chmod +x qualia
   ```

#### Option 2: Build from Source

```bash
# Clone the repository
git clone https://github.com/mediaprophet/qualiaDB.git
cd qualiaDB

# Build the CLI
cargo build --release -p qualia-cli

# The binary will be at ./target/release/qualia
```

### Verify Installation

```bash
qualia --help
```

You should see the command-line interface help output.

---

## Your First Query

### 1. Create a Simple Dataset

Create a file called `people.ttl` with the following Turtle RDF content:

```turtle
@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:alice a ex:Person ;
    ex:name "Alice Smith" ;
    ex:age 30 ;
    ex:knows ex:bob .

ex:bob a ex:Person ;
    ex:name "Bob Jones" ;
    ex:age 35 ;
    ex:knows ex:alice .
```

### 2. Ingest the Data

```bash
qualia ingest people.ttl people.q42
```

This converts the RDF data into a **unified Q42 v3** volume (magic `Q42\0`):
lexicon, BIDX, and LZ4 SuperBlocks are embedded. No `.q42.lex` / `.c.q42` sibling
is written.

```bash
qualia q42 inspect people.q42
qualia q42 verify people.q42
```

`inspect` reports header state (version, flags, block count, lexicon terms).
`verify` walks every SuperBlock and five-field ECC (`subject ^ predicate ^ object ^ context ^ metadata`).

### 3. Query the Data

```bash
qualia query people.q42
```

Enter a SPARQL query:

```sparql
SELECT ?name ?age WHERE {
  ?person a ex:Person ;
           ex:name ?name ;
           ex:age ?age .
}
```

---

## Running the Daemon

The QualiaDB daemon provides a REST API for applications to interact with your semantic database.

### Start the Daemon

```bash
qualia daemon start
```

The daemon will start on `http://localhost:4242`.

### Check Daemon Status

```bash
curl http://localhost:4242/health
```

### Query via API

```bash
curl -X POST http://localhost:4242/query \
  -H "Content-Type: application/json" \
  -d '{
    "query": "SELECT ?s ?p ?o WHERE { ?s ?p ?o } LIMIT 10"
  }'
```

### Stop the Daemon

```bash
qualia daemon stop
```

---

## Web Interface

QualiaDB includes a browser-based playground for interactive experimentation.

### Serve the Documentation

```bash
# From the docs directory
cd docs
python -m http.server 8000
```

Then open `http://localhost:8000/playground/` in your browser.

### Features Available in the Browser

- **Interactive Playground**: Experiment with the Prolog Webizen VM
- **Benchmark Suite**: Test performance in both Native and WASM modes
- **SPARQL Showcase**: Try SPARQL queries with live examples
- **Science Playground**: Explore scientific computing primitives
- **Edge LLM Hub**: Browse GGUF model catalog and demo inference

---

## Capability Profiles

QualiaDB uses capability profiles to control what operations are allowed in a given session.

### List Available Profiles

```bash
qualia profile list
```

### Inspect a Profile

```bash
qualia profile inspect general.qchk
```

### Compile a Custom Profile

Create a JSON-LD profile file `my-profile.jsonld`:

```json
{
  "@context": "https://webizen.org/q42",
  "@type": "CapabilityProfile",
  "profile_id": "my-custom-profile",
  "allowed_namespaces": [
    "http://example.org/",
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
  ],
  "allowed_operations": [
    "query",
    "ingest",
    "export"
  ]
}
```

Compile it:

```bash
qualia profile compile my-profile.jsonld my-profile.qchk
```

Use it with ingest:

```bash
qualia ingest --profile my-profile.qchk data.ttl output.q42
```

---

## Next Steps

- **Learn the Architecture**: Read [ARCHITECTURE.md](ARCHITECTURE.md) for deep technical details
- **Explore the Developer Guide**: See [developer-guide.md](developer-guide.md) for development workflows
- **Try the Examples**: Check out the [online examples](../playground/) for interactive demos
- **Read the Glossary**: Consult [glossary.md](glossary.md) for terminology
- **Review Standards**: See the [standards directory](standards/) for protocol specifications

---

## System Requirements

### Minimum Requirements

- **RAM**: 512 MB (strict floor)
- **Storage**: 100 MB for base installation
- **CPU**: Any modern 64-bit processor (x86_64, ARM64)

### Recommended Requirements

- **RAM**: 2 GB or more
- **Storage**: 1 GB or more for datasets
- **GPU**: Optional, for LLM inference and GPU sieving
  - Windows: DirectX 12 compatible GPU
  - macOS: Apple Silicon (M1/M2/M3) or Intel GPU
  - Linux: Vulkan-compatible GPU

### Platform Support

- **Windows**: 10/11 (x86_64)
- **macOS**: 11+ (Intel and Apple Silicon)
- **Linux**: Most modern distributions (x86_64, ARM64)
- **Browser**: Chrome/Edge/Firefox/Safari (for WASM target)

---

## Troubleshooting

### "WASM Loading..." Badge Stays Yellow

The WASM module may be loading slowly. Try:
1. Refresh the page
2. Check your browser console for errors
3. Ensure you're serving the docs directory (not opening files directly)

### Daemon Won't Start

Check if port 4242 is already in use:
```bash
# Linux/macOS
lsof -i :4242

# Windows
netstat -ano | findstr :4242
```

### Out of Memory Errors

QualiaDB enforces a 512 MB RAM floor. If you encounter OOM errors:
1. Close other applications
2. Use smaller datasets
3. Enable lazy SuperBlock querying (default for large datasets)

### GPU Inference Not Working

Ensure:
1. You have a compatible GPU (see System Requirements)
2. GPU drivers are up to date
3. You're not in a virtualized environment without GPU passthrough

---

## Getting Help

- **Documentation**: See the [manuals directory](./) for comprehensive guides
- **GitHub Issues**: Report bugs at [github.com/mediaprophet/qualiaDB/issues](https://github.com/mediaprophet/qualiaDB/issues)
- **Community**: Join discussions in the GitHub repository

---

## License

QualiaDB is released under the Threshold Shift License. See [LICENSE](../../LICENSE) for details.
