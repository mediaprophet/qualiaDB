#[cfg(not(target_arch = "wasm32"))]
fn main() {
    if let Err(error) = qualia_core_db::platform::device_benchmark::run_worker_from_env() {
        eprintln!("qualia-device-benchmark-worker: {error}");
        std::process::exit(2);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {}
