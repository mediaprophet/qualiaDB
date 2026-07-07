//! W5b Phase 4b step 2 — the engine's core artifact loader.
//!
//! Proves the forge↔engine contract end-to-end: the forge frames a dictionary artifact (real
//! `frame_artifact` + `Provenance`), and the CORE engine loader (`kv_dict_runtime::load_certified`,
//! available without... well, this test needs the forge to BUILD the fixture, but the loader under test
//! is core) parses it, VERIFIES the provenance gate, installs the dicts, and reconstructs through them —
//! while refusing an artifact that failed its gate, a wrong-kind artifact, or garbage (fail-closed).

#![cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]

use qualia_core_db::kv_dict::KvDictionary;
use qualia_core_db::kv_dict_runtime as rt;
use qualia_core_db::kv_dict_runtime::KvDictArtifact;
use qualia_core_db::wgsl_forge::calibration::kv_dictionary::learn_dictionary;
use qualia_core_db::wgsl_forge::calibration::package::frame_artifact;
use qualia_core_db::wgsl_forge::calibration::{ArtifactKind, Provenance};
use serial_test::serial;

/// A tiny but real dictionary (learned from deterministic synthetic data), layer-0 K only.
fn tiny_dicts() -> (
    usize,
    usize,
    Vec<Option<KvDictionary>>,
    Vec<Option<KvDictionary>>,
) {
    let dim = 8usize;
    let n_atoms = 4usize;
    let k = 2usize;
    let data: Vec<Vec<f32>> = (0..60)
        .map(|i| {
            let mut v = vec![0f32; dim];
            v[i % dim] = 1.0;
            v[(i * 3) % dim] += 0.5;
            v
        })
        .collect();
    let dict = learn_dictionary(&data, dim, n_atoms, k, 15);
    (dim, k, vec![Some(dict)], vec![None])
}

/// Frame a dictionary artifact the way the forge packager does.
fn framed(passed: bool, kind: ArtifactKind) -> Vec<u8> {
    let (dim, k, kd, vd) = tiny_dicts();
    let art = KvDictArtifact {
        sparsity: k,
        head_dim: dim,
        k: kd,
        v: vd,
    };
    let mut payload = Vec::new();
    ciborium::into_writer(&art, &mut payload).unwrap();
    let prov = Provenance::new(kind, 0xABCD, 3, 10.0, 10.05, 0.005, passed);
    frame_artifact(&payload, &prov)
}

fn write_temp(name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(name);
    std::fs::write(&p, bytes).unwrap();
    p
}

#[test]
#[serial]
fn loads_certified_installs_and_reconstructs() {
    rt::disable();
    rt::clear();
    let p = write_temp(
        "kvdict_ok.q42art",
        &framed(true, ArtifactKind::KvDictionary),
    );

    let info = rt::load_certified(&p).expect("a certified KvDictionary artifact must load");
    assert!(
        rt::is_enabled(),
        "loading a certified artifact installs + enables it"
    );
    assert_eq!(info.k_layers, 1);
    assert_eq!(info.v_layers, 0);
    assert_eq!(info.head_dim, 8);
    assert!(
        (info.delta_ppl - 0.005).abs() < 1e-9,
        "gate ΔPPL surfaced from provenance"
    );

    // Layer-0 K has a dictionary → reconstruct runs (lossy; must not panic on a valid head vector).
    let mut kproj = vec![1.0f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    rt::reconstruct_kv(0, true, &mut kproj, 1, 8);
    assert!(kproj.iter().all(|x| x.is_finite()));
    // Layer-0 V has no dictionary → passthrough (unchanged).
    let mut vproj = vec![2.0f32; 8];
    let snap = vproj.clone();
    rt::reconstruct_kv(0, false, &mut vproj, 1, 8);
    assert_eq!(
        vproj, snap,
        "a stream/layer with no dictionary must be passthrough"
    );

    rt::disable();
    rt::clear();
    let _ = std::fs::remove_file(&p);
}

#[test]
#[serial]
fn fail_closed_refuses_uncertified_wrong_kind_and_garbage() {
    rt::disable();
    rt::clear();

    let p1 = write_temp(
        "kvdict_failed.q42art",
        &framed(false, ArtifactKind::KvDictionary),
    );
    assert!(
        rt::load_certified(&p1).is_err(),
        "an artifact that did NOT pass its ΔPPL gate must be refused"
    );
    assert!(
        !rt::is_enabled(),
        "a refused artifact must not be installed"
    );

    let p2 = write_temp(
        "kvdict_wrongkind.q42art",
        &framed(true, ArtifactKind::AwqScales),
    );
    assert!(
        rt::load_certified(&p2).is_err(),
        "a non-KvDictionary artifact must be refused"
    );

    let p3 = write_temp("kvdict_garbage.q42art", b"definitely-not-a-QCAL-frame");
    assert!(
        rt::load_certified(&p3).is_err(),
        "a bad frame magic must be refused"
    );

    assert!(
        !rt::is_enabled(),
        "no refused artifact left the runtime enabled"
    );
    for p in [p1, p2, p3] {
        let _ = std::fs::remove_file(&p);
    }
}
