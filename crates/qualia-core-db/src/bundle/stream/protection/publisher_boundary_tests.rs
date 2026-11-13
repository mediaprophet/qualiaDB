use super::*;
#[test]
fn opaque_ids_preserve_original_slots_and_bound_entropy_retries() {
    fixture(false, |p, _, _, _| {
        let source = &p.geometry.source;
        let n = source.objects().len();
        let mut maps = [Mapping::EMPTY; 7];
        let maps = &mut maps[..n];
        let mut draw = 0;
        assign_fixture(source, maps, |out| {
            draw += 1;
            out.copy_from_slice(&((n + 1 - draw) as u32).to_be_bytes());
            Ok(())
        })
        .unwrap();
        assert_eq!(draw, n);
        for (i, m) in maps.iter().enumerate() {
            assert_eq!(m.id, (i + 1) as u32);
            assert_eq!(m.source_slot, n - 1 - i);
            assert_eq!(
                m.digest,
                sha384(source.object_bytes(m.source_slot).unwrap())
            );
        }
        for collision in [false, true] {
            let mut calls = 0;
            let result = assign_fixture(source, maps, |out| {
                calls += 1;
                out.copy_from_slice(&u32::from(collision).to_be_bytes());
                Ok(())
            });
            assert_eq!(result, Err(PublisherError::Entropy));
            assert_eq!(calls, if collision { 17 } else { 16 });
        }
        assert_eq!(
            assign_fixture(source, maps, |_| Err(PublisherError::Entropy)),
            Err(PublisherError::Entropy)
        );
    });
}
struct ExpiringSink<'a> {
    time: &'a Cell<u64>,
    calls: usize,
}
impl CandidateSink for ExpiringSink<'_> {
    fn artifact(&mut self, _: Artifact, _: u16, _: &[u8]) -> Result<(), ()> {
        Ok(())
    }
    fn outboard(&mut self, _: &TreeSpec, _: OutboardNode) -> Result<(), ()> {
        Ok(())
    }
    fn ciphertext(&mut self, _: ObjectScope, _: u64, _: &[u8]) -> Result<(), ()> {
        self.calls += 1;
        self.time.set(u64::MAX);
        Ok(())
    }
}
#[test]
fn expiry_inside_effect_cannot_finish_or_reuse_publication() {
    fixture(false, |p, _, time, _| {
        p.stage_header(&mut Sink::default()).unwrap();
        let mut sink = ExpiringSink { time, calls: 0 };
        let mut leaf = [99; 16400];
        while !p.is_poisoned() {
            if p.seal_next_object(&mut leaf, &mut sink).is_err() {
                break;
            }
        }
        assert_eq!(sink.calls, 1);
        assert!(p.is_poisoned());
        assert!(leaf.iter().all(|v| *v == 0));
        assert!(p.publication.metadata_master_row(0, &mut [0; 32]).is_err());
        assert!(p.seal_next_object(&mut leaf, &mut sink).is_err());
        assert_eq!(sink.calls, 1);
    });
}
#[test]
fn noncanonical_empty_root_refuses_descriptor_after_payload_completion() {
    fixture(false, |p, _, _, _| {
        let mut sink = Sink::default();
        payloads(p, &mut sink);
        let empty = p
            .geometry
            .mappings
            .iter()
            .position(|m| p.geometry.source.objects()[m.source_slot].length() == 0)
            .unwrap();
        p.roots[empty].root = D([1; 48]);
        let mut m = vec![0; p.geometry.metadata_length()];
        p.seal_metadata(refs(), &mut m, &mut [0; 16400], &mut sink)
            .unwrap();
        assert!(matches!(
            p.finish(&mut [0; MAX_DESCRIPTOR], &mut sink),
            Err(PublisherError::Proof)
        ));
        assert!(p.is_poisoned());
        assert!(
            !sink
                .artifacts
                .iter()
                .any(|(k, _)| *k == Artifact::Descriptor)
        );
    });
}
#[test]
fn exact_candidate_digest_audience_policy_and_generation_are_required_to_sign() {
    for mode in 0..6 {
        fixture(true, |p, control, _, public| {
            let public = public.unwrap();
            let mut sink = Sink::default();
            let mut bytes = complete(p, &mut sink);
            let mut digest = sha384(&bytes);
            let mut audience = p.context.fields().scope_token;
            if mode == 0 {
                digest.0[0] ^= 1;
            }
            if mode == 1 {
                audience[0] ^= 1;
            }
            if mode == 2 {
                control.advance_generation(2).unwrap();
            }
            if mode == 3 {
                // Valid alternate descriptor with unchanged context but a different metadata root.
                let mut alternate = [0; MAX_DESCRIPTOR];
                let n =
                    encode_descriptor(p.context, D([77; 48]), p.root_refs, &mut alternate).unwrap();
                bytes = alternate[..n].to_vec();
                digest = sha384(&bytes);
            }
            let mut request = PairedSigningRequest {
                ml_key_id: "ml_private",
                ed_key_id: "ed_private",
                ml_policy_id: "ml_private",
                ed_policy_id: "ed_private",
                binding: cose::Binding {
                    descriptor_digest: &digest.0,
                    key_ids: public.ids,
                    controller_epoch: 1,
                    purpose: PUBLISHER_PURPOSE,
                    audience: &audience,
                },
                grant: grant(KeyPurpose::ControllerSign),
                access: access(),
            };
            if mode == 4 {
                request.ml_policy_id = "absent";
            }
            if mode == 5 {
                request.binding.key_ids.ml_dsa_65[0] ^= 1;
            }
            assert!(
                p.sign_candidate(&bytes, request, &mut cose::Workspace::new(), &mut sink)
                    .is_err()
            );
            assert!(p.is_poisoned());
            assert!(p.publication.metadata_master_row(0, &mut [0; 32]).is_err());
            assert!(
                !sink
                    .artifacts
                    .iter()
                    .any(|(k, _)| *k == Artifact::PublisherProof)
            );
        });
    }
}

#[test]
fn expiry_before_sealer_and_metadata_preflight_clear_all_private_scratch() {
    fixture(false, |p, _, time, _| {
        let mut sink = Sink::default();
        p.stage_header(&mut sink).unwrap();
        time.set(u64::MAX);
        let mut leaf = [99; 16400];
        assert!(p.seal_next_object(&mut leaf, &mut sink).is_err());
        assert!(p.is_poisoned());
        assert!(leaf.iter().all(|v| *v == 0));
        assert!(sink.leaves.is_empty());
        assert!(p.publication.metadata_master_row(0, &mut [0; 32]).is_err());
    });
    fixture(false, |p, _, _, _| {
        let mut private = [99; 32];
        let mut leaf = [99; 16400];
        assert!(
            p.seal_metadata(refs(), &mut private, &mut leaf, &mut Sink::default())
                .is_err()
        );
        assert!(private.iter().all(|v| *v == 0));
        assert!(leaf.iter().all(|v| *v == 0));
    });
}
