fn main() {
    // The semantic/N3 command builds a sizeable cold parser frame before handing records to
    // QualiaDB's bounded arenas. Windows PE defaults the main thread to a 1 MiB stack, which
    // overflows even for the small bundled agency.n3 fixture. This changes only CLI process stack
    // reservation; evaluator hot paths, the 42 MiB Sentinel, and ABI buffers are unaffected.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/STACK:8388608");
    }
}
