//! Caller-budgeted cold state and allocation-free mixed-attention token execution.
//! A token commits only after every layer, final norm, and output projection.
use super::dense_host::DenseHost;
use super::{weights::DenseSource, Plan};
use etalon_inference::ordinary::qwen35::{self, Host};
use etalon_model::inference::{qwen_attention::QkNorm, TensorInfo};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    Budget,
    Shape,
    Invalidated,
    Position,
    ContextFull,
    Provider,
    Numerical,
    Canceled,
}
impl From<qwen35::Error> for SessionError {
    fn from(_: qwen35::Error) -> Self {
        Self::Provider
    }
}

#[derive(Default)]
struct LayerState {
    history: Vec<f32>,
    recurrent: Vec<f32>,
    keys: Vec<f32>,
    values: Vec<f32>,
}
struct Buffers {
    layers: Vec<LayerState>,
    workspace: Vec<f32>,
    normalized: Vec<f32>,
    norm: Vec<f32>,
    router: Vec<f32>,
    gate_up: Vec<f32>,
    gate: Vec<f32>,
    up: Vec<f32>,
    swiglu: Vec<f32>,
    expert: Vec<f32>,
    combined: Vec<f32>,
    logits: Vec<f32>,
}
#[derive(Default)]
struct Sequence {
    cursor: usize,
    valid: bool,
}
struct Token<'a> {
    sequence: &'a mut Sequence,
    committed: bool,
}
impl Token<'_> {
    fn commit(mut self) {
        self.sequence.cursor += 1;
        self.committed = true;
    }
    fn commit_partial(mut self) {
        self.committed = true;
    }
}
impl Drop for Token<'_> {
    fn drop(&mut self) {
        if !self.committed {
            self.sequence.valid = false;
        }
    }
}

pub struct Session {
    plan: Arc<Plan>,
    buffers: Buffers,
    sequence: Sequence,
    reserved_bytes: usize,
}
fn add(a: usize, b: usize) -> Result<usize, SessionError> {
    a.checked_add(b).ok_or(SessionError::Budget)
}
fn mul(a: usize, b: usize) -> Result<usize, SessionError> {
    a.checked_mul(b).ok_or(SessionError::Budget)
}
fn zeros(n: usize) -> Result<Vec<f32>, SessionError> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(n)
        .map_err(|_| SessionError::Budget)?;
    result.resize(n, 0.0);
    Ok(result)
}
pub(super) fn finite(values: &[f32]) -> Result<(), SessionError> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(SessionError::Numerical)
    }
}

