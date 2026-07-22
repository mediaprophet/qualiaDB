use super::{AppRoute, Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // â”€â”€ Platform â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        QApp {
            id: "context-studio",
            name: "Context Studio",
            tagline: "Semantic Workspace",
            desc: "Node-graph canvas with Selfhood/Personhood zone segregation, Inforg assistant, \
                   Commons Gateway, Contextual Lenses, and temporal scrubber for AS OF queries over the live NQuin graph.",
            icon: "diagram-3",
            route: Some(AppRoute::ContextStudio),
            stat: Stat::Active,
            cat: Cat::Platform,
        },
        QApp {
            id: "qapp-studio",
            name: "QApp Studio",
            tagline: "Layout Builder",
            desc: "Drag-and-drop Shoelace + Qualia pane composer. Arrange, resize, and wire panes \
                   into custom dashboards â€” output is a signed QApp manifest written to the WAL.",
            icon: "layers",
            route: Some(AppRoute::QAppStudio),
            stat: Stat::Active,
            cat: Cat::Platform,
        },
        QApp {
            id: "profile-identity",
            name: "Profile & Identity",
            tagline: "DID Management",
            desc: "Manage did:q42 identifiers, ed25519 keypairs, Verifiable Credentials, and \
                   Principal-scoped capability grants via key_vault, profiles, and identifier modules.",
            icon: "person-vcard",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Platform,
        },
        QApp {
            id: "hardware-config",
            name: "Hardware Configurator",
            tagline: "Device Management",
            desc: "Configure GPU backend (DirectML / Vulkan / Metal / WebGPU), ZNS/NVMe storage \
                   zones, thermal governor thresholds, QPU provider credentials, and NPU FFI bindings.",
            icon: "tools",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Platform,
        },
        QApp {
            id: "notification-center",
            name: "Notification Center",
            tagline: "Alerts & Events",
            desc: "Unified event stream from WAL mutations, deontic violations, QPU job completions, \
                   and governance alerts â€” each surfaced as a signed NQuin notification quin.",
            icon: "bell",
            route: None,
            stat: Stat::Soon,
            cat: Cat::Platform,
        },
    ]
}
