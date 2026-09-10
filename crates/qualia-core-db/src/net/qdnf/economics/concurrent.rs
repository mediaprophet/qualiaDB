//! Concurrent last-payment and chargeback races (E17.7).
//!
//! Live rails remain [`QdnfError::Unsupported`]. These tests share one
//! [`Obligation`] owner; they do not mint copied [`super::RemainingTarget`]
//! authority.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::classify::ActingClass;
use super::obligation::Obligation;
use super::reserve::reserve_hold;
use super::settle::{finalise, reverse};

fn open(t: u64) -> Obligation {
    Obligation::open(sha384(b"concurrent-ob"), t).unwrap()
}

fn op(tag: u8) -> StrongDigest {
    sha384(&[tag])
}

/// Two last payments against remaining 10: only the first corporate reserve
/// of 10 succeeds; the second is Denied. Cap is conserved.
#[test]
fn concurrent_last_payments_conserve_cap() {
    use std::sync::{Arc, Mutex};
    use std::thread;

    let ob = Arc::new(Mutex::new(open(10)));
    let a = {
        let ob = Arc::clone(&ob);
        thread::spawn(move || reserve_hold(&mut ob.lock().unwrap(), 10, ActingClass::Corporate, op(1)))
    };
    let b = {
        let ob = Arc::clone(&ob);
        thread::spawn(move || reserve_hold(&mut ob.lock().unwrap(), 10, ActingClass::Corporate, op(2)))
    };
    let ra = a.join().expect("thread a");
    let rb = b.join().expect("thread b");
    let mut wins = 0u8;
    for r in [ra, rb] {
        match r {
            Ok(10) => wins = wins.saturating_add(1),
            Err(QdnfError::Denied) => {}
            other => panic!("unexpected reserve: {other:?}"),
        }
    }
    assert_eq!(wins, 1);
    let locked = ob.lock().unwrap();
    assert_eq!(locked.holds_h, 10);
    assert!(locked.holds_h + locked.settled_s + locked.discharged_w <= locked.target_t);
}

#[test]
fn chargeback_after_finalise_does_not_grow_target() {
    let mut ob = open(8);
    reserve_hold(&mut ob, 8, ActingClass::Corporate, op(2)).unwrap();
    finalise(&mut ob, 8, op(2), false).unwrap();
    let t = ob.target_t;
    reverse(&mut ob, 8, op(2)).unwrap();
    assert_eq!(ob.target_t, t);
    assert_eq!(ob.settled_s, 0);
}

#[test]
fn duplicate_callback_is_idempotent_under_lock() {
    let mut ob = open(4);
    reserve_hold(&mut ob, 4, ActingClass::Corporate, op(3)).unwrap();
    finalise(&mut ob, 4, op(3), false).unwrap();
    finalise(&mut ob, 4, op(3), true).unwrap();
    assert_eq!(ob.settled_s, 4);
    assert_eq!(ob.holds_h, 0);
}
