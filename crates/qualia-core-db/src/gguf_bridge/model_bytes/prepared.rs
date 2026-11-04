//! Cold immutable native backing. Fresh engines read retained lease/snapshot
//! owners through the same byte accessor used by every numerical consumer.
//! File-backed leases preserve lazy mapping and large-model streaming. Explicit
//! budgeted anonymous copies are an alternative, never an implicit fallback.
use super::super::*;
use crate::inference::moe::ftw_loader::FtwModelPackage;
use etalon_model_io::lease::ModelLease;
use sha2::{Digest, Sha256};

#[path = "prepared_metadata.rs"]
mod metadata;

#[derive(Clone)]
pub(crate) struct NativeModelSources {
    primary: ModelByteHandle,
    package: Option<Arc<FtwModelPackage>>,
    metadata: [u8; 32],
    digest: [u8; 32],
    settings_digest: Option<[u8; 32]>,
    residency_generation: Option<u64>,
}

fn snapshot(bytes: &[u8]) -> Result<ModelByteHandle, String> {
    if bytes.is_empty() {
        return Err("empty model snapshot".into());
    }
    let mut copy = memmap2::MmapMut::map_anon(bytes.len()).map_err(|error| error.to_string())?;
    copy.copy_from_slice(bytes);
    Ok(ModelByteHandle::snapshot(Arc::new(
        copy.make_read_only().map_err(|error| error.to_string())?,
    )))
}

impl QTensorEngine {
    /// Explicit snapshot option. The budget admits all copied payload plus
    /// conservative metadata/sort scratch. Existing device/KV budgets are separate.
    /// A snapshot identifies its consumed bytes, not file authenticity or coherent
    /// historical source revision under concurrent external file mutation.
    pub(crate) fn prepare_owned_native_model(
        &self,
        byte_budget: usize,
    ) -> Result<(QTensorEngine, NativeModelSources), String> {
        let mut required = self.prepared_metadata_reservation()?;
        if let Some(package) = &self.ftw_package {
            for index in 0..package.manifest.shards.len() {
                required = required
                    .checked_add(package.shard_bytes(index).ok_or("missing FTW shard")?.len())
                    .ok_or("FTW snapshot budget overflow")?;
            }
        } else {
            required = required
                .checked_add(self.model_bytes().ok_or("missing model backing")?.len())
                .ok_or("snapshot budget overflow")?;
        }
        if required > byte_budget {
            return Err("immutable model snapshot budget exceeded".into());
        }
        if let Some(package) = &self.ftw_package {
            let mut sources = Vec::with_capacity(package.manifest.shards.len());
            for index in 0..package.manifest.shards.len() {
                sources.push(snapshot(
                    package.shard_bytes(index).ok_or("missing FTW shard")?,
                )?);
            }
            let primary = sources.first().cloned().ok_or("empty FTW sources")?;
            let package = Arc::new(package.prepare_shards(sources)?);
            self.prepare_native_backing(primary, Some(package))
        } else {
            self.prepare_native_backing(
                snapshot(self.model_bytes().ok_or("missing model backing")?)?,
                None,
            )
        }
    }

    /// Preferred lazy file-backed path. Callers must supply actual verified
    /// exclusion leases (full-file identity and OS-backed writer exclusion), not
    /// claimed path/digest pairs. FTW leases follow the bound manifest shard order.
    pub(crate) fn prepare_leased_native_model(
        &self,
        primary: Arc<ModelLease>,
        shards: Vec<Arc<ModelLease>>,
        metadata_budget: usize,
    ) -> Result<(QTensorEngine, NativeModelSources), String> {
        if self.prepared_metadata_reservation()? > metadata_budget {
            return Err("prepared model metadata budget exceeded".into());
        }
        if let Some(package) = &self.ftw_package {
            let sources: Vec<_> = shards.into_iter().map(ModelByteHandle::leased).collect();
            let first = sources.first().cloned().ok_or("missing FTW leases")?;
            let supplied = ModelByteHandle::leased(primary);
            if !first.same_owner(&supplied) {
                return Err("FTW primary must retain first shard lease".into());
            }
            self.prepare_native_backing(first, Some(Arc::new(package.prepare_shards(sources)?)))
        } else {
            if !shards.is_empty() {
                return Err("unexpected companion leases".into());
            }
            self.prepare_native_backing(ModelByteHandle::leased(primary), None)
        }
    }

