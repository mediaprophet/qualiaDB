//! SPARQL-MM (Multimedia) Support
//!
//! Implements SPARQL-MM for media fragments and time-series windowing.
//! Supports Media Annotations Ontology (MA Ontology, http://www.w3.org/ns/ma-ont#).
//!
//! V6 repair: MA/C2PA constants use canonical `q_hash` (no placeholder collisions);
//! caller-buffered region/time queries; real spatial intersection; honest C2PA status.

use crate::q_hash;
use crate::sparql_ast::*;
use crate::NQuin;

/// Media Annotations Ontology predicate hashes (`q_hash` of canonical IRIs).
pub mod ma_ont {
    use crate::q_hash;

    pub const HAS_FRAGMENT: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasFragment");
    pub const HAS_TEMPORAL_FRAGMENT: u64 =
        q_hash("http://www.w3.org/ns/ma-ont#hasTemporalFragment");
    pub const HAS_SPATIAL_FRAGMENT: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasSpatialFragment");
    pub const HAS_TRACK_FRAGMENT: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasTrackFragment");

    pub const HAS_START_TIME: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasStartTime");
    pub const HAS_END_TIME: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasEndTime");
    pub const DURATION: u64 = q_hash("http://www.w3.org/ns/ma-ont#duration");

    pub const HAS_X: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasX");
    pub const HAS_Y: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasY");
    pub const HAS_WIDTH: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasWidth");
    pub const HAS_HEIGHT: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasHeight");

    pub const HAS_TRACK: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasTrack");
    pub const HAS_TRACK_NAME: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasTrackName");
    pub const HAS_TRACK_NUMBER: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasTrackNumber");

    pub const HAS_FORMAT: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasFormat");
    pub const HAS_MIME_TYPE: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasMimeType");
    pub const HAS_CODEC: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasCodec");

    pub const HAS_BITRATE: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasBitrate");
    pub const HAS_FRAMERATE: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasFramerate");
    pub const HAS_SAMPLERATE: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasSamplerate");
    pub const HAS_CHANNELS: u64 = q_hash("http://www.w3.org/ns/ma-ont#hasChannels");

    /// Qualia extension: packed media time base (ms), separate from Lamport bits.
    pub const MEDIA_TIME_MS: u64 = q_hash("https://ns.webizen.org/q42/mediaTimeMs");
}

/// C2PA predicate hashes (`q_hash` of documented IRIs — vocabulary for graph edges only).
pub mod c2pa {
    use crate::q_hash;

    pub const HAS_CREDENTIAL: u64 = q_hash("http://ns.c2pa.org/credentials/hasCredential");
    pub const HAS_MANIFEST: u64 = q_hash("http://ns.c2pa.org/manifest/hasManifest");
    pub const HAS_SIGNATURE: u64 = q_hash("http://ns.c2pa.org/signature/hasSignature");
    pub const HAS_PROVENANCE: u64 = q_hash("http://ns.c2pa.org/provenance/hasProvenance");
    pub const HAS_ASSERTION: u64 = q_hash("http://ns.c2pa.org/assertion/hasAssertion");

    pub const CREATED_AT: u64 = q_hash("http://ns.c2pa.org/provenance/createdAt");
    pub const CREATED_BY: u64 = q_hash("http://ns.c2pa.org/provenance/createdBy");
    pub const MODIFIED_AT: u64 = q_hash("http://ns.c2pa.org/provenance/modifiedAt");
    pub const MODIFIED_BY: u64 = q_hash("http://ns.c2pa.org/provenance/modifiedBy");
    pub const HAS_TOOL: u64 = q_hash("http://ns.c2pa.org/provenance/hasTool");

    pub const DERIVED_FROM: u64 = q_hash("http://ns.c2pa.org/asset/derivedFrom");
    pub const COMPONENT_OF: u64 = q_hash("http://ns.c2pa.org/asset/componentOf");
    pub const HAS_COMPONENT: u64 = q_hash("http://ns.c2pa.org/asset/hasComponent");

    pub const IS_VERIFIED: u64 = q_hash("http://ns.c2pa.org/validation/isVerified");
    pub const VERIFICATION_STATUS: u64 = q_hash("http://ns.c2pa.org/validation/verificationStatus");
    pub const HAS_CERTIFICATE: u64 = q_hash("http://ns.c2pa.org/validation/hasCertificate");
}

