use super::*;
use crate::p64::integrity::crc32c;

pub const MODEL_ROUTING_EXTENSION_BYTES: usize = 784;
const MAGIC: &[u8; 8] = b"ETLMOE2\0";

impl ModelMetadata {
    pub fn extension_bytes(&self) -> usize {
        if self.moe_routing == Default::default() {
            MODEL_METADATA_EXTENSION_BYTES
        } else {
            MODEL_ROUTING_EXTENSION_BYTES
        }
    }

    pub fn write_extension_into(&self, out: &mut [u8]) -> Result<usize, PrepareModelError> {
        if self.moe_routing == Default::default() {
            return self.write_legacy_extension_into(out);
        }
        validate(self.moe_routing)?;
        let out = out
            .get_mut(..MODEL_ROUTING_EXTENSION_BYTES)
            .ok_or(PrepareModelError::BudgetExceeded)?;
        out.fill(0);
        out[..8].copy_from_slice(MAGIC);
        out[8..12].copy_from_slice(&2u32.to_le_bytes());
        out[12..16].copy_from_slice(&(MODEL_ROUTING_EXTENSION_BYTES as u32).to_le_bytes());
        self.write_legacy_extension_into(&mut out[16..752])?;
        let flags = u32::from(self.moe_routing.expert_count.is_some())
            | (u32::from(self.moe_routing.experts_per_token.is_some()) << 1);
        out[752..756].copy_from_slice(&flags.to_le_bytes());
        out[756..760].copy_from_slice(&self.moe_routing.expert_count.unwrap_or(0).to_le_bytes());
        out[760..764].copy_from_slice(
            &self
                .moe_routing
                .experts_per_token
                .unwrap_or(0)
                .to_le_bytes(),
        );
        let crc = crc32c(&out[..780]);
        out[780..784].copy_from_slice(&crc.to_le_bytes());
        Ok(out.len())
    }

    pub fn read_extension(bytes: &[u8]) -> Result<Self, PrepareModelError> {
        if bytes.len() == MODEL_METADATA_EXTENSION_BYTES {
            return Self::read_legacy_extension(bytes);
        }
        let bad = PrepareModelError::MetadataExtension;
        if bytes.len() != MODEL_ROUTING_EXTENSION_BYTES
            || bytes.get(..8) != Some(MAGIC.as_slice())
            || bytes[8..12] != 2u32.to_le_bytes()
            || bytes[12..16] != (MODEL_ROUTING_EXTENSION_BYTES as u32).to_le_bytes()
            || bytes[764..780].iter().any(|&b| b != 0)
            || crc32c(&bytes[..780]) != u32::from_le_bytes(bytes[780..784].try_into().unwrap())
        {
            return Err(bad);
        }
        let flags = u32::from_le_bytes(bytes[752..756].try_into().unwrap());
        let count = u32::from_le_bytes(bytes[756..760].try_into().unwrap());
        let top = u32::from_le_bytes(bytes[760..764].try_into().unwrap());
        if flags == 0
            || flags & !3 != 0
            || (flags & 1 == 0 && count != 0)
            || (flags & 2 == 0 && top != 0)
        {
            return Err(bad);
        }
        let mut metadata = Self::read_legacy_extension(&bytes[16..752])?;
        metadata.moe_routing = moe_routing::MoeRoutingSettings {
            expert_count: (flags & 1 != 0).then_some(count),
            experts_per_token: (flags & 2 != 0).then_some(top),
        };
        validate(metadata.moe_routing)?;
        Ok(metadata)
    }
}

pub(super) fn validate(settings: moe_routing::MoeRoutingSettings) -> Result<(), PrepareModelError> {
    settings
        .admit(settings.expert_count.unwrap_or(512) as usize)
        .map(|_| ())
        .map_err(|_| PrepareModelError::MetadataExtension)
}

#[cfg(test)]
mod tests;
