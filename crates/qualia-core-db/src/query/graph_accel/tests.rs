//! CPU oracles and GPU differential tests (skip when no adapter).

use super::*;
use crate::NQuin;

fn quin(s: u64, p: u64, o: u64) -> NQuin {
    NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: 0,
        metadata: 0,
        parity: s ^ p ^ o,
    }
}

fn objects(qs: &[NQuin]) -> Vec<u64> {
    qs.iter().map(|q| q.object).collect()
}

#[test]
fn cpu_radix_matches_unstable_sort() {
    let mut a: Vec<NQuin> = (0..2000u64)
        .rev()
        .map(|i| quin(i, i * 3, i.wrapping_mul(0x9e37) ^ (i << 7)))
        .collect();
    a[10] = quin(1, 1, a[11].object);
    let mut b = a.clone();
    a.sort_unstable_by_key(|q| q.object);
    sort_quins_by_object_cpu_only(&mut b);
    assert_eq!(objects(&a), objects(&b));
}

#[test]
fn cpu_radix_empty_and_singleton() {
    let mut empty: [NQuin; 0] = [];
    sort_quins_by_object_cpu_only(&mut empty);
    let mut one = [quin(1, 2, 3)];
    sort_quins_by_object_cpu_only(&mut one);
    assert_eq!(one[0].object, 3);
}

#[test]
fn sieve_eq_cpu_compacts() {
    let qs: Vec<NQuin> = (0..100)
        .map(|i| quin(i, 7, if i % 10 == 0 { 42 } else { i }))
        .collect();
    let mut out = [NQuin::default(); 16];
    let n = super::cpu::sieve_eq_cpu(&qs, QuinField::Object, 42, &mut out);
    // i % 10 == 0 → 42 (10 rows) plus i == 42 itself.
    assert_eq!(n, 11);
    assert!(out[..n].iter().all(|q| q.object == 42));
}

#[test]
fn join_cartesian_on_duplicate_keys() {
    let build = [1u64, 2, 2, 3];
    let probe = [2u64, 2, 9];
    let mut out = [(0u32, 0u32); 16];
    let n = hash_join_u64_cpu(&build, &probe, &mut out);
    assert_eq!(n, 4);
    let mut pairs = out[..n].to_vec();
    pairs.sort_unstable();
    assert_eq!(pairs, vec![(0, 1), (0, 2), (1, 1), (1, 2)]);
}

#[test]
fn join_truncates_fail_closed() {
    let build = [5u64, 5, 5];
    let probe = [5u64, 5];
    let mut out = [(0u32, 0u32); 3];
    let r = hash_join_u64(&build, &probe, &mut out);
    assert_eq!(r.written, 3);
    assert!(r.truncated);
}

#[test]
fn npu_is_not_faked() {
    assert!(!npu_available());
}

#[test]
fn policy_cpu_env_forces_cpu_sort() {
    let prev = std::env::var("QUALIA_GRAPH_ACCEL").ok();
    std::env::set_var("QUALIA_GRAPH_ACCEL", "cpu");
    let mut qs: Vec<NQuin> = (0..8).rev().map(|i| quin(i, 0, i)).collect();
    let r = sort_quins_by_object(&mut qs);
    assert_eq!(r.path, AccelPath::Cpu);
    assert_eq!(objects(&qs), vec![0, 1, 2, 3, 4, 5, 6, 7]);
    match prev {
        Some(v) => std::env::set_var("QUALIA_GRAPH_ACCEL", v),
        None => std::env::remove_var("QUALIA_GRAPH_ACCEL"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
#[serial_test::serial(gpu)]
fn gpu_radix_matches_cpu_when_device_present() {
    if !super::path::gpu_available() {
        return;
    }
    // Force GPU even below the default threshold by using a large-enough slab.
    let n = GPU_SORT_MIN;
    let keys: Vec<u64> = (0..n as u64)
        .map(|i| i.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i << 17))
        .collect();
    let mut cpu_idx: Vec<u32> = (0..n as u32).collect();
    super::cpu::radix_sort_u64_indices(&keys, &mut cpu_idx);
    let Some(gpu_idx) = super::gpu::radix_sort_u64_indices_gpu(&keys) else {
        return;
    };
    let cpu_keys: Vec<u64> = cpu_idx.iter().map(|&i| keys[i as usize]).collect();
    let gpu_keys: Vec<u64> = gpu_idx.iter().map(|&i| keys[i as usize]).collect();
    assert_eq!(cpu_keys, gpu_keys, "GPU radix key order must match CPU");
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
#[serial_test::serial(gpu)]
fn gpu_sieve_matches_cpu_when_device_present() {
    if !super::path::gpu_available() {
        return;
    }
    let n = GPU_SIEVE_MIN.max(8_192);
    let qs: Vec<NQuin> = (0..n as u64)
        .map(|i| quin(i, 1, if i % 17 == 0 { 99 } else { i }))
        .collect();
    let mut cpu_idx = vec![0u32; n];
    let cn = super::cpu::sieve_eq_indices_cpu(&qs, QuinField::Object, 99, &mut cpu_idx);
    let Some(gpu_idx) = super::gpu::sieve_eq_indices_gpu(&qs, QuinField::Object, 99) else {
        return;
    };
    assert_eq!(&cpu_idx[..cn], gpu_idx.as_slice());
}
