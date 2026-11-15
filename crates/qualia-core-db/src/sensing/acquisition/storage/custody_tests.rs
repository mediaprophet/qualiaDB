use super::*;
use crate::sensing::{acquisition as a, SemanticId};

struct TempRoot(PathBuf);
impl TempRoot {
    fn new(label: &str) -> Self {
        let pid = std::process::id();
        let seq = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("qualia-acq-{label}-{pid}-{seq}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
}
impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn sample_container(family: Family) -> Vec<u8> {
    let payload = [0xAAu8, 0xBB, 0x10, 0x00, 0x20, 0x00];
    let orig = a::Original::admit(&payload, payload.len()).unwrap();
    let view = a::TensorView::admit(
        orig.region(2, 4).unwrap(),
        a::Layout {
            rank: 1,
            extents: [2, 0, 0, 0],
            strides: [2, 0, 0, 0],
            axes: [a::Axis::Sample; 4],
            encoding: a::Encoding::U16,
            byte_order: a::ByteOrder::Little,
            components: a::ComponentOrder::Scalar,
        },
        a::Context {
            quantity: a::Quantity {
                kind: SemanticId::new(11).unwrap(),
                unit: SemanticId::new(12).unwrap(),
            },
            stage: a::SampleStage::RawCodes,
            sensor: SemanticId::new(13).unwrap(),
            geometry: None,
            calibration: None,
            uncertainty: None,
        },
        None,
        a::SignalAxes {
            carrier: a::Carrier::Unknown,
            baseband_hz: None,
            pulse_rate: None,
            complex_convention: None,
            phase_origin: None,
        },
    )
    .unwrap();
    let mut meta = [0u8; 4096];
    let mut staging = vec![0u8; 16_384];
    let mut out = vec![0u8; 16_384];
    let mut m = [sem::NodeState::default(); 64];
    let mut e = [sem::NodeState::default(); 64];
    let len = encode(
        Input {
            family,
            view: View::Tensor(view),
            derivation: None,
        },
        &mut meta,
        &mut staging,
        &mut sem::Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        &mut out,
        Default::default(),
        Default::default(),
        Default::default(),
    )
    .unwrap();
    out.truncate(len);
    out
}

#[test]
fn publish_and_read_roundtrip_idempotent_no_clobber() {
    let root = TempRoot::new("roundtrip");
    let bytes = sample_container(Family::EmfV4Candidate);
    let mut m = [sem::NodeState::default(); 64];
    let mut e = [sem::NodeState::default(); 64];
    let mut scratch = sem::Scratch {
        metadata: &mut m,
        entries: &mut e,
    };
    let art1 = publish_container(
        &root.0,
        Family::EmfV4Candidate,
        1,
        &bytes,
        &mut scratch,
        PublishOptions::default(),
    )
    .unwrap();
    let art2 = publish_container(
        &root.0,
        Family::EmfV4Candidate,
        1,
        &bytes,
        &mut scratch,
        PublishOptions::default(),
    )
    .unwrap();
    assert_eq!(art1, art2);
    let mut buf = vec![0u8; 16_384];
    let mut originals = [None; 8];
    let (n, art3, doc) = read_container(
        &root.0,
        Family::EmfV4Candidate,
        &art1.blake3,
        1,
        &mut buf,
        &mut originals,
        &mut scratch,
        PublishOptions::default(),
    )
    .unwrap();
    assert_eq!(n, bytes.len());
    assert_eq!(art3.blake3, art1.blake3);
    assert_eq!(
        doc.original(1).unwrap().bytes(),
        &[0xAA, 0xBB, 0x10, 0x00, 0x20, 0x00]
    );
}

#[test]
fn rejects_invalid_before_writing_and_detects_conflict_and_undersized_read_buffer() {
    let root = TempRoot::new("conflict");
    let valid = sample_container(Family::QacV1Candidate);
    let mut m = [sem::NodeState::default(); 64];
    let mut e = [sem::NodeState::default(); 64];
    let mut scratch = sem::Scratch {
        metadata: &mut m,
        entries: &mut e,
    };
    assert!(matches!(
        publish_container(
            &root.0,
            Family::QacV1Candidate,
            1,
            &valid[..32],
            &mut scratch,
            PublishOptions::default(),
        ),
        Err(PublishError::Admission(_))
    ));
    assert!(!root.0.join("sensing").exists());

    let art = publish_container(
        &root.0,
        Family::QacV1Candidate,
        1,
        &valid,
        &mut scratch,
        PublishOptions::default(),
    )
    .unwrap();
    let mut tampered = valid.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    fs::write(&art.path, &tampered).unwrap();

    let mut small = vec![0u8; valid.len() - 1];
    let mut originals = [None; 8];
    assert!(matches!(
        read_container(
            &root.0,
            Family::QacV1Candidate,
            &art.blake3,
            1,
            &mut small,
            &mut originals,
            &mut scratch,
            PublishOptions::default(),
        ),
        Err(PublishError::Budget)
    ));
    let mut exact = vec![0u8; valid.len()];
    assert!(matches!(
        read_container(
            &root.0,
            Family::QacV1Candidate,
            &art.blake3,
            1,
            &mut exact,
            &mut originals,
            &mut scratch,
            PublishOptions::default(),
        ),
        Err(PublishError::Conflict)
    ));
}
