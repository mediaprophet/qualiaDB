//! Length-prefixed local-stream framing for later named-pipe / UDS backends.
//!
//! No OS handles. `windows.rs` and `unix.rs` implement [`StreamBytes`] and wrap
//! [`FramedStream`]. Profile is existing [`BearerProfile::LocalIpcV1`].

mod frame;

#[cfg(all(windows, target_arch = "x86_64"))]
mod windows;

#[cfg(unix)]
mod unix;

#[cfg(all(windows, target_arch = "x86_64"))]
pub use windows::{
    child_connect, framed_connect, framed_named_pipe_pair, named_pipe_pair, NamedPipeListener,
    NamedPipeStream, PipeName, PIPE_NAME_PREFIX,
};

#[cfg(unix)]
pub use unix::{UnixIpcListener, UnixStreamBytes, SOCKET_FILE};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod two_process;

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::contract::{check_frame_mtu, Bearer, BearerCapabilities, RecvMeta};
use super::lifecycle::{BearerLifecycle, BearerPhase};

pub use frame::{
    decode_stream_frame, encode_stream_frame, next_stream_frame, ReadCursor, WriteCursor,
    FRAME_CAP, LEN_SIZE, WIRE_CAP,
};

/// Non-blocking byte pipe. Named-pipe and UDS adapters implement this.
/// `Ok(n)` is accepted bytes; cursors advance only by that `n`.
pub trait StreamBytes {
    /// Write up to `buf.len()` bytes.
    fn write(&mut self, buf: &[u8]) -> Result<usize, QdnfError>;
    /// Read up to `buf.len()` bytes. `Ok(0)` is orderly EOF.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, QdnfError>;
    /// Close the byte pipe. Idempotent at this layer.
    fn shutdown(&mut self) -> Result<(), QdnfError>;
}

/// Connected length-prefixed stream. Partial writes stay in [`WriteCursor`].
pub struct FramedStream<T> {
    io: T,
    write: WriteCursor,
    read: ReadCursor,
    lifecycle: BearerLifecycle,
    local: ObservedLocator,
    peer: ObservedLocator,
    scope: ScopeEpoch,
}

impl<T: StreamBytes> FramedStream<T> {
    /// Ready stream. `mtu == 0` is Malformed; `mtu` above [`FRAME_CAP`] is Capacity.
    pub fn connect(
        io: T,
        local: ObservedLocator,
        peer: ObservedLocator,
        scope: ScopeEpoch,
        mtu: u16,
    ) -> Result<Self, QdnfError> {
        if mtu == 0 {
            return Err(QdnfError::Malformed);
        }
        if mtu as usize > FRAME_CAP {
            return Err(QdnfError::Capacity);
        }
        let mut lifecycle = BearerLifecycle::new(mtu);
        lifecycle.begin_init()?;
        lifecycle.finish_init()?;
        Ok(Self {
            io,
            write: WriteCursor::new(),
            read: ReadCursor::new(),
            lifecycle,
            local,
            peer,
            scope,
        })
    }

    /// Bearer capabilities. Ordered, no group delivery, no duplicates.
    pub fn capabilities(&self) -> BearerCapabilities {
        BearerCapabilities {
            profile: BearerProfile::LocalIpcV1,
            mtu: self.lifecycle.mtu(),
            group_delivery: false,
            ordered: true,
            may_duplicate: false,
        }
    }

    #[inline]
    /// Lifecycle phase and MTU owner.
    pub fn lifecycle(&self) -> &BearerLifecycle {
        &self.lifecycle
    }

    #[inline]
    /// Local adapter locator.
    pub fn locator(&self) -> ObservedLocator {
        self.local
    }

    #[inline]
    /// Observed peer locator. Never taken from a payload field.
    pub fn peer(&self) -> ObservedLocator {
        self.peer
    }

    #[inline]
    /// Pending write cursor (named-pipe / UDS flush sites).
    pub fn write_cursor(&self) -> &WriteCursor {
        &self.write
    }

    #[inline]
    /// Partial-read cursor.
    pub fn read_cursor(&self) -> &ReadCursor {
        &self.read
    }

    #[inline]
    /// Borrow the byte pipe. Named-pipe / UDS adapters keep OS state here.
    pub fn inner(&self) -> &T {
        &self.io
    }

    #[inline]
    /// Mutably borrow the byte pipe.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.io
    }

    /// Ready → Draining. New [`Bearer::send`] fails Closed; recv still drains.
    pub fn begin_drain(&mut self) -> Result<(), QdnfError> {
        self.lifecycle.begin_drain()
    }

    /// Flush an already-sealed write. Does not accept a new frame.
    pub fn drain_write(&mut self) -> Result<(), QdnfError> {
        match self.lifecycle.phase() {
            BearerPhase::Ready | BearerPhase::Draining => self.write.flush(&mut self.io),
            BearerPhase::Closed => Err(QdnfError::Closed),
            _ => Err(QdnfError::Conflict),
        }
    }
}

impl<T: StreamBytes> Bearer for FramedStream<T> {
    fn profile(&self) -> BearerProfile {
        BearerProfile::LocalIpcV1
    }

    fn mtu(&self) -> u16 {
        self.lifecycle.mtu()
    }

    fn scope(&self) -> ScopeEpoch {
        self.scope
    }

    fn send(&mut self, dest: &ObservedLocator, frame: &[u8]) -> Result<usize, QdnfError> {
        if self.lifecycle.phase() != BearerPhase::Ready {
            return Err(QdnfError::Closed);
        }
        if dest.as_slice() != self.peer.as_slice() {
            return Err(QdnfError::Unauthorized);
        }
        check_frame_mtu(frame.len(), self.lifecycle.mtu())?;
        self.write.seal(frame)?;
        self.write.flush(&mut self.io)?;
        Ok(frame.len())
    }

    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError> {
        match self.lifecycle.phase() {
            BearerPhase::Ready | BearerPhase::Draining => {}
            BearerPhase::Closed => return Err(QdnfError::Closed),
            _ => return Err(QdnfError::Conflict),
        }
        let n = self.read.recv(&mut self.io, out)?;
        Ok((
            n,
            RecvMeta {
                observed_source: self.peer,
                scope: self.scope,
                mtu: self.lifecycle.mtu(),
            },
        ))
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        self.write.abort();
        self.read.abort();
        self.lifecycle.shutdown()?;
        self.io.shutdown()
    }
}