/// Honest C2PA verification status (design §6.3.7).
/// Field presence alone is never "verified".
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C2paVerificationStatus {
    /// No C2PA path implemented for this asset.
    Unsupported = 0,
    /// Manifest/claim edges present; no crypto check run.
    ParsedOnly = 1,
    /// Integrity hash matched; signature not checked.
    IntegrityChecked = 2,
    /// Signature verified against key material (not yet implemented here).
    SignatureVerified = 3,
    /// Full trust chain evaluated (not yet implemented here).
    TrustChainEvaluated = 4,
}

/// Media fragment dimensions
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaFragmentDimension {
    Temporal {
        start: u64,
        end: u64,
    },
    Spatial {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    Track {
        track_id: u64,
        track_number: u32,
    },
}

/// Media fragment
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MediaFragment {
    pub media_uri: u64,
    pub dimensions: [Option<MediaFragmentDimension>; 4],
    pub dimension_count: u8,
}

/// Time window type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    Tumbling { size_ms: u64 },
    // NOTE: this WindowType is a SPARQL-MM media-fragment window, NOT a
    // continuous-query (RSP-QL/C-SPARQL) stream window over the graph. Streaming
    // SPARQL is planned but unimplemented — see
    // docs/plans/immersive-sparql-hypermedia-profile.md §15d.
    Sliding { size_ms: u64, slide_ms: u64 },
    Session { gap_ms: u64 },
}

/// Time window
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TimeWindow {
    pub window_type: WindowType,
    pub start_ms: u64,
    pub end_ms: u64,
}

/// SPARQL-MM Media Handler
pub struct SparqlMmHandler<'a> {
    pub quins: &'a [NQuin],
    pub windows: [TimeWindow; 64],
    pub window_count: u8,
    pub media_fragments: [MediaFragment; 128],
    pub fragment_count: u8,
}

impl<'a> SparqlMmHandler<'a> {
    pub fn new(quins: &'a [NQuin]) -> Self {
        Self {
            quins,
            windows: [TimeWindow {
                window_type: WindowType::Tumbling { size_ms: 1000 },
                start_ms: 0,
                end_ms: 0,
            }; 64],
            window_count: 0,
            media_fragments: [MediaFragment {
                media_uri: 0,
                dimensions: [None; 4],
                dimension_count: 0,
            }; 128],
            fragment_count: 0,
        }
    }

    /// Build a media fragment from **explicit** dimensions (no hash-derived pseudo-parse).
    ///
    /// Callers that only have a media URI hash must pass dimensions separately —
    /// inventing temporal/spatial ranges from hash bits is forbidden (design §6.3.6).
    pub fn make_media_fragment(
        media_uri: u64,
        dimensions: &[MediaFragmentDimension],
    ) -> Result<MediaFragment, String> {
        if dimensions.len() > 4 {
            return Err("At most 4 fragment dimensions".to_string());
        }
        let mut fragment = MediaFragment {
            media_uri,
            dimensions: [None; 4],
            dimension_count: 0,
        };
        for (i, d) in dimensions.iter().enumerate() {
            fragment.dimensions[i] = Some(*d);
            fragment.dimension_count += 1;
        }
        Ok(fragment)
    }

    /// Legacy entry: returns a fragment with **media_uri only** (no invented dimensions).
    /// Prefer `make_media_fragment` with explicit temporal/spatial/track dims.
    pub fn parse_media_fragment(&mut self, fragment_uri: u64) -> Result<MediaFragment, String> {
        Self::make_media_fragment(fragment_uri, &[])
    }

    /// Pack xywh into a single object payload (same layout as vision `pack_bbox` family).
    #[inline]
    pub fn pack_spatial_u64(x: u32, y: u32, width: u32, height: u32) -> u64 {
        // Clamp to u16 lanes for fixed packing (normalized or pixel coords ≤ 65535).
        let x = (x.min(0xFFFF)) as u64;
        let y = (y.min(0xFFFF)) as u64;
        let w = (width.min(0xFFFF)) as u64;
        let h = (height.min(0xFFFF)) as u64;
        x | (y << 16) | (w << 32) | (h << 48)
    }

