use super::*;
#[test]
fn actual_channel_hot_success_and_wouldblock_use_zero_raw_allocations() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    with_pair(|mut p| {
        let mut out = [0; 1024];
        queue(&mut p.i, &p.f.pins_i, 11, b"warm").unwrap();
        send(&mut p.i, &p.f.pins_i).unwrap();
        receive(&mut p.r, &p.f.pins_r, &mut out).unwrap();
        let measure = AllocGuard::begin("actual received-Finished channel", true);
        for id in 12..16 {
            queue(&mut p.i, &p.f.pins_i, id, b"hot").unwrap();
            send(&mut p.i, &p.f.pins_i).unwrap();
            assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Ok(3));
            assert_eq!(&out[..3], b"hot");
        }
        assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Err(E::WouldBlock));
        measure.check().unwrap();
        let positive = AllocGuard::begin("channel positive control", true);
        std::hint::black_box(vec![1u8; 8]);
        assert!(positive.check().is_err());
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}