impl Session {
    pub fn plan(&self) -> &Arc<Plan> {
        &self.plan
    }
    pub fn is_valid(&self) -> bool {
        self.sequence.valid
    }
    /// The byte budget covers newly allocated state/scratch and owner metadata.
    /// Immutable admitted weights remain owned by the separately budgeted plan.
    pub fn new(plan: Arc<Plan>, byte_budget: usize) -> Result<Self, SessionError> {
        let p = &plan.profile;
        if p.hidden == 0
            || p.max_context == 0
            || p.layers != plan.layers.len()
            || p.layers == 0
            || p.intermediate == 0
            || p.vocab == 0
        {
            return Err(SessionError::Shape);
        }
        let mut floats = 0;
        let mut workspace = 0;
        for layer in &plan.layers {
            let r = layer.attention.requirements(p.max_context)?;
            floats = add(floats, add(add(r.history, r.recurrent)?, mul(r.kv, 2)?)?)?;
            workspace = workspace.max(r.workspace);
        }
        // norm/normalized/expert/combined + gate_up/gate/up/swiglu.
        floats = add(
            floats,
            add(
                add(mul(p.hidden, 4)?, mul(p.intermediate, 5)?)?,
                add(add(p.experts, p.vocab)?, workspace)?,
            )?,
        )?;
        let reserved_bytes = add(
            add(
                mul(floats, 4)?,
                mul(p.layers, std::mem::size_of::<LayerState>())?,
            )?,
            std::mem::size_of::<Self>(),
        )?;
        if reserved_bytes > byte_budget {
            return Err(SessionError::Budget);
        }
        let mut layers = Vec::new();
        layers
            .try_reserve_exact(p.layers)
            .map_err(|_| SessionError::Budget)?;
        for layer in &plan.layers {
            let r = layer.attention.requirements(p.max_context)?;
            layers.push(LayerState {
                history: zeros(r.history)?,
                recurrent: zeros(r.recurrent)?,
                keys: zeros(r.kv)?,
                values: zeros(r.kv)?,
            });
        }
        let buffers = Buffers {
            layers,
            workspace: zeros(workspace)?,
            normalized: zeros(p.hidden)?,
            norm: zeros(p.hidden)?,
            router: zeros(p.experts)?,
            gate_up: zeros(mul(p.intermediate, 2)?)?,
            gate: zeros(p.intermediate)?,
            up: zeros(p.intermediate)?,
            swiglu: zeros(p.intermediate)?,
            expert: zeros(p.hidden)?,
            combined: zeros(p.hidden)?,
            logits: zeros(p.vocab)?,
        };
        Ok(Self {
            plan,
            buffers,
            sequence: Sequence {
                cursor: 0,
                valid: true,
            },
            reserved_bytes,
        })
    }
    pub fn reserved_bytes(&self) -> usize {
        self.reserved_bytes
    }
    /// Allocated keys and values, excluding recurrent state and scratch.
    pub fn kv_cache_bytes(&self) -> usize {
        // Construction checked the aggregate float/byte count against the
        // selected budget before allocation; these private slices never grow.
        self.buffers
            .layers
            .iter()
            .map(|state| (state.keys.len() + state.values.len()) * std::mem::size_of::<f32>())
            .sum()
    }
    pub fn position(&self) -> Result<usize, SessionError> {
        if self.sequence.valid {
            Ok(self.sequence.cursor)
        } else {
            Err(SessionError::Invalidated)
        }
    }
    pub fn logits(&self) -> Result<&[f32], SessionError> {
        self.position()?;
        Ok(&self.buffers.logits)
    }
    pub fn reset(&mut self) {
        for state in &mut self.buffers.layers {
            state.history.fill(0.0);
            state.recurrent.fill(0.0);
            state.keys.fill(0.0);
            state.values.fill(0.0);
        }
        let b = &mut self.buffers;
        for values in [
            &mut b.workspace,
            &mut b.normalized,
            &mut b.norm,
            &mut b.router,
            &mut b.gate_up,
            &mut b.gate,
            &mut b.up,
            &mut b.swiglu,
            &mut b.expert,
            &mut b.combined,
            &mut b.logits,
        ] {
            values.fill(0.0);
        }
        self.sequence = Sequence {
            cursor: 0,
            valid: true,
        };
    }
    pub fn embed(&self, token: u32, hidden: &mut [f32]) -> Result<(), SessionError> {
        self.position()?;
        let p = &self.plan.profile;
        if token as usize >= p.vocab || hidden.len() != p.hidden {
            return Err(SessionError::Shape);
        }
        let mut row = self.plan.embedding;
        let width = match row.encoding() {
            etalon_model::inference::WeightEncoding::IeeeF32 => 4,
            etalon_model::inference::WeightEncoding::IeeeF16
            | etalon_model::inference::WeightEncoding::BrainF16 => 2,
            _ => return Err(SessionError::Shape),
        };
        let offset = mul(mul(token as usize, p.hidden)?, width)?;
        row.byte_offset = row
            .byte_offset
            .checked_add(offset as u64)
            .ok_or(SessionError::Shape)?;
        row.n_dims = 1;
        row.dims = [p.hidden as u64, 0, 0, 0];
        let source = DenseSource(&self.plan.pkg);
        hidden.fill(f32::NAN);
        DenseHost { source: &source }.read(&row, hidden)?;
        finite(hidden)
    }
    /// Hidden initially contains the embedding for this position. A partial
    /// error or unwind poisons the entire sequence; callers must reset or drop it.
    pub fn step(
        &mut self,
        hidden: &mut [f32],
        position: usize,
    ) -> Result<(u32, f32), SessionError> {
        self.step_controlled(hidden, position, &|| false)
    }
    pub fn step_controlled(
        &mut self,
        hidden: &mut [f32],
        position: usize,
        canceled: &dyn Fn() -> bool,
    ) -> Result<(u32, f32), SessionError> {
        if !self.sequence.valid {
            return Err(SessionError::Invalidated);
        }
        if position != self.sequence.cursor {
            return Err(SessionError::Position);
        }
        if position >= self.plan.profile.max_context {
            return Err(SessionError::ContextFull);
        }
        if hidden.len() != self.plan.profile.hidden {
            return Err(SessionError::Shape);
        }
        let token = Token {
            sequence: &mut self.sequence,
            committed: false,
        };
        let result = execute(&self.plan, &mut self.buffers, hidden, position, canceled)?;
        token.commit();
        Ok(result)
    }
    /// Advance prompt state without projecting unused vocabulary logits.
    pub fn prefill_controlled(
        &mut self,
        hidden: &mut [f32],
        position: usize,
        canceled: &dyn Fn() -> bool,
    ) -> Result<(), SessionError> {
        if !self.sequence.valid {
            return Err(SessionError::Invalidated);
        }
        if position != self.sequence.cursor {
            return Err(SessionError::Position);
        }
        if position >= self.plan.profile.max_context {
            return Err(SessionError::ContextFull);
        }
        if hidden.len() != self.plan.profile.hidden {
            return Err(SessionError::Shape);
        }
        let token = Token {
            sequence: &mut self.sequence,
            committed: false,
        };
        execute_layers(&self.plan, &mut self.buffers, hidden, position, canceled)?;
        token.commit();
        Ok(())
    }
}