    #[inline]
    pub fn unpack_spatial_u64(v: u64) -> (u32, u32, u32, u32) {
        (
            (v & 0xFFFF) as u32,
            ((v >> 16) & 0xFFFF) as u32,
            ((v >> 32) & 0xFFFF) as u32,
            ((v >> 48) & 0xFFFF) as u32,
        )
    }

    /// Axis-aligned box intersection (x,y,w,h).
    #[inline]
    pub fn spatial_intersects(
        ax: u32,
        ay: u32,
        aw: u32,
        ah: u32,
        bx: u32,
        by: u32,
        bw: u32,
        bh: u32,
    ) -> bool {
        let ax1 = ax.saturating_add(aw);
        let ay1 = ay.saturating_add(ah);
        let bx1 = bx.saturating_add(bw);
        let by1 = by.saturating_add(bh);
        ax < bx1 && ax1 > bx && ay < by1 && ay1 > by
    }

    /// Media time from a quin that uses `ma_ont::MEDIA_TIME_MS` (object = ms),
    /// **not** the Lamport field in metadata.
    pub fn media_time_ms(quin: &NQuin) -> Option<u64> {
        if quin.predicate == ma_ont::MEDIA_TIME_MS {
            Some(quin.object)
        } else {
            None
        }
    }

    /// Get MA Ontology property for a media resource
    pub fn get_ma_property(&self, media_uri: u64, predicate: u64) -> Result<u64, String> {
        for quin in self.quins {
            if quin.subject == media_uri && quin.predicate == predicate {
                return Ok(quin.object);
            }
        }
        Err("Property not found".to_string())
    }

    /// Get temporal fragment using MA Ontology
    pub fn get_temporal_fragment(&self, media_uri: u64) -> Result<(u64, u64), String> {
        let start = self.get_ma_property(media_uri, ma_ont::HAS_START_TIME)?;
        let end = self.get_ma_property(media_uri, ma_ont::HAS_END_TIME)?;
        Ok((start, end))
    }

    /// Get spatial fragment using MA Ontology
    pub fn get_spatial_fragment(&self, media_uri: u64) -> Result<(u32, u32, u32, u32), String> {
        let x = self.get_ma_property(media_uri, ma_ont::HAS_X)? as u32;
        let y = self.get_ma_property(media_uri, ma_ont::HAS_Y)? as u32;
        let width = self.get_ma_property(media_uri, ma_ont::HAS_WIDTH)? as u32;
        let height = self.get_ma_property(media_uri, ma_ont::HAS_HEIGHT)? as u32;
        Ok((x, y, width, height))
    }

    /// Get track fragment using MA Ontology
    pub fn get_track_fragment(&self, media_uri: u64) -> Result<(u64, u32), String> {
        let track_id = self.get_ma_property(media_uri, ma_ont::HAS_TRACK)?;
        let track_number = self.get_ma_property(media_uri, ma_ont::HAS_TRACK_NUMBER)? as u32;
        Ok((track_id, track_number))
    }

    /// Add a media fragment
    pub fn add_media_fragment(&mut self, fragment: MediaFragment) -> Result<u8, String> {
        if self.fragment_count >= 128 {
            return Err("Fragment overflow".to_string());
        }
        let idx = self.fragment_count;
        self.media_fragments[idx as usize] = fragment;
        self.fragment_count += 1;
        Ok(idx)
    }

    /// Create a tumbling time window
    pub fn create_tumbling_window(&mut self, size_ms: u64, start_ms: u64) -> Result<u8, String> {
        if self.window_count >= 64 {
            return Err("Window overflow".to_string());
        }
        let idx = self.window_count;
        self.windows[idx as usize] = TimeWindow {
            window_type: WindowType::Tumbling { size_ms },
            start_ms,
            end_ms: start_ms + size_ms,
        };
        self.window_count += 1;
        Ok(idx)
    }

