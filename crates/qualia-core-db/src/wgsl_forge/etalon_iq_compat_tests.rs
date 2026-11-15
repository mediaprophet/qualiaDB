//! Independent fixtures frozen against actual pre-IQ source at 07018bb8.
//! These are CPU/source tests. They make no GPU execution claim.

use super::{
    emit_shader, BufferAccess, BufferElement, BuiltinKernel, Op, ScalarType, Schedule, SharedLen,
    SharedMemorySpec, TargetBackend,
};

const LEGACY: [(BuiltinKernel, &str, &str); 9] = [
    (
        BuiltinKernel::AffineF32,
        "affine-f32",
        "5ad91ac72d82f4b43cb70e65027c2e4ab79ebbfbe2e2b9d599780bf07c5389c7",
    ),
    (
        BuiltinKernel::FusedFfn,
        "fused-ffn",
        "9d5ae6e875cc2414ac8110b9f0871427f3c9f007cab2f1cfc35876649aa344d0",
    ),
    (
        BuiltinKernel::P64Project,
        "p64-project",
        "4a6c6a36a9710a0bdf682011e2930304dccf0e505df38140a8c4d5714e1da53c",
    ),
    (
        BuiltinKernel::TopK,
        "topk",
        "d1ac7495020a86389e3ea52bc363e9e91f2f767e2ac90fff34107bd81c0de858",
    ),
    (
        BuiltinKernel::RayProbe,
        "ray-probe",
        "173d5724820672aa550cd3035ff3faebc29b575419ded34742a6ae364276a243",
    ),
    (
        BuiltinKernel::TernaryGemv,
        "ternary-gemv",
        "66f8df34180571f1007cd620c21cae6e49a403d2277dfe6eaf989b7acf16809a",
    ),
    (
        BuiltinKernel::Gemm,
        "gemm",
        "51cad73e49a5ddb2bce73c3df8265b5f0d2179c6c37bd28d6c77c8d196fa6440",
    ),
    (
        BuiltinKernel::Gemv,
        "gemv",
        "b60af8bbc2d44a011fb745f58dab3a42829fa9670ec50a839df0cd6f62f766a2",
    ),
    (
        BuiltinKernel::Fft,
        "fft",
        "128f03f6bac3a18ecfd39e9e7009c9665e9982df6475af1ca518830b38caf4cc",
    ),
];

#[test]
fn legacy_names_discriminants_serde_and_complete_spec_hashes_are_unchanged() {
    for (ordinal, (variant, name, hash)) in LEGACY.into_iter().enumerate() {
        assert_eq!(variant as usize, ordinal, "{name} discriminant");
        assert_eq!(variant.name(), name);
        // Frozen actual legacy serde uses kebab-case TopK -> "top-k";
        // its public display/parse name remains "topk".
        let serialized_name = if variant == BuiltinKernel::TopK {
            "top-k"
        } else {
            name
        };
        assert_eq!(
            serde_json::to_value(variant).unwrap(),
            serde_json::json!(serialized_name)
        );
        assert_eq!(
            serde_json::from_value::<BuiltinKernel>(serde_json::json!(serialized_name)).unwrap(),
            variant
        );
        assert_eq!(name.parse::<BuiltinKernel>().unwrap(), variant);
        assert_eq!(
            variant.spec().semantic_hash().unwrap(),
            hash,
            "{name} full spec"
        );
        assert!(
            variant.has_gpu_oracle(),
            "legacy capability removed: {name}"
        );
    }
}

