let meta = #{
    name: "q42-volume-remaining",
    description: "Finish remaining Q42 volume jobs at 0.0.30: compact, range SPARQL, Pages reprocess, OPFS source. Same tree, no worktrees, not the 0.1.0 product programme.",
    when_to_use: "When finishing the Q42 large-dataset volume plan remaining lanes (R4/R5/R7/R8/R9) without touching gated v4 codecs.",
    phases: [
        #{ title: "Implement", detail: "four disjoint same-tree lanes" },
        #{ title: "Integrate", detail: "wire exports and CLI after lanes land" },
    ],
};

let report_schema = #{
    "type": "object",
    "required": ["lane", "files", "done", "blocked", "tests"],
    "properties": #{
        "lane": #{ "type": "string" },
        "files": #{ "type": "array", "items": #{ "type": "string" } },
        "done": #{ "type": "string" },
        "blocked": #{ "type": "string" },
        "tests": #{ "type": "string" },
    },
};

phase("Implement");
log("Four disjoint lanes in C:/Projects/qualia-27062026. No git worktrees. R10 v4 is gated.");

let jobs = [];

let a = "";
a += "Work ONLY in C:/Projects/qualia-27062026. Do not create git worktrees. ";
a += "LANE A — Q42 compaction. Create ONLY crates/qualia-core-db/src/q42/volume/compact.rs. ";
a += "Do not edit volume/mod.rs, CLI, SPARQL, or docs/data. ";
a += "Implement compact_volume_set(root, out_dir) that opens a Q42VolumeSet root, streams child SuperBlocks in object order via read_superblock_into, merges lexicons, rewrites through Q42RolloverPublisher or StreamingQ42VolumeWriter, and publishes a new generation root via write_volume_root / append pattern (temp + rename). ";
a += "Single-file volumes: rewrite through the streaming writer (PIDX+FIDX+BIDX). ";
a += "Preserve publication: if any quin_requires_sanctuary, do not declare_permissive_commons; otherwise declare_permissive_commons only when every input had FLAG_PERMISSIVE_COMMONS. ";
a += "Constraints: 48-byte NQuin, 40960 SuperBlock, no Vec in a hot per-quin loop beyond the existing writer. Tests in compact.rs: two children compact to a readable root; object order preserved; sanctuary child cannot become commons. ";
a += "Read publish.rs, stream_writer.rs, q42_volume.rs, publication.rs, manifest.rs before writing. ";
a += "Return {lane, files, done, blocked, tests}.";
jobs.push(#{
    prompt: a,
    label: "lane-a-compact",
    capability_mode: "all",
    output_schema: report_schema,
});

let b = "";
b += "Work ONLY in C:/Projects/qualia-27062026. Do not create git worktrees. ";
b += "LANE B — Range SPARQL. Create new files only under crates/qualia-core-db/src/sparql_library/: range_optional.rs, range_union.rs, range_bind.rs, range_hash_join.rs. ";
b += "You MAY edit crates/qualia-cli/src/sparql.rs and sparql_library/mod.rs to register the new modules. ";
b += "Do not edit q42/volume/* or docs/data. ";
b += "Read range_join_select.rs, range_select_apply.rs, sparql_planner.rs PhysicalOperatorType, and qualia-cli/src/sparql.rs. ";
b += "1) Wire Q42QueryMode: if the vault file is larger than RESIDENT_QUERY_MAX_BYTES (4 MiB), do not fall back to read_q42_quins / read_all_quins; fail with a clear error if no range plan applies. Small files may still use the resident executor. ";
b += "2) Implement range Optional (left join), Union, Bind by paging the existing range scan/join kernels. ";
b += "3) HashJoin: bounded hash of the smaller side into a caller-capped table; if it would exceed the cap, return an error telling the planner to keep NestedLoopJoin — do not allocate unbounded. ";
b += "Tests in each new file. 48-byte NQuin ABI unchanged. ";
b += "Return {lane, files, done, blocked, tests}.";
jobs.push(#{
    prompt: b,
    label: "lane-b-sparql",
    capability_mode: "all",
    output_schema: report_schema,
});

