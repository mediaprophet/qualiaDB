//! Path dispatch for native Vibe bindings and catalog ids.

use super::Host;
use crate::bind::{call_math, call_quin, call_rdf};
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;

pub fn dispatch<H: Host>(
    host: &mut H,
    path: &str,
    args: &[Value],
    named: &[(String, Value)],
    span: Span,
) -> Result<Value, Diagnostic> {
    if let Some(v) = call_math(path, args, span)? {
        return Ok(v);
    }
    if let Some(v) = call_rdf(path, args, span)? {
        return Ok(v);
    }
    if let Some(v) = call_quin(host, path, named, span)? {
        return Ok(v);
    }
    match path {
        "receipt_empty" => Ok(Value::Receipt),
        "graph.query" => {
            let take = named
                .iter()
                .find(|(k, _)| k == "take")
                .and_then(|(_, v)| v.as_i64())
                .unwrap_or(0) as u64;
            host.graph_query(args, take, span)
        }
        "graph.stage" => {
            let term = args
                .first()
                .ok_or_else(|| Diagnostic::new(DiagCode::E100, span, "graph.stage needs a term"))?;
            host.graph_stage(term, span)
        }
        "graph.commit" => host.graph_commit(span),
        "aura.validate" => {
            let n = args.first().unwrap_or(&Value::Null);
            let s = args.get(1).unwrap_or(&Value::Null);
            host.aura_validate(n, s, span)
        }
        "pulse.publish" => {
            let topic = match args.first() {
                Some(Value::String(t)) => t.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "pulse.publish topic must be a string",
                    ))
                }
            };
            let payload = args.get(1).cloned().unwrap_or(Value::Null);
            host.pulse_publish(&topic, &payload, span)
        }
        "graph.snapshot" => host.graph_snapshot(span),
        "time.unix" => host.time_unix(span),
        "time.unix_nanos" => host.time_unix_nanos(span),
        "time.monotonic_nanos" => host.time_monotonic_nanos(span),
        "time.now" => host.time_now(span),
        "instant.to_unix_secs" => {
            let inst = match args.first() {
                Some(Value::Instant(i)) => i.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "instant.to_unix_secs needs an Instant argument",
                    ))
                }
            };
            Ok(Value::I64(inst.secs))
        }
        "instant.to_unix_nanos" => {
            let inst = match args.first() {
                Some(Value::Instant(i)) => i.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "instant.to_unix_nanos needs an Instant argument",
                    ))
                }
            };
            Ok(Value::U64(
                (inst.secs as u64) * 1_000_000_000 + inst.nanos as u64,
            ))
        }
        "host.version" => Ok(Value::String(host.host_version().into())),
        "time.proper_time" => {
            let worldline_id = args.first().and_then(|v| v.as_i64()).unwrap_or(0) as u64;
            host.time_proper_time(worldline_id, span)
        }
        "receipt.clock" => host.receipt_clock(span),
        "field.sample" => {
            let field_ref = args.first().and_then(|v| v.as_i64()).unwrap_or(0) as u64;
            let pose = args.get(1).unwrap_or(&Value::Null);
            host.field_sample(field_ref, pose, span)
        }
        "law.apply" => {
            let law_ref = args.first().and_then(|v| v.as_i64()).unwrap_or(0) as u64;
            let law_args: &[Value] = match args.get(1) {
                Some(Value::List(xs)) => xs.as_slice(),
                _ => &[],
            };
            host.law_apply(law_ref, law_args, span)
        }
        "capability.resolve" => {
            let id = match args.first() {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Iri(s)) => s.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "capability.resolve needs a string id",
                    ))
                }
            };
            host.capability_resolve(&id, span)
        }
        "capability.invoke" => {
            let raw_id = match args.first() {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.clone(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "capability.invoke needs a string id",
                    ))
                }
            };
            let mut payload = args.get(1).cloned().unwrap_or(Value::Null);
            crate::catalog::apply_preset_alias(&raw_id, &mut payload);
            let id = crate::catalog::canonical_id(&raw_id)
                .map(str::to_string)
                .unwrap_or(raw_id);
            host.capability_invoke(&id, &payload, span)
        }
        "conservation.check" => {
            use crate::value::ConservationQuantity;
            let quantity = match args.first() {
                Some(Value::String(s)) => match s.as_str() {
                    "mass" => ConservationQuantity::Mass,
                    "mole" => ConservationQuantity::Mole,
                    "energy" => ConservationQuantity::Energy,
                    "charge" => ConservationQuantity::Charge,
                    "momentum" => ConservationQuantity::Momentum,
                    "angular_momentum" => ConservationQuantity::AngularMomentum,
                    _ => {
                        return Err(Diagnostic::new(
                            DiagCode::E100,
                            span,
                            format!("unknown conserved quantity '{s}'"),
                        ))
                    }
                },
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "conservation.check needs a quantity name string",
                    ))
                }
            };
            let before = args.get(1).unwrap_or(&Value::Null);
            let after = args.get(2).unwrap_or(&Value::Null);
            let tolerance = named
                .iter()
                .find(|(k, _)| k == "tolerance")
                .and_then(|(_, v)| v.as_f64())
                .unwrap_or(1e-9);
            host.conservation_check(&quantity, before, after, tolerance, span)
        }
        "causal.relation" => {
            let event_a = args.first().unwrap_or(&Value::Null);
            let event_b = args.get(1).unwrap_or(&Value::Null);
            host.causal_relation(event_a, event_b, span)
        }
        "dag.execute" => {
            let pipeline = args.first().unwrap_or(&Value::Null);
            let blackboard = args.get(1).unwrap_or(&Value::Null);
            host.dag_execute(pipeline, blackboard, span)
        }
        "dag.validate" => {
            let pipeline = args.first().unwrap_or(&Value::Null);
            host.dag_validate(pipeline, span)
        }
        "deontic.check" => {
            let capability = match args.first() {
                Some(Value::String(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "deontic.check needs a capability string",
                    ))
                }
            };
            let phase = match args.get(1) {
                Some(Value::String(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "deontic.check needs a phase string",
                    ))
                }
            };
            host.deontic_check(capability, phase, span)
        }
        "hid.poll" => host.hid_poll(span),
        "hid.wait" => {
            let timeout_ns = match args.first() {
                Some(Value::U64(n)) => *n,
                Some(Value::I64(n)) => (*n).max(0) as u64,
                _ => 0,
            };
            host.hid_wait(timeout_ns, span)
        }
        "cue.post" => {
            let cue_id = match args.first() {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "cue.post needs a cue id string",
                    ))
                }
            };
            let payload = args.get(1).unwrap_or(&Value::Null);
            host.cue_post(cue_id, payload, span)
        }
        // â”€â”€ Crypto operations (T-crypto) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        "crypto.sha256" => {
            let data = crate::crypto::extract_string_arg(args, 0, "sha256", span)?;
            host.crypto_hash("SHA-256", &data, span)
        }
        "crypto.sha512" => {
            let data = crate::crypto::extract_string_arg(args, 0, "sha512", span)?;
            host.crypto_hash("SHA-512", &data, span)
        }
        "crypto.blake3" => {
            let data = crate::crypto::extract_string_arg(args, 0, "blake3", span)?;
            host.crypto_hash("BLAKE3", &data, span)
        }
        "crypto.hkdf_sha256" => {
            let ikm = crate::crypto::extract_string_arg(args, 0, "hkdf_sha256", span)?;
            let info = crate::crypto::extract_string_arg(args, 1, "hkdf_sha256", span)?;
            let length = crate::crypto::extract_u64_arg(args, 2, "hkdf_sha256", span)?;
            host.crypto_hkdf(&ikm, &info, length, span)
        }
        "crypto.aead_encrypt" => {
            let algorithm = crate::crypto::extract_string_arg(args, 0, "aead_encrypt", span)?;
            let key_hex = crate::crypto::extract_string_arg(args, 1, "aead_encrypt", span)?;
            let nonce_hex = crate::crypto::extract_string_arg(args, 2, "aead_encrypt", span)?;
            let plaintext = crate::crypto::extract_string_arg(args, 3, "aead_encrypt", span)?;
            let aad = args
                .get(4)
                .and_then(|v| match v {
                    Value::String(s) => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or("");
            host.crypto_aead_encrypt(&algorithm, &key_hex, &nonce_hex, &plaintext, aad, span)
        }
        "crypto.aead_decrypt" => {
            let algorithm = crate::crypto::extract_string_arg(args, 0, "aead_decrypt", span)?;
            let key_hex = crate::crypto::extract_string_arg(args, 1, "aead_decrypt", span)?;
            let nonce_hex = crate::crypto::extract_string_arg(args, 2, "aead_decrypt", span)?;
            let ciphertext_hex = crate::crypto::extract_string_arg(args, 3, "aead_decrypt", span)?;
            let tag_hex = crate::crypto::extract_string_arg(args, 4, "aead_decrypt", span)?;
            let aad = args
                .get(5)
                .and_then(|v| match v {
                    Value::String(s) => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or("");
            host.crypto_aead_decrypt(
                &algorithm,
                &key_hex,
                &nonce_hex,
                &ciphertext_hex,
                &tag_hex,
                aad,
                span,
            )
        }
        "crypto.sign" => {
            let key_id = crate::crypto::extract_string_arg(args, 0, "sign", span)?;
            let data = crate::crypto::extract_string_arg(args, 1, "sign", span)?;
            host.crypto_sign(&key_id, &data, span)
        }
        "crypto.verify" => {
            let key_id = crate::crypto::extract_string_arg(args, 0, "verify", span)?;
            let data = crate::crypto::extract_string_arg(args, 1, "verify", span)?;
            let signature_hex = crate::crypto::extract_string_arg(args, 2, "verify", span)?;
            host.crypto_verify(&key_id, &data, &signature_hex, span)
        }
        "crypto.generate_key" => {
            let algorithm = crate::crypto::extract_string_arg(args, 0, "generate_key", span)?;
            host.crypto_generate_key(&algorithm, span)
        }
        // â”€â”€ ZK proof operations (zk-SNARKs) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        "zk.prove_threshold" => {
            let value = crate::crypto::extract_u64_arg(args, 0, "prove_threshold", span)?;
            let threshold = crate::crypto::extract_u64_arg(args, 1, "prove_threshold", span)?;
            host.zk_prove_threshold(value, threshold, span)
        }
        "zk.verify_threshold" => {
            let proof_hex = crate::crypto::extract_string_arg(args, 0, "verify_threshold", span)?;
            let vk_hex = crate::crypto::extract_string_arg(args, 1, "verify_threshold", span)?;
            let threshold = crate::crypto::extract_u64_arg(args, 2, "verify_threshold", span)?;
            host.zk_verify_threshold(&proof_hex, &vk_hex, threshold, span)
        }
        "zk.prove_range" => {
            let value = crate::crypto::extract_u64_arg(args, 0, "prove_range", span)?;
            let lo = crate::crypto::extract_u64_arg(args, 1, "prove_range", span)?;
            let hi = crate::crypto::extract_u64_arg(args, 2, "prove_range", span)?;
            host.zk_prove_range(value, lo, hi, span)
        }
        "zk.verify_range" => {
            let proof_hex = crate::crypto::extract_string_arg(args, 0, "verify_range", span)?;
            let vk_hex = crate::crypto::extract_string_arg(args, 1, "verify_range", span)?;
            let lo = crate::crypto::extract_u64_arg(args, 2, "verify_range", span)?;
            let hi = crate::crypto::extract_u64_arg(args, 3, "verify_range", span)?;
            host.zk_verify_range(&proof_hex, &vk_hex, lo, hi, span)
        }
        "zk.prove_matmul" => {
            let m = crate::crypto::extract_u64_arg(args, 0, "prove_matmul", span)? as usize;
            let k = crate::crypto::extract_u64_arg(args, 1, "prove_matmul", span)? as usize;
            let n = crate::crypto::extract_u64_arg(args, 2, "prove_matmul", span)? as usize;
            // args[3] = a (List<I64>), args[4] = b (List<I64>)
            let a: Vec<i128> = match args.get(3) {
                Some(Value::List(xs)) => xs
                    .iter()
                    .filter_map(|v| match v {
                        Value::I64(n) => Some(*n as i128),
                        Value::U64(n) => Some(*n as i128),
                        _ => None,
                    })
                    .collect(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "zk.prove_matmul: expected a List at position 3",
                    ))
                }
            };
            let b: Vec<i128> = match args.get(4) {
                Some(Value::List(xs)) => xs
                    .iter()
                    .filter_map(|v| match v {
                        Value::I64(n) => Some(*n as i128),
                        Value::U64(n) => Some(*n as i128),
                        _ => None,
                    })
                    .collect(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "zk.prove_matmul: expected a List at position 4",
                    ))
                }
            };
            host.zk_prove_matmul(m as u64, k as u64, n as u64, &a, &b, span)
        }
        "zk.list_circuits" => host.zk_list_circuits(span),
        // â”€â”€ Cosmic coordinate operations (OCS) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        "cosmic.usri.parse" => {
            let s = match args.first() {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "cosmic.usri.parse needs a USRI string",
                    ))
                }
            };
            match crate::cosmic::usri::Usri::parse(s) {
                Ok(u) => Ok(u.to_value()),
                Err(e) => Err(Diagnostic::new(DiagCode::E100, span, e)),
            }
        }
        "cosmic.geodetic_to_ecef" => {
            let lat = crate::crypto::extract_f64_arg(args, 0, "geodetic_to_ecef", span)?;
            let lon = crate::crypto::extract_f64_arg(args, 1, "geodetic_to_ecef", span)?;
            let alt = crate::crypto::extract_f64_arg(args, 2, "geodetic_to_ecef", span)?;
            let e =
                crate::cosmic::transforms::geodetic_to_ecef(crate::cosmic::transforms::Geodetic {
                    lat_deg: lat,
                    lon_deg: lon,
                    alt_m: alt,
                });
            Ok(crate::cosmic::transforms::ecef_to_value(e))
        }
        "cosmic.ecef_to_geodetic" => {
            let x = crate::crypto::extract_f64_arg(args, 0, "ecef_to_geodetic", span)?;
            let y = crate::crypto::extract_f64_arg(args, 1, "ecef_to_geodetic", span)?;
            let z = crate::crypto::extract_f64_arg(args, 2, "ecef_to_geodetic", span)?;
            let g = crate::cosmic::transforms::ecef_to_geodetic(crate::cosmic::transforms::Ecef {
                x,
                y,
                z,
            });
            Ok(crate::cosmic::transforms::geodetic_to_value(g))
        }
        "cosmic.geodetic_distance" => {
            let lat1 = crate::crypto::extract_f64_arg(args, 0, "geodetic_distance", span)?;
            let lon1 = crate::crypto::extract_f64_arg(args, 1, "geodetic_distance", span)?;
            let lat2 = crate::crypto::extract_f64_arg(args, 2, "geodetic_distance", span)?;
            let lon2 = crate::crypto::extract_f64_arg(args, 3, "geodetic_distance", span)?;
            let d = crate::cosmic::transforms::geodetic_distance(
                crate::cosmic::transforms::Geodetic {
                    lat_deg: lat1,
                    lon_deg: lon1,
                    alt_m: 0.0,
                },
                crate::cosmic::transforms::Geodetic {
                    lat_deg: lat2,
                    lon_deg: lon2,
                    alt_m: 0.0,
                },
            );
            Ok(Value::F64(d))
        }
        "cosmic.stardate_to_year" => {
            let s = crate::crypto::extract_f64_arg(args, 0, "stardate_to_year", span)?;
            let sd = crate::cosmic::stardate::Stardate::new(s);
            Ok(Value::F64(sd.to_gregorian_year()))
        }
        "cosmic.warp_velocity" => {
            let w = crate::crypto::extract_f64_arg(args, 0, "warp_velocity", span)?;
            let scale = match args.get(1) {
                Some(Value::String(s)) if s == "tos" => crate::cosmic::warp::WarpScale::Tos,
                _ => crate::cosmic::warp::WarpScale::Tng,
            };
            Ok(Value::F64(crate::cosmic::warp::warp_velocity(w, scale)))
        }
        "cosmic.body.gravity" => {
            let name = match args.first() {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "cosmic.body.gravity needs a body name",
                    ))
                }
            };
            let profile = match name {
                "earth" => crate::cosmic::celestial::earth_profile(),
                "mars" => crate::cosmic::celestial::mars_profile(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("unknown body: {name}"),
                    ))
                }
            };
            Ok(Value::F64(profile.surface_gravity()))
        }
        "cosmic.body.profile" => {
            let name = match args.first() {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "cosmic.body.profile needs a body name",
                    ))
                }
            };
            match crate::cosmic::celestial::body_profile_by_name(name) {
                Some(p) => Ok(p.to_value()),
                None => Err(Diagnostic::new(
                    DiagCode::E100,
                    span,
                    format!("unknown body: {name}"),
                )),
            }
        }
        "cosmic.flrw.metric" => {
            // Returns the flat present-epoch FLRW metric
            let m = crate::cosmic::flrw::FlrwMetric::flat_present_epoch();
            Ok(m.to_value())
        }
        "cosmic.flrw.redshift" => {
            // redshift(a_emit) â†’ z
            let a = crate::crypto::extract_f64_arg(args, 0, "flrw.redshift", span)?;
            let m = crate::cosmic::flrw::FlrwMetric::flat_present_epoch();
            Ok(Value::F64(m.redshift(a)))
        }
        "cosmic.flrw.hubble_velocity" => {
            let d = crate::crypto::extract_f64_arg(args, 0, "flrw.hubble_velocity", span)?;
            let m = crate::cosmic::flrw::FlrwMetric::flat_present_epoch();
            Ok(Value::F64(m.hubble_velocity(d)))
        }
        "cosmic.flrw.redshift_to_distance" => {
            let z = crate::crypto::extract_f64_arg(args, 0, "flrw.redshift_to_distance", span)?;
            let m = crate::cosmic::flrw::FlrwMetric::flat_present_epoch();
            Ok(Value::F64(m.redshift_to_distance(z)))
        }
        "cosmic.atmosphere.profile" => {
            let name = match args.first() {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "cosmic.atmosphere.profile needs a body name",
                    ))
                }
            };
            let atm = match name {
                "earth" => crate::cosmic::atmosphere::AtmosphericProfile::earth(),
                "mars" => crate::cosmic::atmosphere::AtmosphericProfile::mars(),
                "venus" => crate::cosmic::atmosphere::AtmosphericProfile::venus(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("unknown atmosphere body: {name}"),
                    ))
                }
            };
            Ok(atm.to_value())
        }
        "cosmic.atmosphere.pressure" => {
            let name = match args.first() {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "cosmic.atmosphere.pressure needs a body name",
                    ))
                }
            };
            let alt = crate::crypto::extract_f64_arg(args, 1, "atmosphere.pressure", span)?;
            let atm = match name {
                "earth" => crate::cosmic::atmosphere::AtmosphericProfile::earth(),
                "mars" => crate::cosmic::atmosphere::AtmosphericProfile::mars(),
                "venus" => crate::cosmic::atmosphere::AtmosphericProfile::venus(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("unknown atmosphere body: {name}"),
                    ))
                }
            };
            Ok(Value::F64(atm.pressure_at_altitude(alt)))
        }
        "cosmic.magnetosphere.profile" => {
            let name = match args.first() {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "cosmic.magnetosphere.profile needs a body name",
                    ))
                }
            };
            let mag = match name {
                "earth" => crate::cosmic::atmosphere::MagnetosphereProfile::earth(),
                "jupiter" => crate::cosmic::atmosphere::MagnetosphereProfile::jupiter(),
                "mars" => crate::cosmic::atmosphere::MagnetosphereProfile::mars(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("unknown magnetosphere body: {name}"),
                    ))
                }
            };
            Ok(mag.to_value())
        }
        "cosmic.microverse.particle" => {
            let name = match args.first() {
                Some(Value::String(s)) | Some(Value::Iri(s)) => s.as_str(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        "cosmic.microverse.particle needs a particle name",
                    ))
                }
            };
            let p = match name {
                "electron" => crate::cosmic::microverse::ParticleProfile::electron(),
                "proton" => crate::cosmic::microverse::ParticleProfile::proton(),
                "neutron" => crate::cosmic::microverse::ParticleProfile::neutron(),
                "photon" => crate::cosmic::microverse::ParticleProfile::photon(),
                "up-quark" => crate::cosmic::microverse::ParticleProfile::up_quark(),
                "higgs" => crate::cosmic::microverse::ParticleProfile::higgs_boson(),
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("unknown particle: {name}"),
                    ))
                }
            };
            Ok(p.to_value())
        }
        "cosmic.microverse.scale" => {
            // scale(from_level, to_level, length) â†’ transformed length
            let from = crate::crypto::extract_f64_arg(args, 0, "microverse.scale", span)?;
            let to = crate::crypto::extract_f64_arg(args, 1, "microverse.scale", span)?;
            let len = crate::crypto::extract_f64_arg(args, 2, "microverse.scale", span)?;
            let from_level = crate::cosmic::cb_usri::HierarchyLevel::from_u8(from as u8)
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("invalid hierarchy level: {from}"),
                    )
                })?;
            let to_level =
                crate::cosmic::cb_usri::HierarchyLevel::from_u8(to as u8).ok_or_else(|| {
                    Diagnostic::new(
                        DiagCode::E100,
                        span,
                        format!("invalid hierarchy level: {to}"),
                    )
                })?;
            let lens = crate::cosmic::microverse::ScalingLens::between(from_level, to_level);
            Ok(Value::F64(lens.transform_length(len)))
        }
        _ => {
            if let Some(id) = crate::catalog::canonical_id(path) {
                let mut payload = crate::catalog::payload_from_args(args, named);
                crate::catalog::apply_preset_alias(path, &mut payload);
                return host.capability_invoke(id, &payload, span);
            }
            if crate::catalog::looks_like_catalog_path(path) {
                let msg = match crate::catalog::did_you_mean(path) {
                    Some(s) => format!("unknown capability `{path}`; did you mean `{s}`?"),
                    None => format!("unknown capability `{path}`"),
                };
                return Err(Diagnostic::new(DiagCode::E100, span, msg));
            }
            Err(Diagnostic::new(
                DiagCode::E100,
                span,
                format!("unknown binding {path}"),
            ))
        }
    }
}
