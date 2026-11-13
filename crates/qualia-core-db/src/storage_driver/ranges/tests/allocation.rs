use super::*;
use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;

fn exercise(session: &mut RangeSession<'_, '_>) {
    for file in [RangeFile::Payload, RangeFile::Proof, RangeFile::Checkpoint] {
        session.write_exact_at(file, 0, &[1, 2]).unwrap();
        let mut actual = [0; 2];
        session.read_exact_at(file, 0, &mut actual).unwrap();
        assert_eq!(actual, [1, 2]);
        assert_eq!(
            session.sync_file(file).unwrap(),
            FileBarrier::SyncAllReturned
        );
        let cap = session.limits().ceiling(file);
        assert_eq!(
            session.write_exact_at(file, cap, &[0]),
            Err(RangeError::Bounds)
        );
        assert_eq!(
            session.read_exact_at(file, cap, &mut [0]),
            Err(RangeError::Bounds)
        );
        assert_eq!(
            session.write_exact_at(file, u64::MAX, &[0]),
            Err(RangeError::Overflow)
        );
        assert_eq!(
            session.read_exact_at(file, u64::MAX, &mut [0]),
            Err(RangeError::Overflow)
        );
        assert_eq!(
            session.read_exact_at(file, cap - 1, &mut [0]),
            Err(RangeError::Io(std::io::ErrorKind::UnexpectedEof))
        );
    }
}

#[test]
fn actual_hot_io_has_zero_raw_allocation_calls_with_transient_positive_control() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = new_quota::<1>(root.path(), 28);
    let mut session = driver
        .create_range_staging(quota.admit_staging([2; 32], 28).unwrap(), limits())
        .unwrap();
    exercise(&mut session); // Warm native/lazy state outside measurement.

    let positive = AllocGuard::begin("range-counter-positive", true);
    let mut allocated = Vec::<u8>::with_capacity(64);
    std::hint::black_box(&mut allocated);
    drop(allocated); // Zero net allocations must still fail the raw counter.
    assert!(positive.check().is_err());

    let guard = AllocGuard::begin("range-hot-real-io", true);
    exercise(&mut session);
    guard
        .check()
        .expect("real bounded I/O must perform zero raw allocation calls");
    session.close().unwrap(); // Creation and cleanup are cold, outside measure.
}
