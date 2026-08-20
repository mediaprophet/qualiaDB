//! `Render.gpu_compute_dispatch` / `Render.gpu_compute_readback` invoke
//! handlers — expose WebGPU compute shaders to VibeScript (plan §7.3 W6).
//!
//! These are Tier-2 (cold construction): they marshal Vibe `Value` arguments
//! into `ComputeBinding` descriptors and dispatch to
//! `render::gpu::PortalGpu::compute_dispatch`. The compute pass itself is
//! recorded by `PortalGpu` with no heap growth in the hot encoding path.
//!
//! ## Invoke surface
//!
//! | ID | Arguments | Returns |
//! |----|-----------|---------|
//! | `Render.gpu_compute_dispatch` | `{ handle, wgsl, entry?, workgroups_x/y/z?, bindings, readback_binding?, readback_bytes? }` | `{ dispatched, dispatch_id }` |
//! | `Render.gpu_compute_readback` | `{ handle }` | `{ ready, output?, bytes? }` |
//!
//! `bindings` is a list of `{ binding, kind, data }` records where `kind` is
//! `"uniform"`, `"storage"`, or `"storage_rw"`, and `data` is a `[u8]` list
//! (empty for an output-only read-write buffer whose size is given by
//! `readback_bytes` at the readback binding).

use super::super::args;
use super::gpu::slot_with;
use poet_vibe::{Diagnostic, Span, Value};

#[cfg(not(target_arch = "wasm32"))]
use crate::render::gpu::{ComputeBinding, ComputeBufferKind};

/// Parse a binding kind string into a `ComputeBufferKind`.
#[cfg(not(target_arch = "wasm32"))]
fn parse_kind(s: &str) -> Result<ComputeBufferKind, String> {
    match s {
        "uniform" => Ok(ComputeBufferKind::Uniform),
        "storage" => Ok(ComputeBufferKind::StorageRead),
        "storage_rw" | "storage_read_write" | "read_write" => {
            Ok(ComputeBufferKind::StorageReadWrite)
        }
        other => Err(format!(
            "unknown binding kind '{other}' (expected uniform|storage|storage_rw)"
        )),
    }
}

/// `Render.gpu_compute_dispatch` — submit a WGSL compute dispatch.
pub fn gpu_compute_dispatch(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle")
            .ok_or_else(|| args::bad(span, "gpu_compute_dispatch needs { handle: u64 }"))?;
        let wgsl = args::rec_str(args, "wgsl")
            .ok_or_else(|| args::bad(span, "gpu_compute_dispatch needs { wgsl: string }"))?
            .to_string();
        let entry = args::rec_str(args, "entry").unwrap_or("main").to_string();
        let wx = args::rec_u64(args, "workgroups_x").unwrap_or(1) as u32;
        let wy = args::rec_u64(args, "workgroups_y").unwrap_or(1) as u32;
        let wz = args::rec_u64(args, "workgroups_z").unwrap_or(1) as u32;
        let readback_binding = args::rec_u64(args, "readback_binding").map(|n| n as u32);
        let readback_bytes = args::rec_u64(args, "readback_bytes").unwrap_or(0) as usize;

        let list = args::rec(args, "bindings")
            .and_then(args::list)
            .ok_or_else(|| args::bad(span, "gpu_compute_dispatch needs { bindings: [...] }"))?;

        // Marshal owned byte buffers + binding metadata first, then build the
        // borrowing `ComputeBinding` slice referencing the owned buffers. Both
        // live in this scope and outlive the synchronous `slot_with` closure.
        let mut owned: Vec<Vec<u8>> = Vec::with_capacity(list.len());
        let mut metas: Vec<(u32, ComputeBufferKind)> = Vec::with_capacity(list.len());
        for (i, entry_v) in list.iter().enumerate() {
            let binding = args::rec_u64(entry_v, "binding").ok_or_else(|| {
                args::bad(span, format!("bindings[{i}] needs {{ binding: u64 }}"))
            })?;
            let kind_str = args::rec_str(entry_v, "kind").unwrap_or("storage");
            let kind =
                parse_kind(kind_str).map_err(|e| args::bad(span, format!("bindings[{i}]: {e}")))?;
            let data = args::rec(entry_v, "data")
                .and_then(args::u8s)
                .ok_or_else(|| args::bad(span, format!("bindings[{i}] needs {{ data: [u8] }}")))?;
            owned.push(data);
            metas.push((binding as u32, kind));
        }
        let bindings: Vec<ComputeBinding<'_>> = owned
            .iter()
            .zip(metas.iter())
            .map(|(buf, (b, k))| ComputeBinding {
                binding: *b,
                kind: *k,
                data: buf.as_slice(),
            })
            .collect();

        let dispatch_id = slot_with(handle, |portal| {
            portal.compute_dispatch(
                &wgsl,
                &entry,
                [wx, wy, wz],
                &bindings,
                readback_binding,
                readback_bytes,
            )
        })
        .ok_or_else(|| args::bad(span, "gpu_compute_dispatch: invalid handle"))?
        .map_err(|e| args::bad(span, format!("gpu_compute_dispatch: {e}")))?;

        // `owned` / `bindings` dropped here, after the synchronous dispatch.
        drop(owned);

        Ok(args::record([
            ("dispatched", Value::Bool(true)),
            ("dispatch_id", Value::U64(dispatch_id)),
        ]))
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(
            span,
            "gpu_compute_dispatch requires native build with gpu-runtime",
        ))
    }
}

