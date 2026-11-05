use super::*;
use crate::p2p::connectivity::wss_tls::mint_ca;

struct TaskDropped(std::sync::mpsc::Sender<()>);

impl Drop for TaskDropped {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

#[test]
fn endpoints_own_listener_and_join_socket_tasks_before_release() {
    let ca = mint_ca().unwrap();
    let (client, server) = loopback_tls_h2_capsule(&ca).unwrap();
    let drive = Arc::downgrade(&client._drive);
    let addr = client._drive._listener.local_addr().unwrap();
    // The listener itself, rather than just a remembered port number, remains
    // reserved throughout the endpoint lifetime.
    let collision = std::net::TcpListener::bind(addr).unwrap_err();
    assert_eq!(collision.kind(), std::io::ErrorKind::AddrInUse);

    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (dropped_tx, dropped_rx) = std::sync::mpsc::channel();
    client._drive.rt.as_ref().unwrap().spawn(async move {
        let _guard = TaskDropped(dropped_tx);
        started_tx.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();

    drop(client);
    assert!(
        drive.upgrade().is_some(),
        "server must retain port/runtime ownership"
    );
    assert!(matches!(
        dropped_rx.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    // Last endpoint drop must join the worker which owns the task, without
    // waiting/retrying in the assertion for background teardown to catch up.
    drop(server);
    assert!(drive.upgrade().is_none());
    assert_eq!(dropped_rx.try_recv(), Ok(()));
}

#[test]
fn last_endpoint_can_be_dropped_inside_another_runtime() {
    let ca = mint_ca().unwrap();
    let (client, server) = loopback_tls_h2_capsule(&ca).unwrap();
    let (dropped_tx, dropped_rx) = std::sync::mpsc::channel();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    client._drive.rt.as_ref().unwrap().spawn(async move {
        let _guard = TaskDropped(dropped_tx);
        started_tx.send(()).unwrap();
        std::future::pending::<()>().await;
    });
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let outer = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    outer.block_on(async move {
        drop(client);
        drop(server);
    });
    assert_eq!(dropped_rx.try_recv(), Ok(()));
}
