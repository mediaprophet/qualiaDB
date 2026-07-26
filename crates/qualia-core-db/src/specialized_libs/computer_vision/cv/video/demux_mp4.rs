//! MP4 container **demux** (D1.11) — video file I/O.
//!
//! Scope, stated honestly: this parses the ISO-BMFF / MP4 **container** using
//! the permissive pure-Rust `mp4` crate and exposes the first video track's
//! metadata plus its **encoded** sample packets (presentation timestamp,
//! duration, byte size, keyframe/sync flag).
//!
//! It does **NOT** decode H.264 / H.265 / VP9 bitstreams to RGB pixels. Turning
//! an encoded packet into a picture requires a codec decoder (a separate,
//! codec-gated concern). Decoded RGB frames live in
//! [`super::frame_sequence::FrameSequence`]. Nothing here fabricates frames:
//! `VideoPacket::size` is the size of the real coded sample, and the packet
//! bytes are never invented.

use crate::specialized_libs::computer_vision::cv::error::CvError;
use mp4::{Mp4Reader, TrackType};
use std::io::{Cursor, Read, Seek};

/// Metadata of one video track read from an MP4 container header.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoTrackInfo {
    /// Sample-entry FourCC of the coded format (e.g. `"avc1"`, `"hev1"`, `"vp09"`).
    pub codec: String,
    /// Coded frame width in pixels.
    pub width: u16,
    /// Coded frame height in pixels.
    pub height: u16,
    /// Media timescale (ticks per second) of the track.
    pub timescale: u32,
    /// Track duration in milliseconds.
    pub duration_ms: u64,
    /// Number of coded samples (frames) in the track.
    pub frame_count: u32,
    /// Average frames per second over the track (frame_count / duration).
    pub avg_fps: f32,
}

/// One **encoded** sample packet from a video track. This is container-level
/// information about a coded frame; it is not a decoded RGB image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoPacket {
    /// 1-based sample id within the track.
    pub sample_id: u32,
    /// Presentation timestamp (start time) in milliseconds.
    pub offset_ms: u64,
    /// Sample duration in milliseconds.
    pub duration_ms: u64,
    /// Coded sample size in bytes.
    pub size: u32,
    /// True if this is a sync sample (keyframe / IDR).
    pub is_keyframe: bool,
}

/// Read the MP4 container and return metadata for the FIRST video track.
///
/// - `EmptyInput` if `bytes` is empty.
/// - `InvalidParameter` on any container parse failure, if the track timescale
///   is zero, or if there is no video track (e.g. an audio-only container).
///
/// Never panics.
pub fn demux_mp4_info(bytes: &[u8]) -> Result<VideoTrackInfo, CvError> {
    if bytes.is_empty() {
        return Err(CvError::EmptyInput);
    }

    let reader = Mp4Reader::read_header(Cursor::new(bytes), bytes.len() as u64)
        .map_err(|_| CvError::InvalidParameter)?;

    let track_id = first_video_track_id(&reader).ok_or(CvError::InvalidParameter)?;

    let track = reader
        .tracks()
        .get(&track_id)
        .ok_or(CvError::InvalidParameter)?;

    let timescale = track.timescale();
    if timescale == 0 {
        // Guard against divide-by-zero inside the crate's duration math.
        return Err(CvError::InvalidParameter);
    }

    let codec = match track.box_type() {
        Ok(fourcc) => fourcc.to_string(),
        Err(_) => "unknown".to_string(),
    };

    let duration_ms = track.duration().as_millis() as u64;
    let frame_count = track.sample_count();

    let avg_fps = if duration_ms > 0 {
        (frame_count as f64 * 1000.0 / duration_ms as f64) as f32
    } else {
        0.0
    };

    Ok(VideoTrackInfo {
        codec,
        width: track.width(),
        height: track.height(),
        timescale,
        duration_ms,
        frame_count,
        avg_fps,
    })
}

