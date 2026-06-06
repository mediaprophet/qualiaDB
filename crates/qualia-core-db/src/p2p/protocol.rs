use async_trait::async_trait;
use libp2p::request_response::Codec;
use libp2p::StreamProtocol;
use serde::{Deserialize, Serialize};
use std::io;
use libp2p::futures::{AsyncRead, AsyncWrite, AsyncReadExt, AsyncWriteExt};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QualiaQuin {
    pub subject: [u8; 8],   
    pub predicate: [u8; 8], 
    pub object: [u8; 8],    
    pub context: [u8; 8],   
    pub clock_sig: [u8; 16], 
}
const _: () = assert!(std::mem::size_of::<QualiaQuin>() == 48);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualiaRequest {
    Handshake {
        // Flattened buffer containing sequences of (48-byte Quin + 64-byte Ed25519 Signature)
        compressed_vcs: Vec<u8>, 
    },
    Sync {
        hop_count: u8,
        gatekeeper_token: Option<String>,
        target_shapes: Vec<String>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualiaResponse {
    HandshakeAck {
        success: bool,
    },
    SyncAck {
        success: bool,
        message: String,
        blocks_sent: u64,
    }
}

#[derive(Clone, Default)]
pub struct QualiaSyncCodec;

#[async_trait]
impl Codec for QualiaSyncCodec {
    type Protocol = StreamProtocol;
    type Request = QualiaRequest;
    type Response = QualiaResponse;

    async fn read_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        ciborium::from_reader(&buf[..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut len_buf = [0u8; 4];
        io.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        
        let mut buf = vec![0u8; len];
        io.read_exact(&mut buf).await?;

        ciborium::from_reader(&buf[..])
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let mut buf = Vec::new();
        ciborium::into_writer(&req, &mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            
        let len = buf.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;
        io.write_all(&buf).await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let mut buf = Vec::new();
        ciborium::into_writer(&res, &mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            
        let len = buf.len() as u32;
        io.write_all(&len.to_be_bytes()).await?;
        io.write_all(&buf).await?;
        Ok(())
    }
}
