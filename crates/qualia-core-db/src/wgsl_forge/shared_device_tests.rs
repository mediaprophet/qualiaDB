use crate::wgsl_forge::graph_ops::executor::*;
#[test]
#[serial_test::serial(gpu)]
fn shared_device_executor_matches_cpu_oracle() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut exec =
        ForgeGraphExecutor::on_shared_gpu().expect("forge executor on shared_gpu device");

    // The forge must report the SAME adapter as the process-wide shared device — i.e. it did
    // not silently spin up a second adapter/device.
    let shared = crate::gpu_context::shared_gpu();
    let forge_adapter = &exec.context().adapter;
    assert_eq!(
        forge_adapter.vendor, shared.adapter_caps.vendor,
        "forge ran on a different vendor than shared_gpu"
    );
    assert_eq!(
        forge_adapter.device, shared.adapter_caps.device,
        "forge ran on a different device than shared_gpu"
    );

    // softmax (1024-wide) on the shared device matches the oracle and is a distribution.
    {
        let n = 1024usize;
        let x: Vec<f32> = (0..n).map(|i| ((i * 13 % 97) as f32) * 0.1 - 5.0).collect();
        let g = softmax_graph(n as u32).unwrap();
        let gpu = exec.run(&g, &[x.clone()]).expect("softmax shared-gpu");
        let cpu = execute_graph_cpu(&g, &[x]).unwrap();
        for (a, b) in gpu.iter().zip(cpu.iter()) {
            assert!((a - b).abs() <= 1e-4, "softmax(shared): {a} vs {b}");
        }
        assert!((gpu.iter().sum::<f32>() - 1.0).abs() < 1e-3);
    }

    // Full faithful decode block on the SAME held executor (the decode-step usage pattern):
    // externals = [x, Kᵀ, V, Wg, Wu, Wd, inv_scale, eps].
    {
        let (d, kv, ffn) = (64u32, 32u32, 128u32);
        let inv_scale = 1.0f32 / (d as f32).sqrt();
        let eps = 1e-5f32;
        let x: Vec<f32> = (0..d).map(|i| ((i % 17) as f32) * 0.05 - 0.4).collect();
        let kt: Vec<f32> = (0..d * kv)
            .map(|i| ((i * 5 % 7) as f32) * 0.03 - 0.09)
            .collect();
        let v: Vec<f32> = (0..kv * d)
            .map(|i| ((i * 3 % 5) as f32) * 0.04 - 0.08)
            .collect();
        let wg: Vec<f32> = (0..d * ffn)
            .map(|i| ((i % 13) as f32) * 0.02 - 0.12)
            .collect();
        let wu: Vec<f32> = (0..d * ffn)
            .map(|i| ((i % 11) as f32) * 0.015 - 0.07)
            .collect();
        let wd: Vec<f32> = (0..ffn * d)
            .map(|i| ((i % 7) as f32) * 0.01 - 0.03)
            .collect();
        let ext = vec![x, kt, v, wg, wu, wd, vec![inv_scale], vec![eps]];
        let g = decode_block_graph(d, kv, ffn).unwrap();
        let gpu = exec.run(&g, &ext).expect("decode-block shared-gpu");
        let cpu = execute_graph_cpu(&g, &ext).unwrap();
        assert_eq!(gpu.len(), cpu.len());
        for (a, b) in gpu.iter().zip(cpu.iter()) {
            assert!(
                (a - b).abs() <= 1e-2 * b.abs().max(1.0),
                "decode-block(shared): {a} vs {b}"
            );
        }
    }
}
