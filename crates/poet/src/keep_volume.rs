//! Keep volume browse helpers — sayable reopen, held-gate, real commit only.
//!
//! Shared by Poet save-dialog chrome. Live binds only:
//! `GraphDatabase.volume_open` · `GraphDatabase.volume_commit`. No Host invent.

/// Live ALL_BOUND id. Do not invent a Host method.
pub const OPEN_ID: &str = "GraphDatabase.volume_open";
/// Live ALL_BOUND id. Do not invent a Host method.
pub const COMMIT_ID: &str = "GraphDatabase.volume_commit";

pub const HELD_WHY: &str = "held / not yet — keep a volume";
pub const RECENT_STORAGE_KEY: &str = "qualia-ui:keep-recent-volumes";
pub const MAX_RECENTS: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecentVolume {
    pub path: String,
    pub sayable: String,
}

pub fn sayable_name(path: &str) -> String {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path).trim();
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

/// Celebrate only on a real durable write. HTTP 200 with `ok:false` or `written:0` never.
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
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<u64>() {
                return n;
            }
        }
    }
    0
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_are_live_graph_volume_ids() {
        assert_eq!(OPEN_ID, "GraphDatabase.volume_open");
        assert_eq!(COMMIT_ID, "GraphDatabase.volume_commit");
    }

    #[test]
    fn sayable_hides_absolute_paths() {
        assert_eq!(
            sayable_name("/workspace/qualia-data/uat-sanctuary.q42"),
            "uat sanctuary"
        );
        assert!(!sayable_name("/workspace/qualia-data/uat-sanctuary.q42").contains('/'));
    }

    #[test]
    fn celebrate_requires_ok_and_written() {
        assert!(celebrate_commit(true, 1));
        assert!(!celebrate_commit(true, 0));
        assert!(!celebrate_commit(false, 4));
        assert_eq!(parse_u64_field(r#"{written: 1, path: "x"}"#, "written"), 1);
        assert_eq!(parse_u64_field(r#"{ok: true}"#, "written"), 0);
    }

    #[test]
    fn missing_or_denied_is_held_never_unavailable() {
        assert_eq!(sanitize_held_why("Unavailable: start daemon"), HELD_WHY);
        assert!(copy_avoids_unavailable(HELD_WHY));
        assert!(copy_avoids_unavailable(&sanitize_held_why("E300")));
    }

    #[test]
    fn recents_prefer_last_opened_sayable() {
        let first = remember_recent(&[], "/vault/session.q42");
        let next = remember_recent(&first, "/workspace/qualia-data/uat-sanctuary.q42");
        assert_eq!(next[0].sayable, "uat sanctuary");
        assert_eq!(next[1].sayable, "session");
        assert_eq!(parse_recents_json(&recents_json(&next)).len(), 2);
    }

    #[test]
    fn save_dialog_chrome_never_says_unavailable() {
        let src = include_str!("browser/topbar/save_dialog.rs");
        assert!(
            !src.to_ascii_lowercase().contains("unavailable"),
            "save dialog must use held / not yet"
        );
        assert!(src.contains("celebrate_commit"));
        assert!(src.contains("keep_picker"));
    }
}
