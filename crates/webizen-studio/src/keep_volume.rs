//! Keep volume browse — sayable reopen, held-gate, real commit only.
//!
//! Live ALL_BOUND binds: `GraphDatabase.volume_open` · `GraphDatabase.volume_commit`.
//! No Host invent. Missing volume → held / not yet. Never "unavailable".
//! Celebrate only when `volume_commit` returns `written > 0`.

use serde::{Deserialize, Serialize};

/// Live ALL_BOUND id. Do not invent a Host method.
pub const OPEN_ID: &str = "GraphDatabase.volume_open";
/// Live ALL_BOUND id. Do not invent a Host method.
pub const COMMIT_ID: &str = "GraphDatabase.volume_commit";

/// Soft why-text for missing / unknown / E300. Never "broken". Never "unavailable".
pub const HELD_WHY: &str = "held / not yet — keep a volume";

pub const RECENT_STORAGE_KEY: &str = "qualia-ui:keep-recent-volumes";
pub const MAX_RECENTS: usize = 8;
pub const MAX_BROWSE: usize = 12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VolumeLook {
    Closed,
    Open,
    Committed,
    Held,
    Fault,
}

impl VolumeLook {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
            Self::Committed => "committed",
            Self::Held => "held",
            Self::Fault => "fault",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
            Self::Committed => "committed",
            Self::Held | Self::Fault => "held / not yet",
        }
    }

    pub fn named_beat(self) -> &'static str {
        match self {
            Self::Open => "dwell",
            Self::Committed => "exit",
            Self::Closed | Self::Held | Self::Fault => "entrance",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentVolume {
    pub path: String,
    pub sayable: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct VaultVolume {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub relative: String,
    #[serde(default)]
    pub open_error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeepOutcome {
    Held { why: String },
    Open { path: String, sayable: String },
    Committed { path: String, sayable: String, written: u64 },
}

impl KeepOutcome {
    pub fn look(&self) -> VolumeLook {
        match self {
            Self::Held { .. } => VolumeLook::Held,
            Self::Open { .. } => VolumeLook::Open,
            Self::Committed { .. } => VolumeLook::Committed,
        }
    }

    pub fn celebrates(&self) -> bool {
        matches!(self, Self::Committed { written, .. } if *written > 0)
    }
}

pub fn sayable_name(path: &str) -> String {
    let file = path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
        .trim();
    let stem = file
        .strip_suffix(".q42")
        .or_else(|| file.strip_suffix(".Q42"))
        .unwrap_or(file);
    let pretty = stem.replace(['-', '_'], " ");
    let trimmed = pretty.trim();
    if trimmed.is_empty() {
        "keep".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn held_outcome(why: &str) -> KeepOutcome {
    KeepOutcome::Held {
        why: sanitize_held_why(why),
    }
}

pub fn sanitize_held_why(raw: &str) -> String {
    let folded = raw.to_ascii_lowercase();
    if raw.trim().is_empty()
        || folded.contains("unavailable")
        || folded.contains("broken")
        || folded.contains("e300")
    {
        HELD_WHY.to_string()
    } else if folded.contains("held / not yet") {
        raw.trim().to_string()
    } else {
        HELD_WHY.to_string()
    }
}

pub fn copy_avoids_unavailable(text: &str) -> bool {
    !text.to_ascii_lowercase().contains("unavailable")
}

pub fn copy_avoids_broken(text: &str) -> bool {
    !text.to_ascii_lowercase().contains("broken")
}

/// Celebrate only on a real durable write. Local checkpoint / deny / empty written never.
pub fn celebrate_commit(ok: bool, written: u64) -> bool {
    ok && written > 0
}

pub fn parse_u64_field(src: &str, key: &str) -> u64 {
    let patterns = [
        format!("{key}: "),
        format!("{key}:"),
        format!("\"{key}\": "),
        format!("\"{key}\":"),
    ];
    for pat in patterns {
        if let Some(start) = src.find(&pat) {
            let rest = src[start + pat.len()..].trim_start();
            let digits: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(n) = digits.parse::<u64>() {
                return n;
            }
        }
    }
    0
}

pub fn interpret_open(ok: bool, value: &str, diagnostic: Option<&str>, path: &str) -> KeepOutcome {
    if ok {
        return KeepOutcome::Open {
            path: path.to_string(),
            sayable: sayable_name(path),
        };
    }
    let _ = (value, diagnostic);
    held_outcome(HELD_WHY)
}

pub fn interpret_commit(ok: bool, value: &str, diagnostic: Option<&str>, path: &str) -> KeepOutcome {
    let written = parse_u64_field(value, "written");
    if celebrate_commit(ok, written) {
        return KeepOutcome::Committed {
            path: path.to_string(),
            sayable: sayable_name(path),
            written,
        };
    }
    let _ = diagnostic;
    held_outcome(HELD_WHY)
}

pub fn remember_recent(recents: &[RecentVolume], path: &str) -> Vec<RecentVolume> {
    let path = path.trim();
    if path.is_empty() {
        return recents.to_vec();
    }
    let mut out = vec![RecentVolume {
        path: path.to_string(),
        sayable: sayable_name(path),
    }];
    for item in recents {
        if item.path != path {
            out.push(item.clone());
        }
        if out.len() >= MAX_RECENTS {
            break;
        }
    }
    out
}

pub fn parse_recents_json(raw: &str) -> Vec<RecentVolume> {
    serde_json::from_str::<Vec<RecentVolume>>(raw)
        .unwrap_or_default()
        .into_iter()
        .filter(|item| !item.path.trim().is_empty())
        .take(MAX_RECENTS)
        .map(|mut item| {
            if item.sayable.trim().is_empty() {
                item.sayable = sayable_name(&item.path);
            }
            item
        })
        .collect()
}

pub fn recents_json(recents: &[RecentVolume]) -> String {
    serde_json::to_string(recents).unwrap_or_else(|_| "[]".into())
}

pub fn merge_browse_list(recents: &[RecentVolume], vault: &[VaultVolume]) -> Vec<RecentVolume> {
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for item in recents {
        if seen.iter().any(|p| p == &item.path) {
            continue;
        }
        seen.push(item.path.clone());
        out.push(item.clone());
        if out.len() >= MAX_BROWSE {
            return out;
        }
    }
    for item in vault {
        if item.path.trim().is_empty() || seen.iter().any(|p| p == &item.path) {
            continue;
        }
        if item.open_error.is_some() {
            continue;
        }
        seen.push(item.path.clone());
        out.push(RecentVolume {
            path: item.path.clone(),
            sayable: if item.display_name.trim().is_empty() {
                sayable_name(&item.path)
            } else {
                sayable_name(&item.display_name)
            },
        });
        if out.len() >= MAX_BROWSE {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_are_live_graph_volume_ids() {
        assert_eq!(OPEN_ID, "GraphDatabase.volume_open");
        assert_eq!(COMMIT_ID, "GraphDatabase.volume_commit");
        assert!(!OPEN_ID.contains("qualia."));
        assert!(!COMMIT_ID.contains("qualia."));
    }

    #[test]
    fn sayable_strips_absolute_path_soup() {
        assert_eq!(
            sayable_name("/workspace/qualia-data/uat-sanctuary.q42"),
            "uat sanctuary"
        );
        assert_eq!(sayable_name("C:\\\\Vault\\\\session.q42"), "session");
        assert_eq!(sayable_name("keep.q42"), "keep");
        assert!(!sayable_name("/workspace/qualia-data/uat-sanctuary.q42").contains('/'));
    }

    #[test]
    fn celebrate_only_on_real_written() {
        assert!(celebrate_commit(true, 1));
        assert!(!celebrate_commit(true, 0));
        assert!(!celebrate_commit(false, 1));
        assert!(!celebrate_commit(false, 0));
        match interpret_commit(
            true,
            r#"{path: "/tmp/keep.q42", written: 1, sanctuary: true}"#,
            None,
            "/tmp/keep.q42",
        ) {
            KeepOutcome::Committed { written, .. } => {
                assert_eq!(written, 1);
                assert!(KeepOutcome::Committed {
                    path: "/tmp/keep.q42".into(),
                    sayable: "keep".into(),
                    written: 1,
                }
                .celebrates());
            }
            other => panic!("{other:?}"),
        }
        match interpret_commit(true, r#"{path: "/tmp/keep.q42", written: 0}"#, None, "/tmp/x.q42")
        {
            KeepOutcome::Held { why } => {
                assert_eq!(why, HELD_WHY);
                assert!(copy_avoids_unavailable(&why));
            }
            other => panic!("{other:?}"),
        }
        match interpret_commit(false, "", Some("E300 denied"), "/tmp/x.q42") {
            KeepOutcome::Held { why } => assert!(copy_avoids_unavailable(&why)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn missing_volume_is_held_never_unavailable() {
        match interpret_open(false, "", Some("Unavailable: no daemon"), "/missing.q42") {
            KeepOutcome::Held { why } => {
                assert_eq!(why, HELD_WHY);
                assert!(copy_avoids_unavailable(&why));
                assert!(copy_avoids_broken(&why));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(sanitize_held_why("Unavailable: start daemon"), HELD_WHY);
        assert!(copy_avoids_unavailable(VolumeLook::Held.label()));
        assert!(copy_avoids_unavailable(HELD_WHY));
    }

    #[test]
    fn recent_and_vault_merge_prefers_sayable_reopen() {
        let recents = remember_recent(
            &[],
            "/workspace/qualia-data/uat-sanctuary.q42",
        );
        assert_eq!(recents[0].sayable, "uat sanctuary");
        let vault = [VaultVolume {
            path: "/vault/session.q42".into(),
            display_name: "session.q42".into(),
            relative: "session.q42".into(),
            open_error: None,
        }];
        let merged = merge_browse_list(&recents, &vault);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].sayable, "uat sanctuary");
        assert_eq!(merged[1].sayable, "session");
        assert!(!merged.iter().any(|v| v.sayable.contains('/')));
    }

    #[test]
    fn recents_json_round_trip() {
        let raw = recents_json(&[RecentVolume {
            path: "/tmp/keep.q42".into(),
            sayable: "keep".into(),
        }]);
        let parsed = parse_recents_json(&raw);
        assert_eq!(parsed[0].path, "/tmp/keep.q42");
        assert_eq!(RECENT_STORAGE_KEY, "qualia-ui:keep-recent-volumes");
    }

    #[test]
    fn keep_hub_chrome_is_browse_not_unavailable() {
        let src = include_str!("components/keep_hub.rs");
        let ui: String = src
            .lines()
            .filter(|line| !line.trim_start().starts_with("//"))
            .collect();
        assert!(src.contains("data-keep-volume-browse"));
        assert!(src.contains("Browse"));
        assert!(src.contains("celebrate"));
        assert!(!ui.to_ascii_lowercase().contains("unavailable"));
        assert!(src.contains("OPEN_ID"));
        assert!(src.contains("COMMIT_ID"));
    }
}