let c = "";
c += "Work ONLY in C:/Projects/qualia-27062026. Do not create git worktrees. ";
c += "LANE C — Reprocess Pages catalogs + commons ingest flag. ";
c += "Sources: bundled/ontologies/**/*.ttl (and rdf) map onto docs/data/**/*.q42 (dublincore, w3c, w3c-archives, purl, geonames, fibo, schemaorg). ";
c += "1) In sparql_library/external_sort.rs ExternalSorter::merge (and merge_volume_set if present), after StreamingQ42VolumeWriter::new, call declare_permissive_commons() — this ingest path is catalog ontologies, not personal records. Sanctuary bits on a quin still win inside the writer. ";
c += "2) Reprocess every docs/data .q42 that has a matching bundled ontology source via crate::query::ingest::streaming_import_rdf (Complete/lossless). Do not invent Monarch 5.4GB. ";
c += "3) After a successful rewrite, delete leftover sidecars docs/data/schemaorg/30.0/*.q42.lex and *.q42.bidx. ";
c += "4) Grep docs/js and docs/pkg for hardcoded assumptions that break if PIDX/FIDX exist (header is still 256 bytes / Q42\\\\0). Fix only if the loader would fail; do not rewrite the whole portal. ";
c += "Do not edit compact.rs or SPARQL operator files. ";
c += "Verify one file with Q42InspectReport: lexicon_has_no_terms must be false for a reprocessed dct.q42. ";
c += "Return {lane, files, done, blocked, tests}.";
jobs.push(#{
    prompt: c,
    label: "lane-c-reprocess",
    capability_mode: "all",
    output_schema: report_schema,
});

let d = "";
d += "Work ONLY in C:/Projects/qualia-27062026. Do not create git worktrees. ";
d += "LANE D — OPFS / local CAR range source. Create ONLY crates/qualia-core-db/src/q42/volume/opfs_source.rs. ";
d += "Do not edit docs/data, CLI SPARQL, or compact.rs. ";
d += "Read volume/range.rs (Q42RangeSource, LocalFileRangeSource, HttpRangeSource) and volume/car.rs. ";
d += "Implement a caller-buffered Q42RangeSource that reads from a provided byte slice or a callback (Fn(offset,len,buf)->Result<usize>). This is the WASM/OPFS adapter: JS/OPFS fills the buffer; Rust never maps the whole file. ";
d += "Add verify_local_car_as_q42_source(path) that decode_and_verify_car then exposes entity bytes through the same source. ";
d += "Do not claim a live public IPFS gateway was dogfooded. Tests: slice source returns the same bytes as LocalFileRangeSource for a tiny write_unified_volume file; tampered CAR is rejected. ";
d += "Return {lane, files, done, blocked, tests}.";
jobs.push(#{
    prompt: d,
    label: "lane-d-opfs",
    capability_mode: "all",
    output_schema: report_schema,
});

let results = parallel(jobs);

phase("Integrate");
let summary = "";
let i = 0;
while i < results.len() {
    let r = results[i];
    if r == () || !r.success {
        log("lane slot " + i.to_string() + " failed or empty");
    } else {
        log(r.output.lane + ": " + r.output.done);
    }
    i += 1;
}

let integ = "";
integ += "Work ONLY in C:/Projects/qualia-27062026. No git worktrees. ";
integ += "You are the integrator for the Q42 remaining-jobs swarm. ";
integ += "Wire crates/qualia-core-db/src/q42/volume/mod.rs to `mod compact` and `mod opfs_source` if those files exist, and re-export their public types. ";
integ += "Re-export from q42_volume.rs pub use. ";
integ += "Add `qualia-cli q42 compact <root>` in crates/qualia-cli/src/cli/misc.rs and handlers/misc.rs if compact.rs landed. ";
integ += "Run: cargo test -p qualia-core-db --lib q42_volume ; cargo check -p qualia-cli. Fix compile breaks you caused. ";
integ += "Append docs/plans/q42-volume-PROGRESS-LOG.md honestly. Update the R4-R9 status lines in the plan if a lane was blocked. ";
integ += "Do not implement v4 codecs. Do not claim Monarch 5.4GB was rebuilt. ";
integ += "Return {lane, files, done, blocked, tests}.";

let ir = agent(integ, #{
    label: "integrator",
    capability_mode: "all",
    output_schema: report_schema,
});

if ir == () || !ir.success {
    complete(#{
        summary: "Implement lanes finished; integrator failed or empty. Check /workflows.",
        integrator: "failed",
    });
}

complete(#{
    summary: ir.output.done,
    tests: ir.output.tests,
    blocked: ir.output.blocked,
    files: ir.output.files,
});