#[test]
fn legacy_parse_aliases_remain_supported() {
    let aliases = [
        ("affine", BuiltinKernel::AffineF32),
        ("affine_f32", BuiltinKernel::AffineF32),
        ("ffn", BuiltinKernel::FusedFfn),
        ("fused_ffn", BuiltinKernel::FusedFfn),
        ("p64", BuiltinKernel::P64Project),
        ("p64_project", BuiltinKernel::P64Project),
        ("top-k", BuiltinKernel::TopK),
        ("top_k", BuiltinKernel::TopK),
        ("ray_probe", BuiltinKernel::RayProbe),
        ("rayquery", BuiltinKernel::RayProbe),
        ("ray_query", BuiltinKernel::RayProbe),
        ("rt", BuiltinKernel::RayProbe),
        ("ternary_gemv", BuiltinKernel::TernaryGemv),
        ("ternary", BuiltinKernel::TernaryGemv),
        ("bitnet", BuiltinKernel::TernaryGemv),
        ("matmul", BuiltinKernel::Gemm),
        ("dense-gemm", BuiltinKernel::Gemm),
        ("dense_gemm", BuiltinKernel::Gemm),
        ("matvec", BuiltinKernel::Gemv),
        ("dft", BuiltinKernel::Fft),
        ("radix2", BuiltinKernel::Fft),
        ("radix-2", BuiltinKernel::Fft),
    ];
    for (name, variant) in aliases {
        assert_eq!(
            format!(" {} ", name.to_uppercase())
                .parse::<BuiltinKernel>()
                .unwrap(),
            variant
        );
    }
    assert!("not-a-kernel".parse::<BuiltinKernel>().is_err());
}

#[test]
fn iq_names_and_capability_are_explicit_and_unique() {
    assert_eq!(BuiltinKernel::ALL.len(), 11);
    for (index, variant) in BuiltinKernel::ALL.into_iter().enumerate() {
        assert!(!BuiltinKernel::ALL[..index].contains(&variant));
    }
    for (variant, name) in [
        (BuiltinKernel::IqPowerF32, "iq-power-f32"),
        (BuiltinKernel::IqPowerI16, "iq-power-i16"),
    ] {
        assert_eq!(variant.name(), name);
        assert_eq!(name.parse::<BuiltinKernel>().unwrap(), variant);
        assert_eq!(
            serde_json::to_value(variant).unwrap(),
            serde_json::json!(name)
        );
        assert_eq!(
            serde_json::from_value::<BuiltinKernel>(serde_json::json!(name)).unwrap(),
            variant
        );
        assert!(
            !variant.has_gpu_oracle(),
            "local dedicated oracle is not implemented in this packet"
        );
    }
}

#[cfg(feature = "gpu-runtime")]
#[test]
fn all_public_builtin_conversions_are_lossless() {
    for variant in BuiltinKernel::ALL {
        let external: etalon_wgsl_forge::BuiltinKernel = variant.into();
        assert_eq!(BuiltinKernel::from(external), variant);
        assert_eq!(super::etalon_bridge::from_etalon_builtin(external), variant);
        assert_eq!(super::etalon_bridge::to_etalon_builtin(variant), external);
    }
    for external in etalon_wgsl_forge::BuiltinKernel::ALL {
        let local: BuiltinKernel = external.into();
        assert_eq!(etalon_wgsl_forge::BuiltinKernel::from(local), external);
    }
}

#[test]
fn iq_template_rejects_mutated_metadata_bindings_and_operations() {
    for variant in [BuiltinKernel::IqPowerF32, BuiltinKernel::IqPowerI16] {
        let original = variant.spec();
        original.validate().unwrap();
        for mutation in 0..15 {
            let mut spec = original.clone();
            match mutation {
                0 => spec.entry_point = "other_entry".into(),
                1 => spec.semantic_version += 1,
                2 => spec.description.push_str(" changed"),
                3 => spec.buffers[0].group = 1,
                4 => spec.buffers[0].binding = 7,
                5 => spec.buffers[0].name = "different_input".into(),
                6 => spec.buffers[0].access = BufferAccess::StorageReadWrite,
                7 => spec.buffers[2].element = BufferElement::AffineParams,
                8 => {
                    spec.buffers.pop();
                }
                9 => spec.ops.push(Op::Barrier),
                10 => spec.id = "not-iq".into(),
                11 => spec.buffers.swap(0, 1),
                12 => spec.buffers[0].element = BufferElement::Scalar(ScalarType::I32),
                13 => spec.buffers[1].element = BufferElement::Scalar(ScalarType::U32),
                14 => spec.shared_memory.push(SharedMemorySpec {
                    name: "extra".into(),
                    element: ScalarType::F32,
                    length: SharedLen::Fixed(1),
                }),
                _ => unreachable!(),
            }
            assert!(spec.validate().is_err(), "{variant:?} mutation {mutation}");
            for target in [
                TargetBackend::Wgsl,
                TargetBackend::CudaC,
                TargetBackend::Hlsl,
                TargetBackend::Msl,
            ] {
                assert!(
                    emit_shader(&spec, Schedule::default(), target).is_err(),
                    "{variant:?} mutation {mutation} {target:?}"
                );
            }
        }
    }
}

