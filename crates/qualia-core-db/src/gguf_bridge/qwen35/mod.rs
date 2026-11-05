//! Explicit developer/lab admission of one Qwen3.5 linear layer.
//! This is not whole-model decode admission. Packing and ordinary norm storage
//! are caller-selected source contracts; no exporter-version heuristic is used.
mod source;
#[cfg(not(target_arch = "wasm32"))]
pub mod ftw;
#[cfg(test)]
mod tests;

use super::QTensorEngine;
use etalon_inference::ordinary::qwen35::{self, weights::CpuByteHost, Host, WorkspaceLayout};
use etalon_model::inference::{
    qwen_roles::{Qwen35Packing, Qwen35Roles},
    TensorInfo,
};
use source::RetainedBytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryNorm {
    /// HF checkpoint stores gamma minus one.
    RawZeroCentered,
    /// Exporter already added one to the checkpoint weight.
    AlreadyFolded,
}

#[derive(Debug, Clone, Copy)]
pub struct Profile {
    pub packing: Qwen35Packing,
    pub ordinary_norm: OrdinaryNorm,
    pub epsilon: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    MissingSource,
    MissingNorm,
    AmbiguousNorm,
    Budget,
    Execution(qwen35::Error),
}
impl From<qwen35::Error> for Error {
    fn from(e: qwen35::Error) -> Self {
        Self::Execution(e)
    }
}

/// Cold owned execution buffers plus retained exact source owners. Dropping or
/// replacing the engine cannot redirect these descriptors to another model.
/// Only validated native prepared owners or explicit immutable leases enter.
pub struct PreparedLinearLayer {
    source: RetainedBytes,
    roles: Qwen35Roles,
    profile: Profile,
    norm: Vec<f32>,
    normalized: Vec<f32>,
    history: Vec<f32>,
    state: Vec<f32>,
    workspace: Vec<f32>,
    reserved_bytes: usize,
}

impl QTensorEngine {
    /// Explicit CPU lab operation; does not grant native Webizen inference
    /// permissions, install fallback policy, or admit the full architecture.
    /// Budget covers newly owned buffers and this owner; payload is retained,
    /// never copied. Existing source-loader memory is caller-owned separately.
    pub fn prepare_qwen35_linear_layer(
        &self,
        layer: u32,
        profile: Profile,
        byte_budget: usize,
    ) -> Result<PreparedLinearLayer, Error> {
        let metadata = self.qwen35_metadata_reservation()?;
        let available = byte_budget.checked_sub(metadata).ok_or(Error::Budget)?;
        let mut prepared = self.prepare_qwen35_with_factory(layer, profile, available, || {
            RetainedBytes::from_engine(self)
        })?;
        prepared.reserved_bytes += metadata;
        Ok(prepared)
    }

    /// Explicit lab custody: independently selected full-file identities must
    /// already have been verified by ModelLease. FTW leases use manifest order.
    /// Monolithic GGUF/P64 leases must already be bound by native preparation;
    /// this method never reparses an unbounded replacement directory.
    /// Does not activate the model or allocate a full-model KV/recurrent arena.
    pub fn prepare_qwen35_linear_from_leases(
        &self,
        layer: u32,
        profile: Profile,
        byte_budget: usize,
        primary: std::sync::Arc<etalon_model_io::lease::ModelLease>,
        shards: &[std::sync::Arc<etalon_model_io::lease::ModelLease>],
    ) -> Result<PreparedLinearLayer, Error> {
        let metadata = self.qwen35_metadata_reservation()?;
        let available = byte_budget.checked_sub(metadata).ok_or(Error::Budget)?;
        let mut prepared = self.prepare_qwen35_with_factory(layer, profile, available, || {
            RetainedBytes::from_leases(self, primary, shards)
        })?;
        prepared.reserved_bytes += metadata;
        Ok(prepared)
    }

    fn qwen35_metadata_reservation(&self) -> Result<usize, Error> {
        let directory = self
            .tensor_index_cache
            .as_ref()
            .ok_or(Error::MissingSource)?
            .entries
            .capacity()
            .checked_mul(128)
            .ok_or(Error::Budget)?;
        self.ftw_package
            .as_ref()
            .map_or(Some(0), |p| p.prepared_metadata_bytes())
            .ok_or(Error::Budget)?
            .checked_add(directory)
            .ok_or(Error::Budget)
    }

    #[cfg(test)]
    fn prepare_qwen35_with_source(
        &self,
        layer: u32,
        profile: Profile,
        byte_budget: usize,
        source: RetainedBytes,
    ) -> Result<PreparedLinearLayer, Error> {
        self.prepare_qwen35_with_factory(layer, profile, byte_budget, || Ok(source))
    }

