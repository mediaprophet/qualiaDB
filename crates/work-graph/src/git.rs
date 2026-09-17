//! Git tip + tracked file list. Beat B needs SHA, branch, and dirty flag.

use std::path::Path;
use std::process::Command;

use crate::IndexError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitTip {
    pub sha: String,
    pub branch: String,
    pub dirty: bool,
}

pub fn read_tip(root: &Path) -> Result<GitTip, IndexError> {
    let sha = git_stdout(root, &["rev-parse", "HEAD"])?;
    let branch = git_stdout(root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let porcelain = git_stdout_raw(root, &["status", "--porcelain"])?;
    Ok(GitTip {
        sha,
        branch,
        dirty: !porcelain.is_empty(),
    })
}

pub fn ls_files(root: &Path) -> Result<Vec<String>, IndexError> {
    let out = git_stdout_raw(root, &["ls-files", "-z"])?;
    let mut files = Vec::new();
    for chunk in out.split('\0') {
        if chunk.is_empty() {
            continue;
        }
        files.push(chunk.replace('\\', "/"));
    }
    files.sort();
    Ok(files)
}

fn git_stdout(root: &Path, args: &[&str]) -> Result<String, IndexError> {
    Ok(git_stdout_raw(root, args)?.trim().to_string())
}

fn git_stdout_raw(root: &Path, args: &[&str]) -> Result<String, IndexError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| IndexError::Git(format!("failed to spawn git: {e}")))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(IndexError::Git(format!(
            "git {} failed: {}",
            args.join(" "),
            err.trim()
        )));
    }
    String::from_utf8(output.stdout)
        .map_err(|e| IndexError::Git(format!("git output was not utf-8: {e}")))
}
