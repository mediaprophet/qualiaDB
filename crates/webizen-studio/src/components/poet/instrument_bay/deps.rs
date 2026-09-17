//! Studio dependency election (SI-10). Unresolved required deps stay held.
//! Host.* names are refused. Studio does not depend on poet.

use dioxus::prelude::*;

const HELD_UNRESOLVED: &str = "held / not yet — unresolved required dependency";
const CLOSED: &str = "dependencies closed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepReq {
    pub name: &'static str,
    pub required: bool,
    pub resolved: bool,
}

const SEED: &[DepReq] = &[
    DepReq {
        name: "visual .10d",
        required: false,
        resolved: true,
    },
    DepReq {
        name: "definition N3",
        required: true,
        resolved: true,
    },
    DepReq {
        name: "executable form",
        required: true,
        resolved: false,
    },
];

pub fn refuse_host_dep(name: &str) -> bool {
    name.contains("Host.")
}

pub fn election_status(deps: &[DepReq]) -> &'static str {
    if deps
        .iter()
        .any(|d| !refuse_host_dep(d.name) && d.required && !d.resolved)
    {
        HELD_UNRESOLVED
    } else {
        CLOSED
    }
}

#[component]
pub fn DepsPanel() -> Element {
    let mut resolved_exec = use_signal(|| false);
    let deps: Vec<DepReq> = SEED
        .iter()
        .filter(|d| !refuse_host_dep(d.name))
        .map(|d| {
            if d.name == "executable form" {
                DepReq {
                    resolved: resolved_exec(),
                    ..*d
                }
            } else {
                *d
            }
        })
        .collect();
    let status = election_status(&deps);

    rsx! {
        div {
            class: "instrument-deps-panel",
            "data-deps-panel": "1",
            role: "region",
            "aria-label": "instrument dependency election",

            div { class: "lexicon-bay-title", "Dependencies" }
            ul {
                role: "list",
                for dep in deps.iter().copied() {
                    li {
                        role: "listitem",
                        "data-dep-name": "{dep.name}",
                        "data-required": "{dep.required}",
                        "data-resolved": "{dep.resolved}",
                        "{dep.name}"
                    }
                }
            }
            button {
                r#type: "button",
                class: "lexicon-open-btn",
                "aria-label": "Resolve executable form",
                "data-deps-resolve": "1",
                onclick: move |_| resolved_exec.set(true),
                "Resolve executable"
            }
            div {
                class: "lexicon-held-gate",
                role: "status",
                "data-deps-status": "1",
                "{status}"
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unresolved_required_is_held() {
        assert_eq!(election_status(SEED), HELD_UNRESOLVED);
    }

    #[test]
    fn all_required_resolved_is_closed() {
        let closed: Vec<DepReq> = SEED
            .iter()
            .map(|d| DepReq {
                resolved: true,
                ..*d
            })
            .collect();
        assert_eq!(election_status(&closed), CLOSED);
        assert_eq!(election_status(&[]), CLOSED);
    }

    #[test]
    fn host_name_refused() {
        assert!(refuse_host_dep("Host.ClinicalRisk"));
        assert!(!refuse_host_dep("definition N3"));
        for d in SEED {
            assert!(!d.name.contains("Host."));
        }
        let mixed = [
            SEED[0],
            SEED[1],
            DepReq {
                name: "Host.x",
                required: true,
                resolved: false,
            },
            DepReq {
                name: "executable form",
                required: true,
                resolved: true,
            },
        ];
        assert_eq!(election_status(&mixed), CLOSED);
    }
}
