//! Endpoint-only installation checks a received peer Finished, never a trust bool.
use super::*;
impl InstalledSessionKeys {
    pub(crate) fn from_received_finished(
        generation: Generation,
        send: &[u8; 32],
        recv: &[u8; 32],
        peer_finished_key: &[u8; 32],
        transcript: &StrongDigest,
        peer_is_initiator: bool,
        received: &StrongDigest,
    ) -> Result<Self, QdnfError> {
        crate::net::qdnf::crypto::finished::verify_finished(
            peer_finished_key,
            transcript,
            peer_is_initiator,
            received,
        )?;
        if generation == Generation::ZERO {
            return Err(QdnfError::Unauthorized);
        }
        if send == &[0; 32] || recv == &[0; 32] || send == recv {
            return Err(QdnfError::CryptoFailure);
        }
        Ok(Self {
            generation,
            send: *send,
            recv: *recv,
            confirmed: true,
        })
    }
}
