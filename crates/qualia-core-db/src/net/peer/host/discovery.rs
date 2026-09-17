//! QLink announce/accept with pre-auth budget and reachability challenges (E05.3).

use super::NativePeer;
use crate::net::qdnf::bearer::contract::Bearer;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::frame::{copy_payload, decode_frame, encode_frame, FrameHeader};
use crate::net::qdnf::link::{
    accept_beacon, insert_after_challenge, rotating_tag, Adjacency, AdjacencyState, Beacon,
    DiscoveryMode,
};
use crate::net::qdnf::registries::{FrameType, NextProtocol};
use crate::net::qdnf::types::{LinkId, ObservedLocator};

impl NativePeer {
    /// QLink beacon announce. Replaces mDNS. Tag is HMAC-bound, not a DID.
    pub fn announce(&mut self, dest: &ObservedLocator, now_unix: u64) -> Result<(), QdnfError> {
        let mut tag = [0u8; 16];
        rotating_tag(self.identity.digest().as_bytes(), 1, &mut tag)?;
        let beacon = Beacon {
            mode: DiscoveryMode::PrivatePairwise,
            tag,
            link_id: self.local_link,
            epoch: 1,
            expiry_unix: now_unix.saturating_add(60),
            mtu: self.bearer.mtu(),
        };
        let mut payload = [0u8; Beacon::WIRE_LEN];
        let pn = beacon.encode(&mut payload)?;
        let mut header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        header.source_link_id = self.local_link;
        header.payload_len = pn as u16;
        let mut wire = [0u8; 256];
        let n = encode_frame(&header, &payload[..pn], &mut wire)?;
        self.bearer.send(dest, &wire[..n])?;
        Ok(())
    }

    /// Accept using `now_unix = 1`. Callers with a clock use [`Self::accept_announce_at`].
    pub fn accept_announce(&mut self) -> Result<LinkId, QdnfError> {
        self.accept_announce_at(1)
    }

    /// Charge the pre-auth budget, reject expired beacons, then insert only after
    /// a locator-bound challenge. Link possession is not DID authority.
    pub fn accept_announce_at(&mut self, now_unix: u64) -> Result<LinkId, QdnfError> {
        let mut out = [0u8; 256];
        let (got, meta) = self.bearer.recv(&mut out)?;
        let (decoded, off, len) = decode_frame(&out[..got])?;
        if decoded.frame_type != FrameType::DiscoveryBeacon {
            return Err(QdnfError::Malformed);
        }
        let mut payload = [0u8; 64];
        let copied = copy_payload(&out[..got], off, len, &mut payload)?;
        let beacon = Beacon::decode(&payload[..copied])?;
        accept_beacon(&beacon, now_unix, &mut self.preauth)?;
        let mut secret = [0u8; 32];
        secret.copy_from_slice(&self.identity.digest().as_bytes()[..32]);
        let challenge = self.challenges.issue(
            self.bearer.scope(),
            beacon.link_id,
            meta.observed_source,
            now_unix,
            60,
            &secret,
        )?;
        insert_after_challenge(
            &mut self.neighbors,
            Adjacency {
                local: self.local_link,
                remote: beacon.link_id,
                observed_peer: meta.observed_source,
                state: AdjacencyState::Adjacent,
                generation: 1,
                mtu: beacon.mtu,
            },
            &challenge,
            self.bearer.scope(),
            now_unix,
        )?;
        Ok(beacon.link_id)
    }
}

#[cfg(test)]
mod tests {
    use super::super::NativePeer;
    use crate::net::qdnf::errors::QdnfError;
    use crate::net::qdnf::types::ScopeEpoch;

    #[test]
    fn expired_beacon_is_not_installed() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, mut b) = NativePeer::pair_ipc(b"did:q42:a", b"did:q42:b", scope, 1280).unwrap();
        let now = 1_700_000_000u64;
        a.announce(&b.locator(), now).unwrap();
        assert_eq!(
            b.accept_announce_at(now.saturating_add(120)).unwrap_err(),
            QdnfError::Expired
        );
        assert_eq!(b.neighbor_count(), 0);
    }
}