#[cfg(feature = "gpu-runtime")]
#[test]
fn iq_emission_is_canonical_deterministic_and_metadata_exact() {
    for variant in [BuiltinKernel::IqPowerF32, BuiltinKernel::IqPowerI16] {
        let spec = variant.spec();
        for target in [
            TargetBackend::Wgsl,
            TargetBackend::CudaC,
            TargetBackend::Hlsl,
            TargetBackend::Msl,
        ] {
            for workgroup_size in [1, 64, 1024] {
                let schedule = Schedule {
                    workgroup_size,
                    items_per_invocation: 1,
                    vector_width: 1,
                };
                let generated = emit_shader(&spec, schedule, target).unwrap();
                assert_eq!(emit_shader(&spec, schedule, target).unwrap(), generated);
                assert_eq!(generated.kernel_id, spec.id);
                assert_eq!(generated.semantic_hash, spec.semantic_hash().unwrap());
                assert_eq!(
                    generated.source_hash,
                    blake3::hash(generated.source.as_bytes())
                        .to_hex()
                        .to_string()
                );
                assert_eq!(generated.schedule, schedule);
                assert!(generated.source.contains(&spec.entry_point));
                assert!(!generated.source.contains("affine_f32"));
                let external_spec = if variant == BuiltinKernel::IqPowerF32 {
                    etalon_wgsl_forge::BuiltinKernel::IqPowerF32.spec()
                } else {
                    etalon_wgsl_forge::BuiltinKernel::IqPowerI16.spec()
                };
                let canonical =
                    etalon_wgsl_forge::emit_shader(&external_spec, schedule.into(), target.into())
                        .unwrap();
                assert_eq!(generated.source, canonical.source);
            }
        }
    }
}

#[test]
fn iq_invalid_schedules_cannot_be_silently_rewritten() {
    for variant in [BuiltinKernel::IqPowerF32, BuiltinKernel::IqPowerI16] {
        for schedule in [
            Schedule {
                workgroup_size: 0,
                ..Schedule::default()
            },
            Schedule {
                workgroup_size: 1025,
                ..Schedule::default()
            },
            Schedule {
                items_per_invocation: 2,
                ..Schedule::default()
            },
            Schedule {
                vector_width: 2,
                ..Schedule::default()
            },
        ] {
            for target in [
                TargetBackend::Wgsl,
                TargetBackend::CudaC,
                TargetBackend::Hlsl,
                TargetBackend::Msl,
            ] {
                assert!(emit_shader(&variant.spec(), schedule, target).is_err());
            }
        }
    }
}

#[test]
fn direct_public_emitters_enforce_iq_contract_and_target_support() {
    type Emitter =
        fn(&super::KernelSpec, Schedule) -> Result<super::GeneratedShader, super::ForgeError>;
    let emitters: [(TargetBackend, Emitter); 6] = [
        (TargetBackend::Wgsl, super::emit::emit_wgsl),
        (TargetBackend::Hlsl, super::emit::emit_hlsl),
        (TargetBackend::Msl, super::emit::emit_msl),
        (TargetBackend::CudaC, super::emit::emit_cuda_c),
        (TargetBackend::Ptx, super::emit::emit_ptx),
        (TargetBackend::Spirv, super::emit::emit_spirv),
    ];
    for variant in [BuiltinKernel::IqPowerF32, BuiltinKernel::IqPowerI16] {
        let spec = variant.spec();
        let mut malformed = spec.clone();
        malformed.ops.push(Op::Barrier);
        for (target, direct) in emitters {
            assert!(
                direct(&malformed, Schedule::default()).is_err(),
                "direct {target:?} accepted altered ops"
            );
            assert!(emit_shader(&malformed, Schedule::default(), target).is_err());
            let actual = direct(&spec, Schedule::default());
            if target == TargetBackend::Ptx {
                assert!(
                    actual.is_err(),
                    "IQ PTX must explicitly reject unsupported emission"
                );
                continue;
            }
            #[cfg(feature = "gpu-runtime")]
            {
                let actual = actual.unwrap();
                assert_eq!(
                    actual,
                    emit_shader(&spec, Schedule::default(), target).unwrap()
                );
                if target == TargetBackend::Spirv {
                    let words = super::decode_spirv_words(&actual.source).unwrap();
                    assert!(words.len() > 5);
                    assert_eq!(words[0], 0x0723_0203);
                    assert_eq!(actual.semantic_hash, spec.semantic_hash().unwrap());
                    assert_eq!(
                        actual.source_hash,
                        blake3::hash(actual.source.as_bytes()).to_hex().to_string()
                    );
                }
            }
            #[cfg(not(feature = "gpu-runtime"))]
            {
                let error = actual.expect_err("IQ requires the unavailable Etalon dependency");
                assert!(error
                    .to_string()
                    .contains("requires the Etalon gpu-runtime dependency"));
                assert!(emit_shader(&spec, Schedule::default(), target).is_err());
            }
        }
    }
}

