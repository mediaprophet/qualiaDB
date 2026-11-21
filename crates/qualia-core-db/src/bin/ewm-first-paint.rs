//! Native InventoryVerifier first-paint producer. Browser/wasm never calls finish.
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if let Err(error) = run() {
        eprintln!("ewm-first-paint: {error}");
        std::process::exit(1);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn run() -> Result<(), String> {
    use qualia_core_db::bundle::stream::publish::{publish_first_paint, FirstPaintSource};
    use sha2::{Digest, Sha256};
    use std::path::Path;

    const ZERO_SHA384: &str =
        "000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
    const MAX_FIRST_PAINT: usize = 64;

    let args = parse_args()?;
    let catalog_text = std::fs::read_to_string(&args.catalog)
        .map_err(|error| format!("catalog read {}: {error}", args.catalog.display()))?;
    let catalog: serde_json::Value = serde_json::from_str(&catalog_text)
        .map_err(|error| format!("catalog JSON {}: {error}", args.catalog.display()))?;

    let first_paint = catalog
        .get("first_paint")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "catalog first_paint must be an array".to_string())?;
    if first_paint.is_empty() || first_paint.len() > MAX_FIRST_PAINT {
        return Err(format!(
            "catalog first_paint must list 1..={MAX_FIRST_PAINT} members, found {}",
            first_paint.len()
        ));
    }
    let entries = catalog
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "catalog entries must be an array".to_string())?;

    let mut owned: Vec<(String, Vec<u8>)> = Vec::with_capacity(first_paint.len());
    for id_value in first_paint {
        let scene_id = id_value
            .as_str()
            .ok_or_else(|| "catalog first_paint members must be strings".to_string())?;
        let row = entries
            .iter()
            .find(|entry| entry.get("scene_id").and_then(serde_json::Value::as_str) == Some(scene_id))
            .ok_or_else(|| format!("no entries row for {scene_id}"))?;
        let want_sha = row
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{scene_id} sha256 missing"))?;
        let want_bytes = row
            .get("bytes")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("{scene_id} bytes missing"))? as usize;
        let url = row
            .get("url")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{scene_id} url missing"))?;
        let relative = url
            .strip_prefix('/')
            .ok_or_else(|| format!("{scene_id} url must start with /"))?;
        let path = args.root.join(relative);
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("{scene_id} read {}: {error}", path.display()))?;
        if bytes.len() != want_bytes {
            return Err(format!(
                "{scene_id} byte length {} does not match catalog {want_bytes}",
                bytes.len()
            ));
        }
        let got_sha = hex::encode(Sha256::digest(&bytes));
        if got_sha != want_sha {
            return Err(format!("{scene_id} sha256 {got_sha} does not match catalog {want_sha}"));
        }
        owned.push((format!("{scene_id}.10d"), bytes));
    }

    let sources: Vec<FirstPaintSource<'_>> = owned
        .iter()
        .map(|(key, bytes)| FirstPaintSource {
            key: key.as_str(),
            bytes: bytes.as_slice(),
        })
        .collect();
    let identity = publish_first_paint(&sources).map_err(|error| error.to_string())?;
    if !is_nonzero_sha384(&identity.descriptor_sha384)
        || !is_nonzero_sha384(&identity.context_sha384)
        || identity.descriptor_sha384 == ZERO_SHA384
        || identity.context_sha384 == ZERO_SHA384
    {
        return Err("refused to write a zero SHA-384 digest".to_string());
    }
    if identity.entries as usize != sources.len() {
        return Err(format!(
            "InventoryVerifier entries {} != first-paint member count {}",
            identity.entries,
            sources.len()
        ));
    }

    if let Some(parent) = args.out.parent() {
        if parent != Path::new("") {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("create {}: {error}", parent.display()))?;
        }
    }
    std::fs::write(&args.out, identity.sidecar_json())
        .map_err(|error| format!("write {}: {error}", args.out.display()))?;

    println!("{}", identity.descriptor_sha384);
    println!("{}", identity.context_sha384);
    println!("{}", identity.generation);
    println!("{}", identity.entries);
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
struct Args {
    catalog: std::path::PathBuf,
    root: std::path::PathBuf,
    out: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_args() -> Result<Args, String> {
    use std::path::PathBuf;

    let mut catalog = None;
    let mut root = None;
    let mut out = None;
    let mut argv = std::env::args().skip(1);
    while let Some(flag) = argv.next() {
        let value = argv
            .next()
            .ok_or_else(|| format!("missing path after {flag}"))?;
        match flag.as_str() {
            "--catalog" => catalog = Some(PathBuf::from(value)),
            "--root" => root = Some(PathBuf::from(value)),
            "--out" => out = Some(PathBuf::from(value)),
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(Args {
        catalog: catalog.ok_or_else(|| "missing --catalog".to_string())?,
        root: root.ok_or_else(|| "missing --root".to_string())?,
        out: out.ok_or_else(|| "missing --out".to_string())?,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn is_nonzero_sha384(text: &str) -> bool {
    text.len() == 96
        && text.bytes().all(|byte| byte.is_ascii_hexdigit())
        && text.bytes().any(|byte| byte != b'0')
}
