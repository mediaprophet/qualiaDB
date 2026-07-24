use super::{Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // ── Network & Distribution ────────────────────────────────────────────
        QApp {
            id: "webtorrent",
            name: "WebTorrent Seeder",
            tagline: "Ontology Distribution",
            desc: "Seed .c.q42 ontology artifacts to the WebTorrent DHT. Manage magnet links, \
                   announce to trackers, monitor peer connections, and verify integrity against NQuin hashes.",
            icon: "share",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Network,
        },
        QApp {
            id: "p2p-dashboard",
            name: "P2P Node Dashboard",
            tagline: "Gossip Network",
            desc: "Monitor the gossip/DHT overlay: peer table, routing buckets, message throughput, \
                   and DaemonSwarm coordination. View live Webizen node topology.",
            icon: "diagram-3-fill",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Network,
        },
        QApp {
            id: "ebpf-filter",
            name: "eBPF Filter Manager",
            tagline: "Network Control",
            desc: "Platform-aware packet filtering via open_platform_filter(): Linux eBPF, Windows WFP, \
                   macOS NEFilter/XPC. Define rules, inspect matched flows, and audit egress.",
            icon: "funnel",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Network,
        },
        QApp {
            id: "acoustic-ble",
            name: "Acoustic BLE Mesh",
            tagline: "Zero-Infrastructure Net",
            desc: "Configure and monitor the acoustic/BLE mesh for offline-first Webizen clustering. \
                   No infrastructure required — peer discovery via acoustic and Bluetooth signals.",
            icon: "broadcast",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Network,
        },
        QApp {
            id: "nym-gateway",
            name: "Nym Privacy Gateway",
            tagline: "Mixnet Routing",
            desc: "Route Remote inference API calls through the Nym mixnet via nym_adapter. Configure \
                   anonymity set size, latency budget, and ILP metering for privacy-preserving egress.",
            icon: "shield-lock",
            route: None,
            stat: Stat::Soon,
            cat: Cat::Network,
        },
    ]
}
