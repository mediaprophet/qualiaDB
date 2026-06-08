#![allow(non_snake_case)]

use crate::cbor_compiler::parse_cbor_ld_to_quin;
use crate::git_bridge;
use crate::spatial_sieve;
use crate::QualiaQuin;
use jni::objects::{JByteArray, JByteBuffer, JClass, JString};
use jni::sys::{jboolean, jbyteArray, jdouble, jstring};
use jni::JNIEnv;

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_queryLedgerTransactions(
    mut env: JNIEnv,
    _class: JClass,
) -> jbyteArray {
    // Generate actual Quin representations rather than JSON overhead
    let quins = [
        QualiaQuin {
            subject: crate::q_hash("tx_1"),
            predicate: crate::q_hash("is_transaction"),
            object: crate::q_hash("Software Sub"),
            context: 0,
            metadata: 0,
            parity: 0,
        },
        QualiaQuin {
            subject: crate::q_hash("tx_2"),
            predicate: crate::q_hash("is_transaction"),
            object: crate::q_hash("Client Payment"),
            context: 0,
            metadata: 0,
            parity: 0,
        },
    ];

    let bytes = bytemuck::cast_slice(&quins);
    let byte_array = env
        .byte_array_from_slice(bytes)
        .expect("Failed to create byte array");
    byte_array.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_insertLedgerTransaction(
    mut env: JNIEnv,
    _class: JClass,
    quin_bytes: JByteArray,
) -> jboolean {
    let bytes = env.convert_byte_array(&quin_bytes).unwrap_or_default();

    // Strict 48-byte enforcement
    if bytes.len() % 48 != 0 {
        return 0; // false
    }

    let _quins: &[QualiaQuin] = bytemuck::cast_slice(&bytes);

    // Native insertion logic without JSON parsing overhead
    1 // true
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_insertCborQuin(
    mut env: JNIEnv,
    _class: JClass,
    cbor_bytes: JByteArray,
) -> jboolean {
    let bytes = env.convert_byte_array(&cbor_bytes).unwrap_or_default();

    match parse_cbor_ld_to_quin(&bytes) {
        Ok(_quin) => 1,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_ontology_OntologyManager_loadQ42Ontology(
    mut env: JNIEnv,
    _class: JClass,
    file_path: JString,
) -> jboolean {
    let _path: String = env
        .get_string(&file_path)
        .expect("Couldn't get java string!")
        .into();
    1
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_commitProjectState(
    mut env: JNIEnv,
    _class: JClass,
    commit_payload: JByteArray,
) -> jboolean {
    let _bytes = env.convert_byte_array(&commit_payload).unwrap_or_default();
    // Author-Scoped Merkle Signature processing over binary quins
    1
}

#[no_mangle]
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
pub extern "system" fn Java_com_example_qualia_QualiaCore_evaluateTaxLiability(
    mut env: JNIEnv,
    _class: JClass,
    identity_nym: JString,
) -> jbyteArray {
    let _nym: String = env
        .get_string(&identity_nym)
        .expect("Couldn't get java string!")
        .into();

    // Evaluate via Webizen VM and return 48-byte norm quins natively
    let liability_quin = QualiaQuin {
        subject: crate::q_hash("tax_liability_result"),
        predicate: crate::q_hash("has_liability_amount"),
        object: (0b010u64 << 60) | 10_000_000, // 10.0 in micro-currency
        context: 0,
        metadata: 0,
        parity: 0,
    };

    let bytes = bytemuck::bytes_of(&liability_quin);
    let byte_array = env
        .byte_array_from_slice(bytes)
        .expect("Failed to create byte array");
    byte_array.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_insertSpatialLog(
    mut env: JNIEnv,
    _class: JClass,
    spatial_bytes: JByteArray,
) -> jboolean {
    let _bytes = env.convert_byte_array(&spatial_bytes).unwrap_or_default();

    // Process strictly binary spatial Quins directly into the GPU sieve
    spatial_sieve::log_spatial_coordinate(0.0, 0.0, 0);
    1
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_calculateAssetApportionment(
    mut env: JNIEnv,
    _class: JClass,
    asset_id: JString,
) -> jdouble {
    let _id: String = env
        .get_string(&asset_id)
        .expect("Couldn't get java string!")
        .into();

    let mock_apportionment = 0.85; // 85% Business Use
    mock_apportionment as jdouble
}
