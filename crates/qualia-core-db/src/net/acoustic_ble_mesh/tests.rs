use super::*;

#[test]
fn test_mesh_network_manager_creation() {
    let manager = MeshNetworkManager::new();
    assert!(manager.get_network_status().total_nodes == 0);
}

#[test]
fn test_network_initialization() {
    let mut manager = MeshNetworkManager::new();
    let result = manager.initialize();
    assert!(result.is_ok());
}

#[test]
fn test_node_discovery() {
    let mut manager = MeshNetworkManager::new();
    manager.initialize().unwrap();

    let discovered_nodes = manager.discover_nodes().unwrap();
    assert!(discovered_nodes.len() > 0);

    let status = manager.get_network_status();
    assert!(status.total_nodes > 0);
}

#[test]
fn test_message_sending() {
    let mut manager = MeshNetworkManager::new();
    manager.initialize().unwrap();

    let message_id = manager
        .send_message(
            "test_destination".to_string(),
            vec![1, 2, 3, 4],
            MessagePriority::Normal,
        )
        .unwrap();

    assert!(!message_id.is_empty());
}

#[test]
fn test_performance_monitoring() {
    let manager = MeshNetworkManager::new();
    let stats = manager.get_performance_stats();
    assert_eq!(stats.total_messages, 0);
}
