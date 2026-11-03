//! Consumer contract: policy and public error behavior remain owned by Qualia.
use super::{
    ComputeUniverse, GraphicsReservationError, OperationalMode, VramLedger, VramResourceClass,
};

const CLASSES: [VramResourceClass; 6] = [
    VramResourceClass::Geometry,
    VramResourceClass::TextureResidency,
    VramResourceClass::FrameTarget,
    VramResourceClass::UploadStaging,
    VramResourceClass::RecoveryCopy,
    VramResourceClass::FieldResidency,
];

#[test]
fn migration_all_six_classes_keep_identity_accounting_and_owned_release() {
    let ledger = VramLedger::new(1000);
    ledger.record_kv_cache(200);
    let guards: [_; 6] = core::array::from_fn(|index| {
        assert_eq!(CLASSES[index] as usize, index);
        ledger
            .try_reserve_graphics(CLASSES[index], (index as u64 + 1) * 5)
            .unwrap()
    });
    for (index, class) in CLASSES.iter().enumerate() {
        assert_eq!(
            ledger.reserved_graphics_bytes(*class),
            (index as u64 + 1) * 5
        );
    }
    assert_eq!(ledger.used_bytes(), 305);
    assert_eq!(ledger.universe_used_bytes(ComputeUniverse::Viewport), 105);
    assert_eq!(
        ledger.universe_used_bytes(ComputeUniverse::LlmInference),
        200
    );
    let error = ledger
        .try_reserve_graphics(VramResourceClass::Geometry, 46)
        .unwrap_err();
    assert_eq!(
        error,
        GraphicsReservationError {
            requested_bytes: 46,
            available_bytes: 45
        }
    );
    assert_eq!(
        error.to_string(),
        "graphics reservation refused: requested 46 bytes, 45 available"
    );
    drop(guards);
    assert_eq!(ledger.used_bytes(), 200);
    assert_eq!(ledger.universe_used_bytes(ComputeUniverse::Viewport), 0);
    for class in CLASSES {
        assert_eq!(ledger.reserved_graphics_bytes(class), 0);
    }
}

#[test]
fn migration_drop_refreshes_mode_after_removing_reservation_including_unwind() {
    let ledger = VramLedger::new(1000);
    ledger.record_tensor(700);
    assert_eq!(ledger.mode(), OperationalMode::Full);
    let guard = ledger
        .try_reserve_graphics(VramResourceClass::FrameTarget, 20)
        .unwrap();
    assert_eq!(ledger.mode(), OperationalMode::Eco);
    drop(guard);
    assert_eq!(ledger.mode(), OperationalMode::Full);
    let result = std::panic::catch_unwind(|| {
        let _guard = ledger
            .try_reserve_graphics(VramResourceClass::UploadStaging, 20)
            .unwrap();
        assert_eq!(ledger.mode(), OperationalMode::Eco);
        panic!("injected consumer unwind");
    });
    assert!(result.is_err());
    assert_eq!(ledger.used_bytes(), 700);
    assert_eq!(
        ledger.reserved_graphics_bytes(VramResourceClass::UploadStaging),
        0
    );
    assert_eq!(ledger.mode(), OperationalMode::Full);
}

#[test]
fn migration_stored_mode_and_refused_admission_do_not_recompute_policy() {
    let ledger = VramLedger::new(1000);
    ledger.record_tensor(800);
    assert_eq!(ledger.mode(), OperationalMode::Eco);
    ledger.set_budget(2000); // Deliberately does not refresh the stored mode.
    assert_eq!(ledger.mode(), OperationalMode::Eco);
    let before = ledger.used_bytes();
    assert_eq!(
        ledger
            .try_reserve_graphics(VramResourceClass::Geometry, 501)
            .unwrap_err(),
        GraphicsReservationError {
            requested_bytes: 501,
            available_bytes: 500
        }
    );
    assert_eq!(ledger.used_bytes(), before);
    assert_eq!(ledger.mode(), OperationalMode::Eco);
    for class in CLASSES {
        assert_eq!(ledger.reserved_graphics_bytes(class), 0);
    }
    // Eco's stored 25% viewport partition admits 400; recomputing Full first
    // would incorrectly restrict this request to 300.
    let guard = ledger
        .try_reserve_graphics(VramResourceClass::Geometry, 400)
        .unwrap();
    assert_eq!(ledger.mode(), OperationalMode::Full);
    assert_eq!(ledger.used_bytes(), 1200);
    drop(guard);
    assert_eq!(ledger.used_bytes(), 800);
}

#[test]
fn migration_exact_capacity_zero_and_shrunk_budget_refusals_are_transactional() {
    let ledger = VramLedger::new(1000);
    ledger.record_render(20);
    let guard = ledger
        .try_reserve_graphics(VramResourceClass::TextureResidency, 130)
        .unwrap();
    let zero = ledger
        .try_reserve_graphics(VramResourceClass::RecoveryCopy, 0)
        .unwrap();
    assert_eq!(ledger.used_bytes(), 150);
    ledger.set_budget(100);
    let mode = ledger.mode();
    for request in [0, 1, u64::MAX] {
        assert_eq!(
            ledger
                .try_reserve_graphics(VramResourceClass::FieldResidency, request)
                .unwrap_err(),
            GraphicsReservationError {
                requested_bytes: request,
                available_bytes: 0
            }
        );
        assert_eq!(ledger.used_bytes(), 150);
        assert_eq!(ledger.mode(), mode);
        assert_eq!(
            ledger.reserved_graphics_bytes(VramResourceClass::TextureResidency),
            130
        );
        assert_eq!(
            ledger.reserved_graphics_bytes(VramResourceClass::FieldResidency),
            0
        );
    }
    drop(zero);
    drop(guard);
    assert_eq!(ledger.used_bytes(), 20);
    assert_eq!(ledger.mode(), OperationalMode::Full);
}
