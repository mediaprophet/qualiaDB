#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellAction {
    NewWindow,
    Quit,
    NavBack,
    NavForward,
    NavReload,
    Navigate(String),
    ToggleAmbient,
    ZoomIn,
    ZoomOut,
    ResetZoom,
    OpenDiagnostics,
    ImportSamsung,
    SyncRelay,
    Backup,
    HelpAbout,
    HelpUpdate,
    HelpPortal,
    SanctuaryLock,
    SanctuaryUnlock,
    SanctuaryStatus,
    DaemonRestart,
    DaemonStop,
    OpenMedReminders,
    OpenSyncInbox,
    RevokeSessions,
    /// Open the shell command palette (Ctrl+K / Ctrl+P).
    OpenCommandPalette,
}

impl ShellAction {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "new_window" => Some(Self::NewWindow),
            "quit_app" | "quit" => Some(Self::Quit),
            "nav_back" => Some(Self::NavBack),
            "nav_forward" => Some(Self::NavForward),
            "nav_reload" => Some(Self::NavReload),
            "toggle_gpu" => Some(Self::Navigate("gpu-viewport".to_string())),
            "toggle_ambient" => Some(Self::ToggleAmbient),
            "zoom_in" => Some(Self::ZoomIn),
            "zoom_out" => Some(Self::ZoomOut),
            "reset_zoom" => Some(Self::ResetZoom),
            "open_wellfair" => Some(Self::Navigate("wellfair".to_string())),
            "open_chora" => Some(Self::Navigate("chora".to_string())),
            "open_browser" => Some(Self::Navigate("browser".to_string())),
            "open_10d" => Some(Self::Navigate("10d-browser".to_string())),
            // Home is Talk (human-first). Legacy open_dashboard / tray "show" land on talk.
            "open_dashboard" | "open_talk" | "show" => Some(Self::Navigate("talk".to_string())),
            "open_qapp_studio" => Some(Self::Navigate("qapp-studio".to_string())),
            "open_qapp_manager" => Some(Self::Navigate("qapps".to_string())),
            "open_settings" | "settings" => Some(Self::Navigate("settings".to_string())),
            "open_diagnostics" | "health_diagnostics" => Some(Self::OpenDiagnostics),
            "open_library" => Some(Self::Navigate("library".to_string())),
            "open_wallet" => Some(Self::Navigate("wallet".to_string())),
            "import_samsung" => Some(Self::ImportSamsung),
            "sync_relay" => Some(Self::SyncRelay),
            "backup" | "health_backup" => Some(Self::Backup),
            "help_about" => Some(Self::HelpAbout),
            "help_update" => Some(Self::HelpUpdate),
            "help_logs" => Some(Self::Navigate("logs".to_string())),
            "help_portal" => Some(Self::HelpPortal),
            "sanctuary_lock" => Some(Self::SanctuaryLock),
            "sanctuary_unlock" => Some(Self::SanctuaryUnlock),
            "sanctuary_status" => Some(Self::SanctuaryStatus),
            "daemon_restart" => Some(Self::DaemonRestart),
            "daemon_stop" => Some(Self::DaemonStop),
            "health_med_reminders" => Some(Self::OpenMedReminders),
            "sync_inbox" => Some(Self::OpenSyncInbox),
            "revoke" => Some(Self::RevokeSessions),
            "open_command_palette" | "command_palette" => Some(Self::OpenCommandPalette),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ShellAction;

    #[test]
    fn command_palette_action_ids() {
        assert_eq!(
            ShellAction::from_id("open_command_palette"),
            Some(ShellAction::OpenCommandPalette)
        );
        assert_eq!(
            ShellAction::from_id("command_palette"),
            Some(ShellAction::OpenCommandPalette)
        );
        assert_eq!(
            ShellAction::from_id("open_talk"),
            Some(ShellAction::Navigate("talk".to_string()))
        );
    }
}
