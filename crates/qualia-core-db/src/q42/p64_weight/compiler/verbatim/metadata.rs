use super::*;

pub(crate) fn prepare(index: &GgufTensorIndex, hparams: &mut [u8], manifolds: &mut [u8]) {
    let hp = P64HParams {
        n_layer: index.hyperparams.n_layer,
        n_embd: index.hyperparams.n_embd,
        n_head: index.hyperparams.n_head,
        n_kv_head: index.hyperparams.effective_n_kv_head(),
        vocab_size: index.vocab_dim() as u32,
        rope_freq_base: index.hyperparams.effective_rope_freq_base(),
        rope_scale: index.hyperparams.effective_rope_scale(),
        head_dim: index.hyperparams.head_dim,
        head_dim_swa: index.hyperparams.head_dim_swa,
        sliding_window: index.hyperparams.sliding_window,
        shared_kv_layers: index.hyperparams.shared_kv_layers,
        logit_softcap: index.hyperparams.logit_softcap,
        architecture: index.hyperparams.architecture,
        arch_flags: index.hyperparams.arch_flags,
        reserved: [0; 8],
    };
    hp.write_le(&mut hparams[..64]);
    for (layer, record) in manifolds.chunks_exact_mut(64).enumerate() {
        let coordinate = crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(
            layer.min(index.hyperparams.n_layer as usize) as u32,
            index.hyperparams.n_layer.max(1),
        );
        write_manifold_coordinate(&coordinate, record);
    }
}