    /// Create a sliding time window
    pub fn create_sliding_window(
        &mut self,
        size_ms: u64,
        slide_ms: u64,
        start_ms: u64,
    ) -> Result<u8, String> {
        if self.window_count >= 64 {
            return Err("Window overflow".to_string());
        }
        let idx = self.window_count;
        self.windows[idx as usize] = TimeWindow {
            window_type: WindowType::Sliding { size_ms, slide_ms },
            start_ms,
            end_ms: start_ms + size_ms,
        };
        self.window_count += 1;
        Ok(idx)
    }

    /// Create a session window
    pub fn create_session_window(&mut self, gap_ms: u64, start_ms: u64) -> Result<u8, String> {
        if self.window_count >= 64 {
            return Err("Window overflow".to_string());
        }
        let idx = self.window_count;
        self.windows[idx as usize] = TimeWindow {
            window_type: WindowType::Session { gap_ms },
            start_ms,
            end_ms: 0, // Dynamic
        };
        self.window_count += 1;
        Ok(idx)
    }

    /// Query quins within a time window (heap-compatible wrapper).
    /// Prefer `query_window_into` on hot paths.
    pub fn query_window(
        &self,
        window_id: u8,
        _timestamp_field: u64,
    ) -> Result<Vec<&NQuin>, String> {
        let mut buf = [None; 256];
        let n = self.query_window_into(window_id, &mut buf)?;
        Ok(buf[..n].iter().filter_map(|x| *x).collect())
    }

