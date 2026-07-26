use super::{
    segments_for_position, MAX_ATTENTION_SEGMENTS, PAGED_GQA_SEGMENTED_MERGE_ENTRY,
    PAGED_GQA_SEGMENTED_PARTIAL_ENTRY, PAGED_GQA_SEGMENTED_SRC, PAGED_GQA_TILED_ENTRY,
    PAGED_GQA_TILED_SRC,
};
use crate::inference::cuda_lane::device::{ensure_device, multi_weight_device};
use crate::inference::runtime::kv::paged::{paged_gqa_attention_into, PagedKvConfig};
use crate::inference_modes::{set_inference_mode, InferenceMode};
use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

#[test]
#[serial_test::serial]
fn tiled_cuda_matches_scalar_paged_oracle_across_tiles_and_pages() {
    const N_HEAD: usize = 8;
    const N_KV: usize = 2;
    const HEAD_DIM: usize = 64;
    const MAX_CONTEXT: usize = 4096;
    const POSITIONS: [u32; 10] = [0, 1, 15, 16, 31, 32, 63, 1023, 2047, 4095];

    let mut config =
        PagedKvConfig::new(1, N_KV as u32, HEAD_DIM as u32, MAX_CONTEXT as u32).unwrap();
    config.physical_blocks = config.logical_blocks_per_layer();
    let block_elems = config.block_elems();
    let slot_kv = config.slot_kv_elems() as usize;
    let block_table: Vec<u32> = (0..config.logical_blocks_per_layer()).rev().collect();
    let mut arena = vec![0.0f32; block_elems * config.physical_blocks as usize];
    for token in 0..MAX_CONTEXT {
        let logical = token / config.block_size as usize;
        let physical = block_table[logical] as usize;
        let offset = token % config.block_size as usize;
        let slot_base = physical * block_elems + offset * slot_kv * 2;
        for index in 0..slot_kv * 2 {
            arena[slot_base + index] =
                (((token * 131 + index * 37 + 17) % 257) as f32 - 128.0) / 73.0;
        }
    }
    let query: Vec<f32> = (0..N_HEAD * HEAD_DIM)
        .map(|index| (((index * 29 + 7) % 113) as f32 - 56.0) / 41.0)
        .collect();

    set_inference_mode(InferenceMode::CudaTc);
    let Ok(mut guard) = multi_weight_device().lock() else {
        set_inference_mode(InferenceMode::Portable);
        return;
    };
    if !ensure_device(&mut guard) {
        set_inference_mode(InferenceMode::Portable);
        return;
    }
    let dev = guard.as_mut().unwrap();
    dev.ctx.restore_checkpoint(dev.permanent_end);

    let mut q_view = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&query), 0, 0)
        .unwrap();
    let mut kv_view = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&arena), 1, 0)
        .unwrap();
    let base_params = [
        N_HEAD as u32,
        N_KV as u32,
        HEAD_DIM as u32,
        MAX_CONTEXT as u32,
        config.block_size,
        config.logical_blocks_per_layer(),
        config.slot_kv_elems(),
        (N_HEAD / N_KV) as u32,
        1,
    ];
    let scale = [(1.0f32 / (HEAD_DIM as f32).sqrt()).to_bits()];
    let mut scale_view = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&scale), 4, 0)
        .unwrap();
    let mut layer_view = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&[0u32]), 5, 0)
        .unwrap();
    let mut table_view = dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&block_table), 7, 0)
        .unwrap();
    q_view.binding = 0;
    kv_view.binding = 1;
    scale_view.binding = 4;
    layer_view.binding = 5;
    table_view.binding = 7;

    let mut cases = Vec::with_capacity(POSITIONS.len());
    for position in POSITIONS {
        let segments = segments_for_position(position);
        let mut params = base_params;
        params[8] = segments as u32;
        let mut params_view = dev
            .ctx
            .allocate_and_write(bytemuck::cast_slice(&params), 3, 0)
            .unwrap();
        let mut out_view = dev
            .ctx
            .allocate_and_write(bytemuck::cast_slice(&vec![0.0f32; N_HEAD * HEAD_DIM]), 2, 0)
            .unwrap();
        let mut step_view = dev
            .ctx
            .allocate_and_write(bytemuck::cast_slice(&[position, position]), 6, 0)
            .unwrap();
        out_view.binding = 2;
        params_view.binding = 3;
        step_view.binding = 6;
        cases.push((position, segments, out_view, params_view, step_view));
    }

    let mut partial_view = dev
        .ctx
        .allocate_and_write(
            bytemuck::cast_slice(&vec![
                0.0f32;
                N_HEAD * MAX_ATTENTION_SEGMENTS * (HEAD_DIM + 2)
            ]),
            2,
            0,
        )
        .unwrap();
    let pipeline = CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        PAGED_GQA_TILED_SRC,
        PAGED_GQA_TILED_ENTRY,
        &[0, 1, 2, 3, 4, 5, 6, 7],
    )
    .expect("paged tiled online attention compile");
    let segmented_partial = CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        PAGED_GQA_SEGMENTED_SRC,
        PAGED_GQA_SEGMENTED_PARTIAL_ENTRY,
        &[0, 1, 2, 3, 4, 5, 6, 7],
    )
    .expect("segmented attention partial compile");
    let segmented_merge = CudaPipeline::compile_cuda_c_source_cached(
        &dev.ctx,
        PAGED_GQA_SEGMENTED_SRC,
        PAGED_GQA_SEGMENTED_MERGE_ENTRY,
        &[0, 1, 2],
    )
    .expect("segmented attention merge compile");
    let schedule = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let mut expected = vec![0.0f32; N_HEAD * HEAD_DIM];
    for (position, segments, out_view, params_view, step_view) in cases {
        let bindings = [
            q_view,
            kv_view,
            out_view,
            params_view,
            scale_view,
            layer_view,
            step_view,
            table_view,
        ];
        pipeline
            .dispatch(&bindings, &schedule, N_HEAD * 256)
            .expect("paged tiled online attention dispatch");
        let actual = dev.ctx.read_buffer_f32(&out_view).unwrap();
        paged_gqa_attention_into(
            &query,
            &arena,
            &block_table,
            &config,
            position,
            N_HEAD as u32,
            &mut expected,
        )
        .unwrap();
        for (index, (observed, reference)) in actual.iter().zip(&expected).enumerate() {
            let tolerance = 2.0e-4 * reference.abs().max(1.0);
            assert!(
                (observed - reference).abs() <= tolerance,
                "position={position} index={index} cuda={observed} oracle={reference} \
                tolerance={tolerance}"
            );
        }
        if segments > 1 {
            partial_view.binding = 2;
            let partial_bindings = [
                q_view,
                kv_view,
                partial_view,
                params_view,
                scale_view,
                layer_view,
                step_view,
                table_view,
            ];
            segmented_partial
                .dispatch(
                    &partial_bindings,
                    &schedule,
                    N_HEAD * segments * 256,
                )
                .expect("segmented attention partial dispatch");
            let mut merge_partial = partial_view;
            let mut merge_out = out_view;
            let mut merge_params = params_view;
            merge_partial.binding = 0;
            merge_out.binding = 1;
            merge_params.binding = 2;
            segmented_merge
                .dispatch(
                    &[merge_partial, merge_out, merge_params],
                    &schedule,
                    N_HEAD * 256,
                )
                .expect("segmented attention merge dispatch");
            let segmented = dev.ctx.read_buffer_f32(&out_view).unwrap();
            for (index, (observed, reference)) in segmented.iter().zip(&expected).enumerate() {
                let tolerance = 3.0e-4 * reference.abs().max(1.0);
                assert!(
                    (observed - reference).abs() <= tolerance,
                    "segmented position={position} index={index} cuda={observed} \
                    oracle={reference} tolerance={tolerance}"
                );
            }
        }
        if position == 63 {
            for _ in 0..8 {
                pipeline
                    .dispatch_async_sorted(&bindings, &schedule, N_HEAD * 256)
                    .unwrap();
            }
            dev.ctx.stream.synchronize().unwrap();
            let mut samples_ms = [0.0f32; 32];
            for sample in &mut samples_ms {
                *sample = pipeline
                    .dispatch_gpu_timed_ms_sorted(&bindings, &schedule, N_HEAD * 256)
                    .unwrap();
            }
            samples_ms.sort_by(f32::total_cmp);
            eprintln!(
                "paged_tile32_cuda_event position=63 median_us={:.3} p95_us={:.3}",
                samples_ms[16] * 1_000.0,
                samples_ms[30] * 1_000.0
            );
        }
    }
    set_inference_mode(InferenceMode::Portable);
}