fn normalize(
    host: &impl Host,
    info: &TensorInfo,
    hidden: &[f32],
    norm: &mut [f32],
    normalized: &mut [f32],
    convention: QkNorm,
    epsilon: f32,
) -> Result<(), SessionError> {
    norm.fill(f32::NAN);
    host.read(info, norm)?;
    qwen35::ordinary_rms_norm(
        hidden,
        norm,
        convention == QkNorm::RawZeroCentered,
        epsilon,
        normalized,
    )?;
    Ok(())
}
fn execute(
    plan: &Plan,
    b: &mut Buffers,
    hidden: &mut [f32],
    position: usize,
    canceled: &dyn Fn() -> bool,
) -> Result<(u32, f32), SessionError> {
    let p = &plan.profile;
    let source = DenseSource(&plan.pkg);
    let host = DenseHost { source: &source };
    finite(hidden)?;
    execute_layers(plan, b, hidden, position, canceled)?;
    normalize(
        &host,
        &plan.final_norm,
        hidden,
        &mut b.norm,
        &mut b.normalized,
        p.ordinary_norm,
        p.epsilon,
    )?;
    let result = project_output(plan, &mut b.logits, &b.normalized)?;
    if canceled() {
        return Err(SessionError::Canceled);
    }
    Ok(result)
}
fn execute_layers(
    plan: &Plan,
    b: &mut Buffers,
    hidden: &mut [f32],
    position: usize,
    canceled: &dyn Fn() -> bool,
) -> Result<(), SessionError> {
    finite(hidden)?;
    for index in 0..plan.layers.len() {
        if canceled() {
            return Err(SessionError::Canceled);
        }
        execute_attention(plan, b, index, hidden, position)?;
        if canceled() {
            return Err(SessionError::Canceled);
        }
        execute_ffn(plan, b, index, hidden)?;
    }
    if canceled() {
        return Err(SessionError::Canceled);
    }
    Ok(())
}
fn execute_attention(
    plan: &Plan,
    b: &mut Buffers,
    index: usize,
    hidden: &mut [f32],
    position: usize,
) -> Result<(), SessionError> {
    let source = DenseSource(&plan.pkg);
    let host = DenseHost { source: &source };
    let layer = &plan.layers[index];
    let state = &mut b.layers[index];
    let p = &plan.profile;
    normalize(
        &host,
        &layer.input_norm,
        hidden,
        &mut b.norm,
        &mut b.normalized,
        p.ordinary_norm,
        p.epsilon,
    )?;
    layer.attention.execute(
        &host,
        &b.normalized,
        hidden,
        &mut state.history,
        &mut state.recurrent,
        &mut state.keys,
        &mut state.values,
        position,
        &p.rotary_inv_freq,
        p.epsilon,
        &mut b.workspace,
    )?;
    Ok(())
}
fn execute_ffn(
    plan: &Plan,
    b: &mut Buffers,
    index: usize,
    hidden: &mut [f32],
) -> Result<(), SessionError> {
    let source = DenseSource(&plan.pkg);
    let host = DenseHost { source: &source };
    let p = &plan.profile;
    normalize(
        &host,
        &plan.layers[index].post_norm,
        hidden,
        &mut b.norm,
        &mut b.normalized,
        p.ordinary_norm,
        p.epsilon,
    )?;
    super::session_ffn::execute(
        plan,
        index,
        &host,
        &b.normalized,
        &mut b.router,
        &mut b.gate_up,
        &mut b.gate,
        &mut b.up,
        &mut b.swiglu,
        &mut b.expert,
        &mut b.combined,
    )?;
    for (value, delta) in hidden.iter_mut().zip(&b.combined) {
        *value += delta;
    }
    finite(hidden)
}
fn project_output(
    plan: &Plan,
    logits: &mut [f32],
    hidden: &[f32],
) -> Result<(u32, f32), SessionError> {
    plan.output
        .gemv(&plan.pkg, hidden, logits)
        .map_err(|_| SessionError::Provider)?;
    finite(logits)?;
    let mut best = 0;
    for i in 1..logits.len() {
        if logits[i] > logits[best] {
            best = i;
        }
    }
    Ok((best as u32, logits[best]))
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

#[path = "session_lab.rs"]
mod lab;
