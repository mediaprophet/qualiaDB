//! QSession Bearer over an already-upgraded WSS stream.

#![cfg(not(target_arch = "wasm32"))]

use std::io::{Read, Write};

use crate::net::qdnf::bearer::contract::{check_frame_mtu, Bearer, RecvMeta};
use crate::net::qdnf::bearer::mtu::MIN_QDNF_MTU;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::wss::{
    map_io, recv_checked, seal_close, seal_datagram, FrameType, ReadCursor, WriteCursor,
};

pub struct WssBearer<S> {
    stream: S,
    write: WriteCursor,
    read: ReadCursor,
    _local: ObservedLocator,
    remote: ObservedLocator,
    scope: ScopeEpoch,
    circuit: u32,
    gen: u32,
    client: bool,
    closing: bool,
}

impl<S> WssBearer<S> {
    /// Creates a default `WssBearer` bound to circuit/generation `(1, 1)`.
    pub fn new(
        stream: S,
        local: ObservedLocator,
        remote: ObservedLocator,
        scope: ScopeEpoch,
        client: bool,
    ) -> Self {
        Self {
            stream,
            write: WriteCursor::new(),
            read: ReadCursor::new(),
            _local: local,
            remote,
            scope,
            circuit: 1,
            gen: 1,
            client,
            closing: false,
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
        if self.closing {
            return Err(QdnfError::Closed);
        }
        seal_datagram(&mut self.write, self.circuit, self.gen, frame, self.client)?;
        self.write.flush(&mut self.stream)?;
        self.stream.flush().map_err(map_io)?;
        Ok(frame.len())
    }

    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError> {
        let (typ, n) = recv_checked(
            &mut self.stream,
            &mut self.read,
            self.circuit,
            self.gen,
            Some(!self.client),
            out,
        )?;
        match typ {
            FrameType::Datagram => Ok((
                n,
                RecvMeta {
                    observed_source: self.remote,
                    scope: self.scope,
                    mtu: self.mtu(),
                },
            )),
            FrameType::Close => Err(QdnfError::Closed),
            FrameType::Auth => Err(QdnfError::Malformed),
        }
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        self.closing = true;
        seal_close(&mut self.write, self.circuit, self.gen, self.client)?;
        self.write.flush(&mut self.stream)?;
        self.stream.flush().map_err(map_io)?;
        Ok(())
    }
}
