use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceState {
    Starting,
    Ready,
    Degraded,
    Failed,
    Stopping,
    Stopped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationState {
    Queued,
    Running,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize)]
pub struct ServiceSnapshot {
    pub id: String,
    pub state: ServiceState,
    pub detail: String,
    pub updated_at: String,
    pub heartbeat_at: String,
    pub restart_count: u32,
}

#[derive(Clone, Debug, Serialize)]
pub struct OperationSnapshot {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub state: OperationState,
    pub stage: String,
    pub completed_units: u64,
    pub total_units: Option<u64>,
    pub error: Option<String>,
    pub started_at: String,
    pub updated_at: String,
}

#[derive(Default)]
struct SupervisorState {
    services: BTreeMap<String, ServiceSnapshot>,
    operations: BTreeMap<String, OperationSnapshot>,
}

#[derive(Clone, Default)]
pub struct DesktopSupervisor {
    state: Arc<RwLock<SupervisorState>>,
}

impl DesktopSupervisor {
    pub fn service_starting(&self, id: impl Into<String>, detail: impl Into<String>) {
        self.set_service(id, ServiceState::Starting, detail);
    }

    pub fn service_ready(&self, id: impl Into<String>, detail: impl Into<String>) {
        self.set_service(id, ServiceState::Ready, detail);
    }

    pub fn service_failed(&self, id: impl Into<String>, error: impl Into<String>) {
        self.set_service(id, ServiceState::Failed, error);
    }

    pub fn service_degraded(&self, id: impl Into<String>, detail: impl Into<String>) {
        self.set_service(id, ServiceState::Degraded, detail);
    }

    pub fn heartbeat(&self, id: &str) {
        if let Ok(mut state) = self.state.write() {
            if let Some(service) = state.services.get_mut(id) {
                let now = now();
                service.heartbeat_at = now.clone();
                service.updated_at = now;
            }
        }
    }

    pub fn start_operation(
        &self,
        kind: impl Into<String>,
        label: impl Into<String>,
    ) -> OperationHandle {
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = now();
        let snapshot = OperationSnapshot {
            id: id.clone(),
            kind: kind.into(),
            label: label.into(),
            state: OperationState::Running,
            stage: "starting".to_string(),
            completed_units: 0,
            total_units: None,
            error: None,
            started_at: timestamp.clone(),
            updated_at: timestamp,
        };
        if let Ok(mut state) = self.state.write() {
            state.operations.insert(id.clone(), snapshot);
            trim_completed_operations(&mut state.operations);
        }
        OperationHandle {
            supervisor: self.clone(),
            id,
            finished: false,
        }
    }

    pub fn services(&self) -> Vec<ServiceSnapshot> {
        self.state
            .read()
            .map(|state| state.services.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn operations(&self) -> Vec<OperationSnapshot> {
        self.state
            .read()
            .map(|state| state.operations.values().cloned().collect())
            .unwrap_or_default()
    }

    fn set_service(
        &self,
        id: impl Into<String>,
        service_state: ServiceState,
        detail: impl Into<String>,
    ) {
        let id = id.into();
        let timestamp = now();
        if let Ok(mut state) = self.state.write() {
            let restart_count = state
                .services
                .get(&id)
                .map(|service| {
                    if service.state == ServiceState::Failed
                        && service_state == ServiceState::Starting
                    {
                        service.restart_count.saturating_add(1)
                    } else {
                        service.restart_count
                    }
                })
                .unwrap_or(0);
            state.services.insert(
                id.clone(),
                ServiceSnapshot {
                    id,
                    state: service_state,
                    detail: detail.into(),
                    updated_at: timestamp.clone(),
                    heartbeat_at: timestamp,
                    restart_count,
                },
            );
        }
    }

    fn update_operation(&self, id: &str, update: impl FnOnce(&mut OperationSnapshot)) {
        if let Ok(mut state) = self.state.write() {
            if let Some(operation) = state.operations.get_mut(id) {
                update(operation);
                operation.updated_at = now();
            }
        }
    }
}

pub struct OperationHandle {
    supervisor: DesktopSupervisor,
    id: String,
    finished: bool,
}

impl OperationHandle {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn stage(&self, stage: impl Into<String>) {
        let stage = stage.into();
        self.supervisor
            .update_operation(&self.id, |operation| operation.stage = stage);
    }

    pub fn progress(&self, completed_units: u64, total_units: Option<u64>) {
        self.supervisor.update_operation(&self.id, |operation| {
            operation.completed_units = completed_units;
            operation.total_units = total_units;
        });
    }

    pub fn complete(mut self) {
        self.supervisor.update_operation(&self.id, |operation| {
            operation.state = OperationState::Completed;
            operation.stage = "completed".to_string();
        });
        self.finished = true;
    }

    pub fn fail(mut self, error: impl Into<String>) {
        let error = error.into();
        self.supervisor.update_operation(&self.id, |operation| {
            operation.state = OperationState::Failed;
            operation.stage = "failed".to_string();
            operation.error = Some(error);
        });
        self.finished = true;
    }
}

impl Drop for OperationHandle {
    fn drop(&mut self) {
        if !self.finished {
            self.supervisor.update_operation(&self.id, |operation| {
                operation.state = OperationState::Failed;
                operation.stage = "abandoned".to_string();
                operation.error = Some("operation ended without a result".to_string());
            });
        }
    }
}

fn trim_completed_operations(operations: &mut BTreeMap<String, OperationSnapshot>) {
    const MAX_OPERATIONS: usize = 256;
    while operations.len() > MAX_OPERATIONS {
        let removable = operations.iter().find_map(|(id, operation)| {
            matches!(
                operation.state,
                OperationState::Completed | OperationState::Failed | OperationState::Cancelled
            )
            .then(|| id.clone())
        });
        match removable {
            Some(id) => {
                operations.remove(&id);
            }
            None => break,
        }
    }
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_lifecycle_is_visible() {
        let supervisor = DesktopSupervisor::default();
        let operation = supervisor.start_operation("test", "Test operation");
        let id = operation.id().to_string();
        operation.stage("working");
        operation.progress(2, Some(4));
        operation.complete();

        let snapshot = supervisor
            .operations()
            .into_iter()
            .find(|operation| operation.id == id)
            .expect("operation snapshot");
        assert_eq!(snapshot.state, OperationState::Completed);
        assert_eq!(snapshot.completed_units, 2);
        assert_eq!(snapshot.total_units, Some(4));
    }

    #[test]
    fn dropped_operation_becomes_a_failure() {
        let supervisor = DesktopSupervisor::default();
        let id = {
            let operation = supervisor.start_operation("test", "Dropped operation");
            operation.id().to_string()
        };
        let snapshot = supervisor
            .operations()
            .into_iter()
            .find(|operation| operation.id == id)
            .expect("operation snapshot");
        assert_eq!(snapshot.state, OperationState::Failed);
        assert_eq!(snapshot.stage, "abandoned");
    }
}
