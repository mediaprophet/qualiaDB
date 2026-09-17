//! Studio library lifecycle: update / suspend / remove (SI-10).
//! Receipts remain after remove. No Host IDs.

use dioxus::prelude::*;

pub const HELD_SUSPEND_UPDATE: &str = "held / not yet — suspended or removed pack cannot update";
pub const HELD_REMOVED_RUN: &str = "held / not yet — removed pack cannot start a new run";
pub const HELD_SUSPENDED_RUN: &str = "held / not yet — suspended pack cannot start a new run";
pub const HELD_NOT_ACTIVE: &str = "held / not yet — collect and activate before run";
pub const HISTORY_REMAINS: &str = "receipt history remains after remove";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LibraryLife {
    pub collected: bool,
    pub activated: bool,
    pub suspended: bool,
    pub removed: bool,
    pub receipts_remain: bool,
}

pub fn suspend(life: &mut LibraryLife) {
    life.suspended = true;
    life.activated = false;
}

pub fn remove_pack(life: &mut LibraryLife) {
    life.removed = true;
    life.activated = false;
    life.collected = false;
    life.receipts_remain = true;
}

pub fn update_pack(life: &LibraryLife) -> Result<(), &'static str> {
    if life.removed || life.suspended {
        return Err(HELD_SUSPEND_UPDATE);
    }
    if !life.collected {
        return Err("held / not yet — collect before update");
    }
    Ok(())
}

pub fn can_run(life: &LibraryLife) -> Result<(), &'static str> {
    if life.removed {
        return Err(HELD_REMOVED_RUN);
    }
    if life.suspended {
        return Err(HELD_SUSPENDED_RUN);
    }
    if !(life.collected && life.activated) {
        return Err(HELD_NOT_ACTIVE);
    }
    Ok(())
}

#[component]
pub fn LifecyclePanel() -> Element {
    let mut life = use_signal(|| LibraryLife {
        collected: true,
        activated: true,
        receipts_remain: true,
        ..LibraryLife::default()
    });
    let mut status = use_signal(|| "ready".to_string());

    rsx! {
        div {
            class: "instrument-lifecycle-panel",
            "data-lifecycle-panel": "1",
            role: "region",
            "aria-label": "instrument lifecycle",

            div { class: "lexicon-bay-title", "Lifecycle" }

            button {
                r#type: "button",
                class: "lexicon-open-btn",
                "aria-label": "Update pack",
                "data-lifecycle-update": "1",
                onclick: move |_| {
                    match update_pack(&life()) {
                        Ok(()) => status.set("updated".into()),
                        Err(e) => status.set(e.into()),
                    }
                },
                "Update"
            }
            button {
                r#type: "button",
                class: "lexicon-open-btn",
                "aria-label": "Suspend pack",
                "data-lifecycle-suspend": "1",
                onclick: move |_| {
                    let mut next = life();
                    suspend(&mut next);
                    life.set(next);
                    status.set(HELD_SUSPENDED_RUN.into());
                },
                "Suspend"
            }
            button {
                r#type: "button",
                class: "lexicon-open-btn",
                "aria-label": "Remove pack",
                "data-lifecycle-remove": "1",
                onclick: move |_| {
                    let mut next = life();
                    remove_pack(&mut next);
                    life.set(next);
                    status.set(format!("{HELD_REMOVED_RUN} · {HISTORY_REMAINS}"));
                },
                "Remove"
            }
            button {
                r#type: "button",
                class: "lexicon-open-btn",
                "aria-label": "Verify pack run",
                "data-lifecycle-run": "1",
                onclick: move |_| {
                    match can_run(&life()) {
                        Ok(()) => status.set("ready for run".into()),
                        Err(e) => status.set(e.into()),
                    }
                },
                "Verify Run"
            }

            div {
                class: "lexicon-held-gate",
                role: "status",
                "data-lifecycle-status": "1",
                "data-receipts-remain": "{life().receipts_remain}",
                "{status}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspend_blocks_run() {
        let mut life = LibraryLife {
            collected: true,
            activated: true,
            receipts_remain: true,
            ..LibraryLife::default()
        };
        suspend(&mut life);
        assert!(life.suspended);
        assert!(!life.activated);
        assert!(life.receipts_remain);
        assert_eq!(can_run(&life), Err(HELD_SUSPENDED_RUN));
    }

    #[test]
    fn remove_keeps_receipts_and_blocks_run() {
        let mut life = LibraryLife {
            collected: true,
            activated: true,
            receipts_remain: false,
            ..LibraryLife::default()
        };
        remove_pack(&mut life);
        assert!(life.removed);
        assert!(!life.collected);
        assert!(!life.activated);
        assert!(life.receipts_remain);
        assert_eq!(can_run(&life), Err(HELD_REMOVED_RUN));
    }

    #[test]
    fn update_blocked_after_remove() {
        let mut life = LibraryLife {
            collected: true,
            activated: true,
            ..LibraryLife::default()
        };
        assert!(update_pack(&life).is_ok());
        remove_pack(&mut life);
        assert_eq!(update_pack(&life), Err(HELD_SUSPEND_UPDATE));
        assert!(!HELD_SUSPEND_UPDATE.contains("Host."));
        assert!(!HISTORY_REMAINS.contains("Host."));
    }

    #[test]
    fn uncollected_or_inactive_pack_cannot_run() {
        let mut life = LibraryLife::default();
        assert_eq!(can_run(&life), Err(HELD_NOT_ACTIVE));
        life.collected = true;
        assert_eq!(can_run(&life), Err(HELD_NOT_ACTIVE));
        life.activated = true;
        assert_eq!(can_run(&life), Ok(()));
        life.collected = false;
        assert_eq!(can_run(&life), Err(HELD_NOT_ACTIVE));
        assert!(!HELD_NOT_ACTIVE.contains("Host."));
    }
}
