use super::{Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // ── Developer Tools ───────────────────────────────────────────────────
        QApp {
            id: "mcp-inspector",
            name: "MCP Tool Inspector",
            tagline: "Protocol Debugging",
            desc: "Browse, invoke, and test all 41 MCP tools from mcp_server.rs. Inspect \
                   request/response JSON, trace call latency, and verify NQuin citations in results.",
            icon: "plugin",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Developer,
        },
        QApp {
            id: "benchmark",
            name: "Benchmark Harness",
            tagline: "Performance Testing",
            desc: "Run the benchmarks/qualia/runner.py harness against the local daemon: point / \
                   two-hop / filter query latency, graph insert throughput, and inference tokens/sec.",
            icon: "stopwatch",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Developer,
        },
        QApp {
            id: "cli-bridge",
            name: "CLI Bridge",
            tagline: "Command Line",
            desc: "GUI wrapper over qualia-cli: ingest RDF/Turtle, run SPARQL queries, invoke \
                   solve(ode/quantum/symbolic), trigger science runners, and browse ETL pipeline state.",
            icon: "terminal",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Developer,
        },
        QApp {
            id: "extension-bus",
            name: "Extension Bus",
            tagline: "FFI Extensions",
            desc: "Manage heavy computational extensions (QPU, PINN, SNN, fluid dynamics) via the \
                   extension_bus FFI bridge. Load, unload, and inspect extension manifests.",
            icon: "puzzle",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Developer,
        },
        QApp {
            id: "marketplace",
            name: "QApp Marketplace",
            tagline: "Community Extensions",
            desc: "Browse and install community QApps distributed over WebTorrent. Each app is \
                   sandboxed by the Webizen VM; N3Logic permission declarations are auditable before install.",
            icon: "shop",
            route: None,
            stat: Stat::Soon,
            cat: Cat::Developer,
        },
    ]
}