    /// Caller-buffered window query. Uses **media time** when
    /// `predicate == MEDIA_TIME_MS`; otherwise falls back to metadata low 29 bits
    /// with the understanding that that path is Lamport-mixed (not pure media time).
    pub fn query_window_into(
        &'a self,
        window_id: u8,
        out: &mut [Option<&'a NQuin>],
    ) -> Result<usize, String> {
        let window = self
            .windows
            .get(window_id as usize)
            .filter(|_| (window_id as usize) < self.window_count as usize)
            .ok_or("Window ID out of bounds")?;

        let mut w = 0usize;
        for quin in self.quins {
            let t = if let Some(ms) = Self::media_time_ms(quin) {
                ms
            } else {
                quin.metadata & 0x1FFF_FFFF
            };
            if t >= window.start_ms && t <= window.end_ms {
                if w >= out.len() {
                    break;
                }
                out[w] = Some(quin);
                w += 1;
            }
        }
        Ok(w)
    }

    /// Query media fragment (heap wrapper). Prefer `query_media_fragment_into`.
    pub fn query_media_fragment(&self, fragment_id: u8) -> Result<Vec<&NQuin>, String> {
        let mut buf = [None; 256];
        let n = self.query_media_fragment_into(fragment_id, &mut buf)?;
        Ok(buf[..n].iter().filter_map(|x| *x).collect())
    }

    /// Caller-buffered media-fragment query with real spatial intersection.
    pub fn query_media_fragment_into(
        &'a self,
        fragment_id: u8,
        out: &mut [Option<&'a NQuin>],
    ) -> Result<usize, String> {
        let fragment = self
            .media_fragments
            .get(fragment_id as usize)
            .filter(|_| (fragment_id as usize) < self.fragment_count as usize)
            .ok_or("Fragment ID out of bounds")?;

        let mut w = 0usize;
        for quin in self.quins {
            if !self.check_fragment_match(quin, fragment) {
                continue;
            }
            if w >= out.len() {
                break;
            }
            out[w] = Some(quin);
            w += 1;
        }
        Ok(w)
    }

    fn check_fragment_match(&self, quin: &NQuin, fragment: &MediaFragment) -> bool {
        if fragment.dimension_count == 0 {
            return quin.subject == fragment.media_uri
                || self.quin_linked_to_media(quin, fragment.media_uri);
        }

        if !(quin.subject == fragment.media_uri
            || self.quin_linked_to_media(quin, fragment.media_uri))
        {
            return false;
        }

        for i in 0..fragment.dimension_count as usize {
            let Some(dim) = fragment.dimensions[i] else {
                continue;
            };
            match dim {
                MediaFragmentDimension::Temporal { start, end } => {
                    let Some(quin_time) = Self::media_time_ms(quin).or_else(|| {
                        if quin.predicate == ma_ont::HAS_START_TIME
                            || quin.predicate == ma_ont::HAS_END_TIME
                        {
                            Some(quin.object)
                        } else {
                            None
                        }
                    }) else {
                        // No media-time payload on this quin → temporal dim does not reject.
                        continue;
                    };
                    if quin_time < start || quin_time > end {
                        return false;
                    }
                }
                MediaFragmentDimension::Spatial {
                    x,
                    y,
                    width,
                    height,
                } => {
                    // If this quin carries a box, require intersection; else pass.
                    if let Some((qx, qy, qw, qh)) = self.quin_spatial_box(quin) {
                        if !Self::spatial_intersects(qx, qy, qw, qh, x, y, width, height) {
                            return false;
                        }
                    }
                }
                MediaFragmentDimension::Track { track_id, .. } => {
                    let is_track_pred = quin.predicate == ma_ont::HAS_TRACK
                        || quin.predicate == ma_ont::HAS_TRACK_NUMBER
                        || quin.predicate == q_hash("https://ns.webizen.org/q42/hasTrackId");
                    if is_track_pred && quin.object != track_id {
                        return false;
                    }
                }
            }
        }
        true
    }

    fn quin_linked_to_media(&self, quin: &NQuin, media_uri: u64) -> bool {
        if quin.subject == media_uri {
            return true;
        }
        // Observation pattern: media --VisualObservation--> instance
        for q in self.quins {
            if q.subject == media_uri && q.object == quin.subject {
                return true;
            }
        }
        false
    }

    /// Resolve a box for this quin: packed HAS_SPATIAL_FRAGMENT object, or HAS_X/Y/W/H props.
    fn quin_spatial_box(&self, quin: &NQuin) -> Option<(u32, u32, u32, u32)> {
        if quin.predicate == ma_ont::HAS_SPATIAL_FRAGMENT
            || quin.predicate == q_hash("https://ns.webizen.org/q42/hasBoundingBox")
        {
            let (x, y, w, h) = Self::unpack_spatial_u64(quin.object);
            // hasBoundingBox stores x0,y0,x1,y1 not x,y,w,h — convert if x1>x0 style.
            if quin.predicate == q_hash("https://ns.webizen.org/q42/hasBoundingBox") {
                let x1 = w;
                let y1 = h;
                let width = x1.saturating_sub(x);
                let height = y1.saturating_sub(y);
                return Some((x, y, width, height));
            }
            return Some((x, y, w, h));
        }
        // Compose from component properties on same subject.
        let mut x = None;
        let mut y = None;
        let mut width = None;
        let mut height = None;
        for q in self.quins {
            if q.subject != quin.subject {
                continue;
            }
            match q.predicate {
                p if p == ma_ont::HAS_X => x = Some(q.object as u32),
                p if p == ma_ont::HAS_Y => y = Some(q.object as u32),
                p if p == ma_ont::HAS_WIDTH => width = Some(q.object as u32),
                p if p == ma_ont::HAS_HEIGHT => height = Some(q.object as u32),
                _ => {}
            }
        }
        match (x, y, width, height) {
            (Some(x), Some(y), Some(w), Some(h)) => Some((x, y, w, h)),
            _ => None,
        }
    }

    /// Get media duration using MA Ontology
    pub fn get_media_duration(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, ma_ont::DURATION)
    }

    /// Get media dimensions using MA Ontology
    pub fn get_media_dimensions(&self, media_uri: u64) -> Result<(u32, u32), String> {
        let width = self.get_ma_property(media_uri, ma_ont::HAS_WIDTH)? as u32;
        let height = self.get_ma_property(media_uri, ma_ont::HAS_HEIGHT)? as u32;
        Ok((width, height))
    }

    /// Get media format using MA Ontology
    pub fn get_media_format(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, ma_ont::HAS_FORMAT)
    }

