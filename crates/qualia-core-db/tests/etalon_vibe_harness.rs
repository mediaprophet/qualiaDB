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

#[test]
fn cpu_etalon_nquin_wire_sieve_and_radix_receipt_qualification() {
    use etalon_nquin::{
        radix_sort_u64_cpu, sieve_eq_cpu, NQuinWire, QuinField, RadixSortOutcome, NQUIN_ALIGN,
        NQUIN_BYTES,
    };
    use qualia_core_db::wgsl_forge::ir::q42_bridge::{
        enumerate_etalon_eval_receipt_to_quins, qualify_nquin_wire_sieve_and_radix,
        EtalonNQuinEvalReceipt,
    };
    use qualia_core_db::NQuin;

    assert_eq!(NQUIN_BYTES, 48);
    assert_eq!(NQUIN_ALIGN, 16);
    assert_eq!(std::mem::size_of::<NQuin>(), 48);

    let mk_quin = |s: u64, p: u64, o: u64, c: u64, m: u64| NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: c,
        metadata: m,
        parity: NQuin::calculate_parity(s, p, o, c, m),
    };

    let quins = [
        mk_quin(0x11, 0x42, 0x900, 0x77, 0x01),
        mk_quin(0x12, 0x42, 0x100, 0x77, 0x02),
        mk_quin(0x13, 0x99, 0x500, 0x77, 0x03),
        mk_quin(0x14, 0x42, 0x300, 0x77, 0x04),
    ];
    let wire: Vec<NQuinWire> = quins
        .iter()
        .map(|q| NQuinWire::new(q.subject, q.predicate, q.object, q.context, q.metadata, q.parity))
        .collect();
    for w in &wire {
        assert!(w.is_parity_valid());
    }

    let mut sieve_buf = vec![NQuinWire::new(0, 0, 0, 0, 0, 0); wire.len()];
    let sieve_len = sieve_eq_cpu(&wire, QuinField::Predicate, 0x42, &mut sieve_buf);
    assert_eq!(sieve_len, 3);

    let mut radix_keys: Vec<u64> = wire.iter().map(|w| w.predicate).collect();
    let mut radix_indices: Vec<u32> = (0..wire.len() as u32).collect();
    assert_eq!(
        radix_sort_u64_cpu(&mut radix_keys, &mut radix_indices),
        RadixSortOutcome::Sorted { passes: 8 }
    );
    assert_eq!(radix_keys, vec![0x42, 0x42, 0x42, 0x99]);

    let receipt = qualify_nquin_wire_sieve_and_radix(
        qualia_core_db::q_hash("urn:qualia:etalon:backend:cpu-reference"),
        &quins,
        QuinField::Predicate,
        0x42,
        &sieve_buf[..sieve_len],
        &radix_keys,
        &radix_indices,
    );
    assert!(!receipt.disqualified);

    let mut receipt_quins = [mk_quin(0, 0, 0, 0, 0); 6];
    enumerate_etalon_eval_receipt_to_quins(&receipt, &mut receipt_quins);
    for q in &receipt_quins {
        assert!(q.verify_ecc_parity());
    }

    // Verify that any mismatch or noop backend is marked disqualified = true.
    let disqualified_sieve = qualify_nquin_wire_sieve_and_radix(
        qualia_core_db::q_hash("urn:qualia:etalon:backend:cpu-reference"),
        &quins,
        QuinField::Predicate,
        0x42,
        &sieve_buf[..sieve_len - 1],
        &radix_keys,
        &radix_indices,
    );
    assert!(disqualified_sieve.disqualified);
    assert!(EtalonNQuinEvalReceipt::new(0, 4, true, true, true).disqualified);

    record_check(
        "cpu_etalon_nquin_wire_sieve_and_radix_receipt_qualification",
        "cpu",
        json!({
            "nquin_bytes": NQUIN_BYTES,
            "nquin_align": NQUIN_ALIGN,
            "receipt_quins": 6,
            "parity_verified": true,
            "disqualified_on_mismatch": true,
        }),
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
