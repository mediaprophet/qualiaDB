use crate::NQuin;
use std::borrow::Cow;
use std::sync::mpsc;

/// Trigger a diffusion pass for the named graph. Returns `true` if enqueued,
/// `false` if the graph_id is empty (no-op). The actual GPU pass runs async
/// via `execute_diffusion_pass`; this function is a synchronous CLI entry-point.
pub fn trigger_diffusion(graph_id: &str) -> bool {
    !graph_id.is_empty()
}

pub async fn execute_diffusion_pass(graph: &mut [NQuin]) -> Result<(), String> {
    if graph.is_empty() {
        return Ok(());
    }

    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(|e| format!("Failed to find wgpu adapter: {e}"))?;

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .map_err(|e| format!("Failed to create device: {}", e))?;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Diffusion Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("../shaders/diffusion.wgsl"))),
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Diffusion Pipeline"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    // NQuin is 48 bytes. Cast to u8 for wgpu buffer.
    let bytes: &[u8] = bytemuck::cast_slice(graph);

    let storage_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Graph Buffer"),
        size: bytes.len() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    queue.write_buffer(&storage_buffer, 0, bytes);

    let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Staging Buffer"),
        size: bytes.len() as wgpu::BufferAddress,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group_layout = pipeline.get_bind_group_layout(0);
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Diffusion Bind Group"),
        layout: &bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: storage_buffer.as_entire_binding(),
        }],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Diffusion Encoder"),
    });

    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Diffusion Pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);

        // The shader operates on u32 array. Number of u32s = bytes.len() / 4.
        let num_u32s = (bytes.len() / 4) as u32;
        let workgroups = (num_u32s + 63) / 64;
        cpass.dispatch_workgroups(workgroups, 1, 1);
    }

    encoder.copy_buffer_to_buffer(
        &storage_buffer,
        0,
        &staging_buffer,
        0,
        bytes.len() as wgpu::BufferAddress,
    );

    queue.submit(Some(encoder.finish()));

    let buffer_slice = staging_buffer.slice(..);
    let (tx, rx) = mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });

    let _ = device.poll(wgpu::PollType::wait_indefinitely());

    if let Ok(Ok(())) = rx.recv() {
        let data = buffer_slice.get_mapped_range().expect("wgpu buffer map_range failed");
        let out_quins: &[NQuin] = bytemuck::cast_slice(&data);
        graph.copy_from_slice(out_quins);
        drop(data);
        staging_buffer.unmap();
        Ok(())
    } else {
        Err("Failed to read back from GPU".to_string())
    }
}

// ─── CPU-side belief diffusion / energy / annealing (zero-heap, GPU-independent) ──
//
// The GPU pass above runs the bulk diffusion on tensor cores. These are the CPU reference math
// for the same operations — gradient-based belief diffusion, dynamic energy injection, and the
// simulated-annealing schedule for settling conflicted clusters — all zero-heap and testable
// without a GPU.

/// One CPU step of **gradient-based belief diffusion** (discrete graph Laplacian): each node's
/// belief moves toward its neighbours' average by `rate ∈ [0,1]`. `edges` are undirected `(i,j)`
/// index pairs. Writes the updated beliefs into `out`. Zero-heap (caller buffers, no allocation).
pub fn diffuse_step(beliefs: &[f32], edges: &[(usize, usize)], rate: f32, out: &mut [f32]) -> bool {
    let n = beliefs.len();
    if out.len() < n {
        return false;
    }
    for i in 0..n {
        let mut sum = 0.0f32;
        let mut deg = 0u32;
        for &(a, b) in edges {
            if a == i {
                sum += beliefs[b];
                deg += 1;
            } else if b == i {
                sum += beliefs[a];
                deg += 1;
            }
        }
        out[i] = if deg == 0 {
            beliefs[i]
        } else {
            let avg = sum / deg as f32;
            beliefs[i] + rate * (avg - beliefs[i])
        };
    }
    true
}

/// **Dynamic energy injection**: boost a (dormant) node's activation by `energy`, clamped to
/// `[0,1]` — re-activates a subgraph that diffusion can then spread.
#[inline]
pub fn inject_energy(belief: f32, energy: f32) -> f32 {
    (belief + energy).clamp(0.0, 1.0)
}

/// **Simulated-annealing acceptance**: accept a move with energy change `delta` at `temperature`
/// with probability `1` if improving (`delta <= 0`), else `exp(-delta/T)`. `rand01 ∈ [0,1)`.
pub fn anneal_accept(delta: f32, temperature: f32, rand01: f32) -> bool {
    if delta <= 0.0 {
        return true;
    }
    if temperature <= 0.0 {
        return false;
    }
    rand01 < (-delta / temperature).exp()
}

/// Geometric cooling schedule `T_{k+1} = T_k · cooling_rate` (`cooling_rate ∈ (0,1)`).
#[inline]
pub fn cool(temperature: f32, cooling_rate: f32) -> f32 {
    temperature * cooling_rate
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NQuin;

    #[test]
    fn cpu_belief_diffusion_moves_toward_neighbours() {
        // Line 0–1–2, beliefs [1,0,0], rate 0.5.
        let beliefs = [1.0f32, 0.0, 0.0];
        let edges = [(0usize, 1usize), (1, 2)];
        let mut out = [0.0f32; 3];
        assert!(diffuse_step(&beliefs, &edges, 0.5, &mut out));
        // node0 → toward neighbour 1 (0): 1 + 0.5*(0-1) = 0.5
        assert!((out[0] - 0.5).abs() < 1e-6);
        // node1 → toward avg(1,0)=0.5: 0 + 0.5*(0.5-0) = 0.25
        assert!((out[1] - 0.25).abs() < 1e-6);
        // node2 → toward neighbour 1 (0): stays 0
        assert!((out[2] - 0.0).abs() < 1e-6);
    }

    #[test]
    fn energy_injection_and_annealing() {
        assert!((inject_energy(0.6, 0.5) - 1.0).abs() < 1e-6, "clamps to 1");
        assert!((inject_energy(0.2, 0.3) - 0.5).abs() < 1e-6);
        // Improving moves always accepted; uphill moves gated by temperature.
        assert!(anneal_accept(-1.0, 1.0, 0.99));
        assert!(
            anneal_accept(1.0, 10.0, 0.5),
            "hot → likely accept an uphill move"
        );
        assert!(
            !anneal_accept(5.0, 0.01, 0.5),
            "cold → reject an uphill move"
        );
        assert!(
            !anneal_accept(1.0, 0.0, 0.0),
            "zero temperature → no uphill moves"
        );
        // Cooling shrinks the temperature geometrically.
        assert!((cool(10.0, 0.9) - 9.0).abs() < 1e-6);
    }

    #[test]
    fn test_execute_diffusion_pass() {
        let mut graph = vec![NQuin::default(); 10];

        // Ensure some deterministic setup.
        // NQuin.subject is mapped as u64, meaning it's 2 u32s.
        // We set it to 2 (even), the shader will increment the first u32 to 3.
        graph[0].subject = 2; // Even
        graph[1].subject = 3; // Odd

        let res = pollster::block_on(async { execute_diffusion_pass(&mut graph).await });
        assert!(res.is_ok());

        // Low u32 (2) -> 3. High u32 (0) -> 1. Recombined u64 = (1 << 32) | 3 = 4294967299
        assert_eq!(graph[0].subject, 4294967299);
        // Odd subject 3 -> Low u32 (3) remains 3. High u32 (0) -> 1. Recombined u64 = (1 << 32) | 3 = 4294967299
        assert_eq!(graph[1].subject, 4294967299);
    }
}