    /// Get media MIME type using MA Ontology
    pub fn get_media_mime_type(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, ma_ont::HAS_MIME_TYPE)
    }

    /// Get media codec using MA Ontology
    pub fn get_media_codec(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, ma_ont::HAS_CODEC)
    }

    /// Get media bitrate using MA Ontology
    pub fn get_media_bitrate(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, ma_ont::HAS_BITRATE)
    }

    /// Get media framerate using MA Ontology
    pub fn get_media_framerate(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, ma_ont::HAS_FRAMERATE)
    }

    /// C2PA: Get content credential for media
    pub fn get_credential(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::HAS_CREDENTIAL)
    }

    /// C2PA: Get manifest for media
    pub fn get_manifest(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::HAS_MANIFEST)
    }

    /// C2PA: Get signature for media
    pub fn get_signature(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::HAS_SIGNATURE)
    }

    /// C2PA: Get provenance for media
    pub fn get_provenance(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::HAS_PROVENANCE)
    }

    /// C2PA: Check if media is cryptographically verified.
    ///
    /// Honest policy: a stored `isVerified=1` edge is **not** sufficient.
    /// Returns `Ok(true)` only when status is SignatureVerified or TrustChainEvaluated.
    /// Today this engine never reaches those levels → typically `Ok(false)` or Err.
    pub fn is_verified(&self, media_uri: u64) -> Result<bool, String> {
        let status = self.c2pa_status(media_uri)?;
        Ok(matches!(
            status,
            C2paVerificationStatus::SignatureVerified | C2paVerificationStatus::TrustChainEvaluated
        ))
    }

    /// Honest verification ladder for C2PA (design §6.3.7).
    pub fn c2pa_status(&self, media_uri: u64) -> Result<C2paVerificationStatus, String> {
        // Full crypto path is not implemented in this module.
        let has_manifest = self.get_ma_property(media_uri, c2pa::HAS_MANIFEST).is_ok();
        let has_sig = self.get_ma_property(media_uri, c2pa::HAS_SIGNATURE).is_ok();
        if !has_manifest && !has_sig {
            return Ok(C2paVerificationStatus::Unsupported);
        }
        // Edges present only → ParsedOnly. Never promote to verified.
        let _claimed = self.get_ma_property(media_uri, c2pa::IS_VERIFIED);
        Ok(C2paVerificationStatus::ParsedOnly)
    }

    /// C2PA: Get verification status as u64 enum discriminant.
    pub fn get_verification_status(&self, media_uri: u64) -> Result<u64, String> {
        Ok(self.c2pa_status(media_uri)? as u64)
    }

    /// C2PA: Get creation timestamp
    pub fn get_created_at(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::CREATED_AT)
    }

    /// C2PA: Get creator
    pub fn get_created_by(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::CREATED_BY)
    }

    /// C2PA: Get modification timestamp
    pub fn get_modified_at(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::MODIFIED_AT)
    }

    /// C2PA: Get modifier
    pub fn get_modified_by(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::MODIFIED_BY)
    }

    /// C2PA: Get tool used to create media
    pub fn get_tool(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::HAS_TOOL)
    }

    /// C2PA: Get source asset (derived from)
    pub fn get_derived_from(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::DERIVED_FROM)
    }

    /// C2PA: Get parent asset (component of)
    pub fn get_component_of(&self, media_uri: u64) -> Result<u64, String> {
        self.get_ma_property(media_uri, c2pa::COMPONENT_OF)
    }

    /// C2PA: Get component assets
    pub fn get_components(&self, media_uri: u64) -> Result<Vec<u64>, String> {
        let mut components = Vec::new();
        for quin in self.quins {
            if quin.subject == media_uri && quin.predicate == c2pa::HAS_COMPONENT {
                components.push(quin.object);
            }
        }
        Ok(components)
    }

    /// C2PA: Verify content signature.
    /// **Unsupported** in this build — always returns `Ok(false)` if a signature
    /// edge exists (ParsedOnly), or `Err` if missing. Never claims crypto success.
    pub fn verify_signature(&self, media_uri: u64) -> Result<bool, String> {
        let _signature = self.get_signature(media_uri)?;
        // Real signature verification is out of scope for SPARQL-MM accessors.
        Ok(false)
    }

    /// Aggregate over time window
    pub fn window_aggregate(
        &self,
        window_id: u8,
        aggregate_fn: fn(&[&NQuin]) -> u64,
    ) -> Result<u64, String> {
        let quins = self.query_window(window_id, 0)?;
        Ok(aggregate_fn(&quins))
    }
}

impl<'a> Default for SparqlMmHandler<'a> {
    fn default() -> Self {
        Self::new(&[])
    }
}

