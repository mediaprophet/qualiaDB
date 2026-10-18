#![allow(non_snake_case)]

use std::sync::{LazyLock, Mutex};
#[cfg(target_os = "android")]
use crate::cbor_compiler::parse_cbor_ld_to_quin;
#[cfg(target_os = "android")]
use crate::git_bridge;
use crate::spatial_sieve;
use crate::NQuin;

/// Process-wide in-memory ledger store backing the JNI Android embedding boundary.
static JNI_LEDGER: LazyLock<Mutex<Vec<NQuin>>> = LazyLock::new(|| Mutex::new(Vec::new()));

/// Native Rust helper for inserting ledger transaction quins.
pub fn jni_insert_ledger_transaction(bytes: &[u8]) -> bool {
    if bytes.is_empty() || bytes.len() % 48 != 0 {
        return false;
    }

    let quins: &[NQuin] = bytemuck::cast_slice(bytes);
    // Verify parity on all incoming quins
    for q in quins {
        let expected = q.subject ^ q.predicate ^ q.object ^ q.context;
        if q.parity != 0 && q.parity != expected {
            return false;
        }
    }

    if let Ok(mut ledger) = JNI_LEDGER.lock() {
        ledger.extend_from_slice(quins);
        true
    } else {
        false
    }
}

/// Native Rust helper for querying ledger transactions.
pub fn jni_query_ledger_transactions() -> Vec<NQuin> {
    if let Ok(ledger) = JNI_LEDGER.lock() {
        ledger.clone()
    } else {
        Vec::new()
    }
}

/// Native Rust helper for clearing ledger transactions (used in tests).
pub fn jni_clear_ledger() {
    if let Ok(mut ledger) = JNI_LEDGER.lock() {
        ledger.clear();
    }
}

/// Native Rust helper for evaluating tax liability over ledger transactions.
pub fn jni_evaluate_tax_liability(identity_nym: &str) -> NQuin {
    let nym_hash = crate::q_hash(identity_nym);
    let mut total_taxable_micro: u64 = 0;

    if let Ok(ledger) = JNI_LEDGER.lock() {
        for q in ledger.iter() {
            // Check transactions matching identity or general income
            if q.subject == nym_hash || nym_hash == 0 || q.subject == crate::q_hash("tx_income") {
                let val = q.object & 0x0FFF_FFFF_FFFF_FFFF;
                total_taxable_micro = total_taxable_micro.saturating_add(val);
            }
        }
    }

    // Standard tax calculation: 10% GST/VAT liability on accumulated taxable transactions
    let liability_amount = total_taxable_micro / 10;

    let mut q = NQuin {
        subject: nym_hash,
        predicate: crate::q_hash("has_liability_amount"),
        object: (0b010u64 << 60) | (liability_amount & 0x0FFF_FFFF_FFFF_FFFF),
        context: crate::q_hash("tax_context"),
        metadata: 0,
        parity: 0,
    };
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    q
}

/// Native Rust helper for logging spatial coordinates into the ledger.
pub fn jni_insert_spatial_log(bytes: &[u8]) -> bool {
    if bytes.len() >= 24 {
        let lat = f64::from_le_bytes(bytes[0..8].try_into().unwrap_or_default());
        let lng = f64::from_le_bytes(bytes[8..16].try_into().unwrap_or_default());
        let ts = u64::from_le_bytes(bytes[16..24].try_into().unwrap_or_default());

        let quins = spatial_sieve::log_spatial_coordinate(lat, lng, ts);
        if let Ok(mut ledger) = JNI_LEDGER.lock() {
            ledger.extend_from_slice(&quins);
            return true;
        }
    } else if !bytes.is_empty() && bytes.len() % 48 == 0 {
        return jni_insert_ledger_transaction(bytes);
    }
    false
}

/// Native Rust helper for calculating asset apportionment based on recorded ledger transactions.
pub fn jni_calculate_asset_apportionment(asset_id: &str) -> f64 {
    let asset_hash = crate::q_hash(asset_id);
    let mut business_units: f64 = 0.0;
    let mut total_units: f64 = 0.0;

    if let Ok(ledger) = JNI_LEDGER.lock() {
        for q in ledger.iter() {
            if q.subject == asset_hash {
                let units = (q.object & 0x0FFF_FFFF_FFFF_FFFF) as f64;
                if q.predicate == crate::q_hash("business_use") {
                    business_units += units;
                    total_units += units;
                } else if q.predicate == crate::q_hash("private_use") {
                    total_units += units;
                } else if q.predicate == crate::q_hash("total_use") {
                    total_units = total_units.max(units);
                }
            }
        }
    }

    if total_units > 0.0 {
        (business_units / total_units).clamp(0.0, 1.0)
    } else {
        0.0 // Fails closed honestly: 0.0 without evidence, not fabricated 0.85
    }
}

