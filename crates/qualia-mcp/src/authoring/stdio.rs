use super::*;
use std::io::{BufRead, BufReader, Write};

/// Run on the process's locked stdin/stdout. No startup banners, diagnostic
/// logging or other non-protocol stdout is emitted. EOF is normal shutdown.
pub fn run_stdio<B: AuthoringBackend>(
    backend: B,
    limits: ProtocolLimits,
) -> Result<(), ProtocolError> {
    let limits = limits.validate()?;
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let reader = BufReader::with_capacity(limits.max_message_bytes.min(8192), stdin.lock());
    serve(reader, stdout.lock(), backend, limits)
}

/// Bounded synchronous stdio transport, also usable with caller-owned streams.
/// No HTTP listener, detached thread, credentials or native service is created.
pub fn serve<R: BufRead, W: Write, B: AuthoringBackend>(
    mut reader: R,
    mut writer: W,
    backend: B,
    limits: ProtocolLimits,
) -> Result<(), ProtocolError> {
    let limits = limits.validate()?;
    let mut server = AuthoringServer::new(backend, limits)?;
    let mut frame = Vec::with_capacity(limits.max_message_bytes.min(4096));
    loop {
        frame.clear();
        if !read_frame(&mut reader, &mut frame, limits.max_message_bytes)? {
            return Ok(());
        }
        let line = std::str::from_utf8(&frame).map_err(|_| ProtocolError::InvalidUtf8)?;
        if let Some(response) = server.process(line) {
            // process counts full serialized response before any bytes reach
            // stdout, so a configured size failure cannot emit a partial frame.
            serde_json::to_writer(&mut writer, &response).map_err(|e| {
                ProtocolError::Io(e.io_error_kind().unwrap_or(std::io::ErrorKind::Other))
            })?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }
}

fn read_frame<R: BufRead>(
    reader: &mut R,
    frame: &mut Vec<u8>,
    limit: usize,
) -> Result<bool, ProtocolError> {
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(false)
            } else {
                Err(ProtocolError::UnterminatedFrame)
            };
        }
        let newline = available.iter().position(|b| *b == b'\n');
        let count = newline.unwrap_or(available.len());
        if count > limit.saturating_sub(frame.len()) {
            return Err(ProtocolError::FrameTooLarge);
        }
        frame
            .try_reserve_exact(count)
            .map_err(|_| ProtocolError::ResourceLimit)?;
        frame.extend_from_slice(&available[..count]);
        reader.consume(count + usize::from(newline.is_some()));
        if newline.is_some() {
            if frame.last() == Some(&b'\r') {
                frame.pop();
            }
            return Ok(true);
        }
    }
}
