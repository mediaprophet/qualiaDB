use super::{E, Intent, types::PROFILE};
use crate::crypto::network::transcript::Transcript;
pub(super) fn client(
    intent: Intent,
    nonce: &[u8; 32],
    offer: &[u8; 8],
    share: &[u8],
) -> Result<Transcript, E> {
    let mut t = Transcript::new();
    let mut bytes = [0; super::intent::INTENT_BYTES];
    intent.encode(&mut bytes)?;
    t.append(b"endpoint-profile", PROFILE)?;
    t.append(b"intent", &bytes)?;
    t.append(b"client-nonce", nonce)?;
    t.append(b"client-offer", offer)?;
    t.append(b"initiator-share", share)?;
    Ok(t)
}
