use super::{Profile, linear, plan};
use crate::gguf_bridge::{ModelByteHandle, QTensorEngine};
use crate::gguf_sharder::{GgufTensorIndex, LayerTensors};

pub(crate) struct Session {
    pub(super) source: ModelByteHandle,
    pub(super) index: GgufTensorIndex,
    pub(super) profile: Profile,
    layers: Vec<LayerTensors>,
    state: Vec<f32>,
    work: Workspace,
    pub(super) position: u32,
    pub(super) next_layer: usize,
    pub(super) poisoned: bool,
}
pub(super) struct Workspace {
    pub norm: Vec<f32>,
    pub input: Vec<f32>,
    pub output: Vec<f32>,
    pub qkv: Vec<f32>,
    pub activated: Vec<f32>,
    pub conv: Vec<f32>,
    pub z: Vec<f32>,
    pub q: Vec<f32>,
    pub k: Vec<f32>,
    pub alpha: Vec<f32>,
    pub beta_proj: Vec<f32>,
    pub a_log: Vec<f32>,
    pub dt: Vec<f32>,
    pub decay: Vec<f32>,
    pub beta: Vec<f32>,
    pub ssm_norm: Vec<f32>,
    pub recurrent: Vec<f32>,
    pub gated: Vec<f32>,
    pub gate: Vec<f32>,
    pub up: Vec<f32>,
}
impl Workspace {
    fn floats(p: Profile) -> usize {
        3 * p.hidden + 6 * p.channels + 3 * p.value + 2 * p.vh * p.kd + 6 * p.vh + p.vd + 2 * p.ff
    }
    fn new(p: Profile) -> Self {
        Self {
            norm: vec![0.; p.hidden],
            input: vec![0.; p.hidden],
            output: vec![0.; p.hidden],
            qkv: vec![0.; p.channels],
            activated: vec![0.; p.channels],
            conv: vec![0.; 4 * p.channels],
            z: vec![0.; p.value],
            q: vec![0.; p.vh * p.kd],
            k: vec![0.; p.vh * p.kd],
            alpha: vec![0.; p.vh],
            beta_proj: vec![0.; p.vh],
            a_log: vec![0.; p.vh],
            dt: vec![0.; p.vh],
            decay: vec![0.; p.vh],
            beta: vec![0.; p.vh],
            ssm_norm: vec![0.; p.vd],
            recurrent: vec![0.; p.value],
            gated: vec![0.; p.value],
            gate: vec![0.; p.ff],
            up: vec![0.; p.ff],
        }
    }
}
impl Session {
    pub(super) fn new(
        source: ModelByteHandle,
        index: GgufTensorIndex,
        p: Profile,
        budget: usize,
    ) -> Result<Self, String> {
        let linear_layers = p.layers - p.layers / p.interval;
        let floats = linear_layers
            .checked_mul(p.state_stride())
            .and_then(|n| n.checked_add(Workspace::floats(p)))
            .ok_or("Qwen state arithmetic")?;
        let metadata = index.entries.capacity()
            * std::mem::size_of::<(u64, etalon_model::inference::TensorInfo)>()
            + p.layers * std::mem::size_of::<LayerTensors>()
            + std::mem::size_of::<Self>();
        if floats
            .checked_mul(4)
            .and_then(|n| n.checked_add(metadata))
            .is_none_or(|n| n > budget)
        {
            return Err("Qwen recurrent/workspace budget exceeded".into());
        }
        let layers = plan::validate(&index, p, &source)?;
        Ok(Self {
            source,
            index,
            profile: p,
            layers,
            state: vec![0.; linear_layers * p.state_stride()],
            work: Workspace::new(p),
            position: 0,
            next_layer: 0,
            poisoned: false,
        })
    }
    pub(crate) fn reset(&mut self) {
        self.state.fill(0.);
        self.position = 0;
        self.next_layer = 0;
        self.poisoned = false;
    }
    pub(super) fn layer(
        &mut self,
        e: &mut QTensorEngine,
        layer: usize,
        position: u32,
        hidden: &mut [f32],
        scratch_a: &mut [f32],
        scratch_b: &mut [f32],
    ) -> bool {
        if self.poisoned
            || position != self.position
            || layer != self.next_layer
            || layer >= self.layers.len()
            || hidden.len() != self.profile.hidden
            || hidden.iter().any(|v| !v.is_finite())
            || e.kv_layout
                .is_none_or(|l| position >= l.max_context || l.int8 || l.dict_k != 0)
        {
            self.poisoned = true;
            return false;
        }
        // Panic/provider failure leaves this state unusable. No partial token may reach logits.
        self.poisoned = true;
        if self
            .execute(e, layer, position, hidden, scratch_a, scratch_b)
            .is_none()
        {
            return false;
        }
        self.next_layer += 1;
        if self.next_layer == self.layers.len() {
            self.next_layer = 0;
            self.position += 1;
        }
        self.poisoned = false;
        true
    }
    fn execute(
        &mut self,
        e: &mut QTensorEngine,
        layer: usize,
        position: u32,
        hidden: &mut [f32],
        a: &mut [f32],
        b: &mut [f32],
    ) -> Option<()> {
        let p = self.profile;
        let t = self.layers[layer];
        if p.linear(layer) {
            let ordinal = layer - layer / p.interval;
            let offset = ordinal * p.state_stride();
            linear::run(
                e,
                &self.index,
                p,
                &t,
                hidden,
                &mut self.state[offset..offset + p.state_stride()],
                &mut self.work,
            )?;
        } else {
            let n = e.dispatch_qwen35_gated_attention_layer(
                &self.index,
                layer as u32,
                position,
                hidden,
                p.hidden,
                &t,
                a,
                b,
            )?;
            if n != p.hidden {
                return None;
            }
            for (h, v) in hidden.iter_mut().zip(&a[..n]) {
                *h += v;
            }
        }
        let w = &mut self.work;
        linear::norm(
            e,
            &t.ffn_norm?,
            hidden,
            p.epsilon,
            &mut w.norm,
            &mut w.input,
        )?;
        if !e.dispatch_gemm_into(
            &self.index,
            &t.ffn_gate?,
            &w.input,
            &mut w.gate,
            p.hidden,
            p.ff,
        ) || !e.dispatch_gemm_into(&self.index, &t.ffn_up?, &w.input, &mut w.up, p.hidden, p.ff)
        {
            return None;
        }
        // Existing canonical SiLU primitive, followed by the architecture's product.
        // FFN can exceed convolution width, so reuse the caller's checked scratch.
        if a.len() < p.ff {
            return None;
        }
        qualia_inference_kernel::recurrent::silu_into(&w.gate, &mut a[..p.ff]).ok()?;
        for (v, u) in a[..p.ff].iter_mut().zip(&w.up) {
            *v *= u;
        }
        if !e.dispatch_gemm_into(
            &self.index,
            &t.ffn_down?,
            &a[..p.ff],
            &mut w.output,
            p.ff,
            p.hidden,
        ) {
            return None;
        }
        for (h, v) in hidden.iter_mut().zip(&w.output) {
            *h += v;
        }
        hidden.iter().all(|v| v.is_finite()).then_some(())
    }
}