/// `Render.gpu_compute_readback` — poll the outstanding compute readback.
pub fn gpu_compute_readback(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let handle = args::rec_u64(args, "handle")
            .ok_or_else(|| args::bad(span, "gpu_compute_readback needs { handle: u64 }"))?;

        let result = slot_with(handle, |portal| portal.compute_readback())
            .ok_or_else(|| args::bad(span, "gpu_compute_readback: invalid handle"))?;

        match result {
            None => Ok(args::record([("ready", Value::Bool(false))])),
            Some(Err(e)) => Err(args::bad(span, format!("gpu_compute_readback: {e}"))),
            Some(Ok(bytes)) => Ok(args::record([
                ("ready", Value::Bool(true)),
                ("bytes", Value::U64(bytes.len() as u64)),
                (
                    "output",
                    Value::List(bytes.into_iter().map(|b| Value::U64(b as u64)).collect()),
                ),
            ])),
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (args, span);
        Err(args::bad(
            span,
            "gpu_compute_readback requires native build with gpu-runtime",
        ))
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use crate::gpu_context;
    use crate::poet_host::invoke::render::gpu::{gpu_destroy, gpu_init};

    fn snap() -> crate::poet_host::PoetSnapshot {
        crate::poet_host::PoetSnapshot::default()
    }

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    const VECTOR_ADD_WGSL: &str = r#"
@group(0) @binding(0) var<storage, read> a: array<f32>;
@group(0) @binding(1) var<storage, read> b: array<f32>;
@group(0) @binding(2) var<storage, read_write> out: array<f32>;
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    out[gid.x] = a[gid.x] + b[gid.x];
}
"#;

    #[test]
    fn g_gpu_compute_dispatch_direct_handler() {
        if gpu_context::try_shared_gpu().is_none() {
            eprintln!("[compute_handler_test] no GPU adapter — skipping");
            return;
        }
        let span = dummy_span();
        let init = gpu_init(
            &args::record([("width", Value::U64(32)), ("height", Value::U64(32))]),
            span,
        )
        .expect("gpu_init");
        let Value::U64(h) = args::rec(&init, "handle").unwrap() else {
            panic!("no handle")
        };

        // f32 little-endian bits: 1.0, 2.0, 3.0, 4.0
        let a: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
        let b: [f32; 4] = [10.0, 20.0, 30.0, 40.0];
        let a_bytes: Vec<u8> = bytemuck::cast_slice::<f32, u8>(&a).to_vec();
        let b_bytes: Vec<u8> = bytemuck::cast_slice::<f32, u8>(&b).to_vec();
        let a_val: Vec<Value> = a_bytes.iter().map(|&b| Value::U64(b as u64)).collect();
        let b_val: Vec<Value> = b_bytes.iter().map(|&b| Value::U64(b as u64)).collect();

        let dispatch_args = args::record([
            ("handle", Value::U64(*h)),
            ("wgsl", Value::String(VECTOR_ADD_WGSL.into())),
            ("entry", Value::String("main".into())),
            ("workgroups_x", Value::U64(4)),
            ("workgroups_y", Value::U64(1)),
            ("workgroups_z", Value::U64(1)),
            (
                "bindings",
                Value::List(vec![
                    args::record([
                        ("binding", Value::U64(0)),
                        ("kind", Value::String("storage".into())),
                        ("data", Value::List(a_val)),
                    ]),
                    args::record([
                        ("binding", Value::U64(1)),
                        ("kind", Value::String("storage".into())),
                        ("data", Value::List(b_val)),
                    ]),
                    args::record([
                        ("binding", Value::U64(2)),
                        ("kind", Value::String("storage_rw".into())),
                        ("data", Value::List(vec![])),
                    ]),
                ]),
            ),
            ("readback_binding", Value::U64(2)),
            ("readback_bytes", Value::U64(16)),
        ]);
        let d = gpu_compute_dispatch(&dispatch_args, span).expect("dispatch");
        assert_eq!(args::rec(&d, "dispatched"), Some(&Value::Bool(true)));

        let rb = gpu_compute_readback(&args::record([("handle", Value::U64(*h))]), span)
            .expect("readback");
        assert_eq!(args::rec(&rb, "ready"), Some(&Value::Bool(true)));
        let Value::List(out) = args::rec(&rb, "output").unwrap() else {
            panic!("no output")
        };
        let bytes: Vec<u8> = out
            .iter()
            .map(|v| {
                if let Value::U64(n) = v {
                    *n as u8
                } else {
                    panic!("non-u8 output")
                }
            })
            .collect();
        let floats: Vec<f32> = bytemuck::cast_slice::<u8, f32>(&bytes).to_vec();
        assert_eq!(floats, vec![11.0, 22.0, 33.0, 44.0]);

        gpu_destroy(&args::record([("handle", Value::U64(*h))]), span).expect("destroy");
    }

    #[test]
    fn g_gpu_compute_readback_no_pending() {
        if gpu_context::try_shared_gpu().is_none() {
            eprintln!("[compute_readback_test] no GPU adapter — skipping");
            return;
        }
        let span = dummy_span();
        let init = gpu_init(
            &args::record([("width", Value::U64(32)), ("height", Value::U64(32))]),
            span,
        )
        .expect("gpu_init");
        let Value::U64(h) = args::rec(&init, "handle").unwrap() else {
            panic!("no handle")
        };

        let rb = gpu_compute_readback(&args::record([("handle", Value::U64(*h))]), span)
            .expect("readback call");
        // No pending dispatch → ready: false.
        assert_eq!(args::rec(&rb, "ready"), Some(&Value::Bool(false)));

        gpu_destroy(&args::record([("handle", Value::U64(*h))]), span).expect("destroy");
    }

    #[test]
    fn g_gpu_compute_dispatch_invalid_handle_via_vibescript() {
        // VibeScript's `.field` is namespace resolution, not record field access,
        // so we cannot chain `gpu_init` → `gpu_compute_dispatch` in a single
        // script (W0 tests have the same constraint). Instead, verify the
        // dispatch invoke is reachable from VibeScript and returns a record
        // shape — the full chained pipeline is covered by the direct-handler
        // test above. An invalid handle yields an E100 diagnostic, which
        // `eval_fn` surfaces as an Err — confirming the dispatch arm is wired.
        let src = r#"
        requires [ capability("capability.invoke") ];
        effect fn go() {
            return capability.invoke("Render.gpu_compute_dispatch", {
                handle: 99999,
                wgsl: "@compute @workgroup_size(1) fn main() {}",
                entry: "main",
                bindings: [{ binding: 0, kind: "storage", data: [0] }]
            });
        }
        "#;
        let mut snap = snap();
        let result = snap.eval_fn(src, "go", vec![]);
        // Invalid handle → E100 diagnostic (the dispatch arm ran and reached
        // the slot lookup, which failed). This proves the VibeScript → invoke
        // → handler wiring is live.
        assert!(
            result.is_err(),
            "expected error for invalid handle, got {result:?}"
        );
        let err = result.unwrap_err();
        assert!(err.message.contains("invalid handle"), "got: {err:?}");
    }
}