#[test]
fn cpu_oracle_rejects_nan_injection_and_nonzero_mismatches() {
    use crate::wgsl_forge::ir::q42_bridge::{
        verify_gpu_against_cpu_oracle, OracleVerificationError,
    };

    let cpu_ref = [1.0f32, -2.5, 3.25, 0.125];
    let mut gpu_with_nan = cpu_ref;
    gpu_with_nan[2] = f32::NAN;
    assert_eq!(
        verify_gpu_against_cpu_oracle(&cpu_ref, &gpu_with_nan, 1e-5, 1e-5),
        Err(OracleVerificationError::NaN { index: 2 })
    );

    // Out-of-tolerance candidate where compare_f32 would return Ok(NumericalComparison { mismatches: 2, .. })
    // MUST be rejected because mismatches != 0.
    let gpu_out_of_tol = [1.0f32, -2.5 + 0.05, 3.25, 0.125 - 0.02];
    match verify_gpu_against_cpu_oracle(&cpu_ref, &gpu_out_of_tol, 1e-4, 1e-4) {
        Err(OracleVerificationError::ToleranceExceeded { mismatches, .. }) => {
            assert_eq!(mismatches, 2);
        }
        other => panic!("expected ToleranceExceeded with mismatches == 2, got {other:?}"),
    }
}

