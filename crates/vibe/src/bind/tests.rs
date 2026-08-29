use super::*;
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::{Instant, Value};

struct NoClockHost;

impl Host for NoClockHost {
    fn graph_query(
        &mut self,
        _args: &[Value],
        _take: u64,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_stage(&mut self, _term: &Value, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_commit(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn aura_validate(
        &mut self,
        _node: &Value,
        _shape: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Bool(true))
    }
    fn pulse_publish(
        &mut self,
        _topic: &str,
        _payload: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
}

#[test]
fn local_host_reports_native_scalar() {
    let host = LocalHost::default();
    assert_eq!(host.environment(), HostEnvironment::NativeDesktop);
    assert_eq!(host.acceleration_tier(), AccelerationTier::ScalarCpu);
    assert_eq!(
        host.available_acceleration(),
        crate::detect_available_tier()
    );
}

#[test]
fn local_host_time_returns_some() {
    let mut host = LocalHost::default();
    let res = host.time_unix(Span::point(0));
    assert_eq!(res.unwrap(), Value::I64(1_000_000_000));
}

#[test]
fn no_clock_host_time_returns_diagnostic() {
    let mut host = NoClockHost;
    let res = host.time_unix(Span::point(0));
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, DiagCode::E702);
}

#[test]
fn time_unix_nanos_none_when_no_clock() {
    let mut host = NoClockHost;
    let res = host.time_unix_nanos(Span::point(0));
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, DiagCode::E702);
}

#[test]
fn time_unix_nanos_returns_structured() {
    let mut host = LocalHost::default();
    let val = host.time_unix_nanos(Span::point(0)).unwrap();
    match val {
        Value::Record(r) => {
            assert_eq!(r.get("secs"), Some(&Value::I64(1_000_000_000)));
            assert_eq!(r.get("nanos"), Some(&Value::U64(500_000)));
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn time_monotonic_nanos_default_zero() {
    let mut host = LocalHost::default();
    assert_eq!(
        host.time_monotonic_nanos(Span::point(0)).unwrap(),
        Value::U64(0)
    );
}

#[test]
fn time_monotonic_nanos_custom() {
    let mut host = LocalHost {
        monotonic_time: 42_000_000,
        ..Default::default()
    };
    assert_eq!(
        host.time_monotonic_nanos(Span::point(0)).unwrap(),
        Value::U64(42_000_000)
    );
}

#[test]
fn default_host_version_is_0_1() {
    let host = LocalHost::default();
    assert_eq!(host.host_version(), "vibe-host-0.1");
}

// Ã¢â€â‚¬Ã¢â€â‚¬ X6: time.now Ã¢â€ â€™ Instant tests Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

#[test]
fn time_now_returns_instant() {
    let mut host = LocalHost::default();
    let res = host.time_now(Span::point(0)).unwrap();
    match res {
        Value::Instant(i) => {
            assert_eq!(i.scale, crate::value::TimeScale::Unix);
            assert_eq!(i.secs, 1_000_000_000);
            assert_eq!(i.nanos, 500_000_000);
        }
        other => panic!("expected Instant, got {other:?}"),
    }
}

#[test]
fn time_now_no_clock_fails_closed() {
    let mut host = NoClockHost;
    let res = host.time_now(Span::point(0));
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, DiagCode::E702);
}

#[test]
fn time_now_dispatches_via_dispatch_table() {
    let mut host = LocalHost::default();
    let res = dispatch(&mut host, "time.now", &[], &[], Span::point(0)).unwrap();
    match res {
        Value::Instant(i) => assert_eq!(i.secs, 1_000_000_000),
        other => panic!("expected Instant, got {other:?}"),
    }
}

#[test]
fn instant_to_unix_secs_dispatches() {
    let mut host = LocalHost::default();
    let inst = Value::Instant(Instant::unix(1234567890, 42));
    let res = dispatch(
        &mut host,
        "instant.to_unix_secs",
        &[inst],
        &[],
        Span::point(0),
    )
    .unwrap();
    assert_eq!(res, Value::I64(1234567890));
}

#[test]
fn instant_to_unix_secs_rejects_non_instant() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "instant.to_unix_secs",
        &[Value::I64(42)],
        &[],
        Span::point(0),
    );
    assert!(res.is_err());
}

#[test]
fn instant_to_unix_nanos_dispatches() {
    let mut host = LocalHost::default();
    let inst = Value::Instant(Instant::unix(1, 500_000_000));
    let res = dispatch(
        &mut host,
        "instant.to_unix_nanos",
        &[inst],
        &[],
        Span::point(0),
    )
    .unwrap();
    assert_eq!(res, Value::U64(1_500_000_000));
}

#[test]
fn host_version_dispatch() {
    let mut host = LocalHost::default();
    let v = dispatch(&mut host, "host.version", &[], &[], Span::point(0)).unwrap();
    assert_eq!(v, Value::String("vibe-host-0.1".into()));
}

struct CustomVersionHost;
impl Host for CustomVersionHost {
    fn graph_query(
        &mut self,
        _args: &[Value],
        _take: u64,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_stage(&mut self, _term: &Value, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_commit(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn aura_validate(
        &mut self,
        _node: &Value,
        _shape: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Bool(true))
    }
    fn pulse_publish(
        &mut self,
        _topic: &str,
        _payload: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn host_version(&self) -> &str {
        "vibe-host-0.2"
    }
}

#[test]
fn custom_host_version() {
    let host = CustomVersionHost;
    assert_eq!(host.host_version(), "vibe-host-0.2");
}

#[test]
fn proper_time_default_e702() {
    let mut host = LocalHost::default();
    let res = host.time_proper_time(42, Span::point(0));
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, DiagCode::E702);
}

#[test]
fn proper_time_dispatch_default_e702() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "time.proper_time",
        &[Value::U64(42)],
        &[],
        Span::point(0),
    );
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, DiagCode::E702);
}

