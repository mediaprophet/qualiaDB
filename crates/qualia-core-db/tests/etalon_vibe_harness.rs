#![cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]

use qualia_core_db::poet_host::VibeHost;
use serde_json::json;
use vibe::{DiagCode, Diagnostic, Value};

fn record_check(name: &str, execution_class: &str, details: serde_json::Value) {
    use std::io::Write;
    static EVENTS: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let Some(path) = std::env::var_os("QUALIA_ETALON_HARNESS_EVENTS") else {
        return;
    };
    let _guard = EVENTS.lock().expect("event writer lock");
    let mut event = serde_json::to_vec(&json!({
        "name": name, "execution_class": execution_class, "outcome": "passed", "details": details,
    }))
    .expect("event serialization");
    event.push(b'\n');
    let mut output = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open caller-selected event file");
    assert!(
        output.metadata().unwrap().len() + event.len() as u64 <= 1024 * 1024,
        "product evidence exceeds its one MiB budget"
    );
    output
        .write_all(&event)
        .expect("write product execution evidence");
}

fn invoke(host: &mut VibeHost, id: &str, args: &str) -> Result<Value, Diagnostic> {
    let source = format!(
        "requires [capability(\"capability.invoke\"), capability(\"{id}\")];\n\
         effect fn run() {{ return capability.invoke(\"{id}\", {args}); }}"
    );
    host.eval_fn(&source, "run", vec![])
}

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    let Value::Record(record) = value else {
        panic!("expected a record, got {value:?}");
    };
    record
        .get(key)
        .unwrap_or_else(|| panic!("missing {key}: {value:?}"))
}

#[test]
fn cpu_graph_transaction_seals_qualia_quins() {
    let mut host = VibeHost::with_seed(vec![]);
    let result = host
        .eval_fn(
            r#"
        requires [capability("graph.write")];
        effect fn run() {
            transaction {
                graph.stage(quin.statement(
                    subject: <https://example.org/etalon/s>,
                    predicate: <https://example.org/etalon/p>,
                    object: <https://example.org/etalon/o>,
                    context: <https://example.org/etalon/g>
                ));
                graph.commit()?;
            };
            return true;
        }
        "#,
            "run",
            vec![],
        )
        .expect("VibeScript graph transaction");
    assert_eq!(result, Value::Bool(true));
    assert_eq!(host.committed.len(), 1);
    assert_eq!(host.staged_count(), 0);
    let q = host.committed[0];
    assert_eq!(
        q.parity,
        qualia_core_db::NQuin::calculate_parity(
            q.subject,
            q.predicate,
            q.object,
            q.context,
            q.metadata,
        )
    );
    record_check(
        "cpu_graph_transaction_seals_qualia_quins",
        "cpu",
        json!({"committed_quins": 1, "parity_verified": true, "host": "detached VibeHost"}),
    );
}

#[test]
fn cpu_invalid_gpu_handle_is_rejected_through_vibe() {
    let mut host = VibeHost::with_seed(vec![]);
    let error = invoke(&mut host, "Render.gpu_compute_readback", "{handle: 999999}")
        .expect_err("an absent GPU handle must fail");
    assert_eq!(error.code, DiagCode::E100);
    assert!(error.message.contains("invalid handle"), "{error:?}");
    record_check(
        "cpu_invalid_gpu_handle_is_rejected_through_vibe",
        "cpu",
        json!({"diagnostic": "E100", "gpu_submission": false}),
    );
}

struct Portal {
    host: VibeHost,
    handle: u64,
    adapter: serde_json::Value,
}

impl Portal {
    fn new() -> Self {
        let mut host = VibeHost::with_seed(vec![]);
        let adapter = invoke(&mut host, "Render.gpu_adapter_info", "{}")
            .expect("adapter query through VibeScript");
        eprintln!("Etalon/Vibe adapter: {adapter:?}");
        if let Ok(expected) = std::env::var("QUALIA_WGPU_BACKEND") {
            assert_eq!(
                field(&adapter, "backend"),
                &Value::String(expected),
                "the requested backend must actually be exercised"
            );
        }
        assert_eq!(
            field(&adapter, "available"),
            &Value::Bool(true),
            "GPU coverage requires a real adapter; run cpu_ tests explicitly for CPU-only coverage"
        );
        assert_ne!(
            field(&adapter, "device_type"),
            &Value::String("cpu".into()),
            "software adapters do not establish hardware GPU coverage"
        );
        let init = invoke(
            &mut host,
            "Render.gpu_init",
            "{width: 32, height: 32, particle_cap: 1}",
        )
        .expect("GPU initialization through VibeScript");
        let Value::U64(handle) = field(&init, "handle") else {
            panic!("invalid GPU handle: {init:?}");
        };
        let adapter = json!({
            "name": format_value(field(&adapter, "device_name")),
            "backend": format_value(field(&adapter, "backend")),
            "device_type": format_value(field(&adapter, "device_type")),
            "driver": format_value(field(&adapter, "driver")),
            "driver_info": format_value(field(&adapter, "driver_info")),
        });
        Self {
            handle: *handle,
            host,
            adapter,
        }
    }

