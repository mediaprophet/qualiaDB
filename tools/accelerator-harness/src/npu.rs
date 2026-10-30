use std::fs;
use std::path::{Path, PathBuf};

use ort::{memory::DeviceType, session::Session, value::Tensor};
use serde::Serialize;
use serde_json::Value;

#[path = "npu_model.rs"]
mod model;

const PROVIDER: &str = "OpenVINOExecutionProvider";
const PROFILE_LIMIT: u64 = 1024 * 1024;

#[derive(Debug, Serialize)]
pub struct NpuExecutionEvidence {
    pub actual_provider: String,
    pub target_device: String,
    pub device_id: u32,
    pub device_vendor: String,
    pub workload: String,
    pub expected: Vec<f32>,
    pub actual: Vec<f32>,
    pub cpu_fallback_disabled: bool,
    pub executed_kernel_events: usize,
    pub placement_evidence: String,
    pub runtime_path: PathBuf,
    pub plugin_path: PathBuf,
}

fn local_library(variable: &str) -> Result<PathBuf, String> {
    let path = std::env::var_os(variable)
        .map(PathBuf::from)
        .ok_or_else(|| format!("{variable} must identify an explicitly installed local library"))?;
    let resolved = path
        .canonicalize()
        .map_err(|error| format!("{variable}: {error}"))?;
    if !resolved.is_file() {
        return Err(format!("{variable} is not a library file"));
    }
    Ok(resolved)
}

fn checked_profile(path: &Path, scratch: &Path) -> Result<usize, String> {
    let path = path.canonicalize().map_err(|error| error.to_string())?;
    let scratch = scratch.canonicalize().map_err(|error| error.to_string())?;
    if !path.starts_with(scratch) {
        return Err("runtime returned a profile outside the owned temporary directory".into());
    }
    let metadata = path.metadata().map_err(|error| error.to_string())?;
    if metadata.len() > PROFILE_LIMIT {
        return Err("NPU profile exceeds the one MiB evidence budget".into());
    }
    let profile = fs::read(&path).map_err(|error| error.to_string())?;
    verified_profile_kernels(&profile)
}

fn verified_profile_kernels(profile: &[u8]) -> Result<usize, String> {
    if profile.len() as u64 > PROFILE_LIMIT {
        return Err("NPU profile exceeds the one MiB evidence budget".into());
    }
    let value: Value =
        serde_json::from_slice(profile).map_err(|error| format!("invalid ORT profile: {error}"))?;
    let events = value
        .as_array()
        .ok_or("ORT profile is not an event array")?;
    let mut kernels = 0;
    for event in events {
        if event.get("cat").and_then(Value::as_str) != Some("Node") {
            continue;
        }
        let name = event.get("name").and_then(Value::as_str).unwrap_or("");
        if !name.ends_with("_kernel_time") {
            continue;
        }
        if event.get("ph").and_then(Value::as_str) != Some("X")
            || !event
                .get("dur")
                .and_then(Value::as_f64)
                .is_some_and(|duration| duration >= 0.0)
        {
            return Err("ORT profile contains an incomplete kernel event".into());
        }
        let provider = event
            .get("args")
            .and_then(|args| args.get("provider"))
            .and_then(Value::as_str)
            .ok_or("executed kernel has no provider attribution")?;
        if provider != PROVIDER {
            return Err(format!(
                "arithmetic executed on {provider}, not the selected NPU provider"
            ));
        }
        kernels += 1;
    }
    if kernels == 0 {
        return Err("ORT profile does not prove execution of an OpenVINO arithmetic kernel".into());
    }
    Ok(kernels)
}

fn compare_output(actual: &[f32], expected: &[f32]) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err("NPU arithmetic returned an unexpected element count".into());
    }
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        if !actual.is_finite() || (actual - expected).abs() > 1.0e-5 {
            return Err(format!(
                "NPU arithmetic mismatch at {index}: {actual} vs {expected}"
            ));
        }
    }
    Ok(())
}

