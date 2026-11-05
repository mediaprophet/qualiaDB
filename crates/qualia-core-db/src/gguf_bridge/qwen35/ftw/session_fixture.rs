//! Tiny mapped FTW fixture with real attention weights and an intentionally
//! missing router. It fails after KV publication, not before token execution.
use super::*;
use crate::gguf_bridge::qwen35::ftw::{
    attention::AttentionPlan, weights::Nvfp4Projection, LayerPlan, Profile,
};
use crate::inference::moe::ftw_loader::FtwModelPackage;
use etalon_model::inference::{
    qwen_attention::{Qwen35AttentionGeometry, Qwen35AttentionRoles},
    qwen_roles::{Projection, Qwen35Geometry, Qwen35Packing},
};
use serde_json::json;

pub(super) struct Fixture {
    pub plan: Arc<Plan>,
    _directory: tempfile::TempDir,
}
impl Fixture {
    pub(super) fn path(&self) -> &std::path::Path {
        self._directory.path()
    }
}
struct Builder {
    bytes: Vec<u8>,
    tensors: Vec<serde_json::Value>,
}
impl Builder {
    fn add(&mut self, name: &str, dtype: &str, shape: &[u64], bytes: &[u8]) -> u64 {
        assert!(
            self.bytes.len().checked_add(bytes.len()).unwrap() <= 256 * 1024,
            "fixture byte budget"
        );
        let offset = self.bytes.len() as u64;
        self.tensors
            .push(json!({"name":name,"dtype":dtype,"shape":shape,
            "kind":"tensor","global_off":offset,"nbytes":bytes.len()}));
        self.bytes.extend_from_slice(bytes);
        offset
    }
    fn floats(&mut self, name: &str, rows: usize, cols: usize, identity: bool) -> TensorInfo {
        let mut values = vec![0f32; rows * cols];
        if identity {
            for i in 0..rows.min(cols) {
                values[i * cols + i] = 1.0;
            }
        }
        let bytes: Vec<u8> = values.into_iter().flat_map(f32::to_le_bytes).collect();
        let offset = self.add(name, "float32", &[rows as u64, cols as u64], &bytes);
        TensorInfo {
            byte_offset: offset,
            dims: [cols as u64, rows as u64, 0, 0],
            n_dims: 2,
            ggml_type: 0,
        }
    }
    fn quant(&mut self, name: &str, rows: usize, cols: usize) {
        self.add(
            &format!("{name}.weight"),
            "uint8",
            &[rows as u64, (cols / 2) as u64],
            &vec![0x22; rows * cols / 2],
        );
        self.add(
            &format!("{name}.weight_scale"),
            "float8_e4m3fn",
            &[rows as u64, (cols / 16) as u64],
            &vec![0x38; rows * cols / 16],
        );
        let globals: Vec<u8> = (0..rows)
            .flat_map(|_| half::f16::ONE.to_le_bytes())
            .collect();
        self.add(
            &format!("{name}.weight_global"),
            "float16",
            &[rows as u64],
            &globals,
        );
    }
}
pub(super) fn fixture() -> Fixture {
    make_fixture(false)
}
pub(super) fn complete_fixture() -> Fixture {
    make_fixture(true)
}
fn make_fixture(complete: bool) -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let mut b = Builder {
        bytes: Vec::new(),
        tensors: Vec::new(),
    };
    let q = b.floats("q", 32, 16, true);
    let k = b.floats("k", 16, 16, true);
    let v = b.floats("v", 16, 16, true);
    let out = b.floats("out", 16, 16, true);
    let norm_offset = b.add(
        "norm",
        "float32",
        &[16],
        &[1.0f32.to_le_bytes(); 16].concat(),
    );
    let norm = TensorInfo {
        byte_offset: norm_offset,
        dims: [16, 0, 0, 0],
        n_dims: 1,
        ggml_type: 0,
    };
    b.quant("output", 16, 16);
    b.quant("shared_up", 32, 16);
    b.quant("shared_down", 16, 16);
    let router = b.floats("router", 256, 16, false);
    let shared_gate = b.floats("shared_gate", 1, 16, false);
    if complete {
        for (name, dtype, shape, len, value) in [
            (
                "gate_up_packed",
                "uint8",
                vec![256, 32, 8],
                256 * 32 * 8,
                0x22,
            ),
            (
                "gate_up_scale",
                "float8_e4m3fn",
                vec![256, 32, 1],
                256 * 32,
                0x38,
            ),
            ("gate_up_global", "float16", vec![256, 32], 256 * 32 * 2, 0),
            ("down_packed", "uint8", vec![256, 16, 8], 256 * 16 * 8, 0x22),
            (
                "down_scale",
                "float8_e4m3fn",
                vec![256, 16, 1],
                256 * 16,
                0x38,
            ),
            ("down_global", "float16", vec![256, 16], 256 * 16 * 2, 0),
        ] {
            let mut bytes = vec![value; len];
            if dtype == "float16" {
                for pair in bytes.chunks_exact_mut(2) {
                    pair.copy_from_slice(&half::f16::ONE.to_le_bytes());
                }
            }
            b.add(&format!("{name}#L00000"), dtype, &shape, &bytes);
            b.tensors.last_mut().unwrap()["kind"] = json!("experts_bank");
        }
    }
    std::fs::write(directory.path().join("fixture.ftw"), &b.bytes).unwrap();
    std::fs::write(
        directory.path().join("freetoken_weight.json"),
        serde_json::to_vec(&json!({
            "format":"ftw","version":1,"total_bytes":b.bytes.len(),"tensors":b.tensors,
            "shards":[{"file":"fixture.ftw","global_off":0,"nbytes":b.bytes.len()}]
        }))
        .unwrap(),
    )
    .unwrap();
    std::fs::write(directory.path().join("config.json"),br#"{"model_type":"qwen2","hidden_size":16,"num_hidden_layers":1,"num_attention_heads":1,"num_key_value_heads":1,"head_dim":16}"#).unwrap();
    let pkg = Arc::new(FtwModelPackage::open_from_dir(directory.path()).unwrap());
    let geometry = Qwen35AttentionGeometry {
        hidden: 16,
        query_heads: 1,
        kv_heads: 1,
        head_dim: 16,
        rotary_dim: 2,
    };
    let projection = |weight| Projection {
        weight,
        row_scale: None,
    };
    let attention = AttentionPlan::Full(Qwen35AttentionRoles {
        geometry,
        q_gate: projection(q),
        k: projection(k),
        v: projection(v),
        out: projection(out),
        q_norm: norm,
        k_norm: norm,
        norm: QkNorm::FoldedMultiplier,
    });
    let invalid = TensorInfo {
        byte_offset: 1_000_000,
        dims: [16, 1, 0, 0],
        n_dims: 2,
        ggml_type: 0,
    };
    let layer = LayerPlan {
        attention,
        input_norm: norm,
        post_norm: norm,
        router: if complete { router } else { invalid },
        shared_gate,
        shared_up: Nvfp4Projection::admit(&pkg, "shared_up", 32, 16).unwrap(),
        shared_down: Nvfp4Projection::admit(&pkg, "shared_down", 16, 16).unwrap(),
    };
    let plan = Arc::new(Plan {
        output: Nvfp4Projection::admit(&pkg, "output", 16, 16).unwrap(),
        pkg,
        profile: Profile {
            hidden: 16,
            layers: 1,
            vocab: 16,
            intermediate: 16,
            experts: 256,
            topk: 1,
            max_context: 2,
            epsilon: 1e-6,
            linear_geometry: Qwen35Geometry {
                hidden: 16,
                key_heads: 1,
                value_heads: 1,
                key_dim: 16,
                value_dim: 16,
                kernel: 2,
                qk_width: 16,
                value_width: 16,
                channels: 48,
                history_floats: 48,
                state_floats: 256,
            },
            attention_geometry: geometry,
            rotary_inv_freq: vec![1.0],
            packing: Qwen35Packing::Separate,
            ordinary_norm: QkNorm::FoldedMultiplier,
        },
        layers: vec![layer],
        final_norm: norm,
        embedding: k,
        reserved_bytes: 0,
    });
    Fixture {
        plan,
        _directory: directory,
    }
}