#[test]
fn tensor10d_distance_metrics_and_yee_fdtd_verify_against_cpu_oracle() {
    use crate::tensor::Tensor10D;
    use crate::wgsl_forge::ir::q42_bridge::verify_gpu_against_cpu_oracle;

    let a_dims = [0.1f32, 0.2, 0.3, 0.4, 0.5, 0.15, 0.25, 0.35, 0.05, 0.1];
    let b_dims = [0.2f32, 0.1, 0.4, 0.3, 0.2, 0.35, 0.15, 0.25, 0.15, 0.2];
    let a = Tensor10D::new(
        a_dims[0], a_dims[1], a_dims[2], a_dims[3], a_dims[4], a_dims[5], a_dims[6], a_dims[7],
        a_dims[8], a_dims[9],
    );
    let b = Tensor10D::new(
        b_dims[0], b_dims[1], b_dims[2], b_dims[3], b_dims[4], b_dims[5], b_dims[6], b_dims[7],
        b_dims[8], b_dims[9],
    );

    // v = 0: Euclidean distance
    let euclid_cpu = a.full_distance(&b);
    // v = 1: Cyclic / Toroidal distance on S^1
    let two_pi = core::f32::consts::TAU;
    let cyclic_cpu = a_dims
        .iter()
        .zip(b_dims.iter())
        .map(|(&x, &y)| {
            let diff = (x - y).abs();
            let wrapped = diff.min(two_pi - diff);
            wrapped * wrapped
        })
        .sum::<f32>()
        .sqrt();
    // v = 2: Hyperbolic Poincare ball distance
    let norm_sq_a: f32 = a_dims.iter().map(|x| x * x).sum::<f32>() * 0.1;
    let norm_sq_b: f32 = b_dims.iter().map(|x| x * x).sum::<f32>() * 0.1;
    let diff_sq: f32 = a_dims
        .iter()
        .zip(b_dims.iter())
        .map(|(&x, &y)| (x - y) * (x - y))
        .sum::<f32>()
        * 0.1;
    let arg: f32 = 1.0 + (2.0 * diff_sq) / ((1.0 - norm_sq_a) * (1.0 - norm_sq_b));
    let hyp_cpu = arg.acosh();

    let cpu_metrics = [euclid_cpu, cyclic_cpu, hyp_cpu];
    let gpu_metrics = [
        euclid_cpu + 1e-7,
        cyclic_cpu - 1e-7,
        hyp_cpu + 2e-7,
    ];
    let metric_receipt =
        verify_gpu_against_cpu_oracle(&cpu_metrics, &gpu_metrics, 1e-5, 1e-5).expect("oracle pass");
    assert_eq!(metric_receipt.mismatches, 0);
    assert_eq!(metric_receipt.comparison.mismatches, 0);
    assert!(metric_receipt.calibration_quin.verify_ecc_parity());

    // 1D Yee FDTD Maxwell wave propagation with Schneider semi-implicit conductive update:
    // Verify phase velocity (Sc = 0.5 => 50 cells in 100 steps), lossless energy conservation,
    // and signed conductive energy dissipation (e1 < e0).
    fn run_semi_implicit_yee_1d(n: usize, steps: usize, sigma: f64) -> (Vec<f32>, f64, f64) {
        let dx = 1.0f64;
        let eps = 1.0f64;
        let mu = 1.0f64;
        let c = 1.0 / (eps * mu).sqrt();
        let dt = 0.5 * dx / c; // Courant number Sc = c * dt / dx = 0.5 <= 1.0
        let eta = (mu / eps).sqrt();
        let mut ey = vec![0.0f64; n];
        let mut hz = vec![0.0f64; n];
        let x0 = n as f64 * 0.25;
        let width = n as f64 / 16.0;
        for i in 0..n {
            let g = (-((i as f64 - x0) / width).powi(2)).exp();
            ey[i] = g;
            hz[i] = g / eta;
        }
        let energy = |ey: &[f64], hz: &[f64]| -> f64 {
            0.5 * (0..n).map(|i| eps * ey[i] * ey[i] + mu * hz[i] * hz[i]).sum::<f64>()
        };
        let e0 = energy(&ey, &hz);
        let ch = dt / (mu * dx);
        let a = 0.5 * sigma * dt / eps;
        for _ in 0..steps {
            for i in 0..n {
                let ip1 = if i + 1 < n { i + 1 } else { 0 };
                hz[i] -= ch * (ey[ip1] - ey[i]);
            }
            for i in 0..n {
                let im1 = if i == 0 { n - 1 } else { i - 1 };
                ey[i] = ((1.0 - a) / (1.0 + a)) * ey[i]
                    - ((dt / (eps * dx)) / (1.0 + a)) * (hz[i] - hz[im1]);
            }
        }
        let e1 = energy(&ey, &hz);
        let ey_f32 = ey.into_iter().map(|v| v as f32).collect();
        (ey_f32, e0, e1)
    }

    let (lossless_ey, e0_lossless, e1_lossless) = run_semi_implicit_yee_1d(200, 100, 0.0);
    assert!(((e1_lossless - e0_lossless) / e0_lossless).abs() < 0.05);
    let peak_idx = lossless_ey
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap()
        .0;
    assert!((peak_idx as i64 - 100).abs() <= 3);

    let (lossy_ey, e0_lossy, e1_lossy) = run_semi_implicit_yee_1d(200, 200, 0.05);
    assert!(e1_lossy.is_finite() && e1_lossy < e0_lossy);

    let yee_receipt =
        verify_gpu_against_cpu_oracle(&lossy_ey, &lossy_ey, 1e-6, 1e-6).expect("yee oracle");
    assert_eq!(yee_receipt.mismatches, 0);
    assert!(yee_receipt.calibration_quin.verify_ecc_parity());
}

