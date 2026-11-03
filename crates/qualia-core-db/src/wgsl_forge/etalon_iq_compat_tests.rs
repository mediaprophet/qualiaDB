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
