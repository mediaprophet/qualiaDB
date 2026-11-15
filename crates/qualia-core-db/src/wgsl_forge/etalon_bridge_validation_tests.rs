use super::{
    certify_etalon_kernel, generate_and_certify_via_etalon, generate_with_validation_via_etalon,
    BuiltinKernel, Schedule, TargetBackend,
};

#[test]
fn generated_wgsl_retains_actual_validation_evidence() {
    let (shader, validation) = generate_with_validation_via_etalon(
        BuiltinKernel::AffineF32,
        Schedule::default(),
        TargetBackend::Wgsl,
    )
    .unwrap();
    let report = validation.expect("WGSL returns its validation report");
    assert_eq!(report, certify_etalon_kernel(&shader.source).unwrap());
    assert_eq!(report.source_hash, shader.source_hash);
    assert_eq!(report.native_tool_validated, None);
    if report.naga_validated {
        assert_eq!(report.entry_points, ["affine_f32"]);
        assert!(report.binding_count > 0);
    } else {
        assert!(report.entry_points.is_empty());
        assert_eq!(report.binding_count, 0);
    }
    assert_eq!(
        shader,
        generate_and_certify_via_etalon(
            BuiltinKernel::AffineF32,
            Schedule::default(),
            TargetBackend::Wgsl,
        )
        .unwrap()
    );
}

#[test]
fn native_emission_does_not_claim_validation() {
    let (shader, validation) = generate_with_validation_via_etalon(
        BuiltinKernel::AffineF32,
        Schedule::default(),
        TargetBackend::Hlsl,
    )
    .unwrap();
    assert!(!shader.source.is_empty());
    assert_eq!(validation, None);
    assert_eq!(
        shader,
        generate_and_certify_via_etalon(
            BuiltinKernel::AffineF32,
            Schedule::default(),
            TargetBackend::Hlsl,
        )
        .unwrap()
    );
}

#[test]
fn supplied_wgsl_uses_validation_without_forge_ir() {
    let source = "@compute @workgroup_size(1) fn supplied_entry() {}";
    let report = certify_etalon_kernel(source).unwrap();
    assert_eq!(
        report.source_hash,
        blake3::hash(source.as_bytes()).to_hex().to_string()
    );
    assert_eq!(report.native_tool_validated, None);
    if report.naga_validated {
        assert_eq!(report.entry_points, ["supplied_entry"]);
        assert_eq!(report.binding_count, 0);
        assert!(certify_etalon_kernel("invalid WGSL").is_err());
    } else {
        assert!(
            !certify_etalon_kernel("invalid WGSL")
                .unwrap()
                .naga_validated
        );
    }
}

#[test]
fn nquin_wire_abi_48_byte_roundtrip_and_5_field_parity() {
    use crate::wgsl_forge::ir::q42_bridge::{
        enumerate_etalon_eval_receipt_to_quins, qualify_nquin_wire_sieve_and_radix,
        EtalonNQuinEvalReceipt,
    };
    use crate::NQuin;
    use etalon_nquin::{
        radix_sort_u64_cpu, sieve_eq_cpu, NQuinWire, QuinField, RadixSortOutcome, NQUIN_ALIGN,
        NQUIN_BYTES,
    };

    assert_eq!(NQUIN_BYTES, 48);
    assert_eq!(NQUIN_ALIGN, 16);
    assert_eq!(core::mem::size_of::<NQuin>(), NQUIN_BYTES);
    assert_eq!(core::mem::size_of::<NQuinWire>(), NQUIN_BYTES);
    assert_eq!(core::mem::align_of::<NQuinWire>(), NQUIN_ALIGN);

    let mk_quin = |s: u64, p: u64, o: u64, c: u64, m: u64| NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: c,
        metadata: m,
        parity: NQuin::calculate_parity(s, p, o, c, m),
    };

    let quins = [
        mk_quin(0x101, 0x202, 0x303, 0x404, 0x505),
        mk_quin(0x109, 0x202, 0x301, 0x404, 0x506),
        mk_quin(0x103, 0x999, 0x302, 0x404, 0x507),
        mk_quin(0x105, 0x202, 0x300, 0x404, 0x508),
    ];
    for q in &quins {
        assert!(q.verify_ecc_parity());
        let w = NQuinWire::new(q.subject, q.predicate, q.object, q.context, q.metadata, q.parity);
        assert!(w.is_parity_valid());
        let raw: [u8; 48] = bytemuck::cast(*q);
        let w_cast: NQuinWire = bytemuck::cast(raw);
        assert_eq!(w, w_cast);
    }

    let wire: Vec<NQuinWire> = quins
        .iter()
        .map(|q| NQuinWire::new(q.subject, q.predicate, q.object, q.context, q.metadata, q.parity))
        .collect();
    let mut sieve_out = vec![NQuinWire::new(0, 0, 0, 0, 0, 0); wire.len()];
    let matched = sieve_eq_cpu(&wire, QuinField::Predicate, 0x202, &mut sieve_out);
    assert_eq!(matched, 3);

    let mut radix_keys: Vec<u64> = wire.iter().map(|w| w.predicate).collect();
    let mut radix_indices: Vec<u32> = (0..wire.len() as u32).collect();
    assert_eq!(
        radix_sort_u64_cpu(&mut radix_keys, &mut radix_indices),
        RadixSortOutcome::Sorted { passes: 8 }
    );
    assert_eq!(radix_keys, vec![0x202, 0x202, 0x202, 0x999]);

    let receipt = qualify_nquin_wire_sieve_and_radix(
        0xE7A1_0414_0001,
        &quins,
        QuinField::Predicate,
        0x202,
        &sieve_out[..matched],
        &radix_keys,
        &radix_indices,
    );
    assert!(!receipt.disqualified);
    assert!(receipt.sieve_matches_cpu);
    assert!(receipt.radix_order_matches_cpu);
    assert!(receipt.parity_verified_all);

    let mut out_quins = [mk_quin(0, 0, 0, 0, 0); 6];
    enumerate_etalon_eval_receipt_to_quins(&receipt, &mut out_quins);
    for (i, q) in out_quins.iter().enumerate() {
        assert!(q.verify_ecc_parity(), "receipt quin {i} must have 5-field parity");
    }

    // Any sieve or radix mismatch or noop backend (hash == 0) must mark disqualified = true.
    let bad_radix = vec![0x999, 0x202, 0x202, 0x202];
    let mismatched = qualify_nquin_wire_sieve_and_radix(
        0xE7A1_0414_0001,
        &quins,
        QuinField::Predicate,
        0x202,
        &sieve_out[..matched],
        &bad_radix,
        &radix_indices,
    );
    assert!(mismatched.disqualified);
    assert!(!mismatched.radix_order_matches_cpu);

    let noop_receipt = EtalonNQuinEvalReceipt::new(0, 4, true, true, true);
    assert!(noop_receipt.disqualified);
}