    fn call(&mut self, id: &str) -> Value {
        invoke(&mut self.host, id, &format!("{{handle: {}}}", self.handle))
            .unwrap_or_else(|error| panic!("{id}: {error:?}"))
    }

    fn dispatch(&mut self, a: &[f32], b: &[f32], entry: &str) -> Vec<f32> {
        assert_eq!(a.len(), b.len());
        let shader =
            serde_json::to_string(include_str!("etalon_vibe_harness/compute.wgsl")).unwrap();
        let args = format!(
            "{{handle: {}, wgsl: {shader}, entry: \"{entry}\", workgroups_x: {},
              bindings: [
                {{binding: 0, kind: \"storage\", data: {}}},
                {{binding: 1, kind: \"storage\", data: {}}},
                {{binding: 2, kind: \"storage_rw\", data: []}}
              ], readback_binding: 2, readback_bytes: {}}}",
            self.handle,
            a.len().div_ceil(64),
            bytes_literal(a),
            bytes_literal(b),
            a.len() * 4,
        );
        let submitted = invoke(&mut self.host, "Render.gpu_compute_dispatch", &args)
            .expect("GPU dispatch through VibeScript");
        assert_eq!(field(&submitted, "dispatched"), &Value::Bool(true));
        let readback = self.call("Render.gpu_compute_readback");
        assert_eq!(field(&readback, "ready"), &Value::Bool(true));
        assert_eq!(field(&readback, "bytes"), &Value::U64((a.len() * 4) as u64));
        let Value::List(output) = field(&readback, "output") else {
            panic!("missing output bytes: {readback:?}");
        };
        let bytes: Vec<u8> = output
            .iter()
            .map(|value| match value {
                Value::U64(n) => u8::try_from(*n).expect("readback byte"),
                _ => panic!("invalid byte: {value:?}"),
            })
            .collect();
        assert_eq!(bytes.len(), a.len() * 4);
        bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
            .collect()
    }
}

fn format_value(value: &Value) -> &str {
    match value {
        Value::String(value) => value,
        _ => panic!("adapter property is not a string: {value:?}"),
    }
}

impl Drop for Portal {
    fn drop(&mut self) {
        let _ = invoke(
            &mut self.host,
            "Render.gpu_destroy",
            &format!("{{handle: {}}}", self.handle),
        );
    }
}

fn bytes_literal(values: &[f32]) -> String {
    let bytes: Vec<u8> = values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    format!("{bytes:?}")
}

#[test]
fn gpu_compute_readback_matches_cpu_and_rejects_invalid_requests() {
    let mut portal = Portal::new();
    assert_eq!(
        field(&portal.call("Render.gpu_compute_readback"), "ready"),
        &Value::Bool(false)
    );
    for count in [1, 63, 64, 65, 257] {
        for offset in [0.0, 7.0] {
            let a: Vec<f32> = (0..count).map(|i| i as f32 * 0.25 + offset).collect();
            let b: Vec<f32> = (0..count).map(|i| 16.0 - i as f32 * 0.5).collect();
            let expected: Vec<f32> = a.iter().zip(&b).map(|(a, b)| a + b).collect();
            assert_eq!(
                portal.dispatch(&a, &b, "add"),
                expected,
                "count={count}, offset={offset}"
            );
            assert_eq!(
                field(&portal.call("Render.gpu_compute_readback"), "ready"),
                &Value::Bool(false)
            );
        }
    }
    let input = vec![3.0; 65];
    let expected: Vec<f32> = (0..65)
        .map(|i| if i % 2 == 0 { 6.0 } else { 0.0 })
        .collect();
    assert_eq!(portal.dispatch(&input, &input, "sparse"), expected);
    let invalid = format!(
        "{{handle: {}, wgsl: \"@compute @workgroup_size(1) fn main() {{}}\",
           bindings: [{{binding: 0, kind: \"invalid\", data: [0,0,0,0]}}]}}",
        portal.handle,
    );
    let error = invoke(&mut portal.host, "Render.gpu_compute_dispatch", &invalid).unwrap_err();
    assert_eq!(error.code, DiagCode::E100);
    assert!(error.message.contains("unknown binding kind"), "{error:?}");
    portal.call("Render.gpu_destroy");
    let error = invoke(
        &mut portal.host,
        "Render.gpu_compute_readback",
        &format!("{{handle: {}}}", portal.handle),
    )
    .unwrap_err();
    assert_eq!(error.code, DiagCode::E100);
    assert!(error.message.contains("invalid handle"), "{error:?}");
    record_check(
        "gpu_compute_readback_matches_cpu_and_rejects_invalid_requests",
        "gpu",
        json!({"adapter": portal.adapter, "vector_add_dispatches": 10,
            "boundary_element_counts": [1,63,64,65,257], "sparse_initialization_dispatches": 1,
            "cpu_oracle": "exact float32 addition", "destroyed_handle_rejected": true,
            "unsupported_binding_kind_rejected": true}),
    );
}
