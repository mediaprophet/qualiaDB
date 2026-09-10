//! Two OS processes exchange an authenticated WSS datagram. Process evidence only.

#![cfg(not(target_arch = "wasm32"))]

use core::sync::atomic::{AtomicBool, Ordering};
use std::net::TcpListener;
use std::process::Command;
use std::time::Duration;

use super::wss::{self, ENV_ADDR, ENV_CHILD, ENV_SECRET};

static DONE: AtomicBool = AtomicBool::new(false);

pub fn local_wss_two_process_executed() -> bool {
    DONE.load(Ordering::SeqCst) || std::env::var(ENV_CHILD).ok().as_deref() == Some("1")
}

fn is_child() -> bool {
    std::env::var(ENV_CHILD).ok().as_deref() == Some("1")
}

fn child_main() -> Result<(), String> {
    let addr = std::env::var(ENV_ADDR).map_err(|e| e.to_string())?;
    let secret = std::env::var(ENV_SECRET).map_err(|e| e.to_string())?;
    let sock: std::net::SocketAddr = addr
        .parse()
        .map_err(|e: std::net::AddrParseError| e.to_string())?;
    let mut s = wss::dial(sock, secret.as_bytes()).map_err(|e| e.to_string())?;
    let (_, _, got) = wss::recv_datagram(&mut s).map_err(|e| e.to_string())?;
    if got != b"from-a" {
        return Err("unexpected payload".into());
    }
    wss::send_datagram(&mut s, 1, 1, b"from-b", true).map_err(|e| e.to_string())?;
    DONE.store(true, Ordering::SeqCst);
    Ok(())
}

fn parent_main() -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let secret = b"qdnf-wss-local-test-secret";
    let hub = std::thread::spawn(move || wss::serve_pair(listener, secret));
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let test_name = std::thread::current()
        .name()
        .ok_or("missing test name")?
        .to_owned();
    let mut child = Command::new(&exe)
        .args(["--exact", &test_name, "--nocapture", "--test-threads", "1"])
        .env(ENV_CHILD, "1")
        .env(ENV_ADDR, addr.to_string())
        .env(ENV_SECRET, core::str::from_utf8(secret).unwrap())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| e.to_string())?;
    std::thread::sleep(Duration::from_millis(50));
    let mut a = wss::dial(addr, secret).map_err(|e| e.to_string())?;
    wss::send_datagram(&mut a, 1, 1, b"from-a", true).map_err(|e| e.to_string())?;
    let (_, _, got) = wss::recv_datagram(&mut a).map_err(|e| e.to_string())?;
    if got != b"from-b" {
        let _ = child.kill();
        return Err("parent did not receive from-b".into());
    }
    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("child failed".into());
    }
    let _ = hub.join();
    DONE.store(true, Ordering::SeqCst);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_process_authenticated_wss() {
        if is_child() {
            child_main().expect("wss child");
            return;
        }
        parent_main().expect("wss parent");
        assert!(local_wss_two_process_executed());
    }
}
