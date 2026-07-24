use super::{Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // ── Security & Governance ─────────────────────────────────────────────
        QApp {
            id: "agreements",
            name: "Agreements & Rights",
            tagline: "Governance",
            desc: "ODRL policy manager for data-sharing agreements. Sign and verify fiduciary \
                   obligations using the deontic_logic engine and agency module's ed25519 Author-scoped Merkle roots.",
            icon: "file-earmark-check",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Security,
        },
        QApp {
            id: "key-vault",
            name: "Key Vault Manager",
            tagline: "Cryptographic Keys",
            desc: "ML-DSA post-quantum keypair management, ed25519 signing, SubgraphKey \
                   (AES-GCM + HKDF) generation, X25519 ECDH encapsulation, and VC credential issuance.",
            icon: "key",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Security,
        },
        QApp {
            id: "zk-studio",
            name: "ZK Proof Studio",
            tagline: "Privacy Proofs",
            desc: "Author, compile, and verify zero-knowledge proofs via the zk_proofs module. \
                   Privacy-preserving selective disclosure of semantic subgraph data without revealing raw quins.",
            icon: "eye-slash",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Security,
        },
        QApp {
            id: "deontic-editor",
            name: "Deontic Logic Editor",
            tagline: "Normative Rules",
            desc: "Visual editor for N3Logic Rights Ontology rules. Author obligations, permissions, \
                   and prohibitions; evaluate them against intent graphs in the validate_intent() pre-flight gate.",
            icon: "journal-text",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Security,
        },
        QApp {
            id: "shacl-validator",
            name: "SHACL Validator",
            tagline: "Shape Constraints",
            desc: "Compile SHACL shapes to SlgOpcodes, validate RDF graphs interactively, inspect \
                   constraint violations, and link shapes to ODRL policies for automated enforcement.",
            icon: "check2-all",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Security,
        },
        QApp {
            id: "credential-manager",
            name: "Credential Manager",
            tagline: "Verifiable Credentials",
            desc: "Issue, hold, verify, and revoke W3C Verifiable Credentials. Manage SubgraphLayer \
                   unlock keys and Principal consent scopes for credential-gated subgraph access.",
            icon: "patch-check",
            route: None,
            stat: Stat::Soon,
            cat: Cat::Security,
        },
    ]
}
