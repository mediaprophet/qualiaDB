//! Desktop build script.
//!
//! - Invokes Tauri codegen / winres (icon, VERSIONINFO, application RT_MANIFEST).
//! - On Windows GNU toolchains, shadows MinGW's auto-linked `default-manifest.o`
//!   so only Tauri's RT_MANIFEST is linked. That removes the GNU-only warning:
//!   `ld: .rsrc merge failure: multiple non-default manifests`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    tauri_build::build();

    // Tauri/winres embeds a full application manifest (Common Controls v6, DPI,
    // execution level, …) as RT_MANIFEST id 1 via resource.rc → libresource.a.
    //
    // MinGW-w64's gcc `*endfile` specs always inject `default-manifest.o` for
    // non-shared links (`%{!shared:%:if-exists(default-manifest.o%s)}`). That
    // object is also RT_MANIFEST id 1. Binutils then warns on the dual
    // non-default `.rsrc` merge. `--exclude-libs` cannot help: the object is
    // not pulled from an archive — collect2 adds it by path.
    //
    // MSVC never does this; the conflict is GNU-linker specific.
    //
    // Fix: emit an empty COFF object named `default-manifest.o` and put its
    // directory first on gcc's `-B` search path so `if-exists(default-manifest.o)`
    // resolves to our resource-free stub. Tauri's polished RT_MANIFEST remains
    // the sole application manifest in the final PE.
    if env::var_os("CARGO_CFG_WINDOWS").is_some()
        && env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu")
    {
        if let Err(err) = suppress_mingw_default_manifest() {
            // Do not fail the build on a missing host `gcc` — the app still
            // links (with the dual-manifest warning). Log clearly so operators
            // can install a matching MinGW or switch to the MSVC target.
            println!(
                "cargo:warning=webizen-desktop: could not suppress MinGW default-manifest.o ({err}); \
                 expect `ld: .rsrc merge failure: multiple non-default manifests` until fixed"
            );
        }
    }
}

fn suppress_mingw_default_manifest() -> Result<(), String> {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR not set")?);
    let override_dir = out_dir.join("mingw_manifest_override");
    fs::create_dir_all(&override_dir).map_err(|e| format!("create override dir: {e}"))?;

    let empty_o = override_dir.join("default-manifest.o");
    // Rebuild only when missing so incremental builds stay cheap.
    if !empty_o.is_file() {
        write_empty_coff_object(&override_dir, &empty_o)?;
    }

    // gcc `-B` prepends this directory when resolving `default-manifest.o%s`.
    // Forward slashes are accepted by the MinGW driver on Windows.
    let b_path = path_as_gcc_b_prefix(&override_dir);
    println!("cargo:rustc-link-arg=-B{b_path}");
    // Re-run build.rs if the stub is deleted out-of-band.
    println!("cargo:rerun-if-changed={}", empty_o.display());
    Ok(())
}

/// Produce a COFF object with **no** `.rsrc` section, named for gcc's endfile hook.
fn write_empty_coff_object(override_dir: &Path, empty_o: &Path) -> Result<(), String> {
    // Prefer compiling a one-line C file with the same host gcc rustc will use
    // as the linker driver — that guarantees a matching object format.
    let empty_c = override_dir.join("empty_default_manifest.c");
    fs::write(
        &empty_c,
        b"/* Intentionally empty: shadows MinGW-w64 default-manifest.o so the\n\
           * application keeps a single RT_MANIFEST (from Tauri/winres).\n\
           */\n",
    )
    .map_err(|e| format!("write empty.c: {e}"))?;

    let gcc = find_host_gcc();
    let status = Command::new(&gcc)
        .arg("-c")
        .arg(&empty_c)
        .arg("-o")
        .arg(empty_o)
        .status()
        .map_err(|e| format!("spawn {gcc}: {e}"))?;

    if !status.success() {
        return Err(format!("{gcc} -c failed with {status}"));
    }
    if !empty_o.is_file() {
        return Err(format!("{} was not produced", empty_o.display()));
    }
    Ok(())
}

fn find_host_gcc() -> String {
    // Prefer the exact triple rustc uses on this host, then plain `gcc`.
    const CANDIDATES: &[&str] = &[
        "x86_64-w64-mingw32-gcc",
        "gcc",
        "cc",
    ];
    for name in CANDIDATES {
        if Command::new(name)
            .arg("-dumpversion")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return (*name).to_string();
        }
    }
    // Fall through to `gcc` and let the spawn error surface.
    "gcc".to_string()
}

fn path_as_gcc_b_prefix(dir: &Path) -> String {
    let mut s = dir.to_string_lossy().replace('\\', "/");
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}
