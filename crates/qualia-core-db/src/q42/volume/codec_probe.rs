//! Cold-path size probes for the R10 v4 gate. Not a new on-disk format.
#![allow(dead_code)]

use std::io;
use std::path::Path;

use super::super::Q42Volume;

#[derive(Clone, Copy, Debug)]
pub struct LexiconCodecProbe {
    pub raw_bytes: u64,
    pub lz4_bytes: u64,
}

impl LexiconCodecProbe {
    pub fn ratio(self) -> f64 {
        if self.raw_bytes == 0 {
            1.0
        } else {
            self.lz4_bytes as f64 / self.raw_bytes as f64
        }
    }
}

/// LZ4 the embedded Q42LEX payload. This is the cheapest v4 candidate:
/// the lexicon is stored uncompressed today.
pub fn probe_lexicon_lz4(path: &Path) -> io::Result<LexiconCodecProbe> {
    let volume = Q42Volume::open(path)?;
    let raw = volume.lex_bytes();
    let lz4 = lz4_flex::compress_prepend_size(raw);
    Ok(LexiconCodecProbe {
        raw_bytes: raw.len() as u64,
        lz4_bytes: lz4.len() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::StreamingQ42VolumeWriter;
    use crate::NQuin;
    use std::collections::HashMap;

    #[test]
    fn lexicon_lz4_is_measured_on_a_tiny_volume() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut lex = HashMap::new();
        for i in 0..64u64 {
            lex.insert(i + 1, format!("http://example.test/term/{i:04}"));
        }
        let mut writer = StreamingQ42VolumeWriter::new(&lex).unwrap();
        writer
            .push_block(
                0,
                &[NQuin {
                    subject: 1,
                    predicate: 2,
                    object: 3,
                    context: 0,
                    metadata: 0,
                    parity: 1 ^ 2 ^ 3,
                }],
            )
            .unwrap();
        writer.finish(file.path()).unwrap();
        let probe = probe_lexicon_lz4(file.path()).unwrap();
        assert!(probe.raw_bytes > 0);
        assert!(probe.lz4_bytes > 0);
    }

    #[test]
    fn probe_schemaorg_lexicon_lz4_when_present() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/data/schemaorg/30.0/schemaorg-current-https.q42");
        if !path.is_file() {
            return;
        }
        let probe = probe_lexicon_lz4(&path).unwrap();
        println!(
            "schema.org lexicon raw={} lz4={} ratio={:.3}",
            probe.raw_bytes,
            probe.lz4_bytes,
            probe.ratio()
        );
        assert!(probe.raw_bytes > 100_000);
    }

    #[test]
    #[ignore = "reads a 440 MiB Monarch lexicon shard"]
    fn probe_monarch_lexicon_lz4() {
        let path = Path::new(r"C:\Projects\monarch-kg\monarch-kg-root.lex-00000.q42");
        if !path.is_file() {
            return;
        }
        let probe = probe_lexicon_lz4(path).unwrap();
        println!(
            "monarch lex-00000 raw={} lz4={} ratio={:.3}",
            probe.raw_bytes,
            probe.lz4_bytes,
            probe.ratio()
        );
        assert!(probe.raw_bytes > 100_000_000);
    }
}
