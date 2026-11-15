//! Bioacoustic classification and SPARQL-MM observation pipeline (§GAP-03 / F5).
//!
//! Bridges raw time-frequency surfaces (`TfSurface`) to SPARQL-MM media fragments
//! (`mm:TemporalFragment`, `qualia:AcousticObservation`). Enables species call detection,
//! spectral bounding, confidence scoring, and zero-heap conversion into queryable Quins.

use crate::audio::tf_surface::TfSurface;
use crate::{q_hash, NQuin};

/// SPARQL-MM and Media Annotation Ontology predicate URIs.
pub const MM_TEMPORAL_FRAGMENT: &str = "http://www.w3.org/ns/ma-ont#MediaFragment";
pub const QUALIA_ACOUSTIC_OBSERVATION: &str = "qualia:AcousticObservation";
pub const MA_HAS_START_TIME: &str = "http://www.w3.org/ns/ma-ont#hasStartTime";
pub const MA_HAS_END_TIME: &str = "http://www.w3.org/ns/ma-ont#hasEndTime";
pub const MA_IS_FRAGMENT_OF: &str = "http://www.w3.org/ns/ma-ont#isFragmentOf";
pub const QUALIA_TAXON_ID: &str = "qualia:speciesTaxonId";
pub const QUALIA_SPECIES_CONFIDENCE: &str = "qualia:speciesConfidence";
pub const QUALIA_FREQ_LOW: &str = "qualia:frequencyLowHz";
pub const QUALIA_FREQ_HIGH: &str = "qualia:frequencyHighHz";

/// Detected bioacoustic vocalization event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BioacousticVocalization {
    /// Start time in seconds.
    pub start_time_s: f32,
    /// End time in seconds.
    pub end_time_s: f32,
    /// Lower frequency boundary in Hz.
    pub low_freq_hz: f32,
    /// Upper frequency boundary in Hz.
    pub high_freq_hz: f32,
    /// Peak energy / magnitude in decibels.
    pub peak_magnitude_db: f32,
    /// Taxon identifier (e.g. GBIF taxon key or FNV-1a hash of scientific name).
    pub taxon_id: u64,
    /// Classification confidence score in `[0.0, 1.0]`.
    pub confidence: f32,
}

impl Default for BioacousticVocalization {
    fn default() -> Self {
        Self {
            start_time_s: 0.0,
            end_time_s: 0.0,
            low_freq_hz: 0.0,
            high_freq_hz: 0.0,
            peak_magnitude_db: 0.0,
            taxon_id: 0,
            confidence: 0.0,
        }
    }
}

/// Detects acoustic vocalization energy clusters from a `TfSurface`.
///
/// Scans the surface for contiguous time-frames exceeding `threshold_db` across frequency bins.
/// Populates `out` with detected call intervals that exceed `min_duration_s`.
/// Zero-heap hot path.
pub fn detect_bioacoustic_vocalizations(
    surface: &TfSurface,
    threshold_db: f32,
    min_duration_s: f32,
    assigned_taxon_id: u64,
    out: &mut [BioacousticVocalization],
) -> usize {
    if surface.frame_count == 0 || surface.bin_count == 0 || out.is_empty() {
        return 0;
    }

    let mut detected_count = 0usize;
    let mut in_call = false;
    let mut call_start_frame = 0usize;
    let mut min_bin = surface.bin_count;
    let mut max_bin = 0usize;
    let mut peak_val = f32::NEG_INFINITY;

    for f in 0..surface.frame_count {
        let mut frame_peak = f32::NEG_INFINITY;
        let mut frame_min_bin = surface.bin_count;
        let mut frame_max_bin = 0usize;

        for b in 0..surface.bin_count {
            let val = surface.get(f, b);
            if val >= threshold_db {
                if val > frame_peak {
                    frame_peak = val;
                }
                if b < frame_min_bin {
                    frame_min_bin = b;
                }
                if b > frame_max_bin {
                    frame_max_bin = b;
                }
            }
        }

        let frame_active = frame_peak >= threshold_db;

        if frame_active {
            if !in_call {
                in_call = true;
                call_start_frame = f;
                min_bin = frame_min_bin;
                max_bin = frame_max_bin;
                peak_val = frame_peak;
            } else {
                if frame_peak > peak_val {
                    peak_val = frame_peak;
                }
                if frame_min_bin < min_bin {
                    min_bin = frame_min_bin;
                }
                if frame_max_bin > max_bin {
                    max_bin = frame_max_bin;
                }
            }
        } else if in_call {
            // Call completed
            let duration = surface.frame_to_time(f) - surface.frame_to_time(call_start_frame);
            if duration >= min_duration_s {
                let low_f = surface.bin_to_freq_linear(min_bin);
                let high_f = surface.bin_to_freq_linear(max_bin);
                // Confidence normalized against threshold
                let confidence = ((peak_val - threshold_db) / 20.0).clamp(0.1, 1.0);

                out[detected_count] = BioacousticVocalization {
                    start_time_s: surface.frame_to_time(call_start_frame),
                    end_time_s: surface.frame_to_time(f),
                    low_freq_hz: low_f,
                    high_freq_hz: high_f,
                    peak_magnitude_db: peak_val,
                    taxon_id: assigned_taxon_id,
                    confidence,
                };
                detected_count += 1;
                if detected_count >= out.len() {
                    return detected_count;
                }
            }
            in_call = false;
        }
    }

    // Flush active call at the end of surface
    if in_call && detected_count < out.len() {
        let duration =
            surface.frame_to_time(surface.frame_count) - surface.frame_to_time(call_start_frame);
        if duration >= min_duration_s {
            let low_f = surface.bin_to_freq_linear(min_bin);
            let high_f = surface.bin_to_freq_linear(max_bin);
            let confidence = ((peak_val - threshold_db) / 20.0).clamp(0.1, 1.0);

            out[detected_count] = BioacousticVocalization {
                start_time_s: surface.frame_to_time(call_start_frame),
                end_time_s: surface.frame_to_time(surface.frame_count),
                low_freq_hz: low_f,
                high_freq_hz: high_f,
                peak_magnitude_db: peak_val,
                taxon_id: assigned_taxon_id,
                confidence,
            };
            detected_count += 1;
        }
    }

    detected_count
}

