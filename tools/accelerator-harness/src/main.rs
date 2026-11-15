use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::{json, Value};

#[path = "../../../crates/qualia-core-db/src/platform/accelerator_inventory/mod.rs"]
mod inventory;
#[cfg(feature = "npu-openvino")]
mod npu;

struct Options {
    output: PathBuf,
    discovery_only: bool,
    require_npu: bool,
    run_npu: bool,
}

fn options() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let mut output = None;
    let mut discovery_only = false;
    let mut require_npu = false;
    let mut run_npu = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => output = Some(PathBuf::from(args.next().ok_or("--output needs a path")?)),
            "--discovery-only" => discovery_only = true,
            "--require-npu" => require_npu = true,
            "--npu-openvino" => run_npu = true,
            _ => return Err(format!("unknown option: {arg}")),
        }
    }
    if discovery_only && (require_npu || run_npu) {
        return Err("discovery-only cannot certify or execute an NPU workload".into());
    }
    Ok(Options {
        output: output.ok_or("--output is required for the retained execution receipt")?,
        discovery_only,
        require_npu,
        run_npu,
    })
}

fn language_checks() -> Result<Value, String> {
    let program = vibe::load_program("fn run() { return (7 * 6) + 1; }")
        .map_err(|error| format!("VibeScript checking: {error:?}"))?;
    let result = vibe::eval_function(
        &program,
        "run",
        vec![],
        &mut vibe::LocalHost::default(),
        &mut vibe::Env::default(),
    )
    .map_err(|error| format!("VibeScript execution: {error:?}"))?;
    if result != vibe::Value::I64(43) {
        return Err(format!("VibeScript CPU reference mismatch: {result:?}"));
    }
    let mut host = vibe::LocalHost::default();
    let mut env = vibe::Env::default();
    if vibe::eval_cell("= time.unix()", &mut host, &mut env).is_ok() {
        return Err("a Pure VibeScript cell admitted an external clock effect".into());
    }
    Ok(json!({"execution": "verified", "passed": 2, "failed": 0,
        "checks": ["checked_program_cpu_oracle", "pure_cell_effect_rejection"]}))
}

fn npu_execution(opts: &Options, detected: bool) -> (Value, bool) {
    if opts.run_npu {
        #[cfg(feature = "npu-openvino")]
        {
            return match npu::run_openvino_npu() {
                Ok(evidence) => (
                    json!({"execution": "verified", "passed": 1,
                    "runtime": "available", "workload": "supported", "evidence": evidence}),
                    true,
                ),
                Err(reason) => (
                    json!({"execution": "failed", "passed": 0, "failed": 1,
                    "reason": reason}),
                    false,
                ),
            };
        }
        #[cfg(not(feature = "npu-openvino"))]
        {
            return (
                json!({"execution": "not_run", "passed": 0,
                "reason": "build with npu-openvino to use the explicitly installed ORT/OpenVINO plugin"}),
                false,
            );
        }
    }
    (
        json!({"execution": "not_run", "passed": 0,
        "runtime": "not_probed", "workload": "not_probed",
        "reason": if detected { "NPU detected; provider execution was not requested" }
            else { "no NPU execution requested; consult the independent hardware discovery outcome" }}),
        false,
    )
}

fn run(opts: Options) -> Result<bool, String> {
    let inventory = inventory::capture();
    let language = if opts.discovery_only {
        json!({"execution": "not_run", "passed": 0, "reason": "discovery-only"})
    } else {
        match language_checks() {
            Ok(receipt) => receipt,
            Err(reason) => {
                json!({"execution": "failed", "passed": 0, "failed": 1, "reason": reason})
            }
        }
    };
    let (npu, npu_verified) = npu_execution(&opts, inventory.has_detected_npu());
    let language_ok = opts.discovery_only || language["execution"] == "verified";
    let success =
        language_ok && (!opts.require_npu || npu_verified) && (!opts.run_npu || npu_verified);
    let receipt = json!({
        "schema_version": 1,
        "qualification": if success { "requested_checks_complete" } else { "failed" },
        "scope": "CPU language and fresh accelerator discovery; GPU/product integration is separate",
        "language": language,
        "npu_inventory": inventory.to_json(),
        "npu_execution": npu,
        "require_npu": opts.require_npu,
        "hardware_absence_is_not_execution_success": true,
    });
    let bytes = serde_json::to_vec_pretty(&receipt).map_err(|error| error.to_string())?;
    if bytes.len() > 64 * 1024 {
        return Err("execution receipt exceeds its 64 KiB budget".into());
    }
    std::fs::write(&opts.output, bytes).map_err(|error| error.to_string())?;
    println!(
        "{}",
        serde_json::to_string(&receipt).map_err(|error| error.to_string())?
    );
    Ok(success)
}

fn main() -> ExitCode {
    match options().and_then(run) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("accelerator harness: {error}");
            ExitCode::FAILURE
        }
    }
}
