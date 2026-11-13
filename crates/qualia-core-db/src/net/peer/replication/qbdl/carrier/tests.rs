use super::super::wire::{self, Header, Kind, ObjectSelector, ServiceRecordHeader};
use super::*;
use crate::net::qdnf::types::StrongDigest as D;
fn header(kind: Kind) -> Header {
    Header {
        kind,
        descriptor: D([1; 48]),
        context: D([2; 48]),
        publication_generation: 1,
        policy_generation: 2,
        cancellation: 0,
        transfer_id: [3; 16],
        entry: 0,
        leaf: 0,
        count: 0,
        credit_serial: 1,
        credit_bytes: 65776,
        deadline: 12345,
    }
}
fn page(n: usize) -> Vec<u8> {
    let mut body = vec![0x42; 32 + n];
    ObjectSelector {
        class: 1,
        number: 0,
        total: n as u64,
        offset: 0,
        chunk: n as u32,
    }
    .encode(false, &mut body[..32])
    .unwrap();
    let mut result = vec![0; 208 + body.len()];
    wire::encode(&header(Kind::DescriptorPage), &body, &mut result).unwrap();
    result
}
fn control() -> Vec<u8> {
    let mut h = header(Kind::Cancel);
    h.cancellation = 1;
    h.credit_serial = 0;
    h.credit_bytes = 0;
    let mut result = vec![0; 216];
    wire::encode(&h, &[0, 1, 0, 0, 0, 0, 0, 0], &mut result).unwrap();
    result
}
fn record(id: u128, total: usize, offset: usize) -> ServiceRecordHeader {
    let n = (total - offset).min(1024);
    ServiceRecordHeader {
        transcript: D([9; 48]),
        message_id: id.to_be_bytes(),
        total: total as u32,
        offset: offset as u32,
        chunk: n as u16,
        final_chunk: offset + n == total,
    }
}
#[test]
fn data_continues_after_newer_control_and_no_slice_escapes() {
    let bytes = page(2048);
    let cancel = control();
    let mut data = [0xa5; 3000];
    let mut ctrl = [0xa5; 1024];
    let mut a = Assembly::new(D([9; 48]), &mut data, &mut ctrl).unwrap();
    assert_eq!(
        a.accept(
            record(1, bytes.len(), 0),
            &bytes[..1024],
            |_, _, first| {
                assert!(first);
                Ok(())
            },
            |_| Ok(())
        )
        .unwrap(),
        (Progress::Partial, None)
    );
    assert_eq!(
        a.accept(
            record(2, cancel.len(), 0),
            &cancel,
            |_, _, _| Ok(()),
            |m| Ok(m.header.kind)
        )
        .unwrap(),
        (Progress::Complete, Some(Kind::Cancel))
    );
    assert_eq!(
        a.accept(
            record(1, bytes.len(), 1024),
            &bytes[1024..2048],
            |_, _, first| {
                assert!(!first);
                Ok(())
            },
            |_| Ok(())
        )
        .unwrap()
        .0,
        Progress::Partial
    );
    assert_eq!(
        a.accept(
            record(1, bytes.len(), 2048),
            &bytes[2048..],
            |_, _, _| Ok(()),
            |m| {
                assert_eq!(&m.body[32..], &bytes[240..]);
                Ok(7)
            }
        )
        .unwrap(),
        (Progress::Complete, Some(7))
    );
    assert!(a
        .accept(
            record(2, cancel.len(), 0),
            &cancel,
            |_, _, _| Ok(()),
            |_| Ok(())
        )
        .is_err());
    drop(a);
    assert!(data.iter().chain(ctrl.iter()).all(|b| *b == 0));
}
#[test]
fn gaps_conflicting_totals_wrong_transcript_and_second_data_poison() {
    let bytes = page(2048);
    for variant in 0..5 {
        let mut data = [0; 3000];
        let mut ctrl = [0; 1024];
        let mut a = Assembly::new(D([9; 48]), &mut data, &mut ctrl).unwrap();
        a.accept(
            record(1, bytes.len(), 0),
            &bytes[..1024],
            |_, _, _| Ok(()),
            |_| Ok(()),
        )
        .unwrap();
        let mut r = record(1, bytes.len(), 1024);
        let chunk = match variant {
            0 => {
                r = record(1, bytes.len(), 2048);
                &bytes[2048..]
            }
            1 => {
                r.total += 1;
                &bytes[1024..2048]
            }
            2 => {
                r.transcript = D([8; 48]);
                &bytes[1024..2048]
            }
            3 => {
                r = record(2, bytes.len(), 0);
                &bytes[..1024]
            }
            _ => {
                r = record(1, bytes.len(), 0);
                &bytes[..1024]
            }
        };
        assert!(a.accept(r, chunk, |_, _, _| Ok(()), |_| Ok(())).is_err());
        assert_eq!(
            a.accept(
                record(3, bytes.len(), 0),
                &bytes[..1024],
                |_, _, _| Ok(()),
                |_| Ok(())
            ),
            Err(CarrierError::Closed)
        );
        drop(a);
        assert!(data.iter().chain(ctrl.iter()).all(|b| *b == 0));
    }
}
#[test]
fn refusal_and_unwind_erase_partial_and_completed_records() {
    let bytes = page(1100);
    for panic_callback in [false, true] {
        let mut data = [0; 1600];
        let mut ctrl = [0; 1024];
        let mut a = Assembly::new(D([9; 48]), &mut data, &mut ctrl).unwrap();
        a.accept(
            record(1, bytes.len(), 0),
            &bytes[..1024],
            |_, _, _| Ok(()),
            |_| Ok(()),
        )
        .unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            a.accept(
                record(1, bytes.len(), 1024),
                &bytes[1024..],
                |_, _, _| Ok(()),
                |_| -> Result<(), CarrierError> {
                    if panic_callback {
                        panic!("consumer unwind fixture")
                    }
                    Err(CarrierError::Admission)
                },
            )
        }));
        if panic_callback {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().is_err());
        }
        drop(a);
        assert!(data.iter().chain(ctrl.iter()).all(|b| *b == 0));
    }
}
#[test]
fn capacity_and_credit_refuse_before_copy_and_drop_erases() {
    let bytes = page(2048);
    let mut data = [0; 1024];
    let mut ctrl = [0; 1024];
    let mut a = Assembly::new(D([9; 48]), &mut data, &mut ctrl).unwrap();
    let mut checked = false;
    assert_eq!(
        a.accept(
            record(1, bytes.len(), 0),
            &bytes[..1024],
            |_, _, _| {
                checked = true;
                Ok(())
            },
            |_| Ok(())
        ),
        Err(CarrierError::Capacity)
    );
    assert!(!checked);
    drop(a);
    let mut data = [0; 3000];
    let mut a = Assembly::new(D([9; 48]), &mut data, &mut ctrl).unwrap();
    assert_eq!(
        a.accept(
            record(1, bytes.len(), 0),
            &bytes[..1024],
            |_, _, _| Err(CarrierError::Admission),
            |_| Ok(())
        ),
        Err(CarrierError::Admission)
    );
    drop(a);
    assert!(data.iter().chain(ctrl.iter()).all(|b| *b == 0));
}
#[test]
fn send_cursor_retries_exact_bytes_and_ids_do_not_reuse() {
    let bytes = page(1100);
    let cancel = control();
    let mut seq = MessageSequence::new(D([9; 48])).unwrap();
    let mut data = seq.begin(&bytes).unwrap();
    let first = data.current().unwrap();
    assert_eq!(data.current().unwrap(), first);
    let control = seq.begin(&cancel).unwrap();
    assert_eq!(control.current().unwrap().0.message_id, 2u128.to_be_bytes());
    data.drained().unwrap();
    assert_eq!(data.current().unwrap().0.offset, 1024);
    assert_eq!(data.current().unwrap().0.message_id, 1u128.to_be_bytes());
    data.drained().unwrap();
    assert!(data.current().is_none());
    assert_eq!(data.drained(), Err(CarrierError::Closed));
}
#[test]
fn actual_raw_allocator_observes_no_hot_assembly_or_cursor_allocations() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    let bytes = page(1100);
    let mut data = [0; 1600];
    let mut ctrl = [0; 1024];
    let guard = AllocGuard::begin("qbdl-service-carrier", true);
    let mut a = Assembly::new(D([9; 48]), &mut data, &mut ctrl).unwrap();
    let mut seq = MessageSequence::new(D([9; 48])).unwrap();
    let mut send = seq.begin(&bytes).unwrap();
    while let Some((r, b)) = send.current() {
        a.accept(r, b, |_, _, _| Ok(()), |m| Ok(m.body.len()))
            .unwrap();
        send.drained().unwrap();
    }
    assert!(a
        .accept(
            record(1, bytes.len(), 0),
            &bytes[..1024],
            |_, _, _| Ok(()),
            |_| Ok(())
        )
        .is_err());
    drop(a);
    guard.check().unwrap();
}

