//! OS-native medication reminder delivery (Q6 MED-01..13 closeout).

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use chrono::Local;
use qualia_client_core::wellfair::med_reminders::DueMedReminder;
use tauri::{AppHandle, Manager};
use tauri::plugin::PermissionState;
use tauri_plugin_notification::NotificationExt;

use crate::commands::HostApiState;

/// Tracks slots already notified today to avoid duplicate OS toasts.
#[derive(Default)]
pub struct MedReminderNotifierState {
    day_key: Mutex<String>,
    notified_slots: Mutex<HashSet<String>>,
}

impl MedReminderNotifierState {
    fn reset_if_new_day(&self) {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let mut day = self.day_key.lock().expect("notifier day lock");
        if *day != today {
            *day = today;
            self.notified_slots.lock().expect("notifier slots lock").clear();
        }
    }

    fn mark_notified(&self, key: &str) -> bool {
        self.reset_if_new_day();
        self.notified_slots
            .lock()
            .expect("notifier slots lock")
            .insert(key.to_string())
    }
}

pub fn request_os_notification_permission(app: &AppHandle) -> Result<bool, String> {
    let permitted = app
        .notification()
        .permission_state()
        .map_err(|e| e.to_string())?;
    if permitted == PermissionState::Granted {
        return Ok(true);
    }
    app.notification()
        .request_permission()
        .map(|state| state == PermissionState::Granted)
        .map_err(|e| e.to_string())
}

pub fn show_med_reminder(app: &AppHandle, due: &DueMedReminder) -> Result<(), String> {
    let title = "WellFair — medication reminder";
    let body = if due.minutes_until_due < 0 {
        format!(
            "Overdue: take {} (scheduled {})",
            due.medication_name, due.schedule_slot
        )
    } else if due.minutes_until_due == 0 {
        format!("Time to take {} now", due.medication_name)
    } else {
        format!(
            "Upcoming: {} in {} min (scheduled {})",
            due.medication_name, due.minutes_until_due, due.schedule_slot
        )
    };
    app.notification()
        .builder()
        .title(title)
        .body(body)
        .show()
        .map_err(|e| e.to_string())
}

fn slot_key(due: &DueMedReminder) -> String {
    let today = Local::now().format("%Y-%m-%d");
    format!("{today}:{}:{}", due.medication_id, due.schedule_slot)
}

pub fn poll_and_notify(app: &AppHandle) -> Result<usize, String> {
    let host_state = app.try_state::<HostApiState>().ok_or("HostApiState missing")?;
    let guard = host_state.0.lock().map_err(|e| e.to_string())?;
    let host = guard
        .as_ref()
        .ok_or_else(|| "Host API not initialized".to_string())?;
    let due_list = host.list_due_med_reminders(15)?;
    if due_list.is_empty() {
        return Ok(0);
    }

    let notifier = app
        .try_state::<MedReminderNotifierState>()
        .ok_or("MedReminderNotifierState missing")?;
    let mut shown = 0usize;
    for due in &due_list {
        let key = slot_key(due);
        if notifier.mark_notified(&key) {
            show_med_reminder(app, due)?;
            shown += 1;
        }
    }
    Ok(shown)
}

pub fn spawn_med_reminder_poller(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            if let Err(e) = poll_and_notify(&app) {
                if !e.contains("not initialized") {
                    eprintln!("med reminder poll: {e}");
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_key_is_stable_per_day() {
        let due = DueMedReminder {
            medication_id: "urn:med:1".into(),
            medication_name: "Metformin".into(),
            schedule_slot: "08:00".into(),
            minutes_until_due: 0,
        };
        let k1 = slot_key(&due);
        let k2 = slot_key(&due);
        assert_eq!(k1, k2);
        assert!(k1.contains("urn:med:1"));
    }
}