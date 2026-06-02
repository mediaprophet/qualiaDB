#![allow(non_snake_case)]

use jni::JNIEnv;
use jni::objects::{JClass, JString, JByteArray};
use jni::sys::{jstring, jboolean, jdouble};
use crate::cbor_compiler::parse_cbor_ld_to_quin;
use crate::git_bridge;
use crate::spatial_sieve;

// A mock implementation of the JNI bridge for Phase 1.
// In a real scenario, this would query the local graph using QualiaSuperBlock.

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_queryLedgerTransactions(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    // Mock response: A JSON array of transactions
    let mock_json = r#"[
        {"id": "1", "date": "2026-06-01", "payee": "Software Sub", "amount": -15.99, "category": "Software", "currency": "USD"},
        {"id": "2", "date": "2026-06-02", "payee": "Client Payment", "amount": 1500.00, "category": "Income", "currency": "USD"}
    ]"#;
    
    let output = env.new_string(mock_json)
        .expect("Couldn't create java string!");
    
    output.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_insertLedgerTransaction(
    mut env: JNIEnv,
    _class: JClass,
    transaction_json: JString,
) -> jstring {
    let _tx: String = env.get_string(&transaction_json)
        .expect("Couldn't get java string!")
        .into();
    
    // In a real scenario, we parse the JSON and insert as a series of Quins.
    // Return a success JSON message for now.
    let success_json = r#"{"status": "success", "message": "Transaction inserted"}"#;
    
    let output = env.new_string(success_json)
        .expect("Couldn't create java string!");
        
    output.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_insertCborQuin(
    mut env: JNIEnv,
    _class: JClass,
    cbor_bytes: JByteArray,
) -> jboolean {
    let bytes = env.convert_byte_array(&cbor_bytes).unwrap_or_default();
    
    // Pass to the native CBOR-LD parser
    match parse_cbor_ld_to_quin(&bytes) {
        Ok(_quin) => {
            // Quin successfully parsed and (mock) inserted
            1 // JNI true
        },
        Err(_) => {
            0 // JNI false
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_ontology_OntologyManager_loadQ42Ontology(
    mut env: JNIEnv,
    _class: JClass,
    file_path: JString,
) -> jboolean {
    let _path: String = env.get_string(&file_path)
        .expect("Couldn't get java string!")
        .into();
    
    // In a real scenario, this would memory-map the .q42 file into the core DB.
    1 // Return true for success
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_commitProjectState(
    mut env: JNIEnv,
    _class: JClass,
    commit_payload: JString,
) -> jboolean {
    let _payload: String = env.get_string(&commit_payload)
        .expect("Couldn't get java string!")
        .into();
    
    // Generates the Author-Scoped Merkle Signature over the uncommitted Quins
    1 // Return true for success
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_generateGitExport(
    mut env: JNIEnv,
    _class: JClass,
    project_id: JString,
) -> jstring {
    let pid: String = env.get_string(&project_id)
        .expect("Couldn't get java string!")
        .into();
    
    let git_stream = git_bridge::generate_fast_export_stream(&pid);
    
    let output = env.new_string(git_stream)
        .expect("Couldn't create java string!");
        
    output.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_evaluateTaxLiability(
    mut env: JNIEnv,
    _class: JClass,
    identity_nym: JString,
) -> jstring {
    let _nym: String = env.get_string(&identity_nym)
        .expect("Couldn't get java string!")
        .into();
    
    // In production, this spins up the Sentinel VM with the SlgOpcode::ApplyTaxSchema
    // For now, we return a mock evaluation result based on the TaxRuleSchema
    let mock_json_result = r#"{"liability": 10.0, "currency": "AUD"}"#;
    
    let output = env.new_string(mock_json_result)
        .expect("Couldn't create java string!");
        
    output.into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_insertSpatialLog(
    mut env: JNIEnv,
    _class: JClass,
    spatial_json: JString,
) -> jboolean {
    let _log: String = env.get_string(&spatial_json)
        .expect("Couldn't get java string!")
        .into();
    
    // In production, parse JSON and call spatial_sieve::log_spatial_coordinate
    spatial_sieve::log_spatial_coordinate(0.0, 0.0, 0);
    
    1 // Return true
}

#[no_mangle]
pub extern "system" fn Java_com_example_qualia_QualiaCore_calculateAssetApportionment(
    mut env: JNIEnv,
    _class: JClass,
    asset_id: JString,
) -> jdouble {
    let _id: String = env.get_string(&asset_id)
        .expect("Couldn't get java string!")
        .into();
    
    // In production, this pulls the route log and asset bounding box from the DB 
    // and passes it to the GPU Sieve. We mock the result here.
    let mock_apportionment = 0.85; // e.g. 85% Business Use
    
    mock_apportionment as jdouble
}
