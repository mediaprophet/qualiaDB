//! Optional host microphone → `qualia_audio::CaptureSession` via cpal.
//!
//! Fail-closed without intent. Stream thread pushes mono f32 into a shared session.

use std::sync::{Arc, Mutex};

use qualia_audio::{CapturePurpose, CaptureSession};

/// Process-wide capture session for desktop shell.
static CAPTURE: Mutex<Option<Arc<Mutex<CaptureSession>>>> = Mutex::new(None);
static STREAM_RUNNING: Mutex<bool> = Mutex::new(false);

pub fn shared_session() -> Arc<Mutex<CaptureSession>> {
    let mut g = CAPTURE.lock().unwrap_or_else(|e| e.into_inner());
    if g.is_none() {
        *g = Some(Arc::new(Mutex::new(CaptureSession::new(
            CapturePurpose::Analysis,
            16_000,
            1,
        ))));
    }
    g.as_ref().unwrap().clone()
}

pub fn grant_and_start(purpose: CapturePurpose) -> Result<String, String> {
    let sess = shared_session();
    {
        let mut s = sess.lock().map_err(|e| e.to_string())?;
        s.purpose = purpose;
        s.grant_intent();
        s.start().map_err(|e| format!("{e:?}"))?;
    }
    start_cpal_stream(sess)?;
    Ok("mic capture armed (cpal)".into())
}

pub fn stop_capture() -> Result<String, String> {
    {
        let mut running = STREAM_RUNNING.lock().map_err(|e| e.to_string())?;
        *running = false;
    }
    let sess = shared_session();
    let mut s = sess.lock().map_err(|e| e.to_string())?;
    s.stop();
    s.revoke_intent();
    Ok("mic capture stopped".into())
}

pub fn status_json() -> Result<serde_json::Value, String> {
    let sess = shared_session();
    let s = sess.lock().map_err(|e| e.to_string())?;
    let running = *STREAM_RUNNING.lock().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "intent_granted": s.intent_granted,
        "live": s.live,
        "cpal_stream": running,
        "sample_rate": s.sample_rate,
        "frames_captured": s.frames_captured,
        "available": s.available(),
        "note": "cpal input stream pushes mono into CaptureSession when armed."
    }))
}

/// Pull available mono samples (for analysis UI).
pub fn pull_mono(max: usize) -> Result<Vec<f32>, String> {
    let sess = shared_session();
    let mut s = sess.lock().map_err(|e| e.to_string())?;
    let mut out = vec![0.0f32; max.min(16_000)];
    let n = s.pull_mono(&mut out);
    out.truncate(n);
    Ok(out)
}

fn start_cpal_stream(sess: Arc<Mutex<CaptureSession>>) -> Result<(), String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    {
        let mut running = STREAM_RUNNING.lock().map_err(|e| e.to_string())?;
        if *running {
            return Ok(());
        }
        *running = true;
    }

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "no default input device".to_string())?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("input config: {e}"))?;
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    {
        let mut s = sess.lock().map_err(|e| e.to_string())?;
        s.sample_rate = sample_rate;
        s.channels = channels.max(1) as u16;
    }

    let stream_config: cpal::StreamConfig = config.clone().into();
    let err_fn = |e| eprintln!("cpal input error: {e}");

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    if !*STREAM_RUNNING.lock().unwrap_or_else(|e| e.into_inner()) {
                        return;
                    }
                    let mono = downmix_f32(data, channels);
                    if let Ok(mut s) = sess.lock() {
                        let _ = s.push_mono(&mono);
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("build stream f32: {e}"))?,
        cpal::SampleFormat::I16 => {
            let sess2 = shared_session();
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        if !*STREAM_RUNNING.lock().unwrap_or_else(|e| e.into_inner()) {
                            return;
                        }
                        let mono: Vec<f32> = data
                            .chunks(channels.max(1))
                            .map(|c| {
                                let sum: f32 = c.iter().map(|&x| x as f32 / 32768.0).sum();
                                sum / c.len() as f32
                            })
                            .collect();
                        if let Ok(mut s) = sess2.lock() {
                            let _ = s.push_mono(&mono);
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("build stream i16: {e}"))?
        }
        other => {
            *STREAM_RUNNING.lock().unwrap_or_else(|e| e.into_inner()) = false;
            return Err(format!("unsupported sample format: {other:?}"));
        }
    };

    stream.play().map_err(|e| format!("play stream: {e}"))?;
    // Leak stream for process lifetime (desktop daemon style).
    std::mem::forget(stream);
    Ok(())
}

fn downmix_f32(data: &[f32], channels: usize) -> Vec<f32> {
    let ch = channels.max(1);
    data.chunks(ch)
        .map(|c| c.iter().sum::<f32>() / c.len() as f32)
        .collect()
}