/// Iterate the FIRST video track's coded sample packets into the caller's
/// buffer. Writes up to `out.len()` packets and returns the number written.
///
/// Timestamps are converted to milliseconds via the track timescale.
/// `is_keyframe` comes from each sample's sync flag.
///
/// - `EmptyInput` if `bytes` is empty.
/// - `InvalidParameter` on parse failure, zero timescale, or no video track.
///
/// Never panics.
pub fn demux_mp4_packets(bytes: &[u8], out: &mut [VideoPacket]) -> Result<usize, CvError> {
    if bytes.is_empty() {
        return Err(CvError::EmptyInput);
    }

    let mut reader = Mp4Reader::read_header(Cursor::new(bytes), bytes.len() as u64)
        .map_err(|_| CvError::InvalidParameter)?;

    let track_id = first_video_track_id(&reader).ok_or(CvError::InvalidParameter)?;

    let (timescale, sample_count) = {
        let track = reader
            .tracks()
            .get(&track_id)
            .ok_or(CvError::InvalidParameter)?;
        (track.timescale(), track.sample_count())
    };
    if timescale == 0 {
        return Err(CvError::InvalidParameter);
    }
    let ts = timescale as u64;

    if out.is_empty() {
        return Ok(0);
    }

    let mut count = 0usize;
    // Samples are 1-based in the mp4 crate.
    for sample_id in 1..=sample_count {
        if count >= out.len() {
            break;
        }
        // read_sample returns Ok(None) when a sample is absent; propagate hard
        // parse errors as InvalidParameter rather than panicking.
        let sample = match reader.read_sample(track_id, sample_id) {
            Ok(Some(s)) => s,
            Ok(None) => continue,
            Err(_) => return Err(CvError::InvalidParameter),
        };

        out[count] = VideoPacket {
            sample_id,
            offset_ms: sample.start_time.saturating_mul(1000) / ts,
            duration_ms: (sample.duration as u64).saturating_mul(1000) / ts,
            size: sample.bytes.len() as u32,
            is_keyframe: sample.is_sync,
        };
        count += 1;
    }

    Ok(count)
}

