//! QSession Bearer over an already-upgraded WSS stream.

#![cfg(not(target_arch = "wasm32"))]

use std::io::{Read, Write};

use crate::net::qdnf::bearer::contract::{check_frame_mtu, Bearer, RecvMeta};
use crate::net::qdnf::bearer::mtu::MIN_QDNF_MTU;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::wss::{recv_datagram, send_datagram};

pub struct WssBearer<S> {
    stream: S,
    _local: ObservedLocator,
    remote: ObservedLocator,
    scope: ScopeEpoch,
    client: bool,
}

impl<S> WssBearer<S> {
    pub fn new(
        stream: S,
        local: ObservedLocator,
        remote: ObservedLocator,
        scope: ScopeEpoch,
        client: bool,
    ) -> Self {
        Self {
            stream,
            _local: local,
            remote,
            scope,
            client,
        }
    }
}

impl<S: Read + Write> Bearer for WssBearer<S> {
    fn profile(&self) -> BearerProfile {
        BearerProfile::TlsWssTransitionV1
    }

    fn mtu(&self) -> u16 {
        MIN_QDNF_MTU
    }

    fn scope(&self) -> ScopeEpoch {
        self.scope
    }

    fn send(&mut self, dest: &ObservedLocator, frame: &[u8]) -> Result<usize, QdnfError> {
        check_frame_mtu(frame.len(), self.mtu())?;
        if dest.as_slice() != self.remote.as_slice() {
            return Err(QdnfError::Unauthorized);
        }
        send_datagram(&mut self.stream, 1, 1, frame, self.client).map_err(|_| QdnfError::Closed)?;
        Ok(frame.len())
    }

    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError> {
        let (_, _, p) = recv_datagram(&mut self.stream).map_err(|_| QdnfError::Closed)?;
        if p.len() > out.len() {
            return Err(QdnfError::Capacity);
        }
        out[..p.len()].copy_from_slice(&p);
        Ok((
            p.len(),
            RecvMeta {
                observed_source: self.remote,
                scope: self.scope,
                mtu: self.mtu(),
            },
        ))
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        Ok(())
    }
}
