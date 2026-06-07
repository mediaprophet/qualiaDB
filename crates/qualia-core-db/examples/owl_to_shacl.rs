//! Generate SHACL shape bundles from local OWL fixtures.
//!
//! ```powershell
//! cargo run -p qualia-core-db --example owl_to_shacl
//! ```

use std::path::PathBuf;

fn main() {
    let root = PathBuf::from("app-development");
    let healthcare = root.join("2015-01-11.n3");
    let radlex = root.join("PunRadLex_Owl4.3/PunRadLex4.3.owl");
    let out = PathBuf::from("bundled/qapps/Anatomy/Knowledge/shapes");

    let written = qualia_core_db::owl_to_shacl::write_anatomy_shape_bundle(
        healthcare.is_file().then_some(healthcare.as_path()),
        radlex.is_file().then_some(radlex.as_path()),
        &out,
        512,
        50_000,
    )
    .expect("shape bundle generation");

    for path in written {
        println!("wrote {path}");
    }
}
