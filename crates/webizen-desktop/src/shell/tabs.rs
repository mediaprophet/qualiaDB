use std::sync::Mutex;
use std::collections::HashMap;

pub type TabId = u32;

#[derive(Clone, Debug)]
pub struct TabInfo {
    pub id: TabId,
    pub title: String,
    pub url: String,
    pub qapp_id: String,
    pub loading: bool,
}

pub struct TabManager {
    tabs: Mutex<HashMap<TabId, TabInfo>>,
    active: Mutex<TabId>,
    next_id: Mutex<u32>,
}

impl Default for TabManager {
    fn default() -> Self {
        Self {
            tabs: Mutex::new(HashMap::new()),
            active: Mutex::new(0),
            next_id: Mutex::new(1),
        }
    }
}

impl TabManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_tab(&self, url: &str, title: &str, qapp_id: &str) -> TabId {
        let mut next = self.next_id.lock().unwrap();
        let id = *next;
        *next += 1;
        drop(next);

        let info = TabInfo {
            id,
            title: title.to_string(),
            url: url.to_string(),
            qapp_id: qapp_id.to_string(),
            loading: true,
        };

        self.tabs.lock().unwrap().insert(id, info);
        *self.active.lock().unwrap() = id;
        id
    }

    pub fn close_tab(&self, id: TabId) -> bool {
        let mut tabs = self.tabs.lock().unwrap();
        let existed = tabs.remove(&id).is_some();
        if existed {
            let active = *self.active.lock().unwrap();
            if active == id {
                if let Some(new_active) = tabs.keys().next() {
                    *self.active.lock().unwrap() = *new_active;
                } else {
                    *self.active.lock().unwrap() = 0;
                }
            }
        }
        existed
    }

    pub fn get_tab(&self, id: TabId) -> Option<TabInfo> {
        self.tabs.lock().unwrap().get(&id).cloned()
    }

    pub fn list_tabs(&self) -> Vec<TabInfo> {
        let tabs = self.tabs.lock().unwrap();
        let mut result: Vec<TabInfo> = tabs.values().cloned().collect();
        result.sort_by_key(|t| t.id);
        result
    }

    pub fn active_tab(&self) -> Option<TabInfo> {
        let active = *self.active.lock().unwrap();
        if active == 0 {
            return None;
        }
        self.tabs.lock().unwrap().get(&active).cloned()
    }

    pub fn set_active(&self, id: TabId) {
        *self.active.lock().unwrap() = id;
    }

    pub fn set_title(&self, id: TabId, title: &str) {
        if let Some(tab) = self.tabs.lock().unwrap().get_mut(&id) {
            tab.title = title.to_string();
        }
    }

    pub fn set_loading(&self, id: TabId, loading: bool) {
        if let Some(tab) = self.tabs.lock().unwrap().get_mut(&id) {
            tab.loading = loading;
        }
    }

    pub fn navigate(&self, id: TabId, url: &str, qapp_id: &str) {
        if let Some(tab) = self.tabs.lock().unwrap().get_mut(&id) {
            tab.url = url.to_string();
            tab.qapp_id = qapp_id.to_string();
            tab.loading = true;
        }
    }
}

pub fn qapp_url(qapp_id: &str) -> String {
    match qapp_id {
        // Talk is home (empty studio hash). Legacy dashboard/home alias the same URL.
        "talk" | "dashboard" | "home" => "/studio/#/".to_string(),
        "wellfair" => "/studio/#/wellfair".to_string(),
        "chora" => "/studio/#/chora".to_string(),
        "browser" => "/studio/#/browser".to_string(),
        "10d-browser" => "/studio/#/10d-browser".to_string(),
        "gpu-viewport" => "/studio/#/gpu-viewport".to_string(),
        "settings" => "/studio/#/settings".to_string(),
        "about" => "/studio/#/about".to_string(),
        "qapp-studio" => "/studio/#/qapp-studio".to_string(),
        "qapps" => "/studio/#/qapps".to_string(),
        "nexus" => "/studio/#/nexus".to_string(),
        "render-preview" => "/studio/#/render-preview".to_string(),
        "anatomy-test" => "/studio/#/anatomy-test".to_string(),
        custom => format!("/studio/#/{}", custom),
    }
}

pub fn qapp_title(qapp_id: &str) -> &'static str {
    match qapp_id {
        "talk" | "dashboard" | "home" => "Talk",
        "wellfair" => "WellFair",
        "chora" => "Chora",
        "browser" => "Browser",
        "10d-browser" => "10D Browser",
        "gpu-viewport" => "GPU Viewport",
        "settings" => "Settings",
        "about" => "About",
        "qapp-studio" => "QApp Studio",
        "qapps" => "QApps",
        "nexus" => "Nexus",
        "render-preview" => "Render Preview",
        "anatomy-test" => "Anatomy Test",
        _ => "Webizen",
    }
}
