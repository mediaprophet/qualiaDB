//! Filesystem-backed Solid LDP resource store for a personal pod.
//!
//! Layout under `root`:
//! ```text
//! profile/card          WebID profile (Turtle)
//! public/               Public container
//! private/              Private container
//! inbox/                LDN inbox container
//! ```
//!
//! Paths are URL-decoded relative paths with `..` rejected (fail-closed).

use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};

/// Safe personal-pod root on disk.
#[derive(Debug, Clone)]
pub struct PodStore {
    root: PathBuf,
}

impl PodStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Create root + default containers, WebID profile, and seed Solid stack ontologies.
    pub fn ensure_defaults(&self, public_base: &str) -> io::Result<()> {
        fs::create_dir_all(self.root.join("profile"))?;
        fs::create_dir_all(self.root.join("public"))?;
        fs::create_dir_all(self.root.join("private"))?;
        fs::create_dir_all(self.root.join("inbox"))?;
        fs::create_dir_all(self.root.join("public").join("ontologies"))?;

        let card = self.root.join("profile").join("card");
        if !card.exists() {
            let webid = format!("{}/profile/card#me", public_base.trim_end_matches('/'));
            let issuer = public_base.trim_end_matches('/');
            // IRIs from bundled W3C ns archives (ldp, solid/terms, pim/space, foaf).
            let body = format!(
                r#"@prefix foaf: <{foaf}> .
@prefix solid: <{solid}> .
@prefix pim: <{pim}> .
@prefix ldp: <{ldp}> .

<#me> a foaf:Person ;
    foaf:name "Local Webizen" ;
    solid:oidcIssuer <{issuer}> ;
    pim:storage <{base}/> ;
    ldp:inbox <{base}/inbox/> .

<{webid}> a foaf:Person .
"#,
                foaf = crate::vocab::NS_FOAF,
                solid = crate::vocab::NS_SOLID,
                pim = crate::vocab::NS_PIM_SPACE,
                ldp = crate::vocab::NS_LDP,
                issuer = issuer,
                base = issuer,
                webid = webid,
            );
            let mut f = fs::File::create(&card)?;
            f.write_all(body.as_bytes())?;
        }

        // Empty container markers so clients can discover trees.
        for dir in ["public", "private", "inbox"] {
            let marker = self.root.join(dir).join(".meta");
            if !marker.exists() {
                let mut f = fs::File::create(marker)?;
                writeln!(
                    f,
                    "@prefix ldp: <{}> .\n<> a ldp:BasicContainer .",
                    crate::vocab::NS_LDP
                )?;
            }
        }

        self.seed_bundled_ontologies()?;
        Ok(())
    }

    /// Copy LDP / Solid / WAC / PIM / FOAF ontology files into the pod so clients
    /// can fetch them offline from `/public/ontologies/`.
    pub fn seed_bundled_ontologies(&self) -> io::Result<()> {
        let ont_dir = self.root.join("public").join("ontologies");
        fs::create_dir_all(&ont_dir)?;
        let mut index = String::from(
            "@prefix ldp: <http://www.w3.org/ns/ldp#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\n<> a ldp:BasicContainer .\n",
        );
        for (file, label) in crate::vocab::POD_ONTOLOGY_FILES {
            let dest = ont_dir.join(file);
            if !dest.exists() {
                if let Some(src) = crate::vocab::resolve_ontology_source(file) {
                    if let Err(e) = fs::copy(&src, &dest) {
                        eprintln!(
                            "solid-bridge: could not seed ontology {file} from {}: {e}",
                            src.display()
                        );
                        continue;
                    }
                } else {
                    eprintln!("solid-bridge: ontology source not found: {file}");
                    continue;
                }
            }
            if dest.exists() {
                index.push_str(&format!(
                    "<> ldp:contains <{file}> .\n<{file}> rdfs:label \"{label}\" .\n"
                ));
            }
        }
        let idx_path = ont_dir.join(".meta");
        fs::write(idx_path, index)?;
        Ok(())
    }

    /// Resolve a URL path (`/public/foo.ttl`) to an absolute filesystem path.
    pub fn resolve(&self, url_path: &str) -> io::Result<PathBuf> {
        let trimmed = url_path.trim_start_matches('/');
        if trimmed.is_empty() {
            return Ok(self.root.clone());
        }
        let mut out = self.root.clone();
        for comp in Path::new(trimmed).components() {
            match comp {
                Component::Normal(s) => out.push(s),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "path traversal rejected",
                    ));
                }
            }
        }
        // Must stay under root
        let canon_root = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        if let Ok(canon) = out.canonicalize() {
            if !canon.starts_with(&canon_root) {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "path escapes pod root",
                ));
            }
        } else {
            // Parent must be under root for create
            if let Some(parent) = out.parent() {
                if parent.exists() {
                    let cp = parent.canonicalize()?;
                    if !cp.starts_with(&canon_root) {
                        return Err(io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            "path escapes pod root",
                        ));
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn read_bytes(&self, url_path: &str) -> io::Result<Vec<u8>> {
        let path = self.resolve(url_path)?;
        if path.is_dir() {
            return Ok(self.container_listing_turtle(url_path)?.into_bytes());
        }
        fs::read(path)
    }

    pub fn write_bytes(&self, url_path: &str, body: &[u8]) -> io::Result<()> {
        let path = self.resolve(url_path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, body)
    }

    pub fn delete(&self, url_path: &str) -> io::Result<()> {
        let path = self.resolve(url_path)?;
        if path.is_dir() {
            fs::remove_dir_all(path)
        } else if path.exists() {
            fs::remove_file(path)
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, "not found"))
        }
    }

    pub fn exists(&self, url_path: &str) -> bool {
        self.resolve(url_path).map(|p| p.exists()).unwrap_or(false)
    }

    pub fn is_container(&self, url_path: &str) -> bool {
        self.resolve(url_path)
            .map(|p| p.is_dir() || url_path.ends_with('/'))
            .unwrap_or(false)
    }

    /// Minimal LDP BasicContainer listing as Turtle.
    pub fn container_listing_turtle(&self, url_path: &str) -> io::Result<String> {
        let path = self.resolve(url_path)?;
        let mut out = String::from(
            "@prefix ldp: <http://www.w3.org/ns/ldp#> .\n@prefix dcterms: <http://purl.org/dc/terms/> .\n\n<> a ldp:BasicContainer, ldp:Container .\n",
        );
        if path.is_dir() {
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                let slash = if entry.path().is_dir() { "/" } else { "" };
                out.push_str(&format!("<> ldp:contains <{}{}> .\n", name, slash));
            }
        }
        Ok(out)
    }

    /// Guess content-type from path suffix.
    pub fn content_type_for(url_path: &str) -> &'static str {
        let lower = url_path.to_ascii_lowercase();
        if lower.ends_with(".json") || lower.ends_with(".jsonld") {
            "application/ld+json"
        } else if lower.ends_with(".acl") || lower.ends_with(".ttl") || !lower.contains('.') {
            "text/turtle"
        } else if lower.ends_with(".html") {
            "text/html"
        } else {
            "application/octet-stream"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn rejects_traversal_and_round_trips() {
        let dir = env::temp_dir().join(format!("qualia-pod-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let store = PodStore::new(&dir);
        store.ensure_defaults("http://127.0.0.1:4243").unwrap();

        store
            .write_bytes("/public/hello.ttl", b"<a> <b> <c> .")
            .unwrap();
        let got = store.read_bytes("/public/hello.ttl").unwrap();
        assert_eq!(got, b"<a> <b> <c> .");

        assert!(
            store.resolve("/public/../profile/card").is_err()
                || store
                    .resolve("/public/../profile/card")
                    .map(|p| p.starts_with(&dir.join("profile")))
                    .unwrap_or(false)
        );

        // Explicit parent components rejected
        assert!(store.resolve("/../etc/passwd").is_err());

        let listing = String::from_utf8(store.read_bytes("/public/").unwrap()).unwrap();
        assert!(listing.contains("ldp:BasicContainer"));
        assert!(listing.contains("hello.ttl"));

        let _ = fs::remove_dir_all(&dir);
    }
}
