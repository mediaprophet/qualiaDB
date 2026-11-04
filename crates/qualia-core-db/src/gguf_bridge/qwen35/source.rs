use super::{Error, QTensorEngine};
use crate::gguf_bridge::ModelByteHandle;
use crate::inference::moe::ftw_loader::FtwModelPackage;
use etalon_inference::ordinary::qwen35::{self, weights::TensorBytes};
use etalon_model::inference::TensorInfo;
use std::sync::Arc;

pub(super) struct RetainedBytes {
    primary: ModelByteHandle,
    package: Option<Arc<FtwModelPackage>>,
    data_start: u64,
}
impl RetainedBytes {
    #[cfg(test)]
    pub(super) fn test_snapshot(primary: ModelByteHandle, data_start: u64) -> Self {
        Self {
            primary,
            package: None,
            data_start,
        }
    }
    pub(super) fn from_engine(engine: &QTensorEngine) -> Result<Self, Error> {
        engine
            .prepared_native_sources
            .as_ref()
            .ok_or(Error::MissingSource)?
            .validate(engine)
            .map_err(|_| Error::MissingSource)?;
        Ok(Self {
            primary: engine.clone_model_bytes().ok_or(Error::MissingSource)?,
            package: engine.ftw_package.clone(),
            data_start: engine.tensor_data_offset,
        })
    }

    pub(super) fn from_leases(
        engine: &QTensorEngine,
        primary: Arc<etalon_model_io::lease::ModelLease>,
        shards: &[Arc<etalon_model_io::lease::ModelLease>],
    ) -> Result<Self, Error> {
        let package = if let Some(package) = &engine.ftw_package {
            if shards.len() != package.manifest.shards.len() {
                return Err(Error::MissingSource);
            }
            let index = engine
                .tensor_index_cache
                .as_ref()
                .ok_or(Error::MissingSource)?;
            if index.entries != package.tensor_index.entries
                || index.hyperparams != package.tensor_index.hyperparams
                || index.moe_routing != package.tensor_index.moe_routing
                || engine.tensor_data_offset != package.tensor_index.tensor_data_start
            {
                return Err(Error::MissingSource);
            }
            if !shards
                .first()
                .is_some_and(|first| Arc::ptr_eq(first, &primary))
            {
                return Err(Error::MissingSource);
            }
            Some(Arc::new(
                package
                    .prepare_shards(
                        shards
                            .iter()
                            .cloned()
                            .map(ModelByteHandle::leased)
                            .collect(),
                    )
                    .map_err(|_| Error::MissingSource)?,
            ))
        } else {
            if !shards.is_empty() {
                return Err(Error::MissingSource);
            }
            let retained = Self::from_engine(engine)?;
            if !retained
                .primary
                .same_owner(&ModelByteHandle::leased(primary))
            {
                return Err(Error::MissingSource);
            }
            return Ok(retained);
        };
        Ok(Self {
            primary: ModelByteHandle::leased(primary),
            package,
            data_start: engine.tensor_data_offset,
        })
    }
}
impl TensorBytes for RetainedBytes {
    fn tensor_bytes(&self, info: &TensorInfo) -> Result<&[u8], qwen35::Error> {
        let len = crate::ggml_quants::tensor_byte_len(info).ok_or(qwen35::Error::Provider)?;
        if let Some(package) = &self.package {
            // Never reinterpret a missing FTW shard as bytes in the primary file.
            return package
                .fetch_by_global_offset(info.byte_offset, len)
                .ok_or(qwen35::Error::Provider);
        }
        crate::ggml_quants::fetch_tensor_bytes(&self.primary, self.data_start, info)
            .map_err(|_| qwen35::Error::Provider)
    }
}
