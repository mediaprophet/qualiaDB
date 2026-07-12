use super::*;
use std::collections::HashMap;

    #[test]
    fn test_cryptographic_library_creation() {
        let library = CryptographicLibrary::new();
        assert_eq!(library.list_keys().len(), 0);
    }

    #[test]
    fn test_mldsa_key_generation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let result = library
            .generate_mldsa_key_pair("test_key".to_string(), SecurityLevel::High)
            .unwrap();

        assert_eq!(result.result.0.key_id, "test_key_private");
        assert_eq!(result.result.1.key_id, "test_key_public");
        assert_eq!(result.result.0.key_algorithm, KeyAlgorithm::MLDSA);
        assert_eq!(result.result.1.key_algorithm, KeyAlgorithm::MLDSA);
        assert_eq!(result.result.0.key_type, KeyType::Private);
        assert_eq!(result.result.1.key_type, KeyType::Public);
        assert!(result.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_data_signing() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate key pair
        let _key_pair = library
            .generate_mldsa_key_pair("test_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Sign data
        let data = b"Hello, World!";
        let signature = library.sign_data("test_key_private", data).unwrap();

        // Verify signature. `generate_mldsa_key_pair` stores the keys under
        // `<id>_private` and `<id>_public`; verification takes the public key id
        // (the prior test passed the bare `test_key`, which is not a stored key).
        let is_valid = library
            .verify_signature("test_key_public", &signature.result, data)
            .unwrap();
        assert!(is_valid.result);

        assert_eq!(signature.result.key_id, "test_key_private");
        assert_eq!(signature.result.algorithm, KeyAlgorithm::MLDSA);
        assert_eq!(signature.result.data, data);
        assert!(signature.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_signature_verification() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate key pair
        let _key_pair = library
            .generate_mldsa_key_pair("test_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Sign data
        let data = b"Hello, World!";
        let signature = library.sign_data("test_key_private", data).unwrap();

        // Verify signature
        let is_valid = library
            .verify_signature("test_key_public", &signature.result, data)
            .unwrap();

        assert!(is_valid.result);
        assert!(is_valid.result);
    }

    #[test]
    fn test_data_encryption() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate symmetric key
        let key = Key {
            key_id: "test_key".to_string(),
            key_type: KeyType::Symmetric,
            key_algorithm: KeyAlgorithm::AES,
            key_data: vec![0u8; 32],
            metadata: KeyMetadata {
                key_id: "test_key".to_string(),
                key_type: KeyType::Symmetric,
                key_algorithm: KeyAlgorithm::AES,
                key_size: 32,
                created_at: 0,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: SecurityLevel::High,
                access_level: AccessLevel::Secret,
            },
        };

        library.key_manager.store_key(key).unwrap();

        // Encrypt data
        let data = b"Hello, World!";
        let encrypted_data = library.encrypt_data("test_key", data, None).unwrap();

        assert_eq!(
            encrypted_data.result.algorithm,
            EncryptionAlgorithm::AES256GCM
        );
        assert_eq!(encrypted_data.result.metadata.mode, EncryptionMode::GCM);
        assert!(encrypted_data.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_data_decryption() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate symmetric key
        let key = Key {
            key_id: "test_key".to_string(),
            key_type: KeyType::Symmetric,
            key_algorithm: KeyAlgorithm::AES,
            key_data: vec![0u8; 32],
            metadata: KeyMetadata {
                key_id: "test_key".to_string(),
                key_type: KeyType::Symmetric,
                key_algorithm: KeyAlgorithm::AES,
                key_size: 32,
                created_at: 0,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: SecurityLevel::High,
                access_level: AccessLevel::Secret,
            },
        };

        library.key_manager.store_key(key).unwrap();

        // Encrypt data
        let data = b"Hello, World!";
        let encrypted_data = library.encrypt_data("test_key", data, None).unwrap();

        // Decrypt data
        let decrypted_data = library
            .decrypt_data("test_key", &encrypted_data.result)
            .unwrap();

        assert_eq!(decrypted_data.result, data);
    }

    fn store_symmetric_key(library: &mut CryptographicLibrary, key_id: &str) {
        let key = Key {
            key_id: key_id.to_string(),
            key_type: KeyType::Symmetric,
            key_algorithm: KeyAlgorithm::ChaCha20,
            key_data: (0u8..32).collect(),
            metadata: KeyMetadata {
                key_id: key_id.to_string(),
                key_type: KeyType::Symmetric,
                key_algorithm: KeyAlgorithm::ChaCha20,
                key_size: 32,
                created_at: 0,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: SecurityLevel::High,
                access_level: AccessLevel::Secret,
            },
        };
        library.key_manager.store_key(key).unwrap();
    }

    #[test]
    fn test_chacha20poly1305_roundtrip() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();
        store_symmetric_key(&mut library, "cc_key");

        // AAD is now persisted and re-supplied on decryption.
        let data = b"The quick brown fox jumps over the lazy dog";
        let aad = b"authenticated additional data";
        let enc = library
            .encrypt_data_with_algorithm(
                "cc_key",
                data,
                Some(aad),
                EncryptionAlgorithm::ChaCha20Poly1305,
            )
            .unwrap();
        assert_eq!(enc.result.algorithm, EncryptionAlgorithm::ChaCha20Poly1305);
        assert_eq!(enc.result.iv.len(), 12);
        assert_eq!(enc.result.tag.len(), 16);
        assert_eq!(enc.result.aad, aad.to_vec());
        assert_ne!(enc.result.ciphertext, data.to_vec());

        // decrypt_data dispatches on the stored algorithm and re-supplies AAD.
        let dec = library.decrypt_data("cc_key", &enc.result).unwrap();
        assert_eq!(dec.result, data);
    }

    #[test]
    fn test_chacha20poly1305_wrong_aad_fails() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();
        store_symmetric_key(&mut library, "cc_key_wrong_aad");

        let data = b"authenticated data";
        let aad = b"correct aad";
        let mut enc = library
            .encrypt_data_with_algorithm(
                "cc_key_wrong_aad",
                data,
                Some(aad),
                EncryptionAlgorithm::ChaCha20Poly1305,
            )
            .unwrap()
            .result;

        // Tamper with the AAD - decryption should fail because AAD is authenticated.
        enc.aad = b"wrong aad".to_vec();
        assert!(library.decrypt_data("cc_key_wrong_aad", &enc).is_err());
    }

    #[test]
    fn test_xchacha20poly1305_roundtrip_uses_24byte_nonce() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();
        store_symmetric_key(&mut library, "xcc_key");

        let data = b"extended-nonce payload";
        let enc = library
            .encrypt_data_with_algorithm(
                "xcc_key",
                data,
                None,
                EncryptionAlgorithm::XChaCha20Poly1305,
            )
            .unwrap();
        assert_eq!(enc.result.iv.len(), 24);
        let dec = library.decrypt_data("xcc_key", &enc.result).unwrap();
        assert_eq!(dec.result, data);
    }

    #[test]
    fn test_chacha20poly1305_tamper_fails() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();
        store_symmetric_key(&mut library, "cc_key2");

        let data = b"authenticated";
        let mut enc = library
            .encrypt_data_with_algorithm(
                "cc_key2",
                data,
                None,
                EncryptionAlgorithm::ChaCha20Poly1305,
            )
            .unwrap()
            .result;
        // Flip a ciphertext bit; AEAD verification must reject it.
        enc.ciphertext[0] ^= 0x01;
        assert!(library.decrypt_data("cc_key2", &enc).is_err());
    }

    #[test]
    fn test_hkdf_sha256_rfc5869_vector() {
        // RFC 5869 Appendix A.1 Test Case 1 (HMAC-SHA256).
        let mut kd = KeyDerivation::new();
        kd.derivation_parameters.salt = (0u8..=0x0c).collect(); // 000102...0c (13 bytes)
        kd.derivation_parameters.output_length = 42;
        let ikm = vec![0x0bu8; 22];
        let info: Vec<u8> = (0xf0u8..=0xf9).collect();
        let okm = kd.derive_hkdf(&ikm, &info).unwrap();
        assert_eq!(
            hex::encode(&okm),
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865"
        );
    }

    #[test]
    fn test_hash_computation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let data = b"Hello, World!";
        let hash_result = library.compute_hash(data).unwrap();

        assert_eq!(hash_result.result.algorithm, "SHA256");
        assert_eq!(hash_result.result.input_data, data);
        assert_eq!(hash_result.result.hash_value.len(), 32); // SHA256 output size
        assert!(hash_result.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_blake3_hash_computation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Known-answer test: BLAKE3 of the empty input is a stable, published vector.
        let empty = library.compute_hash_blake3(b"").unwrap();
        assert_eq!(empty.result.algorithm, "BLAKE3");
        assert_eq!(empty.result.hash_value.len(), 32); // BLAKE3 default digest size
        assert_eq!(
            hex::encode(&empty.result.hash_value),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );

        // Determinism + distinctness from SHA-256 over the same input.
        let data = b"Hello, World!";
        let a = library.compute_hash_blake3(data).unwrap();
        let b = library.compute_hash_blake3(data).unwrap();
        assert_eq!(a.result.hash_value, b.result.hash_value);
        let sha = library.compute_hash(data).unwrap();
        assert_ne!(a.result.hash_value, sha.result.hash_value);
        assert!(a.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_zk_proof_generation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let witness = vec![vec![1u8, 2u8, 3u8]];
        let public_inputs = vec![vec![4u8, 5u8, 6u8]];

        let proof = library
            .generate_zk_proof("test_circuit", &witness, &public_inputs)
            .unwrap();

        assert_eq!(proof.result.system_id, "zk_snarks");
        assert_eq!(proof.result.circuit_id, "test_circuit");
        assert_eq!(proof.result.public_inputs, public_inputs);
        assert!(proof.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_zk_proof_verification() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let witness = vec![vec![1u8, 2u8, 3u8]];
        let public_inputs = vec![vec![4u8, 5u8, 6u8]];

        let proof = library
            .generate_zk_proof("test_circuit", &witness, &public_inputs)
            .unwrap();

        // Verify proof
        let is_valid = library
            .verify_zk_proof(&proof.result, &public_inputs)
            .unwrap();

        assert!(is_valid.result);
        assert!(is_valid.result);
    }

    /// Real arkworks Groth16 round-trip + soundness through the public byte API,
    /// for the plan-critical `deontic_access` credential-gated access circuit.
    #[cfg(feature = "zk-culling")]
    #[test]
    fn test_deontic_groth16_byte_api_roundtrip_and_soundness() {
        use ark_bls12_381::Fr;
        use ark_ff::{BigInteger, PrimeField};
        use sha2::{Digest, Sha256};

        // Mirror the prover's witness reduction (SHA-256 -> Fr) and the public-input
        // serialisation (canonical little-endian Fr) so we can build a satisfying
        // instance: did + role + action == policy_root.
        let hash_to_fr = |d: &[u8]| Fr::from_be_bytes_mod_order(&Sha256::digest(d));
        let fr_le = |f: &Fr| f.into_bigint().to_bytes_le();

        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let did = b"did:webizen:alice".to_vec();
        let role = b"role:guardian".to_vec();
        let action = b"action:read-vault".to_vec();
        let witness = vec![did.clone(), role.clone(), action.clone()];

        let policy_root = hash_to_fr(&did) + hash_to_fr(&role) + hash_to_fr(&action);
        let temporal = Fr::from(1_700_000_000u64);
        let public_inputs = vec![fr_le(&policy_root), fr_le(&temporal)];

        let proof = library
            .generate_zk_proof("deontic_access", &witness, &public_inputs)
            .unwrap();
        // Tag 0x02 = the real Groth16 path (not the 0x01 SHA-256 commitment fallback).
        assert_eq!(
            proof.result.proof_data[64], 0x02,
            "deontic_access must route through the real Groth16 path"
        );
        assert!(
            library
                .verify_zk_proof(&proof.result, &public_inputs)
                .unwrap()
                .result,
            "a valid deontic access proof must verify"
        );

        // Soundness: falsify the policy_root public input -> must be rejected.
        let tampered = vec![fr_le(&(policy_root + Fr::from(1u64))), fr_le(&temporal)];
        assert!(
            !library
                .verify_zk_proof(&proof.result, &tampered)
                .unwrap()
                .result,
            "a deontic proof must NOT verify against a falsified policy_root"
        );
    }

    #[test]
    fn test_key_rotation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate initial key
        let _key_pair = library
            .generate_mldsa_key_pair("test_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Rotate key
        let new_key = library.rotate_key("test_key_private").unwrap();

        assert!(new_key.result.key_id != "test_key_private");
        assert_eq!(new_key.result.key_algorithm, KeyAlgorithm::MLDSA);
        assert_eq!(new_key.result.key_type, KeyType::Private);
        assert!(new_key.compliance_status == ComplianceStatus::Compliant);
    }

    #[test]
    fn test_security_metrics() {
        let library = CryptographicLibrary::new();
        let metrics = library.get_security_metrics();

        assert_eq!(metrics.threat_metrics.threats_detected, 0);
        assert_eq!(metrics.threat_metrics.threats_blocked, 0);
        assert_eq!(metrics.anomaly_metrics.anomalies_detected, 0);
        assert_eq!(metrics.compliance_metrics.compliance_score, 1.0);
    }

    #[test]
    fn test_kyber_key_generation() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let private_key = library
            .key_manager
            .key_generator
            .generate_key(
                "kyber_priv".to_string(),
                KeyType::Private,
                KeyAlgorithm::Kyber,
                SecurityLevel::High,
            )
            .unwrap();
        assert_eq!(private_key.key_algorithm, KeyAlgorithm::Kyber);
        assert_eq!(private_key.key_data.len(), fips203::ml_kem_768::DK_LEN);

        let public_key = library
            .key_manager
            .key_generator
            .derive_public_key(&private_key, "kyber_pub".to_string())
            .unwrap();
        assert_eq!(public_key.key_data.len(), fips203::ml_kem_768::EK_LEN);
    }

    #[test]
    fn test_vc_issue_via_library() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        let _kp = library
            .generate_mldsa_key_pair("issuer".to_string(), SecurityLevel::High)
            .unwrap();
        let issuer_did_hash = 12345u64;
        let claim_quins = vec![crate::NQuin {
            subject: issuer_did_hash,
            predicate: crate::q_hash("test:hasRole"),
            object: crate::q_hash("test:Admin"),
            context: issuer_did_hash,
            metadata: 0,
            parity: 0,
        }];
        let context = CryptoContext {
            domain: "test".to_string(),
            purpose: "vc-issuance".to_string(),
            timestamp: 0,
            nonce: [0u8; 32],
        };

        let proof = library
            .issue_vc_mldsa(&claim_quins, "issuer_private", issuer_did_hash, &context)
            .unwrap();
        let is_valid = library
            .verify_vc_mldsa(&proof.result, &claim_quins, "issuer_public", &context)
            .unwrap();

        assert!(is_valid.result);
        assert!(!proof.result.fragment_quins.is_empty());
    }

    #[test]
    fn test_audit_log_records_signature_operations() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate key pair
        library
            .generate_mldsa_key_pair("audit_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Sign data — should create one Sign audit entry
        let data = b"audited data";
        let signature = library.sign_data("audit_key_private", data).unwrap();

        // Verify signature — should create one Verify audit entry
        let _is_valid = library
            .verify_signature("audit_key_public", &signature.result, data)
            .unwrap();

        // Check that the signature audit log has recorded both operations
        let audit = &library.signature_engine.signature_storage.audit_log;
        assert!(
            audit.entry_count() >= 2,
            "audit log should have at least 2 entries (sign + verify)"
        );
        let entries = audit.entries();
        assert!(
            entries
                .iter()
                .any(|e| e.operation == SignatureOperation::Sign),
            "should have a Sign entry"
        );
        assert!(
            entries
                .iter()
                .any(|e| e.operation == SignatureOperation::Verify),
            "should have a Verify entry"
        );
        // All entries should reference the correct signature_id
        assert!(
            entries
                .iter()
                .all(|e| e.signature_id == signature.result.signature_id),
            "entries should reference the correct signature_id"
        );
    }

    #[test]
    fn test_audit_log_records_hash_operations() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Compute a hash — should create one Compute audit entry
        let _hash = library.compute_hash(b"test data").unwrap();

        let audit = &library.hash_engine.hash_storage.audit_log;
        assert!(
            audit.entry_count() >= 1,
            "audit log should have at least 1 entry"
        );
        assert!(
            audit
                .entries()
                .iter()
                .any(|e| e.operation == HashOperation::Compute),
            "should have a Compute entry"
        );
    }

    #[test]
    fn test_key_relationship_tracking() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate a key pair — should create a KeyPair relationship
        let key_pair = library
            .generate_mldsa_key_pair("rel_key".to_string(), SecurityLevel::High)
            .unwrap();

        let catalog = &library.key_manager.key_storage.key_catalog;

        // The catalog should have registered both keys
        assert!(
            catalog.key_count() >= 2,
            "catalog should have at least 2 keys registered"
        );

        // The KeyPair relationship should exist from private → public
        let rels = catalog.get_relationships(&key_pair.result.0.key_id);
        assert!(
            rels.iter()
                .any(|r| r.relationship_type == KeyRelationshipType::KeyPair),
            "should have a KeyPair relationship from private to public key"
        );

        // find_related should locate the public key
        let related = catalog.find_related(&key_pair.result.0.key_id, KeyRelationshipType::KeyPair);
        assert!(
            related.is_some(),
            "find_related should find the KeyPair relationship"
        );
        assert_eq!(
            related.unwrap().target_key,
            key_pair.result.1.key_id,
            "KeyPair relationship should point to the public key"
        );
    }

    #[test]
    fn test_key_rotation_tracking() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate initial key
        let _key_pair = library
            .generate_mldsa_key_pair("rot_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Rotate key — should create a RotatedFrom relationship
        let new_key = library.rotate_key("rot_key_private").unwrap();

        let catalog = &library.key_manager.key_storage.key_catalog;
        let rels = catalog.get_relationships(&new_key.result.key_id);
        assert!(
            rels.iter()
                .any(|r| r.relationship_type == KeyRelationshipType::RotatedFrom),
            "should have a RotatedFrom relationship from new key to old key"
        );

        assert!(
            catalog.relationship_count() >= 2,
            "should have at least 2 relationships (KeyPair + RotatedFrom)"
        );
    }

    #[test]
    fn test_performance_metrics_recorded() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate key pair
        library
            .generate_mldsa_key_pair("perf_key".to_string(), SecurityLevel::High)
            .unwrap();

        // Sign data — should update signing time metrics
        let signature = library
            .sign_data("perf_key_private", b"performance test")
            .unwrap();

        // Verify — should update verification time metrics
        let _is_valid = library
            .verify_signature("perf_key_public", &signature.result, b"performance test")
            .unwrap();

        let sig_metrics = library.signature_engine.performance_optimizer.metrics();
        assert!(
            sig_metrics.average_signing_time > 0.0,
            "average signing time should be recorded"
        );

        // Compute a hash — should update hash metrics
        let _hash = library.compute_hash(b"metric test").unwrap();
        let hash_metrics = library.hash_engine.performance_optimizer.metrics();
        assert!(
            hash_metrics.average_hash_time >= 0.0,
            "hash metrics should be accessible"
        );
    }

    #[test]
    fn test_access_control_enforces_policies() {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();

        // Generate a key pair
        library
            .generate_mldsa_key_pair("acl_key".to_string(), SecurityLevel::High)
            .unwrap();

        // By default (no policies), access is allowed
        let key = library.key_manager.get_key("acl_key_private").unwrap();
        assert_eq!(key.key_id, "acl_key_private");

        // Register a restrictive policy that only allows Sign on the private key
        let policy = AccessPolicy {
            policy_id: "policy_1".to_string(),
            key_id: "acl_key_private".to_string(),
            allowed_operations: vec![KeyOperation::Sign],
            required_auth: vec![AuthenticationMethod::MultiFactor],
            time_restrictions: TimeRestrictions {
                allowed_hours: vec![],
                allowed_days: vec![],
                start_date: None,
                end_date: None,
            },
            ip_restrictions: vec![],
        };
        library
            .key_manager
            .key_storage
            .access_control
            .add_policy(policy);

        // Sign should be allowed
        assert!(
            library
                .key_manager
                .key_storage
                .access_control
                .check_permission("acl_key_private", KeyOperation::Sign),
            "Sign should be allowed by policy"
        );

        // Read should be denied
        assert!(
            !library
                .key_manager
                .key_storage
                .access_control
                .check_permission("acl_key_private", KeyOperation::Read),
            "Read should be denied by policy"
        );

        // get_key_with_access should deny Read and log the failure
        let result = library.key_manager.key_storage.get_key_with_access(
            "acl_key_private",
            KeyOperation::Read,
            "test_user",
        );
        assert!(result.is_err(), "get_key_with_access should deny Read");

        // Sign should succeed
        let result = library.key_manager.key_storage.get_key_with_access(
            "acl_key_private",
            KeyOperation::Sign,
            "test_user",
        );
        assert!(result.is_ok(), "get_key_with_access should allow Sign");

        // Audit log should have both the denied and allowed entries
        let audit = library.key_manager.key_storage.access_control.audit_log();
        assert!(
            audit.entry_count() >= 2,
            "audit log should have at least 2 entries"
        );
    }

    #[test]
    fn test_encryption_at_rest_roundtrip() {
        let mut ear = EncryptionAtRest::new();
        assert!(!ear.is_enabled(), "KEK should not exist before initialize");

        ear.initialize().unwrap();
        assert!(
            ear.is_enabled(),
            "master KEK should be generated after initialize"
        );
        assert!(ear.kek_count() >= 1, "should have at least one KEK");

        // Encrypt some key data
        let plaintext = b"super_secret_key_material_12345";
        let encrypted = ear.encrypt_key_data(plaintext).unwrap();

        // Ciphertext should be different from plaintext (nonce + tag + ciphertext)
        assert_ne!(
            &encrypted[..],
            plaintext,
            "encrypted data should differ from plaintext"
        );
        assert!(
            encrypted.len() > plaintext.len() + 12,
            "encrypted should be longer due to nonce + tag"
        );

        // Decrypt and verify roundtrip
        let decrypted = ear.decrypt_key_data(&encrypted).unwrap();
        assert_eq!(
            &decrypted[..],
            plaintext,
            "decrypted data should match original plaintext"
        );
    }

    #[test]
    fn test_encryption_at_rest_without_kek_fails() {
        let ear = EncryptionAtRest::new();
        // Without initialize(), no KEK exists
        let result = ear.encrypt_key_data(b"test");
        assert!(result.is_err(), "encryption should fail without a KEK");
    }

    // ---- Feature 1: Key Catalog Search ----

    fn sample_metadata(key_id: &str, algorithm: KeyAlgorithm) -> KeyMetadata {
        KeyMetadata {
            key_id: key_id.to_string(),
            key_type: KeyType::Symmetric,
            key_algorithm: algorithm,
            key_size: 256,
            created_at: 1000,
            expires_at: 0,
            last_used: 0,
            usage_count: 0,
            security_level: SecurityLevel::High,
            access_level: AccessLevel::Secret,
        }
    }

    #[test]
    fn test_key_catalog_search_by_keyword() {
        let mut catalog = KeyCatalog::new();
        catalog.initialize().unwrap();
        catalog.register_key(sample_metadata("aes_signing_key", KeyAlgorithm::AES));
        catalog.register_key(sample_metadata("mldsa_master_key", KeyAlgorithm::MLDSA));

        // Search by algorithm keyword (case-insensitive)
        let aes_hits = catalog.search("aes");
        assert!(
            aes_hits.contains(&"aes_signing_key".to_string()),
            "search should find the AES key"
        );
        assert!(
            !aes_hits.contains(&"mldsa_master_key".to_string()),
            "AES search should not return the MLDSA key"
        );

        // Search by key id substring (case-insensitive)
        let master_hits = catalog.search("MASTER");
        assert!(
            master_hits.contains(&"mldsa_master_key".to_string()),
            "case-insensitive search should find master key"
        );
    }

    #[test]
    fn test_key_catalog_search_by_tag() {
        let mut catalog = KeyCatalog::new();
        catalog.initialize().unwrap();
        catalog.register_key(sample_metadata("key_one", KeyAlgorithm::AES));
        catalog.register_key(sample_metadata("key_two", KeyAlgorithm::ChaCha20));
        catalog.add_tag("key_one", "production");
        catalog.add_tag("key_two", "staging");

        let prod = catalog.get_by_tag("Production");
        assert_eq!(prod, vec!["key_one".to_string()]);

        let staging = catalog.get_by_tag("staging");
        assert_eq!(staging, vec!["key_two".to_string()]);

        // search() should also match tags
        let hits = catalog.search("production");
        assert!(hits.contains(&"key_one".to_string()));
    }

    #[test]
    fn test_key_search_index_index_and_search() {
        let mut index = KeySearchIndex::new();
        index.initialize().unwrap();
        assert_eq!(index.entry_count(), 0);

        index.index(KeyIndexEntry {
            entry_id: "key_1".to_string(),
            keywords: vec!["signing".to_string(), "mldsa".to_string()],
            metadata: HashMap::new(),
            relevance_score: 0.9,
        });
        index.index(KeyIndexEntry {
            entry_id: "key_2".to_string(),
            keywords: vec!["encryption".to_string(), "aes".to_string()],
            metadata: HashMap::new(),
            relevance_score: 0.8,
        });
        assert_eq!(index.entry_count(), 2);

        let signing_hits = index.search_by_keyword("signing");
        assert_eq!(signing_hits.len(), 1);
        assert_eq!(signing_hits[0].entry_id, "key_1");

        let aes_hits = index.search_by_keyword("AES");
        assert_eq!(aes_hits.len(), 1);
        assert_eq!(aes_hits[0].entry_id, "key_2");
    }

    #[test]
    fn test_key_search_index_initialize_sets_strategy() {
        let mut index = KeySearchIndex::new();
        // Before initialize the engine defaults to Encrypted/Encrypted.
        assert_eq!(index.search_engine.engine_type, SearchEngineType::Encrypted);
        assert_eq!(
            index.search_engine.indexing_strategy,
            IndexingStrategy::Encrypted
        );

        index.initialize().unwrap();
        assert_eq!(index.search_engine.engine_type, SearchEngineType::Hybrid);
        assert_eq!(
            index.search_engine.indexing_strategy,
            IndexingStrategy::Inverted
        );
    }

    #[test]
    fn test_register_key_populates_search_index() {
        let mut catalog = KeyCatalog::new();
        catalog.initialize().unwrap();
        assert_eq!(catalog.search_index.entry_count(), 0);

        catalog.register_key(sample_metadata("indexed_key", KeyAlgorithm::AES));
        assert_eq!(
            catalog.search_index.entry_count(),
            1,
            "register_key should populate the search index"
        );

        // The indexed entry should be discoverable via the catalog search.
        let hits = catalog.search("indexed_key");
        assert!(hits.contains(&"indexed_key".to_string()));
    }

    // ---- Key Search Engine (structured SearchQuery) ----

    /// Helper: build a [`KeyMetadata`] with a configurable creation timestamp.
    fn search_metadata(key_id: &str, algorithm: KeyAlgorithm, created_at: u64) -> KeyMetadata {
        KeyMetadata {
            key_id: key_id.to_string(),
            key_type: KeyType::Symmetric,
            key_algorithm: algorithm,
            key_size: 256,
            created_at,
            expires_at: 0,
            last_used: 0,
            usage_count: 0,
            security_level: SecurityLevel::High,
            access_level: AccessLevel::Secret,
        }
    }

    /// Build an index populated with three keys used across the search tests.
    fn populated_index() -> KeySearchIndex {
        let mut index = KeySearchIndex::new();
        index.index_key(
            "signing_master_key",
            &search_metadata("signing_master_key", KeyAlgorithm::MLDSA, 1000),
        );
        index.index_key(
            "aes_encrypt_key",
            &search_metadata("aes_encrypt_key", KeyAlgorithm::AES, 2000),
        );
        index.index_key(
            "rsa_backup_key",
            &search_metadata("rsa_backup_key", KeyAlgorithm::RSA, 3000),
        );

        index.add_tag("signing_master_key", "production");
        index.add_tag("aes_encrypt_key", "production");
        index.add_tag("rsa_backup_key", "backup");

        index.set_purpose("signing_master_key", KeyPurpose::Signing);
        index.set_purpose("aes_encrypt_key", KeyPurpose::Encryption);
        index
    }

    #[test]
    fn test_text_search() {
        let index = populated_index();

        // Partial key_id substring "encrypt" should match only aes_encrypt_key.
        let hits = index.search(&SearchQuery::new().with_text("encrypt"));
        assert_eq!(hits.len(), 1, "partial key_id should match one key");
        assert_eq!(hits[0].key_id, "aes_encrypt_key");

        // Partial substring "key" matches all three key ids.
        let key_hits = index.search(&SearchQuery::new().with_text("key"));
        assert_eq!(key_hits.len(), 3, "common substring should match all keys");
    }

    #[test]
    fn test_algorithm_filter() {
        let index = populated_index();

        let aes_hits = index.search(&SearchQuery::new().with_algorithm(KeyAlgorithm::AES));
        assert_eq!(aes_hits.len(), 1);
        assert_eq!(aes_hits[0].key_id, "aes_encrypt_key");

        let rsa_hits = index.search(&SearchQuery::new().with_algorithm(KeyAlgorithm::RSA));
        assert_eq!(rsa_hits.len(), 1);
        assert_eq!(rsa_hits[0].key_id, "rsa_backup_key");

        // An algorithm with no matching keys returns nothing.
        let none = index.search(&SearchQuery::new().with_algorithm(KeyAlgorithm::Kyber));
        assert!(none.is_empty());
    }

    #[test]
    fn test_tag_search() {
        let index = populated_index();

        let prod = index.search(&SearchQuery::new().with_tag("production"));
        assert_eq!(prod.len(), 2, "two keys are tagged production");
        let prod_ids: Vec<&str> = prod.iter().map(|r| r.key_id.as_str()).collect();
        assert!(prod_ids.contains(&"signing_master_key"));
        assert!(prod_ids.contains(&"aes_encrypt_key"));

        let backup = index.search(&SearchQuery::new().with_tag("backup"));
        assert_eq!(backup.len(), 1);
        assert_eq!(backup[0].key_id, "rsa_backup_key");

        // Tag matching is case-insensitive.
        let prod_upper = index.search(&SearchQuery::new().with_tag("PRODUCTION"));
        assert_eq!(prod_upper.len(), 2);
    }

    #[test]
    fn test_date_range() {
        let index = populated_index();

        // created_after: only keys with created_at >= 2000.
        let after = index.search(&SearchQuery::new().with_created_after(2000));
        let after_ids: Vec<&str> = after.iter().map(|r| r.key_id.as_str()).collect();
        assert!(after_ids.contains(&"aes_encrypt_key"));
        assert!(after_ids.contains(&"rsa_backup_key"));
        assert!(!after_ids.contains(&"signing_master_key"));

        // created_before: only keys with created_at <= 2000.
        let before = index.search(&SearchQuery::new().with_created_before(2000));
        let before_ids: Vec<&str> = before.iter().map(|r| r.key_id.as_str()).collect();
        assert!(before_ids.contains(&"signing_master_key"));
        assert!(before_ids.contains(&"aes_encrypt_key"));
        assert!(!before_ids.contains(&"rsa_backup_key"));

        // Bounded range [1500, 2500] → only aes_encrypt_key (2000).
        let bounded = index.search(
            &SearchQuery::new()
                .with_created_after(1500)
                .with_created_before(2500),
        );
        assert_eq!(bounded.len(), 1);
        assert_eq!(bounded[0].key_id, "aes_encrypt_key");
    }

    #[test]
    fn test_combined_query() {
        let index = populated_index();

        // text "key" + algorithm AES + tag "production" → only aes_encrypt_key
        // satisfies all three constraints.
        let combined = index.search(
            &SearchQuery::new()
                .with_text("key")
                .with_algorithm(KeyAlgorithm::AES)
                .with_tag("production"),
        );
        assert_eq!(combined.len(), 1, "combined query should intersect");
        assert_eq!(combined[0].key_id, "aes_encrypt_key");

        // A combined query that no key satisfies returns empty.
        let impossible = index.search(
            &SearchQuery::new()
                .with_algorithm(KeyAlgorithm::RSA)
                .with_tag("production"),
        );
        assert!(impossible.is_empty(), "rsa key is not tagged production");
    }

    #[test]
    fn test_empty_index() {
        let index = KeySearchIndex::new();
        let hits = index.search(&SearchQuery::new().with_text("anything"));
        assert!(hits.is_empty(), "empty index yields no results");

        // An unconstrained query over an empty index also yields nothing.
        let all = index.search(&SearchQuery::new());
        assert!(all.is_empty());
    }

    #[test]
    fn test_relevance_scoring() {
        let mut index = KeySearchIndex::new();
        index.index_key(
            "alpha_key",
            &search_metadata("alpha_key", KeyAlgorithm::AES, 1000),
        );

        // Exact key_id match scores higher than a partial substring match.
        let exact = index.search(&SearchQuery::new().with_text("alpha_key"));
        assert_eq!(exact.len(), 1);
        let exact_score = exact[0].relevance_score;

        let partial = index.search(&SearchQuery::new().with_text("alpha"));
        assert_eq!(partial.len(), 1);
        let partial_score = partial[0].relevance_score;

        assert!(
            exact_score > partial_score,
            "exact match ({}) should score higher than partial ({})",
            exact_score,
            partial_score
        );
        assert_eq!(exact_score, 1.0, "exact match contributes 1.0");
        assert_eq!(partial_score, 0.5, "partial match contributes 0.5");
    }

    // ---- Feature 2: Entropy Source Selection ----

    #[test]
    fn test_key_generator_list_entropy_sources() {
        let gen = KeyGenerator::new();
        let sources = gen.list_entropy_sources();
        assert!(
            sources.contains(&"HardwareRNG".to_string()),
            "HardwareRNG should be listed"
        );
        assert!(
            sources.contains(&"OSRandom".to_string()),
            "OSRandom should be listed"
        );
        assert!(
            sources.contains(&"Quantum".to_string()),
            "Quantum should be listed"
        );
    }

    #[test]
    fn test_key_generator_set_and_get_entropy_source() {
        let mut gen = KeyGenerator::new();
        assert!(
            gen.get_entropy_source().is_none(),
            "no source selected by default"
        );

        gen.set_entropy_source(EntropySource::HardwareRNG);
        assert_eq!(
            gen.get_entropy_source(),
            Some(&EntropySource::HardwareRNG),
            "selected source should be HardwareRNG"
        );

        gen.set_entropy_source(EntropySource::Quantum);
        assert_eq!(
            gen.get_entropy_source(),
            Some(&EntropySource::Quantum),
            "selected source should be Quantum after re-set"
        );
    }

    #[test]
    fn test_key_generator_generate_key_data() {
        let mut gen = KeyGenerator::new();
        gen.initialize().unwrap();
        gen.set_entropy_source(EntropySource::OSRandom);

        let key_size = 32;
        let data = gen.generate_key_data(key_size).unwrap();
        assert_eq!(
            data.len(),
            key_size,
            "generated data should have the requested length"
        );
        assert!(
            !data.iter().all(|&b| b == 0),
            "generated data should not be all zeros"
        );

        // Quality metrics should be updated.
        assert!(
            gen.quality_metrics.entropy_score > 0.0,
            "entropy score should be updated after generation"
        );
    }

    #[test]
    fn test_key_generator_generate_key_data_default_source() {
        let mut gen = KeyGenerator::new();
        // No source explicitly selected — should fall back to OSRandom.
        let data = gen.generate_key_data(16).unwrap();
        assert_eq!(data.len(), 16);
        assert!(
            !data.iter().all(|&b| b == 0),
            "default-source data should not be all zeros"
        );
    }

    #[test]
    fn test_key_generator_generate_key_data_quantum_placeholder() {
        let mut gen = KeyGenerator::new();
        gen.set_entropy_source(EntropySource::Quantum);
        let data = gen.generate_key_data(64).unwrap();
        assert_eq!(data.len(), 64);
        assert!(
            !data.iter().all(|&b| b == 0),
            "quantum placeholder should still produce non-zero data"
        );
    }

    // ---- Feature: Encryption Policy Enforcement ----

    /// Current unix timestamp in seconds (for deterministic age-based tests).
    fn now_secs() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Build a `Key` with the given algorithm, size (bits) and created_at timestamp.
    fn sample_key(algorithm: KeyAlgorithm, key_size: usize, created_at: u64) -> Key {
        Key {
            key_id: "policy_test_key".to_string(),
            key_type: KeyType::Symmetric,
            key_algorithm: algorithm,
            key_data: vec![0u8; 32],
            metadata: KeyMetadata {
                key_id: "policy_test_key".to_string(),
                key_type: KeyType::Symmetric,
                key_algorithm: algorithm,
                key_size,
                created_at,
                expires_at: 0,
                last_used: 0,
                usage_count: 0,
                security_level: SecurityLevel::High,
                access_level: AccessLevel::Secret,
            },
        }
    }

    /// A standard policy used across the policy-engine tests.
    fn standard_policy() -> EncryptionPolicy {
        EncryptionPolicy {
            policy_id: "std".to_string(),
            name: "Standard Policy".to_string(),
            min_key_size: 256,
            required_algorithms: vec![KeyAlgorithm::AES, KeyAlgorithm::ChaCha20],
            compliance_standards: vec![ComplianceStandard::FIPS140, ComplianceStandard::SOC2],
            key_rotation_interval_days: 90,
            require_encryption_at_rest: true,
        }
    }

    #[test]
    fn test_key_size_validation() {
        let mut engine = EncryptionPolicyEngine::new();
        engine.add_policy(standard_policy());

        // Key is only 128 bits but policy requires >= 256.
        let key = sample_key(KeyAlgorithm::AES, 128, now_secs());
        let result = engine.validate_key(&key, "std").unwrap();

        assert!(!result.passed, "a too-small key should not pass validation");
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.rule == "min_key_size" && v.severity == ViolationSeverity::Critical),
            "expected a critical min_key_size violation, got {:?}",
            result.violations
        );
    }

    #[test]
    fn test_algorithm_validation() {
        let mut engine = EncryptionPolicyEngine::new();
        engine.add_policy(standard_policy());

        // RSA is not in the required_algorithms set.
        let key = sample_key(KeyAlgorithm::RSA, 256, now_secs());
        let result = engine.validate_key(&key, "std").unwrap();

        assert!(!result.passed, "a wrong-algorithm key should not pass");
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.rule == "required_algorithms"
                    && v.severity == ViolationSeverity::Critical),
            "expected a critical required_algorithms violation, got {:?}",
            result.violations
        );
    }

    #[test]
    fn test_key_age_validation() {
        let mut engine = EncryptionPolicyEngine::new();
        engine.add_policy(standard_policy());

        // Key is older than the 90-day rotation interval.
        let now = now_secs();
        let too_old = now.saturating_sub((91 * 86_400) as u64);
        let key = sample_key(KeyAlgorithm::AES, 256, too_old);
        let result = engine.validate_key(&key, "std").unwrap();

        assert!(!result.passed, "an expired key should not pass");
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.rule == "key_rotation_interval_days"
                    && v.severity == ViolationSeverity::Warning),
            "expected a warning key_rotation_interval_days violation, got {:?}",
            result.violations
        );
    }

    #[test]
    fn test_encryption_at_rest_required() {
        let mut engine = EncryptionPolicyEngine::new();
        engine.add_policy(standard_policy());

        // Policy requires encryption at rest, but it is not present.
        let result = engine.validate_encryption_at_rest(false, "std").unwrap();

        assert!(
            !result.passed,
            "missing required encryption at rest should not pass"
        );
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.rule == "require_encryption_at_rest"
                    && v.severity == ViolationSeverity::Critical),
            "expected a critical require_encryption_at_rest violation, got {:?}",
            result.violations
        );

        // When encryption is present, it should pass.
        let ok = engine.validate_encryption_at_rest(true, "std").unwrap();
        assert!(ok.passed, "present encryption at rest should pass");
        assert!(ok.violations.is_empty());
    }

    #[test]
    fn test_valid_key_passes() {
        let mut engine = EncryptionPolicyEngine::new();
        engine.add_policy(standard_policy());

        // Fresh, correctly-sized AES key — all checks pass.
        let key = sample_key(KeyAlgorithm::AES, 256, now_secs());
        let result = engine.validate_key(&key, "std").unwrap();

        assert!(result.passed, "a compliant key should pass");
        assert!(result.violations.is_empty(), "no violations expected");
        assert_eq!(result.policy_id, "std");
        assert!(result.checked_at > 0, "checked_at should be populated");
    }

    #[test]
    fn test_multiple_violations() {
        let mut engine = EncryptionPolicyEngine::new();
        engine.add_policy(standard_policy());

        // A key that is too small, wrong algorithm, AND too old.
        let now = now_secs();
        let too_old = now.saturating_sub((365 * 86_400) as u64);
        let key = sample_key(KeyAlgorithm::RSA, 128, too_old);
        let result = engine.validate_key(&key, "std").unwrap();

        assert!(
            !result.passed,
            "a key violating multiple rules should not pass"
        );
        let rules: Vec<&str> = result.violations.iter().map(|v| v.rule.as_str()).collect();
        assert!(
            rules.contains(&"min_key_size"),
            "expected min_key_size violation, rules = {:?}",
            rules
        );
        assert!(
            rules.contains(&"required_algorithms"),
            "expected required_algorithms violation, rules = {:?}",
            rules
        );
        assert!(
            rules.contains(&"key_rotation_interval_days"),
            "expected key_rotation_interval_days violation, rules = {:?}",
            rules
        );
        assert_eq!(
            result.violations.len(),
            3,
            "expected exactly three violations"
        );
    }

    #[test]
    fn test_unknown_policy() {
        let engine = EncryptionPolicyEngine::new();

        let key = sample_key(KeyAlgorithm::AES, 256, now_secs());
        let key_res = engine.validate_key(&key, "does_not_exist");
        assert!(
            matches!(key_res, Err(PolicyError::UnknownPolicy(_))),
            "validate_key with unknown policy should return UnknownPolicy error"
        );

        let ear_res = engine.validate_encryption_at_rest(true, "does_not_exist");
        assert!(
            matches!(ear_res, Err(PolicyError::UnknownPolicy(_))),
            "validate_encryption_at_rest with unknown policy should return UnknownPolicy error"
        );
    }