pub fn run_openvino_npu() -> Result<NpuExecutionEvidence, String> {
    let runtime_path = local_library("ORT_DYLIB_PATH")?;
    let plugin_path = local_library("QUALIA_OPENVINO_EP_PATH")?;
    let scratch = tempfile::Builder::new()
        .prefix("qualia-npu-harness-")
        .tempdir()
        .map_err(|error| error.to_string())?;
    let environment = ort::init_from(&runtime_path)
        .map_err(|error| format!("ORT load: {error}"))?
        .build()
        .map_err(|error| format!("ORT environment: {error}"))?;
    environment
        .register_ep_library(PROVIDER, &plugin_path)
        .map_err(|error| format!("OpenVINO plugin registration: {error}"))?;
    let device = environment
        .devices()
        .find(|device| {
            device.ep().ok() == Some(PROVIDER) && device.hardware_device().ty() == DeviceType::NPU
        })
        .ok_or(
            "the configured OpenVINO runtime exposes no NPU device; no fallback was attempted",
        )?;
    let device_id = device.hardware_device().id();
    let device_vendor = device
        .hardware_device()
        .vendor()
        .map_err(|error| format!("NPU vendor: {error}"))?
        .to_owned();
    let mut session = Session::builder(&environment)
        .map_err(|error| error.to_string())?
        .with_devices([device], None)
        .map_err(|error| format!("NPU device selection: {error}"))?
        .with_disable_cpu_fallback()
        .map_err(|error| format!("disable CPU fallback: {error}"))?
        .with_profiling(scratch.path().join("placement"))
        .map_err(|error| format!("NPU profiling: {error}"))?
        .commit_from_memory(&model::add_model())
        .map_err(|error| format!("NPU arithmetic graph is unsupported or unavailable: {error}"))?;
    let left: Vec<f32> = (0..model::ELEMENTS)
        .map(|index| index as f32 - 8.0)
        .collect();
    let right: Vec<f32> = (0..model::ELEMENTS)
        .map(|index| index as f32 * 0.5)
        .collect();
    let expected: Vec<f32> = left
        .iter()
        .zip(&right)
        .map(|(left, right)| left + right)
        .collect();
    let left = Tensor::from_array(([1usize, model::ELEMENTS], left.into_boxed_slice()))
        .map_err(|error| error.to_string())?;
    let right = Tensor::from_array(([1usize, model::ELEMENTS], right.into_boxed_slice()))
        .map_err(|error| error.to_string())?;
    let actual = {
        let outputs = session
            .run(ort::inputs!["left" => left, "right" => right])
            .map_err(|error| format!("NPU arithmetic execution: {error}"))?;
        let output = outputs.get("sum").ok_or("NPU graph did not return sum")?;
        let (shape, values) = output
            .try_extract_tensor::<f32>()
            .map_err(|error| format!("NPU output tensor: {error}"))?;
        if &shape[..] != [1, model::ELEMENTS as i64] {
            return Err(format!(
                "NPU arithmetic returned unexpected shape {shape:?}"
            ));
        }
        values.to_vec()
    };
    compare_output(&actual, &expected)?;
    let profile_path = session.end_profiling().map_err(|error| error.to_string())?;
    let executed_kernel_events = checked_profile(Path::new(&profile_path), scratch.path())?;
    Ok(NpuExecutionEvidence {
        actual_provider: PROVIDER.into(),
        target_device: "NPU".into(),
        device_id,
        device_vendor,
        workload: "ONNX Add, IR 8/opset 13, two float32 [1,16] inputs".into(),
        expected,
        actual,
        cpu_fallback_disabled: true,
        executed_kernel_events,
        placement_evidence: "runtime-enumerated OpenVINO NPU device selected exclusively; complete ORT kernel events attribute execution only to OpenVINOExecutionProvider".into(),
        runtime_path,
        plugin_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialized_runtime_does_not_prove_kernel_execution() {
        assert!(verified_profile_kernels(
            br#"[{"cat":"Session","name":"session_initialization","ph":"X","dur":1}]"#
        )
        .is_err());
    }

    #[test]
    fn cpu_fallback_and_unattributed_kernels_are_rejected() {
        for provider in [
            "CPUExecutionProvider",
            "CUDAExecutionProvider",
            "OpenVINOExecutionProvider.AUTO",
            "",
        ] {
            let profile = serde_json::json!([
                {"cat":"Node","name":"add_kernel_time","ph":"X","dur":1,"args":{"provider":provider}}
            ]);
            assert!(verified_profile_kernels(&serde_json::to_vec(&profile).unwrap()).is_err());
        }
        assert!(verified_profile_kernels(
            br#"[{"cat":"Node","name":"add_kernel_time","ph":"X","dur":1}]"#
        )
        .is_err());
    }

    #[test]
    fn complete_openvino_kernel_evidence_is_accepted() {
        let profile = br#"[
            {"cat":"Node","name":"add_fence_before","ph":"X","dur":0},
            {"cat":"Node","name":"fused_add_kernel_time","ph":"X","dur":12,"args":{"provider":"OpenVINOExecutionProvider"}}
        ]"#;
        assert_eq!(verified_profile_kernels(profile).unwrap(), 1);
    }

    #[test]
    fn mixed_provider_execution_cannot_pass() {
        let profile = br#"[
            {"cat":"Node","name":"add_kernel_time","ph":"X","dur":12,"args":{"provider":"OpenVINOExecutionProvider"}},
            {"cat":"Node","name":"extra_kernel_time","ph":"X","dur":1,"args":{"provider":"CPUExecutionProvider"}}
        ]"#;
        assert!(verified_profile_kernels(profile).is_err());
    }

    #[test]
    fn malformed_and_incomplete_profile_evidence_is_rejected() {
        for profile in [
            br#"{"#.as_slice(),
            br#"{}"#.as_slice(),
            br#"[]"#.as_slice(),
            br#"[{"cat":"Node","name":"add_kernel_time","ph":"B","dur":1,"args":{"provider":"OpenVINOExecutionProvider"}}]"#.as_slice(),
            br#"[{"cat":"Node","name":"add_kernel_time","ph":"X","dur":-1,"args":{"provider":"OpenVINOExecutionProvider"}}]"#.as_slice(),
        ] {
            assert!(verified_profile_kernels(profile).is_err());
        }
        assert!(verified_profile_kernels(&vec![b' '; PROFILE_LIMIT as usize + 1]).is_err());
    }

    #[test]
    fn incorrect_and_nonfinite_results_are_rejected() {
        assert!(compare_output(&[1.0], &[1.0, 2.0]).is_err());
        assert!(compare_output(&[2.0], &[1.0]).is_err());
        assert!(compare_output(&[f32::NAN], &[1.0]).is_err());
        assert!(compare_output(&[f32::INFINITY], &[1.0]).is_err());
        compare_output(&[-8.0, 14.5], &[-8.0, 14.5]).unwrap();
    }
}
