use super::{attention, ffn, metadata::Profile, plan::Plan, ple, primitives::named_layer_tensor};
use crate::{
    gguf_bridge::{ModelByteHandle, QTensorEngine},
    gguf_sharder::GgufTensorIndex,
};

pub(crate) struct Session {
    pub(super) source: ModelByteHandle,
    pub(super) plan: Plan,
    pub(super) position: u32,
    pub(super) poisoned: bool,
    kv: Vec<f32>,
    written: Vec<Option<u32>>,
    per_layer: Vec<f32>,
    projected: Vec<f32>,
}
impl Session {
    pub(super) fn new(
        source: ModelByteHandle,
        index: GgufTensorIndex,
        p: Profile,
        context: usize,
        state_budget: usize,
    ) -> Result<Self, String> {
        let count = p.layers * p.ple;
        let metadata = index.entries.capacity()
            * std::mem::size_of::<(u64, etalon_model::inference::TensorInfo)>()
            + p.layers
                * (std::mem::size_of::<super::plan::Layer>()
                    + std::mem::size_of::<Option<u32>>()
                    + 3 * std::mem::size_of::<usize>()
                    + 1)
            + std::mem::size_of::<Self>();
        if count
            .checked_mul(8)
            .and_then(|n| n.checked_add(metadata))
            .is_none_or(|n| n > state_budget)
        {
            return Err("Gemma session workspace/metadata budget".into());
        }
        let plan = Plan::new(index, p, &source, context)?;
        let kv = vec![0.; plan.kv_floats];
        let written = vec![None; plan.layers.len()];
        Ok(Self {
            source,
            plan,
            position: 0,
            poisoned: false,
            kv,
            written,
            per_layer: vec![0.; count],
            projected: vec![0.; count],
        })
    }
    pub(crate) fn kv_cache_bytes(&self) -> usize {
        self.kv.len() * 4
    }
    pub(crate) fn reset(&mut self) {
        self.kv.fill(0.);
        self.written.fill(None);
        self.per_layer.fill(0.);
        self.projected.fill(0.);
        self.position = 0;
        self.poisoned = false;
    }
    pub(super) fn token(
        &mut self,
        e: &mut QTensorEngine,
        token: u32,
        position: u32,
        hidden: &mut [f32],
        a: &mut [f32],
        b: &mut [f32],
    ) -> u32 {
        let p = &self.plan.profile;
        if self.poisoned
            || self.position != position
            || position as usize >= self.plan.context
            || hidden.len() != p.hidden
            || a.len() < p.hidden
            || b.len() < p.hidden
        {
            self.poisoned = true;
            return 0;
        }
        self.poisoned = true;
        if self.execute(e, token, position, hidden, a, b).is_none() {
            return 0;
        }
        self.position += 1;
        self.poisoned = false;
        self.plan.profile.layers as u32
    }
    fn execute(
        &mut self,
        e: &mut QTensorEngine,
        token: u32,
        position: u32,
        hidden: &mut [f32],
        a: &mut [f32],
        b: &mut [f32],
    ) -> Option<()> {
        let p = &self.plan.profile;
        if u64::from(token) >= self.plan.index.token_embd?.dims[1] {
            return None;
        }
        if self
            .plan
            .index
            .dequantize_token_embedding_into(&self.source, token, hidden)
            != p.hidden
        {
            return None;
        }
        let scale = (p.hidden as f32).sqrt();
        for h in hidden.iter_mut() {
            *h *= scale;
        }
        ple::prepare(
            e,
            &self.plan,
            token,
            hidden,
            &mut self.per_layer,
            &mut self.projected,
        )?;
        for layer in 0..p.layers {
            attention::execute(
                e,
                &self.plan,
                layer,
                position,
                hidden,
                &mut self.kv,
                &mut self.written,
                a,
            )?;
            if !ffn::execute(
                e,
                &self.plan.index,
                layer as u32,
                &self.plan.layers[layer].tensors,
                hidden,
                a,
                b,
                p.epsilon,
            ) {
                return None;
            }
            ple::execute(e, &self.plan, layer, hidden, &self.per_layer, a)?;
            if let Some(info) =
                named_layer_tensor(&self.plan.index, layer as u32, b"layer_output_scale.weight")
            {
                let mut s = [0.];
                if e.dequant_norm_tensor_into(&info, &mut s) != 1 || !s[0].is_finite() {
                    return None;
                }
                for h in hidden.iter_mut() {
                    *h *= s[0];
                }
            }
            if hidden.iter().any(|h| !h.is_finite()) {
                return None;
            }
        }
        Some(())
    }
}