struct ProperTimeHost;
impl Host for ProperTimeHost {
    fn graph_query(
        &mut self,
        _args: &[Value],
        _take: u64,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_stage(&mut self, _term: &Value, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_commit(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn aura_validate(
        &mut self,
        _node: &Value,
        _shape: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Bool(true))
    }
    fn pulse_publish(
        &mut self,
        _topic: &str,
        _payload: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn time_proper_time(&mut self, worldline_id: u64, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::F64(worldline_id as f64 * 0.001))
    }
}

#[test]
fn proper_time_custom_value() {
    let mut host = ProperTimeHost;
    let res = host.time_proper_time(100, Span::point(0)).unwrap();
    assert_eq!(res, Value::F64(0.1));
}

#[test]
fn receipt_clock_default_e702() {
    let mut host = LocalHost::default();
    let res = host.receipt_clock(Span::point(0));
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, DiagCode::E702);
}

#[test]
fn receipt_clock_dispatch_default_e702() {
    let mut host = LocalHost::default();
    let res = dispatch(&mut host, "receipt.clock", &[], &[], Span::point(0));
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, DiagCode::E702);
}

struct ReceiptClockHost;
impl Host for ReceiptClockHost {
    fn graph_query(
        &mut self,
        _args: &[Value],
        _take: u64,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_stage(&mut self, _term: &Value, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_commit(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn aura_validate(
        &mut self,
        _node: &Value,
        _shape: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Bool(true))
    }
    fn pulse_publish(
        &mut self,
        _topic: &str,
        _payload: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn receipt_clock(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        let mut rec = std::collections::BTreeMap::new();
        rec.insert("secs".into(), Value::I64(1_000_000_000));
        rec.insert("nanos".into(), Value::U64(42_000));
        Ok(Value::Record(rec))
    }
}

#[test]
fn receipt_clock_custom() {
    let mut host = ReceiptClockHost;
    let val = host.receipt_clock(Span::point(0)).unwrap();
    match val {
        Value::Record(r) => {
            assert_eq!(r.get("secs"), Some(&Value::I64(1_000_000_000)));
            assert_eq!(r.get("nanos"), Some(&Value::U64(42_000)));
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn field_sample_default_e702() {
    let mut host = LocalHost::default();
    let res = host.field_sample(1, &Value::Null, Span::point(0));
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, DiagCode::E702);
}

#[test]
fn law_apply_default_e702() {
    let mut host = LocalHost::default();
    let res = host.law_apply(1, &[], Span::point(0));
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().code, DiagCode::E702);
}

struct FieldLawHost;
impl Host for FieldLawHost {
    fn graph_query(
        &mut self,
        _args: &[Value],
        _take: u64,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_stage(&mut self, _term: &Value, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn graph_commit(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn aura_validate(
        &mut self,
        _node: &Value,
        _shape: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Bool(true))
    }
    fn pulse_publish(
        &mut self,
        _topic: &str,
        _payload: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Null)
    }
    fn field_sample(
        &mut self,
        field_ref: u64,
        _pose: &Value,
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        Ok(Value::Quantity(crate::value::Quantity::new(
            field_ref as f64,
            "qudt:Meter",
        )))
    }
    fn law_apply(
        &mut self,
        law_ref: u64,
        _args: &[Value],
        _span: Span,
    ) -> Result<Value, Diagnostic> {
        let mut rec = std::collections::BTreeMap::new();
        rec.insert("law_id".into(), Value::U64(law_ref));
        Ok(Value::Record(rec))
    }
}

#[test]
fn field_sample_custom() {
    let mut host = FieldLawHost;
    let res = host.field_sample(42, &Value::Null, Span::point(0)).unwrap();
    match res {
        Value::Quantity(q) => assert_eq!(q.value, 42.0),
        other => panic!("expected quantity, got {other:?}"),
    }
}

#[test]
fn law_apply_custom() {
    let mut host = FieldLawHost;
    let res = host.law_apply(7, &[], Span::point(0)).unwrap();
    match res {
        Value::Record(r) => assert_eq!(r.get("law_id"), Some(&Value::U64(7))),
        other => panic!("expected record, got {other:?}"),
    }
}

// Ã¢â€â‚¬Ã¢â€â‚¬ Cosmic coordinate binding tests (OCS) Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬Ã¢â€â‚¬

#[test]
fn cosmic_usri_parse() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.usri.parse",
        &[Value::String(
            "urn:omni:v1:physical:observable:standard:earth:wgs84".into(),
        )],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::Record(r) => {
            assert_eq!(
                r.get("realm_class"),
                Some(&Value::String("physical".into()))
            );
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn cosmic_geodetic_to_ecef() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.geodetic_to_ecef",
        &[Value::F64(0.0), Value::F64(0.0), Value::F64(0.0)],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::Record(r) => {
            // At (0,0,0) geodetic, x should be ~WGS84_A
            if let Some(Value::F64(x)) = r.get("x") {
                assert!((*x - 6_378_137.0).abs() < 1.0);
            } else {
                panic!("expected x coordinate");
            }
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn cosmic_geodetic_distance() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.geodetic_distance",
        &[
            Value::F64(37.7749),
            Value::F64(-122.4194),
            Value::F64(34.0522),
            Value::F64(-118.2437),
        ],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::F64(d) => {
            // SF to LA: ~559 km
            assert!(d > 500_000.0 && d < 600_000.0, "got {} expected ~559km", d);
        }
        other => panic!("expected f64, got {other:?}"),
    }
}

#[test]
fn cosmic_stardate_to_year() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.stardate_to_year",
        &[Value::F64(47634.44)],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::F64(year) => {
            assert!((year - 2370.63444).abs() < 0.01);
        }
        other => panic!("expected f64, got {other:?}"),
    }
}

#[test]
fn cosmic_warp_velocity() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.warp_velocity",
        &[Value::F64(1.0), Value::String("tos".into())],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::F64(v) => {
            // Warp 1 = c
            assert!((v - 299_792_458.0).abs() < 1.0);
        }
        other => panic!("expected f64, got {other:?}"),
    }
}

#[test]
fn cosmic_body_gravity_earth() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.body.gravity",
        &[Value::String("earth".into())],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::F64(g) => {
            assert!((g - 9.81).abs() < 0.1, "got {} expected ~9.81", g);
        }
        other => panic!("expected f64, got {other:?}"),
    }
}

