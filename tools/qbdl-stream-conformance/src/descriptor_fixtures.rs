use crate::allocation::measured;
use crate::bundle::stream::{descriptor as d, index as ix, proof};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
use serde_json::Value;

fn hex(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0, "odd-length hex");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}
fn digest_hex(s: &str) -> StrongDigest {
    StrongDigest::from_bytes(&hex(s)).unwrap()
}
fn u(v: &Value) -> u64 {
    v.as_u64().expect("fixture unsigned integer")
}
fn as_bytes(v: &Value, key: &str) -> Vec<u8> {
    hex(v[key].as_str().expect("fixture hex"))
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/qasd-plain-v1.json")).unwrap()
}

fn source_limits(index_len: usize) -> ix::IndexReadLimits {
    ix::IndexReadLimits {
        index_bytes: index_len,
        ..ix::IndexReadLimits::default()
    }
}

#[test]
fn independent_qasd_bytes_match_production_and_inventory_completes_without_allocations() {
    let corpus = fixture();
    let fixtures = corpus["fixtures"].as_array().unwrap();
    assert_eq!(fixtures.len(), 4);
    let mut accepted = 0usize;

    for f in fixtures {
        let source_obj = &f["qbdl_source"];
        let source_bytes = as_bytes(source_obj, "complete_source_hex");
        let raw_header = as_bytes(source_obj, "header_hex");
        let index_bytes = as_bytes(source_obj, "index_hex");
        let source_len = u(source_obj.get("source_length").unwrap());
        assert_eq!(source_bytes.len() as u64, source_len);
        assert_eq!(raw_header.len(), 64);
        assert_eq!(&source_bytes[..64], &raw_header);
        assert_eq!(
            u(source_obj.get("index_offset").unwrap()) as usize + index_bytes.len(),
            source_bytes.len()
        );
        assert_eq!(
            &source_bytes[u(source_obj.get("index_offset").unwrap()) as usize..],
            &index_bytes
        );
        assert_eq!(
            sha384(&raw_header).as_bytes().as_slice(),
            hex(source_obj["header_sha384_hex"].as_str().unwrap()).as_slice()
        );
        assert_eq!(
            sha384(&index_bytes).as_bytes().as_slice(),
            hex(source_obj["index_sha384_hex"].as_str().unwrap()).as_slice()
        );
        assert_eq!(
            sha384(&source_bytes).as_bytes().as_slice(),
            hex(source_obj["complete_source_sha384_hex"].as_str().unwrap()).as_slice()
        );
        assert_eq!(
            &raw_header[36..40],
            hex(source_obj["crc32c_le_hex"].as_str().unwrap()).as_slice()
        );

        let entries = source_obj["entries"].as_array().unwrap();
        let count = entries.len();
        let index_len = index_bytes.len();
        let limits = source_limits(index_len);
        let checked =
            ix::admit_header(&raw_header, source_len, ix::ExtentLimit::default(), limits).unwrap();
        let mut scratch = vec![ix::EntryScratch::EMPTY; count];
        let admitted = ix::admit_index(&checked, &index_bytes, limits, &mut scratch).unwrap();
        assert_eq!(admitted.entries().count(), count);
        for (view, item) in admitted.entries().zip(entries) {
            let view = view.unwrap();
            assert_eq!(view.key(), item["key"].as_str().unwrap());
            assert_eq!(view.kind(), item["kind"].as_str().unwrap());
            assert_eq!(view.offset(), u(&item["offset"]));
            assert_eq!(view.length(), u(&item["length"]));
            assert_eq!(
                view.sha256().as_slice(),
                hex(item["sha256_hex"].as_str().unwrap()).as_slice()
            );
        }

        let context_bytes = as_bytes(f, "context_root_hex");
        let body_bytes = as_bytes(f, "descriptor_body_hex");
        let expected_context_id = digest_hex(f["context_id_sha384_hex"].as_str().unwrap());
        let expected_descriptor_id = digest_hex(f["descriptor_id_sha384_hex"].as_str().unwrap());
        let context_view = d::decode_context(&context_bytes).unwrap();
        assert_eq!(context_view.entry_count(), count);
        assert_eq!(context_view.parameters().generation, 0);
        assert_eq!(context_view.parameters().predecessor, None);
        assert_eq!(
            context_view.parameters().nonce.as_slice(),
            hex(f["placeholders"]["nonce_hex"].as_str().unwrap()).as_slice()
        );
        assert_eq!(context_view.source().length, source_len);
        assert_eq!(context_view.source().version, 1);
        assert_eq!(context_view.source().flags, 0);
        assert_eq!(
            context_view.source().index_offset,
            u(&source_obj["index_offset"])
        );
        assert_eq!(context_view.source().index_length, index_len as u64);
        assert_eq!(
            context_view.source().header_digest.as_bytes().as_slice(),
            hex(source_obj["header_sha384_hex"].as_str().unwrap()).as_slice()
        );
        assert_eq!(
            context_view.source().index_digest.as_bytes().as_slice(),
            hex(source_obj["index_sha384_hex"].as_str().unwrap()).as_slice()
        );
        assert_eq!(
            context_view.source().source_digest.as_bytes().as_slice(),
            hex(source_obj["complete_source_sha384_hex"].as_str().unwrap()).as_slice()
        );
        assert_eq!(
            context_view.page_refs().len(),
            f["context_pages"].as_array().unwrap().len()
        );
        assert_eq!(
            d::domain_digest(
                d::DigestDomain::Context,
                &context_bytes,
                &mut [0u8; d::HASH_SCRATCH_BYTES]
            )
            .unwrap(),
            expected_context_id
        );

        let context_pages: Vec<Vec<u8>> = f["context_pages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| hex(p["encoded_hex"].as_str().unwrap()))
            .collect();
        let context_page_values = f["context_pages"].as_array().unwrap();
        let mut page_refs = Vec::with_capacity(context_view.page_refs().len());
        for i in 0..context_view.page_refs().len() {
            let r = context_view.page_refs().get(i).unwrap();
            page_refs.push(r);
            let page_bytes = &context_pages[i];
            assert_eq!(r.number as u64, u(&context_page_values[i]["page_number"]));
            assert_eq!(
                r.first as u64,
                u(&context_page_values[i]["first_index_ordinal"])
            );
            assert_eq!(r.rows as u64, u(&context_page_values[i]["row_count"]));
            assert_eq!(r.encoded_length as usize, page_bytes.len());
            assert_eq!(
                r.digest.as_bytes().as_slice(),
                hex(context_page_values[i]["digest_hex"].as_str().unwrap()).as_slice()
            );
            assert_eq!(
                d::domain_digest(
                    d::DigestDomain::ContextPage,
                    page_bytes,
                    &mut [0u8; d::HASH_SCRATCH_BYTES]
                )
                .unwrap(),
                r.digest
            );
        }
        let context_input = d::ContextInput {
            parameters: context_view.parameters(),
            source: context_view.source(),
            entries: count,
            pages: &page_refs,
        };
        let mut encoded_context = vec![0u8; context_bytes.len()];
        let context_n = d::encode_context_into(&context_input, &mut encoded_context).unwrap();
        assert_eq!(&encoded_context[..context_n], context_bytes);

        let body_view = d::decode_body(&body_bytes).unwrap();
        assert_eq!(body_view.context_id(), expected_context_id);
        assert_eq!(body_view.context().encoded_bytes(), context_bytes);
        assert_eq!(body_view.context().entry_count(), count);
        assert_eq!(
            body_view.page_refs().len(),
            f["entry_pages"].as_array().unwrap().len()
        );
        assert_eq!(
            d::domain_digest(
                d::DigestDomain::Body,
                &body_bytes,
                &mut [0u8; d::HASH_SCRATCH_BYTES]
            )
            .unwrap(),
            expected_descriptor_id
        );
        let body_page_values = f["entry_pages"].as_array().unwrap();
        let mut body_refs = Vec::with_capacity(body_view.page_refs().len());
        for i in 0..body_view.page_refs().len() {
            let r = body_view.page_refs().get(i).unwrap();
            body_refs.push(r);
            assert_eq!(r.number as u64, u(&body_page_values[i]["page_number"]));
            assert_eq!(
                r.first as u64,
                u(&body_page_values[i]["first_index_ordinal"])
            );
            assert_eq!(r.rows as u64, u(&body_page_values[i]["row_count"]));
            assert_eq!(
                r.encoded_length as usize,
                hex(body_page_values[i]["encoded_hex"].as_str().unwrap()).len()
            );
            assert_eq!(
                r.digest.as_bytes().as_slice(),
                hex(body_page_values[i]["digest_hex"].as_str().unwrap()).as_slice()
            );
        }
        let body_input = d::BodyInput {
            context_id: body_view.context_id(),
            context_root: body_view.context().encoded_bytes(),
            generation: 0,
            predecessor: None,
            entries: count,
            pages: &body_refs,
        };
        let mut encoded_body = vec![0u8; body_bytes.len()];
        let body_n = d::encode_body_into(&body_input, &mut encoded_body).unwrap();
        assert_eq!(&encoded_body[..body_n], body_bytes);

        let entry_pages: Vec<Vec<u8>> = body_page_values
            .iter()
            .map(|p| hex(p["encoded_hex"].as_str().unwrap()))
            .collect();
        let context_id = body_view.context_id();
        let tree_values = f["entry_trees"].as_array().unwrap();
        let proof_values: Vec<(u32, u64, Vec<u8>, StrongDigest)> = entries
            .iter()
            .zip(tree_values)
            .map(|(entry, tree)| {
                (
                    u(&entry["entry_id"]) as u32,
                    u(&entry["length"]),
                    hex(entry["opaque_entry_bytes_hex"].as_str().unwrap()),
                    digest_hex(tree["root_hex"].as_str().unwrap()),
                )
            })
            .collect();

        // Repeat the allocation-free codec/hash/proof path after all JSON,
        // decoded bytes, output buffers, and index scratch have been prepared.
        let mut codec_context_out = vec![0u8; context_bytes.len()];
        let mut codec_body_out = vec![0u8; body_bytes.len()];
        let mut codec_page_out = vec![0u8; d::MAX_ENCODED];
        let codec_allocations = measured(|| {
            let cv = d::decode_context(&context_bytes).unwrap();
            assert_eq!(
                d::domain_digest(
                    d::DigestDomain::Context,
                    &context_bytes,
                    &mut [0u8; d::HASH_SCRATCH_BYTES]
                )
                .unwrap(),
                expected_context_id
            );
            let ci = d::ContextInput {
                parameters: cv.parameters(),
                source: cv.source(),
                entries: cv.entry_count(),
                pages: &page_refs,
            };
            let n = d::encode_context_into(&ci, &mut codec_context_out).unwrap();
            assert_eq!(&codec_context_out[..n], context_bytes);
            for (i, page) in context_pages.iter().enumerate() {
                let mut rows = [d::ContextEntry::EMPTY; d::PAGE_ROWS];
                let header = d::decode_context_page(page, &mut rows).unwrap();
                assert_eq!(
                    header.rows,
                    u(&context_page_values[i]["row_count"]) as usize
                );
                assert_eq!(
                    d::domain_digest(
                        d::DigestDomain::ContextPage,
                        page,
                        &mut [0u8; d::HASH_SCRATCH_BYTES]
                    )
                    .unwrap(),
                    page_refs[i].digest
                );
                let n = d::encode_context_page_into(
                    header.number,
                    header.first,
                    &rows[..header.rows],
                    &mut codec_page_out,
                )
                .unwrap();
                assert_eq!(&codec_page_out[..n], page);
            }
            let bv = d::decode_body(&body_bytes).unwrap();
            assert_eq!(
                d::domain_digest(
                    d::DigestDomain::Body,
                    &body_bytes,
                    &mut [0u8; d::HASH_SCRATCH_BYTES]
                )
                .unwrap(),
                expected_descriptor_id
            );
            let bi = d::BodyInput {
                context_id: bv.context_id(),
                context_root: bv.context().encoded_bytes(),
                generation: 0,
                predecessor: None,
                entries: count,
                pages: &body_refs,
            };
            let n = d::encode_body_into(&bi, &mut codec_body_out).unwrap();
            assert_eq!(&codec_body_out[..n], body_bytes);
            for (i, page) in entry_pages.iter().enumerate() {
                let mut rows = [d::EntryRecord::EMPTY; d::PAGE_ROWS];
                let header = d::decode_entry_page(page, &mut rows).unwrap();
                assert_eq!(header.rows, u(&body_page_values[i]["row_count"]) as usize);
                assert_eq!(
                    d::domain_digest(
                        d::DigestDomain::EntryPage,
                        page,
                        &mut [0u8; d::HASH_SCRATCH_BYTES]
                    )
                    .unwrap(),
                    body_refs[i].digest
                );
                let n = d::encode_entry_page_into(
                    header.context,
                    header.number,
                    header.first,
                    &rows[..header.rows],
                    &mut codec_page_out,
                )
                .unwrap();
                assert_eq!(&codec_page_out[..n], page);
            }
            for (id, len, data, expected_root) in &proof_values {
                let spec = proof::TreeSpec::new(
                    context_id,
                    *id,
                    0,
                    proof::Representation::Plaintext,
                    0,
                    *len,
                )
                .unwrap();
                let root = if *len == 0 {
                    proof::empty_root(&spec).unwrap()
                } else {
                    let mut tree = proof::TreeBuilder::new(spec);
                    tree.append(data, |_| Ok(())).unwrap();
                    tree.finish(|_| Ok(())).unwrap()
                };
                assert_eq!(root, *expected_root);
            }
        });
        assert_eq!(
            codec_allocations,
            0,
            "{} codec/hash/proof path allocated",
            f["name"].as_str().unwrap()
        );

        for (entry, tree) in entries.iter().zip(tree_values) {
            let id = u(&entry["entry_id"]) as u32;
            let logical_len = u(&entry["length"]);
            assert_eq!(u(&tree["entry_id"]), id as u64);
            assert_eq!(u(&tree["logical_length"]), logical_len);
            let raw = hex(entry["opaque_entry_bytes_hex"].as_str().unwrap());
            let expected_root = digest_hex(tree["root_hex"].as_str().unwrap());
            let spec = proof::TreeSpec::new(
                context_id,
                id,
                0,
                proof::Representation::Plaintext,
                0,
                logical_len,
            )
            .unwrap();
            let actual_root = if logical_len == 0 {
                proof::empty_root(&spec).unwrap()
            } else {
                let mut builder = proof::TreeBuilder::new(spec);
                for leaf in raw.chunks(proof::LEAF_BYTES as usize) {
                    builder.append(leaf, |_| Ok(())).unwrap();
                }
                builder.finish(|_| Ok(())).unwrap()
            };
            assert_eq!(
                actual_root,
                expected_root,
                "tree root for {}",
                entry["key"].as_str().unwrap()
            );
            if logical_len != 0 {
                proof::verify_leaf(&spec, expected_root, 0, &raw, &[]).unwrap();
            }
        }

        // Complete the inventory from the real admitted QBDL directory. All heap
        // setup (JSON, owned byte vectors, index scratch and context storage) is complete.
        let mut context_storage = vec![d::ContextEntry::EMPTY; count];
        let mut id_scratch = vec![0u32; count];
        let context_page_bytes = &context_pages;
        let entry_page_bytes = &entry_pages;
        let mut candidate = None;
        let allocation_count = measured(|| {
            let mut framing = [0u8; d::HASH_SCRATCH_BYTES];
            let mut verifier = d::InventoryVerifier::new(
                body_view,
                &raw_header,
                &admitted,
                &mut context_storage,
                &mut framing,
                d::MAX_METADATA,
            )
            .unwrap();
            for page in context_page_bytes {
                let mut rows = [d::ContextEntry::EMPTY; d::PAGE_ROWS];
                verifier
                    .accept_context_page(page, &mut rows, &mut framing)
                    .unwrap();
            }
            verifier.seal_contexts(&mut id_scratch).unwrap();
            for page in entry_page_bytes {
                let mut rows = [d::EntryRecord::EMPTY; d::PAGE_ROWS];
                verifier
                    .accept_entry_page(page, &mut rows, &mut framing)
                    .unwrap();
            }
            candidate = Some(verifier.finish().unwrap());
        });
        assert_eq!(
            allocation_count,
            0,
            "{} inventory allocated",
            f["name"].as_str().unwrap()
        );
        let candidate = candidate.unwrap();
        assert_eq!(candidate.context_id(), expected_context_id);
        assert_eq!(candidate.descriptor_id(), expected_descriptor_id);
        assert_eq!(candidate.entry_count() as usize, count);
        accepted += 1;
    }
    assert_eq!(accepted, 4);
}

#[test]
fn descriptor_truncations_and_changed_committed_pages_are_refused() {
    let corpus = fixture();
    for f in corpus["fixtures"].as_array().unwrap() {
        let count = f["qbdl_source"]["entries"].as_array().unwrap().len();
        let context = as_bytes(f, "context_root_hex");
        let body = as_bytes(f, "descriptor_body_hex");
        assert!(d::decode_context(&context[..context.len() - 1]).is_err());
        assert!(d::decode_body(&body[..body.len() - 1]).is_err());
        for page in f["context_pages"].as_array().unwrap() {
            let bytes = hex(page["encoded_hex"].as_str().unwrap());
            assert!(d::decode_context_page(
                &bytes[..bytes.len() - 1],
                &mut [d::ContextEntry::EMPTY; d::PAGE_ROWS]
            )
            .is_err());
        }
        for page in f["entry_pages"].as_array().unwrap() {
            let bytes = hex(page["encoded_hex"].as_str().unwrap());
            assert!(d::decode_entry_page(
                &bytes[..bytes.len() - 1],
                &mut [d::EntryRecord::EMPTY; d::PAGE_ROWS]
            )
            .is_err());
        }
        if count == 0 {
            continue;
        }

        let source = &f["qbdl_source"];
        let header = as_bytes(source, "header_hex");
        let index = as_bytes(source, "index_hex");
        let limits = source_limits(index.len());
        let checked = ix::admit_header(
            &header,
            u(&source["source_length"]),
            ix::ExtentLimit::default(),
            limits,
        )
        .unwrap();
        let mut index_scratch = vec![ix::EntryScratch::EMPTY; count];
        let admitted = ix::admit_index(&checked, &index, limits, &mut index_scratch).unwrap();
        let context_pages: Vec<Vec<u8>> = f["context_pages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| hex(p["encoded_hex"].as_str().unwrap()))
            .collect();
        let entry_pages: Vec<Vec<u8>> = f["entry_pages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| hex(p["encoded_hex"].as_str().unwrap()))
            .collect();
        let mut framing = [0u8; d::HASH_SCRATCH_BYTES];
        let body_view = d::decode_body(&body).unwrap();
        let mut context_storage = vec![d::ContextEntry::EMPTY; count];
        let mut verifier = d::InventoryVerifier::new(
            body_view,
            &header,
            &admitted,
            &mut context_storage,
            &mut framing,
            d::MAX_METADATA,
        )
        .unwrap();

        let mut changed_context = context_pages[0].clone();
        let len_byte = changed_context.len() - 4;
        changed_context[len_byte] = if changed_context[len_byte] == 0 {
            1
        } else {
            changed_context[len_byte] ^ 1
        };
        let mut rows = [d::ContextEntry::EMPTY; d::PAGE_ROWS];
        assert_eq!(
            verifier.accept_context_page(&changed_context, &mut rows, &mut framing),
            Err(d::DescriptorError::Commitment)
        );
        assert!(verifier
            .accept_context_page(&context_pages[0], &mut rows, &mut framing)
            .is_ok());
        for page in context_pages.iter().skip(1) {
            assert!(verifier
                .accept_context_page(page, &mut rows, &mut framing)
                .is_ok());
        }
        verifier.seal_contexts(&mut vec![0u32; count]).unwrap();

        let mut changed_entry = entry_pages[0].clone();
        let final_byte = changed_entry.len() - 1;
        changed_entry[final_byte] ^= 1;
        let mut record_rows = [d::EntryRecord::EMPTY; d::PAGE_ROWS];
        assert_eq!(
            verifier.accept_entry_page(&changed_entry, &mut record_rows, &mut framing),
            Err(d::DescriptorError::Commitment)
        );
        assert!(verifier
            .accept_entry_page(&entry_pages[0], &mut record_rows, &mut framing)
            .is_ok());
        for page in entry_pages.iter().skip(1) {
            assert!(verifier
                .accept_entry_page(page, &mut record_rows, &mut framing)
                .is_ok());
        }
        assert!(verifier.finish().is_ok());

        // Altering the body context identifier preserves CBOR shape but must fail
        // the actual domain-digest/context binding check in InventoryVerifier::new.
        let mut wrong_body = body.clone();
        let original_context_id = body_view.context_id();
        let where_id = wrong_body
            .windows(48)
            .position(|w| w == original_context_id.as_bytes())
            .unwrap();
        wrong_body[where_id] ^= 1;
        let wrong_view = d::decode_body(&wrong_body).unwrap();
        let mut storage = vec![d::ContextEntry::EMPTY; count];
        assert_eq!(
            d::InventoryVerifier::new(
                wrong_view,
                &header,
                &admitted,
                &mut storage,
                &mut framing,
                d::MAX_METADATA
            )
            .err(),
            Some(d::DescriptorError::Context)
        );
    }
}
