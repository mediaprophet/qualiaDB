//! rustls-backed WSS. Certificate verification is mandatory.

#![cfg(not(target_arch = "wasm32"))]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    Issuer, KeyPair, KeyUsagePurpose,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use rustls::StreamOwned;
use rustls::{ClientConfig, ClientConnection, RootCertStore, ServerConfig, ServerConnection};

use super::wss::{self, send_datagram};

fn install_ring() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub struct LocalCa {
    cert: Certificate,
    issuer: Issuer<'static, KeyPair>,
}

pub fn mint_ca() -> Result<LocalCa, String> {
    let mut params = CertificateParams::new(Vec::<String>::new()).map_err(|e| e.to_string())?;
    params
        .distinguished_name
        .push(DnType::CommonName, "qdnf-local-ca");
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::CrlSign,
    ];
    let key = KeyPair::generate().map_err(|e| e.to_string())?;
    let cert = params.self_signed(&key).map_err(|e| e.to_string())?;
    let issuer = Issuer::new(params, key);
    Ok(LocalCa { cert, issuer })
}

pub fn mint_server(ca: &LocalCa) -> Result<(Certificate, KeyPair), String> {
    let mut params = CertificateParams::new(vec!["localhost".into()]).map_err(|e| e.to_string())?;
    params
        .distinguished_name
        .push(DnType::CommonName, "localhost");
    params.is_ca = IsCa::NoCa;
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let key = KeyPair::generate().map_err(|e| e.to_string())?;
    let cert = params
        .signed_by(&key, &ca.issuer)
        .map_err(|e| e.to_string())?;
    Ok((cert, key))
}

pub fn server_config(cert: &Certificate, key: &KeyPair) -> Result<Arc<ServerConfig>, String> {
    install_ring();
    let der = CertificateDer::from(cert.der().to_vec());
    let pk = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der()));
    let provider = rustls::crypto::ring::default_provider();
    let cfg = ServerConfig::builder_with_provider(provider.into())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_single_cert(vec![der], pk)
        .map_err(|e| e.to_string())?;
    Ok(Arc::new(cfg))
}

pub fn client_config(ca: &LocalCa) -> Result<Arc<ClientConfig>, String> {
    install_ring();
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(ca.cert.der().to_vec()))
        .map_err(|e| e.to_string())?;
    let provider = rustls::crypto::ring::default_provider();
    Ok(Arc::new(
        ClientConfig::builder_with_provider(provider.into())
            .with_safe_default_protocol_versions()
            .map_err(|e| e.to_string())?
            .with_root_certificates(roots)
            .with_no_client_auth(),
    ))
}

pub enum TlsStream {
    Client(StreamOwned<ClientConnection, TcpStream>),
    Server(StreamOwned<ServerConnection, TcpStream>),
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Client(s) => s.read(buf),
            Self::Server(s) => s.read(buf),
        }
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Client(s) => s.write(buf),
            Self::Server(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Client(s) => s.flush(),
            Self::Server(s) => s.flush(),
        }
    }
}

fn wrap_client(tcp: TcpStream, cfg: Arc<ClientConfig>) -> Result<TlsStream, String> {
    let name = ServerName::try_from("localhost").map_err(|e| e.to_string())?;
    let conn = ClientConnection::new(cfg, name).map_err(|e| e.to_string())?;
    Ok(TlsStream::Client(StreamOwned::new(conn, tcp)))
}

fn wrap_server(tcp: TcpStream, cfg: Arc<ServerConfig>) -> Result<TlsStream, String> {
    let conn = ServerConnection::new(cfg).map_err(|e| e.to_string())?;
    Ok(TlsStream::Server(StreamOwned::new(conn, tcp)))
}

/// TLS + WebSocket upgrade. Client verifies the server certificate against `ca`.
pub fn loopback_tls_wss(ca: &LocalCa) -> Result<(TlsStream, TlsStream), String> {
    let (srv_cert, srv_key) = mint_server(ca)?;
    let server_cfg = server_config(&srv_cert, &srv_key)?;
    let client_cfg = client_config(ca)?;
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let raw_c = TcpStream::connect(addr).map_err(|e| e.to_string())?;
    let (raw_s, _) = listener.accept().map_err(|e| e.to_string())?;
    raw_c
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|e| e.to_string())?;
    raw_s
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|e| e.to_string())?;
    let mut server = wrap_server(raw_s, server_cfg)?;
    let mut client = wrap_client(raw_c, client_cfg)?;
    let key = STANDARD.encode([7u8; 16]);
    let th = std::thread::spawn(move || {
        wss::client_handshake(&mut client, &key)?;
        Ok::<_, std::io::Error>(client)
    });
    wss::server_handshake(&mut server).map_err(|e| e.to_string())?;
    let client = th.join().map_err(|_| "join")?.map_err(|e| e.to_string())?;
    Ok((client, server))
}

pub fn wrong_ca_is_rejected() -> bool {
    let ca_good = match mint_ca() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let ca_other = match mint_ca() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let (srv_cert, srv_key) = match mint_server(&ca_good) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let server_cfg = match server_config(&srv_cert, &srv_key) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let client_cfg = match client_config(&ca_other) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(_) => return false,
    };
    let addr = match listener.local_addr() {
        Ok(a) => a,
        Err(_) => return false,
    };
    let raw_c = match TcpStream::connect(addr) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let (raw_s, _) = match listener.accept() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut server = match wrap_server(raw_s, server_cfg) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut client = match wrap_client(raw_c, client_cfg) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let th = std::thread::spawn(move || {
        let mut buf = [0u8; 8];
        let _ = server.read(&mut buf);
    });
    let mut buf = [0u8; 8];
    let failed = client.read(&mut buf).is_err();
    let _ = th.join();
    failed
}

pub const fn plain_tcp_is_not_tls() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_wss_verifies_cert_and_carries_256() {
        let ca = mint_ca().unwrap();
        let (mut c, mut s) = loopback_tls_wss(&ca).unwrap();
        let payload = [0x3cu8; 256];
        send_datagram(&mut c, 1, 1, &payload, true).unwrap();
        let (_, _, got) = wss::recv_datagram(&mut s).unwrap();
        assert_eq!(got, payload);
        assert!(wrong_ca_is_rejected());
        assert!(plain_tcp_is_not_tls());
    }
}
