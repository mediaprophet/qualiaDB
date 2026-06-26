//! The semantic library: an index over a directory of `.hmc` containers.
//!
//! Reading just the manifests is cheap (they are stored uncompressed at the top
//! of each zip), so a catalog of thousands of documents builds fast without
//! unpacking heavy assets. Embeddings are loaded on demand for dedup and search.
//!
//! This is where the "optimisation manifold" lives: documents become points in
//! embedding space, so the library can collapse near-duplicates, route a query
//! to the relevant region, and rank chunks by relevance/novelty — instead of
//! scanning every byte of a 60 GB corpus.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::container::{AssetKind, HmcContainer, HmcManifest};
use crate::embedding::{centroid, cosine, decode_f32_matrix};

/// One catalogued container (manifest + path; assets read on demand).
pub struct Entry {
    pub path: PathBuf,
    pub manifest: HmcManifest,
}

impl Entry {
    pub fn title(&self) -> &str {
        if self.manifest.source.title.is_empty() {
            &self.manifest.source.filename
        } else {
            &self.manifest.source.title
        }
    }
}

/// In-memory catalog of a library directory.
pub struct Library {
    pub root: PathBuf,
    pub entries: Vec<Entry>,
}

/// A chunk-level search hit.
pub struct Hit {
    pub doc_id: String,
    pub title: String,
    pub chunk_idx: usize,
    pub score: f32,
    pub snippet: String,
}

impl Library {
    /// Scan a directory for `.hmc` containers and read their manifests.
    pub fn scan(root: &Path) -> anyhow::Result<Self> {
        let mut entries = Vec::new();
        for e in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if !e.file_type().is_file() {
                continue;
            }
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some(crate::container::HMC_EXTENSION) {
                continue;
            }
            match HmcContainer::open(p) {
                Ok(c) => entries.push(Entry { path: p.to_path_buf(), manifest: c.manifest().clone() }),
                Err(err) => eprintln!("[library] skip {}: {err}", p.display()),
            }
        }
        Ok(Library { root: root.to_path_buf(), entries })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Exact duplicates by document identity (BLAKE3). Returns groups with >1
    /// member: (doc_id, paths).
    pub fn exact_duplicates(&self) -> Vec<(String, Vec<PathBuf>)> {
        use std::collections::BTreeMap;
        let mut by_id: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
        for e in &self.entries {
            by_id.entry(e.manifest.doc_id.clone()).or_default().push(e.path.clone());
        }
        by_id.into_iter().filter(|(_, v)| v.len() > 1).collect()
    }

    /// Load a container's chunk texts and embedding matrix, if present.
    fn load_vectors(&self, entry: &Entry) -> Option<(Vec<String>, Vec<Vec<f32>>)> {
        let dim = entry.manifest.pipeline.embed_dim as usize;
        if dim == 0 {
            return None;
        }
        let mut c = HmcContainer::open(&entry.path).ok()?;
        let emb = c.read_kind(AssetKind::Embeddings).ok()?;
        let vectors = decode_f32_matrix(&emb, dim);
        let chunks_raw = c.read_kind(AssetKind::Chunks).ok()?;
        let texts: Vec<String> = String::from_utf8_lossy(&chunks_raw)
            .lines()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .map(|v| v.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string())
            .collect();
        Some((texts, vectors))
    }

    /// Document-level centroid embedding (mean of chunk vectors), for near-dup
    /// detection and novelty scoring.
    pub fn document_centroid(&self, entry: &Entry) -> Option<Vec<f32>> {
        let (_, vectors) = self.load_vectors(entry)?;
        let c = centroid(&vectors);
        if c.is_empty() {
            None
        } else {
            Some(c)
        }
    }

    /// Near-duplicate document pairs above `threshold` cosine on their
    /// centroids (catches versions/reprints byte-hashing misses).
    pub fn near_duplicates(&self, threshold: f32) -> Vec<(String, String, f32)> {
        let cents: Vec<(String, Vec<f32>)> = self
            .entries
            .iter()
            .filter_map(|e| self.document_centroid(e).map(|c| (e.manifest.doc_id.clone(), c)))
            .collect();
        let mut out = Vec::new();
        for i in 0..cents.len() {
            for j in (i + 1)..cents.len() {
                let s = cosine(&cents[i].1, &cents[j].1);
                if s >= threshold {
                    out.push((cents[i].0.clone(), cents[j].0.clone(), s));
                }
            }
        }
        out.sort_by(|a, b| b.2.total_cmp(&a.2));
        out
    }

    /// Semantic search: rank chunks across the whole library by cosine to a
    /// query vector. Returns the top `k` hits.
    pub fn search(&self, query: &[f32], k: usize) -> Vec<Hit> {
        let mut hits: Vec<Hit> = Vec::new();
        for e in &self.entries {
            let Some((texts, vectors)) = self.load_vectors(e) else { continue };
            for (i, v) in vectors.iter().enumerate() {
                let score = cosine(query, v);
                let snippet = texts.get(i).map(|t| snippet(t)).unwrap_or_default();
                hits.push(Hit {
                    doc_id: e.manifest.doc_id.clone(),
                    title: e.title().to_string(),
                    chunk_idx: i,
                    score,
                    snippet,
                });
            }
        }
        hits.sort_by(|a, b| b.score.total_cmp(&a.score));
        hits.truncate(k);
        hits
    }

    /// Novelty of each document relative to a set of "known capability" vectors:
    /// 1 − max cosine to any known vector. Higher = more novel = worth attention.
    /// Returns (doc_id, title, novelty), most-novel first.
    pub fn novelty_ranking(&self, known: &[Vec<f32>]) -> Vec<(String, String, f32)> {
        let mut out = Vec::new();
        for e in &self.entries {
            let Some(c) = self.document_centroid(e) else { continue };
            let max_sim = known.iter().map(|k| cosine(&c, k)).fold(0.0f32, f32::max);
            out.push((e.manifest.doc_id.clone(), e.title().to_string(), 1.0 - max_sim));
        }
        out.sort_by(|a, b| b.2.total_cmp(&a.2));
        out
    }
}

fn snippet(text: &str) -> String {
    let s: String = text.chars().take(160).collect();
    s.replace('\n', " ")
}
