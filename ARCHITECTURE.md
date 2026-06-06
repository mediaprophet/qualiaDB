# QualiaDB Architecture

## The Universal Translator: CLI Ingestion Pipeline

The `qualia-cli` operates as the primary entry point for sovereign data ingestion into the QualiaDB `.q42` vaults. It is designed around **Mechanical Sympathy** and strict zero-allocation (heap) constraints where possible, particularly in its hot paths.

### Zero-Allocation Contract for CHK and CBOR-LD

To bridge the string-heavy reality of the Semantic Web with the pristine, hardware-aligned mechanics of the Qualia engine, the CLI relies heavily on two specific binary-friendly formats:
1. **Cognitive AI Chunks and Rules (`.chk`)**
2. **CBOR-LD (`.cbor` / `.cbor-ld`)**

The ingestion pipeline for these formats strictly enforces a **Zero-Allocation Contract**:
- Data is pull-parsed sequentially.
- No `String` or `Vec<u8>` objects are materialized on the heap per-record.
- Values are fed directly into the FNV-1a hasher at ingestion time to generate 48-byte `QualiaQuin` hardware structs.
- Because these formats are designed for native resolution and mechanical sympathy, we **do not build `.lex` string-resolution dictionaries** for them by default. This avoids the memory pressure of maintaining a `HashMap<u64, String>` in RAM.

### Multi-Pass External Sort

To achieve the 42MB Prolog Sentinel and 512MB RAM floor limits, the ingestion pipeline relies on an `ExternalSorter` (`qualia-cli/src/parsers/external_sort.rs`).

1. **Stream & Buffer**: Incoming triples are hashed and buffered into an in-memory `Vec<QualiaQuin>`.
2. **Chunk Flushes**: When the buffer reaches ~50MB, it is sorted by `object` hash and flushed to disk as a raw `.chunk` file.
3. **K-Way Merge**: Once parsing is complete, the CLI performs a K-Way Merge over all temporary chunk files to emit the final contiguous `.q42` SuperBlock stream.
4. **BIDX Sidecar**: During the final merge, the `min` and `max` object hashes for each 40KB block are recorded, creating the `.q42.bidx` block index for binary-search resolution.

### Hybrid Standard Library Usage

While the core logic execution (inside `qualia-core-db`) adheres to `#![no_std]` paradigms to allow compiling into TEEs or WebAssembly, the CLI `qualia-cli` is intentionally an orchestration tool. It uses `std`, `tokio`, `clap`, and `warp` to enable rich multi-threaded async pipelines, Solid pod synchronization, and developer UX.

### Capability Discovery

The `qualia-cli` dynamically discovers the features compiled into its linked engine via the `CAPABILITY_REGISTRY`. This allows developers to interrogate the binary using:
- `qualia-cli capabilities --list`
- `qualia-cli shacl --list-extensions`
