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
    glue_settings_server_parts();
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

    let manifest_o = override_dir.join("default-manifest.o");
    // Rebuild only when missing so incremental builds stay cheap.
    if !manifest_o.is_file() {
        write_manifest_coff_object(&override_dir, &manifest_o)?;
    }

    // gcc `-B` prepends this directory when resolving `default-manifest.o%s`.
    // Forward slashes are accepted by the MinGW driver on Windows.
    let b_path = path_as_gcc_b_prefix(&override_dir);
    println!("cargo:rustc-link-arg=-B{b_path}");
    // Re-run build.rs if the stub is deleted out-of-band.
    println!("cargo:rerun-if-changed={}", manifest_o.display());
    Ok(())
}

/// Produce a COFF object containing Common-Controls v6 RT_MANIFEST, named for gcc's endfile hook.
/// This ensures both test executables and application binaries bind comctl32.dll 6.0
/// (required for TaskDialogIndirect and modern dialog APIs on Windows).
fn write_manifest_coff_object(override_dir: &Path, out_o: &Path) -> Result<(), String> {
    let manifest_xml = override_dir.join("app.manifest");
    let rc_file = override_dir.join("default_manifest.rc");

    fs::write(
        &manifest_xml,
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
</assembly>"#,
    )
    .map_err(|e| format!("write app.manifest: {e}"))?;

    let manifest_path_escaped = manifest_xml.display().to_string().replace('\\', "/");
    fs::write(&rc_file, format!("1 24 \"{manifest_path_escaped}\"\n"))
        .map_err(|e| format!("write default_manifest.rc: {e}"))?;

    let windres = find_host_windres();
    let status = Command::new(&windres)
        .arg(&rc_file)
        .arg("-O")
        .arg("coff")
        .arg("-o")
        .arg(out_o)
        .status()
        .map_err(|e| format!("spawn {windres}: {e}"))?;

    if !status.success() {
        return Err(format!("{windres} failed with {status}"));
    }
    if !out_o.is_file() {
        return Err(format!("{} was not produced", out_o.display()));
    }
    Ok(())
}

fn find_host_windres() -> String {
    const CANDIDATES: &[&str] = &["x86_64-w64-mingw32-windres", "windres"];
    for name in CANDIDATES {
        if Command::new(name)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return (*name).to_string();
        }
    }
    "windres".to_string()
}

fn path_as_gcc_b_prefix(dir: &Path) -> String {
    let mut s = dir.to_string_lossy().replace('\\', "/");
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}


fn glue_settings_server_parts() {
    let _manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // ss_parts emergency glue removed — settings_server.rs is monolithic again.
}
