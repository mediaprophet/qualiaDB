//! Portable, Multi-Repo Qap Package & Host Lifecycle Contract.
//!
//! Defines the multi-repo distribution, repository source, and shared-service
//! dependency model enabling independent Git repository development, versioning,
//! installation, updates, and rollbacks.
//! Follows AGENTS.md §0-B (< 260 lines).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Distribution mode for a Qap application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QapDistributionMode {
    BundledCore,
    SeparateRepoDownloadable,
    LocalDevTree,
}

/// Shared OS-Shell system service dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QapSharedServiceDep {
    Scheduler,
    Networking,
    PermissiveCommons,
    CommunicationsBus,
    IdentifiersAndConsequence,
}

/// Upstream repository and signing provenance for a Qap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QapRepoSource {
    pub repo_url: String,
    pub did_git: String,
    pub release_channel: String,
    pub signer_identifier_did: String,
}

/// Complete multi-repo specification for an installable/updatable Qap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QapPackageSpec {
    pub qap_id: String,
    pub display_name: String,
    pub version: String,
    pub abi_version: String,
    pub distribution: QapDistributionMode,
    pub repo: QapRepoSource,
    pub shared_services: Vec<QapSharedServiceDep>,
    pub required_ontologies: Vec<String>,
    pub entrypoints: BTreeMap<String, String>,
}

impl QapPackageSpec {
    pub fn new(
        qap_id: &str,
        display_name: &str,
        version: &str,
        abi_version: &str,
        distribution: QapDistributionMode,
        repo_url: &str,
        did_git: &str,
        release_channel: &str,
        signer_identifier_did: &str,
        shared_services: Vec<QapSharedServiceDep>,
        required_ontologies: Vec<&str>,
        entrypoint_rel: &str,
    ) -> Self {
        let mut entrypoints = BTreeMap::new();
        entrypoints.insert("main".into(), entrypoint_rel.into());
        Self {
            qap_id: qap_id.into(),
            display_name: display_name.into(),
            version: version.into(),
            abi_version: abi_version.into(),
            distribution,
            repo: QapRepoSource {
                repo_url: repo_url.into(),
                did_git: did_git.into(),
                release_channel: release_channel.into(),
                signer_identifier_did: signer_identifier_did.into(),
            },
            shared_services,
            required_ontologies: required_ontologies.into_iter().map(String::from).collect(),
            entrypoints,
        }
    }
}

/// Returns the 7 canonical core Qap specifications with separate multi-repo sources.
pub fn canonical_core_qaps() -> Vec<QapPackageSpec> {
    use QapDistributionMode::*;
    use QapSharedServiceDep::*;
    let signer = "did:q42:release-signer";
    vec![
        QapPackageSpec::new(
            "poet",
            "POET Core HyperCanvas & Spatial Studio",
            "1.0.0",
            "1.0",
            BundledCore,
            "https://github.com/webizen/qap-poet",
            "did:git:webizen:qap-poet",
            "stable",
            signer,
            vec![Scheduler, Networking, PermissiveCommons, IdentifiersAndConsequence],
            vec!["qualia:PoetCore", "qualia:HyperCanvas"],
            "index.html#studio",
        ),
        QapPackageSpec::new(
            "wallet-crypto",
            "Multi-Chain Sovereign Wallet & Economic Settlement",
            "1.0.0",
            "1.0",
            BundledCore,
            "https://github.com/webizen/qap-wallet",
            "did:git:webizen:qap-wallet",
            "stable",
            signer,
            vec![IdentifiersAndConsequence, PermissiveCommons, Networking],
            vec!["qualia:Econ", "qualia:MultiChain"],
            "index.html#wallet",
        ),
        QapPackageSpec::new(
            "cooperative-projects",
            "Cooperative Projects & Labor Accounting Commons",
            "1.0.0",
            "1.0",
            BundledCore,
            "https://github.com/webizen/qap-coop",
            "did:git:webizen:qap-coop",
            "stable",
            signer,
            vec![Scheduler, CommunicationsBus, PermissiveCommons],
            vec!["qualia:CooperativeWork", "qualia:Labor"],
            "index.html#coop",
        ),
        QapPackageSpec::new(
            "hypermedia-library",
            "Hypermedia Asset & Aspect Graph Library",
            "1.0.0",
            "1.0",
            BundledCore,
            "https://github.com/webizen/qap-hypermedia",
            "did:git:webizen:qap-hypermedia",
            "stable",
            signer,
            vec![Networking, Scheduler, PermissiveCommons],
            vec!["qualia:Hypermedia", "qualia:AspectGraph"],
            "index.html#library",
        ),
        QapPackageSpec::new(
            "llm-agents",
            "Local LLM Swarm & Agentic Choreography",
            "1.0.0",
            "1.0",
            SeparateRepoDownloadable,
            "https://github.com/webizen/qap-llm-agents",
            "did:git:webizen:qap-llm-agents",
            "beta",
            signer,
            vec![Scheduler, PermissiveCommons, CommunicationsBus],
            vec!["qualia:Agentic", "qualia:LLM"],
            "index.html#agents",
        ),
        QapPackageSpec::new(
            "communications-directory",
            "Consent-Gated Purpose Messaging & DID Directory",
            "1.0.0",
            "1.0",
            BundledCore,
            "https://github.com/webizen/qap-comms",
            "did:git:webizen:qap-comms",
            "stable",
            signer,
            vec![CommunicationsBus, IdentifiersAndConsequence, Networking],
            vec!["qualia:Communications", "qualia:Directory"],
            "index.html#directory",
        ),
        QapPackageSpec::new(
            "web-browser",
            "Webizen Spatial Browser & Solid LDP Client",
            "1.0.0",
            "1.0",
            SeparateRepoDownloadable,
            "https://github.com/webizen/qap-browser",
            "did:git:webizen:qap-browser",
            "beta",
            signer,
            vec![Networking, PermissiveCommons, IdentifiersAndConsequence],
            vec!["qualia:Webizen", "qualia:SolidLDP"],
            "index.html#browser",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_canonical_core_qaps_validation() {
        let qaps = canonical_core_qaps();
        assert_eq!(qaps.len(), 7);

        let mut did_gits = BTreeSet::new();
        let mut repo_urls = BTreeSet::new();

        for qap in &qaps {
            assert!(!qap.qap_id.is_empty());
            assert!(!qap.display_name.is_empty());
            assert_eq!(qap.version, "1.0.0");
            assert_eq!(qap.abi_version, "1.0");
            assert!(qap.repo.repo_url.starts_with("https://github.com/webizen/qap-"));
            assert!(qap.repo.did_git.starts_with("did:git:webizen:qap-"));
            assert_eq!(qap.repo.signer_identifier_did, "did:q42:release-signer");
            assert!(!qap.shared_services.is_empty());
            assert!(!qap.entrypoints.is_empty());

            assert!(did_gits.insert(qap.repo.did_git.clone()), "duplicate did_git: {}", qap.repo.did_git);
            assert!(repo_urls.insert(qap.repo.repo_url.clone()), "duplicate repo_url: {}", qap.repo.repo_url);
        }

        assert_eq!(did_gits.len(), 7);
        assert_eq!(repo_urls.len(), 7);
    }
}
