use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha2::{Digest, Sha256};

use super::sha256_file;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceProvenance {
    pub commit: String,
    pub dirty_source_hash: String,
    pub executable_sha256: String,
}

pub fn capture_source_provenance() -> SourceProvenance {
    let executable_sha256 = std::env::current_exe()
        .ok()
        .and_then(|path| sha256_file(&path).ok())
        .unwrap_or_else(|| "unknown".into());
    let root = git_stdout(&["rev-parse", "--show-toplevel"])
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|text| PathBuf::from(text.trim()))
        .filter(|path| path.is_dir());
    let commit = git_stdout(&["rev-parse", "HEAD"])
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "unknown".into());
    let dirty_source_hash = root
        .as_deref()
        .and_then(hash_relevant_dirty_sources)
        .unwrap_or_else(|| "unknown".into());
    SourceProvenance {
        commit,
        dirty_source_hash,
        executable_sha256,
    }
}

fn git_stdout(args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git").args(args).output().ok()?;
    output.status.success().then_some(output.stdout)
}

fn hash_relevant_dirty_sources(root: &Path) -> Option<String> {
    const PATHS: &[&str] = &[
        "crates/qualia-core-db",
        "crates/qualia-cli",
        "Cargo.toml",
        "Cargo.lock",
    ];
    let mut changed = BTreeSet::new();
    collect_git_paths(
        root,
        &[
            "diff",
            "--name-only",
            "-z",
            "HEAD",
            "--",
            PATHS[0],
            PATHS[1],
            PATHS[2],
            PATHS[3],
        ],
        &mut changed,
    )?;
    collect_git_paths(
        root,
        &[
            "ls-files",
            "--others",
            "--exclude-standard",
            "-z",
            "--",
            PATHS[0],
            PATHS[1],
            PATHS[2],
            PATHS[3],
        ],
        &mut changed,
    )?;

    let mut hasher = Sha256::new();
    if changed.is_empty() {
        hasher.update(b"clean");
    }
    let mut buffer = [0u8; 1024 * 1024];
    for relative in changed {
        hasher.update((relative.len() as u64).to_le_bytes());
        hasher.update(relative.as_bytes());
        let path = root.join(&relative);
        match std::fs::File::open(path) {
            Ok(mut file) => loop {
                let read = file.read(&mut buffer).ok()?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                hasher.update(b"<deleted>");
            }
            Err(_) => return None,
        }
    }
    Some(hex::encode(hasher.finalize()))
}

fn collect_git_paths(root: &Path, args: &[&str], output: &mut BTreeSet<String>) -> Option<()> {
    let command = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .ok()?;
    if !command.status.success() {
        return None;
    }
    for path in command.stdout.split(|byte| *byte == 0) {
        if path.is_empty() {
            continue;
        }
        let path = String::from_utf8(path.to_vec()).ok()?;
        output.insert(path.replace('\\', "/"));
    }
    Some(())
}