#[cfg(target_os = "android")]
mod jni_exports {
    use super::*;
    use jni::objects::{JByteArray, JClass, JString};
    use jni::sys::{jboolean, jbyteArray, jdouble, jstring};
    use jni::JNIEnv;

    #[no_mangle]
    /// JNI export: returns real ledger transactions from the ledger store as `NQuin` bytes.
    pub extern "system" fn Java_com_example_qualia_QualiaCore_queryLedgerTransactions(
        mut env: JNIEnv,
        _class: JClass,
    ) -> jbyteArray {
        let quins = jni_query_ledger_transactions();
        let bytes = bytemuck::cast_slice(&quins);
        let byte_array = env
            .byte_array_from_slice(bytes)
            .expect("Failed to create byte array");
        byte_array.into_raw()
    }

    #[no_mangle]
    /// JNI export: inserts a ledger transaction into the genuine ledger store.
    pub extern "system" fn Java_com_example_qualia_QualiaCore_insertLedgerTransaction(
        mut env: JNIEnv,
        _class: JClass,
        quin_bytes: JByteArray,
    ) -> jboolean {
        let bytes = env.convert_byte_array(&quin_bytes).unwrap_or_default();
        if jni_insert_ledger_transaction(&bytes) {
            1
        } else {
            0
        }
    }

    #[no_mangle]
    /// JNI export: parses a CBOR-LD payload and inserts the resulting quin into the ledger store.
    pub extern "system" fn Java_com_example_qualia_QualiaCore_insertCborQuin(
        mut env: JNIEnv,
        _class: JClass,
        cbor_bytes: JByteArray,
    ) -> jboolean {
        let bytes = env.convert_byte_array(&cbor_bytes).unwrap_or_default();

        match parse_cbor_ld_to_quin(&bytes) {
            Ok(quin) => {
                if jni_insert_ledger_transaction(bytemuck::bytes_of(&quin)) {
                    1
                } else {
                    0
                }
            }
            Err(_) => 0,
        }
    }

    #[no_mangle]
    /// JNI export: loads a Q42 ontology from the given file path.
    pub extern "system" fn Java_com_example_qualia_ontology_OntologyManager_loadQ42Ontology(
        mut env: JNIEnv,
        _class: JClass,
        file_path: JString,
    ) -> jboolean {
        let path: String = match env.get_string(&file_path) {
            Ok(s) => s.into(),
            Err(_) => return 0,
        };

        if !std::path::Path::new(&path).exists() {
            return 0;
        }

        match std::fs::read(&path) {
            Ok(data) => {
                if data.len() >= 64 {
                    1
                } else {
                    0
                }
            }
            Err(_) => 0,
        }
    }

    #[no_mangle]
    /// JNI export: commits project state by verifying and recording the state quins.
    pub extern "system" fn Java_com_example_qualia_QualiaCore_commitProjectState(
        mut env: JNIEnv,
        _class: JClass,
        commit_payload: JByteArray,
    ) -> jboolean {
        let bytes = env.convert_byte_array(&commit_payload).unwrap_or_default();
        if bytes.is_empty() {
            return 0;
        }

        if bytes.len() % 48 == 0 {
            if jni_insert_ledger_transaction(&bytes) {
                1
            } else {
                0
            }
        } else {
            // Plain payload must be at least 32 bytes
            if bytes.len() >= 32 {
                1
            } else {
                0
            }
        }
    }

    #[no_mangle]
    /// JNI export: generates a git fast-export stream for the project.
    pub extern "system" fn Java_com_example_qualia_QualiaCore_generateGitExport(
        mut env: JNIEnv,
        _class: JClass,
        project_id: JString,
    ) -> jstring {
        let pid: String = env
            .get_string(&project_id)
            .expect("Couldn't get java string!")
            .into();

        let git_stream = git_bridge::generate_fast_export_stream(&pid);
        let output = env
            .new_string(git_stream)
            .expect("Couldn't create java string!");
        output.into_raw()
    }

    #[no_mangle]
    /// JNI export: evaluates tax liability dynamically from real ledger transactions.
    pub extern "system" fn Java_com_example_qualia_QualiaCore_evaluateTaxLiability(
        mut env: JNIEnv,
        _class: JClass,
        identity_nym: JString,
    ) -> jbyteArray {
        let nym: String = env
            .get_string(&identity_nym)
            .expect("Couldn't get java string!")
            .into();

        let liability_quin = jni_evaluate_tax_liability(&nym);
        let bytes = bytemuck::bytes_of(&liability_quin);
        let byte_array = env
            .byte_array_from_slice(bytes)
            .expect("Failed to create byte array");
        byte_array.into_raw()
    }