/// SPARQL-MM extension functions
pub fn mm_duration(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_media_duration(media_uri) {
        Ok(duration) => {
            result.slots[0] = Some(duration);
            true
        }
        Err(_) => false,
    }
}

pub fn mm_dimensions(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_media_dimensions(media_uri) {
        Ok((width, height)) => {
            result.slots[0] = Some(width as u64);
            result.slots[1] = Some(height as u64);
            true
        }
        Err(_) => false,
    }
}

pub fn mm_temporal_fragment(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.len() < 2 {
        return false;
    }
    let media_uri = args[0];
    let start = args[1];
    let end = args.get(2).copied().unwrap_or(start);

    let mut handler = SparqlMmHandler::new(quins);
    let fragment = MediaFragment {
        media_uri,
        dimensions: [
            Some(MediaFragmentDimension::Temporal { start, end }),
            None,
            None,
            None,
        ],
        dimension_count: 1,
    };

    match handler.add_media_fragment(fragment) {
        Ok(_) => {
            result.slots[0] = Some(1); // Success
            true
        }
        Err(_) => false,
    }
}

/// MA Ontology extension functions
pub fn ma_format(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_media_format(media_uri) {
        Ok(format) => {
            result.slots[0] = Some(format);
            true
        }
        Err(_) => false,
    }
}

pub fn ma_mime_type(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_media_mime_type(media_uri) {
        Ok(mime_type) => {
            result.slots[0] = Some(mime_type);
            true
        }
        Err(_) => false,
    }
}

pub fn ma_codec(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_media_codec(media_uri) {
        Ok(codec) => {
            result.slots[0] = Some(codec);
            true
        }
        Err(_) => false,
    }
}

pub fn ma_bitrate(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_media_bitrate(media_uri) {
        Ok(bitrate) => {
            result.slots[0] = Some(bitrate);
            true
        }
        Err(_) => false,
    }
}
pub fn ma_framerate(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_media_framerate(media_uri) {
        Ok(framerate) => {
            result.slots[0] = Some(framerate);
            true
        }
        Err(_) => false,
    }
}

/// C2PA extension functions

/// c2pa:credential - get content credential
pub fn c2pa_credential(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_credential(media_uri) {
        Ok(credential) => {
            result.slots[0] = Some(credential);
            true
        }
        Err(_) => false,
    }
}

/// c2pa:isVerified - check if media is verified
pub fn c2pa_is_verified(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.is_verified(media_uri) {
        Ok(verified) => {
            result.slots[0] = Some(if verified { 1 } else { 0 });
            true
        }
        Err(_) => false,
    }
}

/// c2pa:verificationStatus - get verification status
pub fn c2pa_verification_status(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_verification_status(media_uri) {
        Ok(status) => {
            result.slots[0] = Some(status);
            true
        }
        Err(_) => false,
    }
}

/// c2pa:createdAt - get creation timestamp
pub fn c2pa_created_at(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_created_at(media_uri) {
        Ok(timestamp) => {
            result.slots[0] = Some(timestamp);
            true
        }
        Err(_) => false,
    }
}

/// c2pa:createdBy - get creator
pub fn c2pa_created_by(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_created_by(media_uri) {
        Ok(creator) => {
            result.slots[0] = Some(creator);
            true
        }
        Err(_) => false,
    }
}

/// c2pa:verifySignature - verify content signature
pub fn c2pa_verify_signature(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.verify_signature(media_uri) {
        Ok(verified) => {
            result.slots[0] = Some(if verified { 1 } else { 0 });
            true
        }
        Err(_) => false,
    }
}

