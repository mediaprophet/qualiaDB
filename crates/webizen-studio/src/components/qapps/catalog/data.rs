use super::{Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // â”€â”€ Data & Storage â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        QApp {
            id: "wal-inspector",
            name: "WAL Inspector",
            tagline: "Write-Ahead Log",
            desc: "Browse the 32-byte WAL header, prev_dag_hash chains, buffered_count, and DagNode \
                   Merkle tree. Trigger checkpoint_to_dag() and verify ed25519 conduct-violation quins.",
            icon: "journal-code",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Data,
        },
        QApp {
            id: "q42-volume",
            name: "Q42 Volume Manager",
            tagline: "Graph Archives",
            desc: "Manage .q42.bidx block-range index files, header-first boot partitions, OPFS \
                   auto-cache, and multi-file Q42 volume manifests with cryptographic checksums.",
            icon: "database",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Data,
        },
        QApp {
            id: "provenance-graph",
            name: "Provenance Graph",
            tagline: "Audit Trails",
            desc: "Visualise PROV-O provenance chains over the temporal graph. Navigate derivation \
                   edges between NQuin citations, WAL entries, and Merkle-DAG checkpoints.",
            icon: "diagram-2-fill",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Data,
        },
        QApp {
            id: "storage-config",
            name: "Storage Driver Config",
            tagline: "Storage Abstraction",
            desc: "Configure the cross-platform StorageDriver (ZnsDriver / WinNvmeDriver / \
                   MmapApfsDriver / MmapDriver). Set ZNS zone limits, mmap parameters, and WSL2 auto-detect.",
            icon: "hdd-stack",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Data,
        },
        QApp {
            id: "crdt-sync",
            name: "CRDT Sync Dashboard",
            tagline: "Distributed Sync",
            desc: "Monitor conflict-free replicated data type convergence across Webizen nodes. \
                   Visualise merge histories, vector clocks, and delta-state gossip round-trips.",
            icon: "arrow-repeat",
            route: None,
            stat: Stat::Soon,
            cat: Cat::Data,
        },
    ]
}
