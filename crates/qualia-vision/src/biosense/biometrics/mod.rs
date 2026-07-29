pub mod sface_embed_from_tensor;
pub mod template_hash;

pub use sface_embed_from_tensor::{sface_cosine, sface_embed_from_tensor, SFACE_EMBED_DIM};
pub use template_hash::{template_hash_from_roi, templates_match, BiometricTemplate};