/// c2pa:derivedFrom - get source asset
pub fn c2pa_derived_from(args: &[u64], quins: &[NQuin], result: &mut BindingRow) -> bool {
    if args.is_empty() {
        return false;
    }
    let media_uri = args[0];

    let handler = SparqlMmHandler::new(quins);
    match handler.get_derived_from(media_uri) {
        Ok(source) => {
            result.slots[0] = Some(source);
            true
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mm_handler_creation() {
        let quins = vec![];
        let handler = SparqlMmHandler::new(&quins);
        assert_eq!(handler.window_count, 0);
    }

    #[test]
    fn test_create_tumbling_window() {
        let quins = vec![];
        let mut handler = SparqlMmHandler::new(&quins);

        let result = handler.create_tumbling_window(1000, 0);
        assert!(result.is_ok());
        assert_eq!(handler.window_count, 1);
    }

    #[test]
    fn test_create_sliding_window() {
        let quins = vec![];
        let mut handler = SparqlMmHandler::new(&quins);

        let result = handler.create_sliding_window(1000, 500, 0);
        assert!(result.is_ok());
        assert_eq!(handler.window_count, 1);
    }

    #[test]
    fn test_parse_media_fragment_no_invented_dims() {
        let quins = vec![];
        let mut handler = SparqlMmHandler::new(&quins);

        let fragment = handler.parse_media_fragment(12345).unwrap();
        assert_eq!(fragment.media_uri, 12345);
        assert_eq!(fragment.dimension_count, 0);
    }

    #[test]
    fn ma_ont_constants_are_distinct() {
        // Former placeholders collided (HAS_FRAGMENT == HAS_CODEC etc.).
        assert_ne!(ma_ont::HAS_FRAGMENT, ma_ont::HAS_CODEC);
        assert_ne!(ma_ont::HAS_BITRATE, ma_ont::HAS_TEMPORAL_FRAGMENT);
        assert_ne!(ma_ont::HAS_X, ma_ont::HAS_Y);
        assert_ne!(c2pa::HAS_CREDENTIAL, c2pa::HAS_MANIFEST);
        assert_ne!(c2pa::DERIVED_FROM, ma_ont::HAS_MIME_TYPE);
    }

    #[test]
    fn spatial_intersection_real() {
        assert!(SparqlMmHandler::spatial_intersects(
            0, 0, 10, 10, 5, 5, 10, 10
        ));
        assert!(!SparqlMmHandler::spatial_intersects(
            0, 0, 10, 10, 20, 20, 5, 5
        ));
    }

    #[test]
    fn query_window_into_media_time() {
        let media = q_hash("media:clip");
        let quins = [
            NQuin {
                subject: media,
                predicate: ma_ont::MEDIA_TIME_MS,
                object: 500,
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: media,
                predicate: ma_ont::MEDIA_TIME_MS,
                object: 2500,
                context: 0,
                metadata: 0,
                parity: 0,
            },
        ];
        // Fix parity for honesty (not required for query)
        let mut handler = SparqlMmHandler::new(&quins);
        handler.create_tumbling_window(1000, 0).unwrap(); // [0,1000]
        let mut out = [None; 8];
        let n = handler.query_window_into(0, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0].unwrap().object, 500);
    }

    #[test]
    fn query_media_fragment_into_spatial() {
        let media = q_hash("media:img");
        let packed = SparqlMmHandler::pack_spatial_u64(10, 10, 50, 50);
        let quins = [NQuin {
            subject: media,
            predicate: ma_ont::HAS_SPATIAL_FRAGMENT,
            object: packed,
            context: 0,
            metadata: 0,
            parity: 0,
        }];
        let mut handler = SparqlMmHandler::new(&quins);
        let frag = SparqlMmHandler::make_media_fragment(
            media,
            &[MediaFragmentDimension::Spatial {
                x: 20,
                y: 20,
                width: 20,
                height: 20,
            }],
        )
        .unwrap();
        handler.add_media_fragment(frag).unwrap();
        let mut out = [None; 4];
        let n = handler.query_media_fragment_into(0, &mut out).unwrap();
        assert_eq!(n, 1);
    }

    #[test]
    fn c2pa_never_claims_verified_from_field_presence() {
        let media = q_hash("media:photo");
        let quins = [
            NQuin {
                subject: media,
                predicate: c2pa::HAS_SIGNATURE,
                object: 0xABC,
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: media,
                predicate: c2pa::IS_VERIFIED,
                object: 1,
                context: 0,
                metadata: 0,
                parity: 0,
            },
        ];
        let handler = SparqlMmHandler::new(&quins);
        assert_eq!(
            handler.c2pa_status(media).unwrap(),
            C2paVerificationStatus::ParsedOnly
        );
        assert!(!handler.is_verified(media).unwrap());
        assert!(!handler.verify_signature(media).unwrap());
    }
}
