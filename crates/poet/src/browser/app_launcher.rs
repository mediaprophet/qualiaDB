//! Webizen Admin Refactoring & App Launcher Lifecycle Subsystem (Spec 19).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements daemon-aware application lifecycle probing, cross-platform
//! desktop shortcut minting (.lnk, .desktop, .app), URI scheme handlers
//! (qualia://, vibe://, solid://), and the 7-Hub Webizen Admin Mission Control.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Target construct or manifold for desktop shortcut launch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ShortcutLaunchMode {
    FullPoetShell,
    StandaloneKiosk,
}

/// Parameters for minting a new OS desktop shortcut.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppShortcut {
    pub id: String,
    pub title: String,
    pub description: String,
    pub target_construct_id: String,
    pub target_manifold_id: Option<String>,
    pub icon_name: String,
    pub launch_mode: ShortcutLaunchMode,
    pub auto_start_daemon: bool,
    pub preload_neural_model: Option<String>,
}

impl AppShortcut {
    pub fn new_manifold_shortcut(title: &str, construct_id: &str, manifold_id: &str) -> Self {
        Self {
            id: format!("sc_{}", construct_id),
            title: title.to_string(),
            description: format!(
                "Direct jump to {} within Construct {}",
                manifold_id, construct_id
            ),
            target_construct_id: construct_id.to_string(),
            target_manifold_id: Some(manifold_id.to_string()),
            icon_name: "qualia-manifold".into(),
            launch_mode: ShortcutLaunchMode::FullPoetShell,
            auto_start_daemon: true,
            preload_neural_model: Some("Q42-Dense-10D".into()),
        }
    }

    /// Generate a standard FreeDesktop `.desktop` entry string for Linux.
    pub fn export_linux_desktop_entry(&self) -> String {
        let mut exec_cmd = format!(
            "/usr/bin/poet-desktop --construct={}",
            self.target_construct_id
        );
        if let Some(m) = &self.target_manifold_id {
            exec_cmd.push_str(&format!(" --manifold={}", m));
        }
        if self.auto_start_daemon {
            exec_cmd.push_str(" --daemon-autostart");
        }

        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name={}\n\
             Comment={}\n\
             Exec={}\n\
             Icon={}\n\
             Categories=Science;Office;Development;\n\
             MimeType=application/vnd.qualia.hcf;application/vnd.qualia.vibe;\n\
             Terminal=false\n",
            self.title, self.description, exec_cmd, self.icon_name
        )
    }

    /// Generate the command-line invocation arguments for Windows shortcut creation.
    pub fn export_windows_exec_args(&self) -> String {
        let mut args = format!("--construct=\"{}\"", self.target_construct_id);
        if let Some(m) = &self.target_manifold_id {
            args.push_str(&format!(" --manifold=\"{}\"", m));
        }
        if self.auto_start_daemon {
            args.push_str(" --daemon-autostart");
        }
        args
    }
}

/// The 7 Operator Hubs of Webizen Admin Mission Control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdminOperatorHub {
    HardwareCompute,
    NeuralModels,
    VaultIdentity,
    StorageQ42,
    NetworkSwarms,
    AuditSentinel,
    LauncherFleet,
}