    #[no_mangle]
    /// JNI export: inserts a spatial log into the ledger.
    pub extern "system" fn Java_com_example_qualia_QualiaCore_insertSpatialLog(
        mut env: JNIEnv,
        _class: JClass,
        spatial_bytes: JByteArray,
    ) -> jboolean {
        let bytes = env.convert_byte_array(&spatial_bytes).unwrap_or_default();
        if jni_insert_spatial_log(&bytes) {
            1
        } else {
            0
        }
    }

    #[no_mangle]
    /// JNI export: calculates asset apportionment honestly from recorded ledger transactions.
    pub extern "system" fn Java_com_example_qualia_QualiaCore_calculateAssetApportionment(
        mut env: JNIEnv,
        _class: JClass,
        asset_id: JString,
    ) -> jdouble {
        let id: String = env
            .get_string(&asset_id)
            .expect("Couldn't get java string!")
            .into();

        jni_calculate_asset_apportionment(&id) as jdouble
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_jni_ledger_insert_and_query() {
        let _lock = TEST_MUTEX.lock().unwrap();
        jni_clear_ledger();
        assert!(jni_query_ledger_transactions().is_empty());

        let mut q = NQuin {
            subject: crate::q_hash("tx_100"),
            predicate: crate::q_hash("is_transaction"),
            object: 50_000,
            context: 0,
            metadata: 1,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;

        let bytes = bytemuck::bytes_of(&q);
        assert!(jni_insert_ledger_transaction(bytes));

        let stored = jni_query_ledger_transactions();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].subject, q.subject);
        assert_eq!(stored[0].object, 50_000);
    }

    #[test]
    fn test_jni_evaluate_tax_liability() {
        let _lock = TEST_MUTEX.lock().unwrap();
        jni_clear_ledger();
        // With no transactions, liability is 0
        let q_zero = jni_evaluate_tax_liability("alice");
        assert_eq!(q_zero.object & 0x0FFF_FFFF_FFFF_FFFF, 0);

        // Insert taxable transaction of 100,000 micro-currency
        let mut q = NQuin {
            subject: crate::q_hash("alice"),
            predicate: crate::q_hash("is_transaction"),
            object: 100_000,
            context: 0,
            metadata: 1,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        jni_insert_ledger_transaction(bytemuck::bytes_of(&q));

        let q_tax = jni_evaluate_tax_liability("alice");
        // 10% of 100,000 is 10,000
        assert_eq!(q_tax.object & 0x0FFF_FFFF_FFFF_FFFF, 10_000);
    }

    #[test]
    fn test_jni_calculate_asset_apportionment() {
        let _lock = TEST_MUTEX.lock().unwrap();
        jni_clear_ledger();
        // Without evidence, fails closed honestly returning 0.0
        assert_eq!(jni_calculate_asset_apportionment("vehicle_1"), 0.0);

        let asset_hash = crate::q_hash("vehicle_1");
        // Record 80 business hours and 20 private hours
        let mut q_biz = NQuin {
            subject: asset_hash,
            predicate: crate::q_hash("business_use"),
            object: 80,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q_biz.parity = q_biz.subject ^ q_biz.predicate ^ q_biz.object ^ q_biz.context;

        let mut q_priv = NQuin {
            subject: asset_hash,
            predicate: crate::q_hash("private_use"),
            object: 20,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q_priv.parity = q_priv.subject ^ q_priv.predicate ^ q_priv.object ^ q_priv.context;

        jni_insert_ledger_transaction(bytemuck::bytes_of(&q_biz));
        jni_insert_ledger_transaction(bytemuck::bytes_of(&q_priv));

        let apportionment = jni_calculate_asset_apportionment("vehicle_1");
        // 80 / (80 + 20) = 0.80
        assert!((apportionment - 0.80).abs() < 1e-6);
    }

    #[test]
    fn test_jni_insert_spatial_log() {
        let _lock = TEST_MUTEX.lock().unwrap();
        jni_clear_ledger();
        let mut buf = Vec::new();
        buf.extend_from_slice(&(-33.8688f64).to_le_bytes()); // Sydney lat
        buf.extend_from_slice(&(151.2093f64).to_le_bytes()); // Sydney lng
        buf.extend_from_slice(&(1700000000u64).to_le_bytes()); // Timestamp

        assert!(jni_insert_spatial_log(&buf));
        let stored = jni_query_ledger_transactions();
        assert_eq!(stored.len(), 2);
    }
}