/// Lowest-id track whose handler type is Video, for deterministic "first"
/// selection (the crate stores tracks in an unordered map).
fn first_video_track_id<R: Read + Seek>(reader: &Mp4Reader<R>) -> Option<u32> {
    let mut best: Option<u32> = None;
    for (&id, track) in reader.tracks().iter() {
        if matches!(track.track_type(), Ok(TrackType::Video)) {
            best = Some(match best {
                Some(cur) if cur <= id => cur,
                _ => id,
            });
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;
    // `TrackType` and `Cursor` come in via `super::*`.
    use mp4::{AvcConfig, Bytes, MediaConfig, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig};

    /// Build a minimal, self-contained, valid MP4 in memory using the crate's
    /// own writer: one 16x16 H.264 (avc1) video track with two coded samples
    /// (sample 1 = keyframe/sync, sample 2 = non-sync). The sample *payloads*
    /// are placeholder bytes — we are testing container demux, not codec
    /// decode — but the container structure (moov/stbl/stss/stsz/stts) is real.
    fn build_min_mp4() -> Vec<u8> {
        let config = Mp4Config {
            major_brand: "isom".parse().unwrap(),
            minor_version: 512,
            compatible_brands: vec![
                "isom".parse().unwrap(),
                "iso2".parse().unwrap(),
                "avc1".parse().unwrap(),
                "mp41".parse().unwrap(),
            ],
            timescale: 1000,
        };

        let mut writer = Mp4Writer::write_start(Cursor::new(Vec::<u8>::new()), &config).unwrap();

        let track = TrackConfig {
            track_type: TrackType::Video,
            timescale: 1000,
            language: String::from("und"),
            media_conf: MediaConfig::AvcConfig(AvcConfig {
                width: 16,
                height: 16,
                // Non-empty placeholder SPS/PPS so the avcC box is well-formed.
                seq_param_set: vec![0x67, 0x64, 0x00, 0x0a],
                pic_param_set: vec![0x68, 0xce, 0x3c, 0x80],
            }),
        };
        writer.add_track(&track).unwrap();

        // Sample 1: keyframe (is_sync = true), 500 ms.
        writer
            .write_sample(
                1,
                &Mp4Sample {
                    start_time: 0,
                    duration: 500,
                    rendering_offset: 0,
                    is_sync: true,
                    bytes: Bytes::from(vec![0xAAu8; 64]),
                },
            )
            .unwrap();

        // Sample 2: non-keyframe (is_sync = false), 500 ms.
        writer
            .write_sample(
                1,
                &Mp4Sample {
                    start_time: 500,
                    duration: 500,
                    rendering_offset: 0,
                    is_sync: false,
                    bytes: Bytes::from(vec![0xBBu8; 32]),
                },
            )
            .unwrap();

        writer.write_end().unwrap();
        writer.into_writer().into_inner()
    }

    #[test]
    fn info_reports_real_metadata() {
        let mp4 = build_min_mp4();
        assert!(mp4.len() > 200, "writer produced a plausibly sized mp4");

        let info = demux_mp4_info(&mp4).expect("demux info");
        assert_eq!(info.width, 16);
        assert_eq!(info.height, 16);
        assert_eq!(info.frame_count, 2);
        assert_eq!(info.timescale, 1000);
        assert_eq!(info.codec, "avc1", "codec fourcc from sample entry");
        // Two 500 ms samples => ~1000 ms total.
        assert_eq!(info.duration_ms, 1000);
        // 2 frames / 1.0 s => ~2 fps.
        assert!(
            (info.avg_fps - 2.0).abs() < 0.01,
            "avg_fps={}",
            info.avg_fps
        );
    }

    #[test]
    fn packets_have_correct_keyframe_flags() {
        let mp4 = build_min_mp4();

        let mut out = [VideoPacket {
            sample_id: 0,
            offset_ms: 0,
            duration_ms: 0,
            size: 0,
            is_keyframe: false,
        }; 8];

        let n = demux_mp4_packets(&mp4, &mut out).expect("demux packets");
        assert_eq!(n, 2, "two samples written");

        assert_eq!(out[0].sample_id, 1);
        assert!(out[0].is_keyframe, "first sample is a keyframe");
        assert_eq!(out[0].offset_ms, 0);
        assert_eq!(out[0].duration_ms, 500);
        assert_eq!(out[0].size, 64);

        assert_eq!(out[1].sample_id, 2);
        assert!(!out[1].is_keyframe, "second sample is not a keyframe");
        assert_eq!(out[1].offset_ms, 500);
        assert_eq!(out[1].duration_ms, 500);
        assert_eq!(out[1].size, 32);
    }

    #[test]
    fn packets_respect_caller_buffer_len() {
        let mp4 = build_min_mp4();
        // Buffer smaller than sample count: write only what fits.
        let mut out = [VideoPacket {
            sample_id: 0,
            offset_ms: 0,
            duration_ms: 0,
            size: 0,
            is_keyframe: false,
        }; 1];
        let n = demux_mp4_packets(&mp4, &mut out).expect("demux packets");
        assert_eq!(n, 1);
        assert_eq!(out[0].sample_id, 1);

        // Zero-length buffer: zero written, no panic.
        let n0 = demux_mp4_packets(&mp4, &mut []).expect("demux packets empty out");
        assert_eq!(n0, 0);
    }

    #[test]
    fn empty_bytes_are_empty_input() {
        assert_eq!(demux_mp4_info(&[]), Err(CvError::EmptyInput));
        let mut out = [VideoPacket {
            sample_id: 0,
            offset_ms: 0,
            duration_ms: 0,
            size: 0,
            is_keyframe: false,
        }; 4];
        assert_eq!(demux_mp4_packets(&[], &mut out), Err(CvError::EmptyInput));
    }

    #[test]
    fn garbage_bytes_fail_closed() {
        let garbage = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        assert_eq!(demux_mp4_info(&garbage), Err(CvError::InvalidParameter));
        let mut out = [VideoPacket {
            sample_id: 0,
            offset_ms: 0,
            duration_ms: 0,
            size: 0,
            is_keyframe: false,
        }; 4];
        assert_eq!(
            demux_mp4_packets(&garbage, &mut out),
            Err(CvError::InvalidParameter)
        );
    }

    #[test]
    fn audio_only_container_has_no_video_track() {
        // Valid MP4 with a single audio (AAC) track => no video track =>
        // InvalidParameter, not a panic and not a fabricated video track.
        use mp4::AacConfig;

        let config = Mp4Config {
            major_brand: "isom".parse().unwrap(),
            minor_version: 512,
            compatible_brands: vec!["isom".parse().unwrap(), "mp41".parse().unwrap()],
            timescale: 1000,
        };
        let mut writer = Mp4Writer::write_start(Cursor::new(Vec::<u8>::new()), &config).unwrap();
        let track = TrackConfig {
            track_type: TrackType::Audio,
            timescale: 48000,
            language: String::from("und"),
            media_conf: MediaConfig::AacConfig(AacConfig::default()),
        };
        writer.add_track(&track).unwrap();
        writer
            .write_sample(
                1,
                &Mp4Sample {
                    start_time: 0,
                    duration: 1024,
                    rendering_offset: 0,
                    is_sync: true,
                    bytes: Bytes::from(vec![0u8; 16]),
                },
            )
            .unwrap();
        writer.write_end().unwrap();
        let mp4 = writer.into_writer().into_inner();

        assert_eq!(demux_mp4_info(&mp4), Err(CvError::InvalidParameter));
    }
}