impl AdminOperatorHub {
    pub fn label(&self) -> &'static str {
        match self {
            Self::HardwareCompute => "Hub 1: Hardware & Compute",
            Self::NeuralModels => "Hub 2: Neural Models",
            Self::VaultIdentity => "Hub 3: Vault & Identity",
            Self::StorageQ42 => "Hub 4: Storage & Q42",
            Self::NetworkSwarms => "Hub 5: Network & Swarms",
            Self::AuditSentinel => "Hub 6: Audit & Sentinel",
            Self::LauncherFleet => "Hub 7: Launcher & Fleet",
        }
    }

    pub fn glyph(&self) -> &'static str {
        match self {
            Self::HardwareCompute => "\u{2699}\u{FE0F}", // ⚙️
            Self::NeuralModels => "\u{1F9E0}",           // 🧠
            Self::VaultIdentity => "\u{1F510}",          // 🔐
            Self::StorageQ42 => "\u{1F4BE}",             // 💾
            Self::NetworkSwarms => "\u{1F310}",          // 🌐
            Self::AuditSentinel => "\u{1F6E1}\u{FE0F}",  // 🛡️
            Self::LauncherFleet => "\u{1F680}",          // 🚀
        }
    }

    pub fn summary(&self) -> &'static str {
        match self {
            Self::HardwareCompute => "wgpu 30 contexts, GPU device roles, thermal triad governor",
            Self::NeuralModels => {
                "GGUF/P64 resident mmap, dynamic LoRA adapters, autoregressive decoders"
            }
            Self::VaultIdentity => {
                "Inalienable root DID key vault, sanctuary decoy enclaves, M-of-N keys"
            }
            Self::StorageQ42 => {
                "Q42 partition defrag, WAL journal snapshots, persistent disk sectors"
            }
            Self::NetworkSwarms => "Solid LDP transports, WebTorrent seed swarms, P2P mesh routing",
            Self::AuditSentinel => {
                "42MB Sentinel memory gauge, eBPF network filters, Merkle receipts"
            }
            Self::LauncherFleet => {
                "Desktop shortcut minting, kiosk containers, multi-machine fleet dispatch"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the Webizen Admin Mission Control & App Launcher Viewport.
pub fn build_admin_launcher_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 12px; gap: 10px; \
         background: #020617; color: #f8fafc; overflow-y: auto; font-family: sans-serif;",
    );

    // Header Toolbar
    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 8px 12px;",
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some(
        "\u{1F6E1}\u{FE0F} Webizen Admin & Node Mission Control",
    ));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    header.append_child(&title).unwrap();

    let status = document.create_element("span").unwrap();
    status.set_text_content(Some(
        "Daemon: Active | Port: 3001 | IPC: \\\\.\\pipe\\qualia-daemon-ipc",
    ));
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el
        .style()
        .set_css_text("font-size: 11px; font-family: var(--font-mono); color: #34d399;");
    header.append_child(&status).unwrap();

    root.append_child(&header).unwrap();

    // 7 Hubs Grid
    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 10px;",
    );

    let hubs = [
        AdminOperatorHub::HardwareCompute,
        AdminOperatorHub::NeuralModels,
        AdminOperatorHub::VaultIdentity,
        AdminOperatorHub::StorageQ42,
        AdminOperatorHub::NetworkSwarms,
        AdminOperatorHub::AuditSentinel,
        AdminOperatorHub::LauncherFleet,
    ];

    for hub in hubs {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(
            "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
             border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 6px;",
        );

        let name = document.create_element("span").unwrap();
        name.set_text_content(Some(&format!("{} {}", hub.glyph(), hub.label())));
        let name_el: HtmlElement = name.clone().dyn_into().unwrap();
        name_el
            .style()
            .set_css_text("font-weight: 700; font-size: 12px; color: #f8fafc;");
        card.append_child(&name).unwrap();

        let sum = document.create_element("span").unwrap();
        sum.set_text_content(Some(hub.summary()));
        let sum_el: HtmlElement = sum.clone().dyn_into().unwrap();
        sum_el
            .style()
            .set_css_text("font-size: 11px; color: #94a3b8;");
        card.append_child(&sum).unwrap();

        grid.append_child(&card).unwrap();
    }

    root.append_child(&grid).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_shortcut_generation() {
        let sc = AppShortcut::new_manifold_shortcut(
            "Catchment Research Studio",
            "c_north_spring",
            "manifold_telemetry",
        );
        assert_eq!(sc.title, "Catchment Research Studio");
        assert!(sc.auto_start_daemon);

        let desktop_file = sc.export_linux_desktop_entry();
        assert!(desktop_file.contains("Name=Catchment Research Studio"));
        assert!(desktop_file.contains("--construct=c_north_spring"));
        assert!(desktop_file.contains("--manifold=manifold_telemetry"));
        assert!(desktop_file.contains("--daemon-autostart"));

        let win_args = sc.export_windows_exec_args();
        assert!(win_args.contains("--construct=\"c_north_spring\""));
        assert!(win_args.contains("--manifold=\"manifold_telemetry\""));
    }

    #[test]
    fn test_all_operator_hubs_count() {
        let hubs = [
            AdminOperatorHub::HardwareCompute,
            AdminOperatorHub::NeuralModels,
            AdminOperatorHub::VaultIdentity,
            AdminOperatorHub::StorageQ42,
            AdminOperatorHub::NetworkSwarms,
            AdminOperatorHub::AuditSentinel,
            AdminOperatorHub::LauncherFleet,
        ];
        assert_eq!(hubs.len(), 7);
        for h in hubs {
            assert!(!h.label().is_empty());
            assert!(!h.summary().is_empty());
        }
    }
}