    fn prepared_metadata_reservation(&self) -> Result<usize, String> {
        let index = self
            .tensor_index_cache
            .as_ref()
            .ok_or("missing native tensor directory")?;
        let mut required = index
            .entries
            .capacity()
            .checked_mul(128)
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<NativeModelSources>()))
            .ok_or("model metadata budget overflow")?;
        if let Some(package) = &self.ftw_package {
            required = required
                .checked_add(
                    package
                        .prepared_metadata_bytes()
                        .ok_or("FTW metadata overflow")?,
                )
                .ok_or("FTW metadata overflow")?;
        }
        Ok(required)
    }

    fn prepare_native_backing(
        &self,
        primary: ModelByteHandle,
        package: Option<Arc<FtwModelPackage>>,
    ) -> Result<(QTensorEngine, NativeModelSources), String> {
        let mut engine = QTensorEngine::new();
        // Parse container directories from the immutable source. Safetensors/FTW
        // configuration is an owned parsed input, copied from the actual loader;
        // never resolve a mutable config filename again after preparation.
        let index = if package.is_none() && crate::p64_weight::has_p64_magic(&primary) {
            let p64 = crate::p64_weight::P64TensorIndex::from_p64(&primary)?;
            let index = p64.to_gguf_index();
            engine.p64_index = Some(p64);
            index
        } else if package.is_none() && primary.starts_with(b"GGUF") {
            crate::gguf_sharder::GgufTensorIndex::from_gguf(&primary)
        } else {
            self.tensor_index_cache
                .clone()
                .ok_or("missing bound model directory")?
        };
        if index.hyperparams.n_layer == 0 || index.hyperparams.n_embd == 0 {
            return Err("invalid prepared model directory".into());
        }
        index.hyperparams.decode_supported()?;
        engine.hyperparams = index.hyperparams;
        engine.tensor_data_offset = index.tensor_data_start;
        engine.tensor_index_cache = Some(index.clone());
        engine.max_tensor_bytes = index.max_layer_tensor_bytes;
        engine.prepared_native_bytes = Some(primary.clone());
        engine.ftw_package = package.clone();
        #[cfg(feature = "gpu-runtime")]
        engine.ensure_gemm_buffers(
            index
                .max_layer_tensor_bytes
                .max(4096)
                .min(MAX_WGPU_WEIGHT_STAGING),
            MAX_STACK_GEMM_OUT as u32,
        );
        engine.ensure_kv_cache(&index.hyperparams);
        engine.init_ssm_state(&index);
        #[cfg(feature = "gpu-runtime")]
        if package.is_none() {
            engine.mc8_upload_resident_logits(&index);
            if let Some(p64) = engine.p64_index.clone() {
                engine.build_ternary_ffn_resident(&p64);
            }
        }
        if engine.kv_layout().is_none() {
            return Err("prepared native KV admission failed".into());
        }
        let metadata = metadata::engine_metadata(&engine)?;
        let mut hash = Sha256::new();
        hash.update(b"qualia:immutable-native-model-closure:v1\0");
        hash.update(metadata);
        if let Some(package) = &package {
            hash.update([1]);
            hash.update(package.prepared_routing_digest());
            hash.update((package.manifest.shards.len() as u64).to_le_bytes());
            for index in 0..package.manifest.shards.len() {
                let bytes = package
                    .shard_bytes(index)
                    .ok_or("missing prepared FTW source")?;
                hash.update((bytes.len() as u64).to_le_bytes());
                hash.update(Sha256::digest(bytes));
            }
        } else {
            hash.update([0]);
            hash.update((primary.len() as u64).to_le_bytes());
            hash.update(Sha256::digest(&primary[..]));
        }
        Ok((
            engine,
            NativeModelSources {
                primary,
                package,
                metadata,
                digest: hash.finalize().into(),
                settings_digest: crate::llm_bench::prepared_settings::read(|settings| {
                    settings.digest()
                }),
                residency_generation: None,
            },
        ))
    }
}

impl NativeModelSources {
    pub(crate) fn digest(&self) -> [u8; 32] {
        self.digest
    }
    /// Equal hashes of newly mutable mappings cannot substitute retained ownership.
    pub(crate) fn validate(&self, engine: &QTensorEngine) -> Result<(), String> {
        if !engine
            .prepared_native_bytes
            .as_ref()
            .is_some_and(|bytes| bytes.same_owner(&self.primary))
        {
            return Err("prepared primary model owner changed".into());
        }
        match (&self.package, &engine.ftw_package) {
            (Some(expected), Some(actual)) if Arc::ptr_eq(expected, actual) => (),
            (None, None) => (),
            _ => return Err("prepared FTW owner changed".into()),
        }
        if metadata::engine_metadata(engine)? != self.metadata {
            return Err("prepared numeric directory changed".into());
        }
        Ok(())
    }
}

impl QTensorEngine {
    pub(crate) fn reuse_leased_native_model(
        &self,
        primary: &Arc<ModelLease>,
        shards: &[Arc<ModelLease>],
        metadata_budget: usize,
    ) -> Result<Option<NativeModelSources>, String> {
        if self.prepared_metadata_reservation()? > metadata_budget {
            return Err("prepared model metadata budget exceeded".into());
        }
        let Some(sources) = &self.prepared_native_sources else {
            return Ok(None);
        };
        let settings = crate::llm_bench::prepared_settings::read(|settings| settings.digest());
        if settings.is_none()
            || sources.settings_digest != settings
            || sources.residency_generation
                != Some(crate::inference::inference_agent::sticky_infer::current_residency_generation())
            || !sources
                .primary
                .same_owner(&ModelByteHandle::leased(primary.clone()))
        {
            return Ok(None);
        }
        let same_shards = match &sources.package {
            Some(package) => package.prepared_leases_match(shards),
            None => shards.is_empty(),
        };
        if !same_shards || sources.validate(self).is_err() {
            return Ok(None);
        }
        Ok(Some(sources.clone()))
    }

    pub(crate) fn retain_native_model_sources(
        &mut self,
        mut sources: NativeModelSources,
        generation: u64,
    ) -> Result<NativeModelSources, String> {
        sources.validate(self)?;
        sources.residency_generation = Some(generation);
        self.prepared_native_sources = Some(sources.clone());
        Ok(sources)
    }

    pub(crate) fn prepared_model_residency_generation(&self) -> Option<u64> {
        self.prepared_native_sources
            .as_ref()
            .and_then(|sources| sources.residency_generation)
    }
}
