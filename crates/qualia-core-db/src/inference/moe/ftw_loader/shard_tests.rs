use super::*;

#[test]
fn expert_components_keep_their_own_shard_owners() {
    let dir = tempfile::tempdir().unwrap();
    let mut tensors = Vec::new();
    let mut first = Vec::new();
    let mut second = Vec::new();
    for (i, name) in [
        "gate_up_packed",
        "gate_up_scale",
        "gate_up_global",
        "down_packed",
        "down_scale",
        "down_global",
    ]
    .iter()
    .enumerate()
    {
        let global = if i < 3 {
            first.len()
        } else {
            768 + second.len()
        };
        let target = if i < 3 { &mut first } else { &mut second };
        target.extend(std::iter::repeat_n((i + 1) as u8, 256));
        tensors.push(serde_json::json!({"name":format!("{name}#L00000"),"kind":"experts_bank","dtype":"uint8","shape":[256,1],"global_off":global,"nbytes":256}));
    }
    std::fs::write(dir.path().join("first.ftw"), first).unwrap();
    std::fs::write(dir.path().join("second.ftw"), second).unwrap();
    let manifest = serde_json::json!({"format":"freetoken_weight","version":1,"total_bytes":1536,"expert_bank_num_layers":1,"tensors":tensors,"shards":[{"file":"first.ftw","global_off":0,"nbytes":768},{"file":"second.ftw","global_off":768,"nbytes":768}]});
    std::fs::write(
        dir.path().join("freetoken_weight.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    std::fs::write(dir.path().join("config.json"),r#"{"model_type":"qwen2","text_config":{"hidden_size":16,"head_dim":8,"num_hidden_layers":1,"num_attention_heads":2,"num_key_value_heads":1}}"#).unwrap();
    let pkg = FtwModelPackage::open_from_dir(dir.path()).unwrap();
    for expert in [0, 127, 255] {
        let view = pkg.get_expert_data(0, expert).unwrap();
        assert_eq!(view.gate_up_packed, [1]);
        assert_eq!(view.gate_up_scale, [2]);
        assert_eq!(view.gate_up_global, [3]);
        assert_eq!(view.down_packed, [4]);
        assert_eq!(view.down_scale, [5]);
        assert_eq!(view.down_global, [6]);
    }
}
