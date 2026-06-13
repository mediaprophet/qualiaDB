use dioxus::prelude::*;
use uuid::Uuid;
use web_sys::window;

pub fn use_telemetry() {
    use_effect(move || {
        let window = window().expect("should have a window in this context");
        let storage = window.local_storage().expect("should have local storage").unwrap();
        
        let mut device_id = storage.get_item("qualia_device_id").unwrap();
        if device_id.is_none() {
            let new_id = Uuid::new_v4().to_string();
            storage.set_item("qualia_device_id", &new_id).unwrap();
            device_id = Some(new_id);
        }
        
        // Telemetry WebSocket is disabled in Webizen Studio standalone environment
        // to prevent connection errors when the QualiaDB backend is not running.
        // In a full production node, this will connect to the local mesh router.
        web_sys::console::log_1(&"Telemetry initialized (Standalone Mode). WebSocket connection disabled.".into());
    });
}