    fn prepare_qwen35_with_factory(
        &self,
        layer: u32,
        profile: Profile,
        byte_budget: usize,
        source: impl FnOnce() -> Result<RetainedBytes, Error>,
    ) -> Result<PreparedLinearLayer, Error> {
        let index = self
            .tensor_index_cache
            .as_ref()
            .ok_or(Error::MissingSource)?;
        if layer >= index.hyperparams.n_layer {
            return Err(qwen35::Error::Shape.into());
        }
        let roles = Qwen35Roles::resolve(index, layer, profile.packing)
            .map_err(|e| Error::Execution(qwen35::Error::Roles(e)))?;
        let norm_info = ordinary_norm(index, layer)?;
        let layout = WorkspaceLayout::checked(roles.geometry)?;
        let hidden = roles.geometry.hidden;
        if norm_info.n_dims != 1 || norm_info.dims[0] != hidden as u64 {
            return Err(qwen35::Error::Shape.into());
        }
        if !profile.epsilon.is_finite() || profile.epsilon <= 0.0 {
            return Err(qwen35::Error::Shape.into());
        }
        let floats = hidden
            .checked_mul(2)
            .and_then(|n| n.checked_add(layout.history))
            .and_then(|n| n.checked_add(layout.recurrent))
            .and_then(|n| n.checked_add(layout.floats))
            .ok_or(Error::Budget)?;
        let required = floats
            .checked_mul(4)
            .and_then(|n| n.checked_add(std::mem::size_of::<PreparedLinearLayer>()))
            .ok_or(Error::Budget)?;
        if required > byte_budget {
            return Err(Error::Budget);
        }
        let source = source()?;
        let host = CpuByteHost { source: &source };
        let mut norm = vec![0.0; hidden];
        host.read(&norm_info, &mut norm)?;
        if !norm.iter().all(|v| v.is_finite()) {
            return Err(qwen35::Error::Provider.into());
        }
        let mut prepared = PreparedLinearLayer {
            source,
            roles,
            profile,
            norm,
            normalized: vec![0.0; hidden],
            history: vec![0.0; layout.history],
            state: vec![0.0; layout.recurrent],
            workspace: vec![0.0; layout.floats],
            reserved_bytes: required,
        };
        prepared.validate_weights(layout)?;
        Ok(prepared)
    }
}

impl PreparedLinearLayer {
    fn validate_weights(&mut self, layout: WorkspaceLayout) -> Result<(), Error> {
        use etalon_model::inference::qwen_roles::Qwen35Projections;
        self.roles
            .validate()
            .map_err(|e| Error::Execution(qwen35::Error::Roles(e)))?;
        let host = CpuByteHost {
            source: &self.source,
        };
        let admitted = match &self.roles.projections {
            Qwen35Projections::Separate { qkv, z, a, b } => {
                [qkv, z, a, b].into_iter().all(|p| host.admits(p))
            }
            Qwen35Projections::Fused { qkvz, ba, .. } => {
                [qkvz, ba].into_iter().all(|p| host.admits(p))
            }
        };
        if !admitted || !host.admits(&self.roles.out) {
            return Err(qwen35::Error::UnsupportedProjection.into());
        }
        let g = self.roles.geometry;
        for (info, count) in [
            (&self.roles.conv, layout.conv_weights),
            (&self.roles.a_log, g.value_heads),
            (&self.roles.dt_bias, g.value_heads),
            (&self.roles.norm, g.value_dim),
        ] {
            let out = &mut self.workspace[..count];
            out.fill(f32::NAN);
            host.read(info, out)?;
            if !out.iter().all(|v| v.is_finite()) {
                return Err(qwen35::Error::Provider.into());
            }
        }
        Ok(())
    }
    pub fn reserved_bytes(&self) -> usize {
        self.reserved_bytes
    }
    pub fn roles(&self) -> &Qwen35Roles {
        &self.roles
    }
    pub fn profile(&self) -> Profile {
        self.profile
    }
    pub fn recurrent_state(&self) -> &[f32] {
        &self.state
    }
    pub fn convolution_history(&self) -> &[f32] {
        &self.history
    }
    pub fn reset(&mut self) {
        self.history.fill(0.0);
        self.state.fill(0.0);
    }
    /// One linear-attention residual step, with no per-step allocation. FFN,
    /// full attention layers, tokenizer, sampling and policy remain outside.
    /// On error hidden/history/recurrent state remain unchanged.
    pub fn step(&mut self, hidden: &mut [f32]) -> Result<(), Error> {
        if hidden.len() != self.norm.len() || !hidden.iter().all(|x| x.is_finite()) {
            return Err(qwen35::Error::Shape.into());
        }
        qwen35::ordinary_rms_norm(
            hidden,
            &self.norm,
            self.profile.ordinary_norm == OrdinaryNorm::RawZeroCentered,
            self.profile.epsilon,
            &mut self.normalized,
        )?;
        qwen35::execute_normalized(
            &CpuByteHost {
                source: &self.source,
            },
            &self.roles,
            &self.normalized,
            hidden,
            &mut self.history,
            &mut self.state,
            self.profile.epsilon,
            &mut self.workspace,
        )?;
        Ok(())
    }
}

fn ordinary_norm(
    index: &crate::gguf_sharder::GgufTensorIndex,
    layer: u32,
) -> Result<TensorInfo, Error> {
    let mut found = None;
    let mut name = [0u8; 128];
    for writer in [
        etalon_model::inference::write_hf_layer_tensor_name,
        etalon_model::inference::write_blk_tensor_name,
    ] {
        let n = writer(layer, b"input_layernorm.weight", &mut name);
        let hash = etalon_model::gguf::fnv1a64(&name[..n]);
        for &(_, info) in index.entries.iter().filter(|(key, _)| *key == hash) {
            if found.is_some_and(|old| old != info) {
                return Err(Error::AmbiguousNorm);
            }
            found = Some(info);
        }
    }
    found.ok_or(Error::MissingNorm)
}