#[test]
fn cosmic_body_gravity_unknown() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.body.gravity",
        &[Value::String("pluto".into())],
        &[],
        Span::point(0),
    );
    assert!(res.is_err());
}

#[test]
fn cosmic_body_profile_jupiter() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.body.profile",
        &[Value::String("jupiter".into())],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::Record(r) => {
            assert_eq!(r.get("name"), Some(&Value::String("Jupiter".into())));
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn cosmic_body_profile_sun() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.body.profile",
        &[Value::String("sun".into())],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::Record(r) => {
            assert_eq!(r.get("name"), Some(&Value::String("Sun".into())));
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn cosmic_body_profile_unknown() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.body.profile",
        &[Value::String("wormhole".into())],
        &[],
        Span::point(0),
    );
    assert!(res.is_err());
}

#[test]
fn cosmic_flrw_metric() {
    let mut host = LocalHost::default();
    let res = dispatch(&mut host, "cosmic.flrw.metric", &[], &[], Span::point(0));
    assert!(res.is_ok());
    match res.unwrap() {
        Value::Record(r) => {
            assert!(r.contains_key("scale_factor"));
            assert!(r.contains_key("hubble_param_km_s_mpc"));
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn cosmic_flrw_redshift() {
    let mut host = LocalHost::default();
    // a_emit = 0.5 Ã¢â€ â€™ z = 1.0
    let res = dispatch(
        &mut host,
        "cosmic.flrw.redshift",
        &[Value::F64(0.5)],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::F64(z) => assert!((z - 1.0).abs() < 1e-10),
        other => panic!("expected f64, got {other:?}"),
    }
}

#[test]
fn cosmic_flrw_hubble_velocity() {
    let mut host = LocalHost::default();
    // 1 Mpc Ã¢â€ â€™ ~67 km/s
    let res = dispatch(
        &mut host,
        "cosmic.flrw.hubble_velocity",
        &[Value::F64(3.085677581e22)],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::F64(v) => assert!((v / 1000.0 - 67.4).abs() < 1.0),
        other => panic!("expected f64, got {other:?}"),
    }
}

#[test]
fn cosmic_atmosphere_profile_earth() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.atmosphere.profile",
        &[Value::String("earth".into())],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::Record(r) => {
            assert_eq!(r.get("body_name"), Some(&Value::String("Earth".into())));
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn cosmic_atmosphere_pressure() {
    let mut host = LocalHost::default();
    // Earth at sea level Ã¢â€ â€™ 101325 Pa
    let res = dispatch(
        &mut host,
        "cosmic.atmosphere.pressure",
        &[Value::String("earth".into()), Value::F64(0.0)],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::F64(p) => assert!((p - 101_325.0).abs() < 1.0),
        other => panic!("expected f64, got {other:?}"),
    }
}

#[test]
fn cosmic_magnetosphere_profile_earth() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.magnetosphere.profile",
        &[Value::String("earth".into())],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::Record(r) => {
            assert!(r.contains_key("surface_field_t"));
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn cosmic_microverse_particle_electron() {
    let mut host = LocalHost::default();
    let res = dispatch(
        &mut host,
        "cosmic.microverse.particle",
        &[Value::String("electron".into())],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::Record(r) => {
            assert_eq!(r.get("name"), Some(&Value::String("Electron".into())));
        }
        other => panic!("expected record, got {other:?}"),
    }
}

#[test]
fn cosmic_microverse_scale() {
    let mut host = LocalHost::default();
    // L5 (1km) Ã¢â€ â€™ L2 (Bohr radius): transform 1.0 km
    // L5 = 5, L2 = 2
    let res = dispatch(
        &mut host,
        "cosmic.microverse.scale",
        &[Value::F64(5.0), Value::F64(2.0), Value::F64(1.0)],
        &[],
        Span::point(0),
    );
    assert!(res.is_ok());
    match res.unwrap() {
        Value::F64(v) => assert!(v > 0.0 && v.is_finite()),
        other => panic!("expected f64, got {other:?}"),
    }
}