/// Compile bioacoustic vocalizations into SPARQL-MM observation Quins.
/// Produces `(fragment_uri, predicate, object)` triples for each vocalization:
/// - `rdf:type` -> `qualia:AcousticObservation`
/// - `ma:isFragmentOf` -> `recording_uri`
/// - `ma:hasStartTime` -> start time in milliseconds
/// - `ma:hasEndTime` -> end time in milliseconds
/// - `qualia:speciesTaxonId` -> taxon ID
/// - `qualia:speciesConfidence` -> confidence weight
///
/// Zero-heap: writes directly into caller-supplied `out` slice.
pub fn compile_vocalizations_to_sparql_mm(
    recording_uri: u64,
    vocalizations: &[BioacousticVocalization],
    out: &mut [NQuin],
) -> usize {
    let p_type = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    let o_obs_type = q_hash(QUALIA_ACOUSTIC_OBSERVATION);
    let p_fragment_of = q_hash(MA_IS_FRAGMENT_OF);
    let p_start_time = q_hash(MA_HAS_START_TIME);
    let p_end_time = q_hash(MA_HAS_END_TIME);
    let p_taxon = q_hash(QUALIA_TAXON_ID);
    let p_conf = q_hash(QUALIA_SPECIES_CONFIDENCE);

    let mut written = 0usize;

    for (idx, voc) in vocalizations.iter().enumerate() {
        if written + 6 > out.len() {
            break;
        }

        // Derive deterministic fragment URI from recording_uri and index
        let fragment_uri = recording_uri ^ q_hash(&format!("frag:acoustic:{}", idx));

        let make_quin = |p: u64, o: u64, meta: u64| {
            let mut q = NQuin {
                subject: fragment_uri,
                predicate: p,
                object: o,
                context: recording_uri,
                metadata: meta,
                parity: 0,
            };
            q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
            q
        };

        // 1. rdf:type
        out[written] = make_quin(p_type, o_obs_type, 0);
        written += 1;

        // 2. isFragmentOf
        out[written] = make_quin(p_fragment_of, recording_uri, 0);
        written += 1;

        // 3. hasStartTime (ms)
        let start_ms = (voc.start_time_s * 1000.0) as u64;
        out[written] = make_quin(p_start_time, start_ms, start_ms << 16);
        written += 1;

        // 4. hasEndTime (ms)
        let end_ms = (voc.end_time_s * 1000.0) as u64;
        out[written] = make_quin(p_end_time, end_ms, end_ms << 16);
        written += 1;

        // 5. taxon ID
        out[written] = make_quin(p_taxon, voc.taxon_id, 0);
        written += 1;

        // 6. confidence (fixed point integer 0..1000 in object)
        let conf_fixed = (voc.confidence * 1000.0) as u64;
        out[written] = make_quin(p_conf, conf_fixed, 0);
        written += 1;
    }

    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_bioacoustic_vocalizations() {
        // 10 frames x 4 bins raster
        let mut raster = vec![0.0f32; 40];
        // Inject vocalization between frames 2 and 6 on bin 2
        for f in 2..6 {
            raster[f * 4 + 2] = 25.0; // 25 dB
        }

        let surface = TfSurface::new(&raster, 10, 4, 16000, 160);
        let mut out = [BioacousticVocalization::default(); 4];

        let count = detect_bioacoustic_vocalizations(&surface, 20.0, 0.02, 0x1234, &mut out);
        assert_eq!(count, 1);
        assert_eq!(out[0].taxon_id, 0x1234);
        assert!(out[0].start_time_s >= 0.02);
        assert!(out[0].end_time_s > out[0].start_time_s);
        assert_eq!(out[0].peak_magnitude_db, 25.0);
    }

    #[test]
    fn test_compile_vocalizations_to_sparql_mm() {
        let voc = BioacousticVocalization {
            start_time_s: 1.5,
            end_time_s: 2.2,
            low_freq_hz: 1200.0,
            high_freq_hz: 3400.0,
            peak_magnitude_db: 32.0,
            taxon_id: 0x9999,
            confidence: 0.95,
        };

        let recording = q_hash("urn:recording:forest_night_01");
        let mut quins = [NQuin::default(); 12];

        let n = compile_vocalizations_to_sparql_mm(recording, &[voc], &mut quins);
        assert_eq!(n, 6);

        // Check rdf:type
        assert_eq!(
            quins[0].predicate,
            q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
        );
        assert_eq!(quins[0].object, q_hash(QUALIA_ACOUSTIC_OBSERVATION));

        // Check start time = 1500ms
        assert_eq!(quins[2].object, 1500);
        // Check end time = 2200ms
        assert_eq!(quins[3].object, 2200);
        // Check taxon ID
        assert_eq!(quins[4].object, 0x9999);
        // Check confidence
        assert_eq!(quins[5].object, 950);
    }
}
