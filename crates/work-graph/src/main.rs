//! Work graph CLI — Beat B presence indexer. No Host invent.

use std::env;
use std::path::{Path, PathBuf};
use std::process;

use work_graph::{default_out_dir, emit, index_root};

fn print_usage() {
    println!(
        "Work graph (Implementation Graph) Beat B MVP indexer\n\
         \n\
         Indexes a QualiaDB checkout: git tip, crates, files, ALL_BOUND /\n\
         vibe catalog invoke ids, and doc-page cites. Emit is Present only.\n\
         Does not invent ImplGraph.* Host ids.\n\
         \n\
         Usage:\n\
           work-graph index [<root>] [--out <dir>]\n\
           work-graph help\n\
         \n\
         Defaults:\n\
           root  current directory (must contain {all_bound})\n\
           out   <root>/{out}\n\
         \n\
         Writes:\n\
           impl-graph.nq    N-Quads presence graph\n\
           impl-graph.json  compact summary for Capt / Vibe query later\n",
        all_bound = work_graph::ALL_BOUND_REL,
        out = work_graph::DEFAULT_OUT_REL
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] == "help" || args[1] == "--help" || args[1] == "-h" {
        print_usage();
        process::exit(if args.len() < 2 { 1 } else { 0 });
    }
    match args[1].as_str() {
        "index" => {
            if let Err(e) = run_index(&args[2..]) {
                eprintln!("work-graph index failed: {e}");
                process::exit(1);
            }
        }
        other => {
            eprintln!("unknown command: {other}");
            print_usage();
            process::exit(1);
        }
    }
}

fn run_index(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let (root, out) = parse_index_args(args)?;
    let graph = index_root(&root)?;
    let (nq, json) = emit::write_emit(&graph, &out)?;
    println!("Work graph Present emit");
    println!("  tip     {} ({})", graph.project.tip_sha, graph.project.branch);
    println!("  dirty   {}", graph.project.dirty);
    println!(
        "  files   {}  crates {}  invokeIds {}  families {}  docCites {}",
        graph.files.len(),
        graph.crates.len(),
        graph.invoke_ids.len(),
        graph.families.len(),
        graph.doc_cites.len()
    );
    println!("  nquads  {}", nq.display());
    println!("  json    {}", json.display());
    if !graph.bound_missing_from_catalog().is_empty() {
        println!(
            "  note    {} ALL_BOUND ids missing from vibe catalog (reported, not invented)",
            graph.bound_missing_from_catalog().len()
        );
    }
    Ok(())
}

fn parse_index_args(args: &[String]) -> Result<(PathBuf, PathBuf), String> {
    let mut root: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                let dir = args.get(i + 1).ok_or("--out needs a directory")?;
                out = Some(PathBuf::from(dir));
                i += 2;
            }
            "--help" | "-h" => return Err("help".into()),
            flag if flag.starts_with('-') => return Err(format!("unknown flag: {flag}")),
            path => {
                if root.is_some() {
                    return Err("root already set".into());
                }
                root = Some(PathBuf::from(path));
                i += 1;
            }
        }
    }
    let root = root.unwrap_or_else(|| PathBuf::from("."));
    let out = out.unwrap_or_else(|| default_out_dir(Path::new(&root)));
    Ok((root, out))
}
