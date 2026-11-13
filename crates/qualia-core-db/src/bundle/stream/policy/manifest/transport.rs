//! Read-only transport projection of the existing canonical closure manifest.
//! These rows are claims; only authenticate_closure supplies issuer recognition.
use super::ManifestView;
use crate::net::qdnf::types::StrongDigest;
use crate::semantic_instruments::resolve::DependencyKind;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestArtifact {
    pub digest: StrongDigest,
    pub kind: DependencyKind,
    pub length: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ManifestDependency {
    pub dependent: u8,
    pub dependency: u8,
}
impl ManifestView<'_> {
    pub fn artifact_count(&self) -> usize {
        self.rows.len() / super::ROW
    }
    pub fn dependency_count(&self) -> usize {
        self.edges.len() / 2
    }
    /// Canonical full-digest sorted ordinal used by QBS1 object selectors.
    pub fn artifact(&self, index: usize) -> Option<ManifestArtifact> {
        if index >= self.artifact_count() {
            return None;
        }
        let (digest, tag, length) = self.row(index);
        let kind = match tag {
            1 => DependencyKind::Ontology,
            2 => DependencyKind::Lexicon,
            3 => DependencyKind::Dataset,
            4 => DependencyKind::LawPackage,
            5 => DependencyKind::Instrument,
            6 => DependencyKind::NativeModule,
            _ => return None,
        };
        Some(ManifestArtifact {
            digest,
            kind,
            length,
        })
    }
    /// Endpoints index the same sorted artifact rows. No second graph format.
    pub fn dependency(&self, index: usize) -> Option<ManifestDependency> {
        let start = index.checked_mul(2)?;
        let pair = self.edges.get(start..start.checked_add(2)?)?;
        Some(ManifestDependency {
            dependent: pair[0],
            dependency: pair[1],
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::stream::policy::{
        decode_manifest, encode_manifest_into, ManifestFields, MAX_MANIFEST_BYTES,
    };
    use crate::crypto::network::digest::sha384;
    use crate::semantic_instruments::resolve::bounded_closure::{
        verify_closure, ClosureEdge, ClosureNode,
    };
    #[test]
    fn canonical_sorted_ordinals_and_edge_endpoints_are_exact_and_bounded() {
        let nodes = [
            ClosureNode {
                digest: sha384(b"root"),
                kind: DependencyKind::Instrument,
                original_bytes: b"root",
            },
            ClosureNode {
                digest: sha384(b"dependency"),
                kind: DependencyKind::LawPackage,
                original_bytes: b"dependency",
            },
        ];
        let edges = [ClosureEdge {
            dependent: nodes[0].digest,
            dependency: nodes[1].digest,
        }];
        let candidate = verify_closure(nodes[0].digest, &nodes, &edges).unwrap();
        let fields = ManifestFields {
            source: sha384(b"source"),
            source_length: 64,
            semantic_root: nodes[0].digest,
            policy: nodes[1].digest,
            consumer: nodes[0].digest,
            issuer: sha384(b"issuer"),
        };
        let mut bytes = [0; MAX_MANIFEST_BYTES];
        let n = encode_manifest_into(fields, &candidate, &mut bytes).unwrap();
        let view = decode_manifest(&bytes[..n]).unwrap();
        use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
        let measure = AllocGuard::begin("manifest-transport", true);
        assert_eq!(view.artifact_count(), 2);
        assert_eq!(view.dependency_count(), 1);
        assert!(view.artifact(0).unwrap().digest.0 < view.artifact(1).unwrap().digest.0);
        let edge = view.dependency(0).unwrap();
        let root = view.artifact(edge.dependent as usize).unwrap();
        let dependency = view.artifact(edge.dependency as usize).unwrap();
        assert_eq!(root.digest, nodes[0].digest);
        assert_eq!(root.kind, DependencyKind::Instrument);
        assert_eq!(root.length, 4);
        assert_eq!(dependency.digest, nodes[1].digest);
        assert_eq!(dependency.length, 10);
        for index in [2, usize::MAX] {
            assert!(view.artifact(index).is_none());
            assert!(view.dependency(index).is_none());
        }
        measure.check().unwrap();
    }
}
