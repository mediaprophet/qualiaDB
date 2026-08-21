//! Interactive hypermedia / HbbTV — interactive, second screen, social, packaging.
//!
//! ~25 required functions.

use std::collections::BTreeMap;

/// An HbbTV / interactive application.
#[derive(Debug, Clone)]
pub struct HbbTVApp {
    pub id: String,
    pub name: String,
    pub pages: BTreeMap<String, InteractivePage>,
    pub current_page: Option<String>,
    pub state: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct InteractivePage {
    pub id: String,
    pub title: String,
    pub elements: Vec<InteractiveElement>,
    pub triggers: Vec<Trigger>,
}

#[derive(Debug, Clone)]
pub struct InteractiveElement {
    pub id: String,
    pub element_type: ElementType,
    pub visible: bool,
    pub action: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Button,
    Text,
    Image,
    Video,
    Input,
    Overlay,
}

#[derive(Debug, Clone)]
pub struct Trigger {
    pub id: String,
    pub event: TriggerEvent,
    pub action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerEvent {
    RedButton,
    GreenButton,
    YellowButton,
    BlueButton,
    OkButton,
    BackButton,
    Timer,
    StreamEvent,
}

impl HbbTVApp {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            pages: BTreeMap::new(),
            current_page: None,
            state: BTreeMap::new(),
        }
    }

    pub fn add_page(&mut self, page: InteractivePage) {
        if self.current_page.is_none() {
            self.current_page = Some(page.id.clone());
        }
        self.pages.insert(page.id.clone(), page);
    }

    pub fn navigate_to(&mut self, page_id: &str) -> bool {
        if self.pages.contains_key(page_id) {
            self.current_page = Some(page_id.to_string());
            true
        } else {
            false
        }
    }

    pub fn set_state(&mut self, key: &str, value: &str) {
        self.state.insert(key.to_string(), value.to_string());
    }

    pub fn get_state(&self, key: &str) -> Option<&String> {
        self.state.get(key)
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}

/// Second screen companion app.
#[derive(Debug, Clone)]
pub struct SecondScreen {
    pub id: String,
    pub device_id: String,
    pub synced_content: String,
    pub sync_offset: f64,
    pub interactive_layer: BTreeMap<String, String>,
}

impl SecondScreen {
    pub fn new(id: &str, device_id: &str) -> Self {
        Self {
            id: id.to_string(),
            device_id: device_id.to_string(),
            synced_content: String::new(),
            sync_offset: 0.0,
            interactive_layer: BTreeMap::new(),
        }
    }

    pub fn sync_to_content(&mut self, content_id: &str, offset: f64) {
        self.synced_content = content_id.to_string();
        self.sync_offset = offset;
    }

    pub fn add_interactive_layer(&mut self, key: &str, value: &str) {
        self.interactive_layer
            .insert(key.to_string(), value.to_string());
    }
}

/// Interactive stream with synchronized triggers.
#[derive(Debug, Clone)]
pub struct InteractiveStream {
    pub id: String,
    pub content_id: String,
    pub triggers: Vec<StreamTrigger>,
    pub social_feed: Vec<SocialPost>,
}

#[derive(Debug, Clone)]
pub struct StreamTrigger {
    pub timestamp: f64,
    pub trigger_type: TriggerEvent,
    pub payload: String,
}

#[derive(Debug, Clone)]
pub struct SocialPost {
    pub id: String,
    pub author: String,
    pub content: String,
    pub timestamp: f64,
}

impl InteractiveStream {
    pub fn new(id: &str, content_id: &str) -> Self {
        Self {
            id: id.to_string(),
            content_id: content_id.to_string(),
            triggers: Vec::new(),
            social_feed: Vec::new(),
        }
    }

    pub fn add_trigger(&mut self, trigger: StreamTrigger) {
        self.triggers.push(trigger);
    }

    pub fn add_social_post(&mut self, post: SocialPost) {
        self.social_feed.push(post);
    }

    pub fn triggers_at(&self, time: f64, window: f64) -> Vec<&StreamTrigger> {
        self.triggers
            .iter()
            .filter(|t| (t.timestamp - time).abs() < window)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hbbtv_app_creation() {
        let app = HbbTVApp::new("app1", "My App");
        assert_eq!(app.name, "My App");
        assert_eq!(app.page_count(), 0);
    }

    #[test]
    fn hbbtv_add_page() {
        let mut app = HbbTVApp::new("app1", "My App");
        app.add_page(InteractivePage {
            id: "p1".into(),
            title: "Home".into(),
            elements: vec![],
            triggers: vec![],
        });
        assert_eq!(app.page_count(), 1);
        assert_eq!(app.current_page, Some("p1".to_string()));
    }

    #[test]
    fn hbbtv_navigate() {
        let mut app = HbbTVApp::new("app1", "App");
        app.add_page(InteractivePage {
            id: "p1".into(),
            title: "Home".into(),
            elements: vec![],
            triggers: vec![],
        });
        app.add_page(InteractivePage {
            id: "p2".into(),
            title: "Info".into(),
            elements: vec![],
            triggers: vec![],
        });
        assert!(app.navigate_to("p2"));
        assert_eq!(app.current_page, Some("p2".to_string()));
        assert!(!app.navigate_to("nonexistent"));
    }

    #[test]
    fn hbbtv_state() {
        let mut app = HbbTVApp::new("app1", "App");
        app.set_state("score", "100");
        assert_eq!(app.get_state("score"), Some(&"100".to_string()));
    }

    #[test]
    fn second_screen_sync() {
        let mut ss = SecondScreen::new("ss1", "device1");
        ss.sync_to_content("content_a", 1.5);
        assert_eq!(ss.synced_content, "content_a");
        assert!((ss.sync_offset - 1.5).abs() < 0.01);
    }

    #[test]
    fn interactive_stream_triggers() {
        let mut stream = InteractiveStream::new("s1", "content_a");
        stream.add_trigger(StreamTrigger {
            timestamp: 5.0,
            trigger_type: TriggerEvent::RedButton,
            payload: "show_menu".into(),
        });
        stream.add_trigger(StreamTrigger {
            timestamp: 10.0,
            trigger_type: TriggerEvent::Timer,
            payload: "show_ad".into(),
        });
        let triggers = stream.triggers_at(5.0, 0.5);
        assert_eq!(triggers.len(), 1);
    }

    #[test]
    fn interactive_stream_social() {
        let mut stream = InteractiveStream::new("s1", "content_a");
        stream.add_social_post(SocialPost {
            id: "p1".into(),
            author: "user1".into(),
            content: "Great show!".into(),
            timestamp: 1.0,
        });
        assert_eq!(stream.social_feed.len(), 1);
    }
}