#[test]
fn maximum_page_has_65_chunks_and_final_240_bytes() {
    let bytes = page(65536);
    let mut data = vec![0; 65776];
    let mut ctrl = [0; 1024];
    let mut a = Assembly::new(D([9; 48]), &mut data, &mut ctrl).unwrap();
    let mut seq = MessageSequence::new(D([9; 48])).unwrap();
    let mut send = seq.begin(&bytes).unwrap();
    let mut count = 0;
    while let Some((r, b)) = send.current() {
        count += 1;
        if r.final_chunk {
            assert_eq!(r.offset, 65536);
            assert_eq!(b.len(), 240);
        }
        a.accept(
            r,
            b,
            |_, total, _| {
                assert_eq!(total, 65776);
                Ok(())
            },
            |m| {
                assert_eq!(m.body.len(), 65568);
                Ok(())
            },
        )
        .unwrap();
        send.drained().unwrap();
    }
    assert_eq!(count, 65);
}
#[test]
fn check_panic_and_outer_final_authority_refusal_erase_and_poison() {
    let bytes = page(1100);
    for panic_check in [false, true] {
        let mut data = [0; 1600];
        let mut ctrl = [0; 1024];
        let mut a = Assembly::new(D([9; 48]), &mut data, &mut ctrl).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            a.with_attempt(|inner| {
                inner.accept(
                    record(1, bytes.len(), 0),
                    &bytes[..1024],
                    |_, _, _| {
                        if panic_check {
                            panic!("admission unwind")
                        }
                        Ok(())
                    },
                    |_| Ok(()),
                )?;
                Err::<(), _>(CarrierError::Admission) // models actual enclosing final guard refusal
            })
        }));
        if panic_check {
            assert!(result.is_err());
        } else {
            assert_eq!(result.unwrap(), Err(CarrierError::Admission));
        }
        assert_eq!(
            a.accept(
                record(2, bytes.len(), 0),
                &bytes[..1024],
                |_, _, _| Ok(()),
                |_| Ok(())
            ),
            Err(CarrierError::Closed)
        );
        drop(a);
        assert!(data.iter().chain(ctrl.iter()).all(|b| *b == 0));
    }
}
