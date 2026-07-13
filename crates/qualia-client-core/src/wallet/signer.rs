use crate::wallet::transaction::{Transaction, encode_varint};
use sha2::{Sha256, Digest};
use k256::ecdsa::{SigningKey, signature::Signer};
use bip32::XPrv;

pub const SIGHASH_ALL: u32 = 0x01;
pub const SIGHASH_FORKID: u32 = 0x40;

pub fn double_sha256(data: &[u8]) -> Vec<u8> {
    let hash1 = Sha256::digest(data);
    let hash2 = Sha256::digest(&hash1);
    hash2.to_vec()
}

pub fn hash160(data: &[u8]) -> Vec<u8> {
    use ripemd::{Ripemd160, Digest as _};
    let sha = Sha256::digest(data);
    let rip = Ripemd160::digest(&sha);
    rip.to_vec()
}

pub fn sign_p2pkh_input(
    tx: &Transaction,
    input_index: usize,
    prev_value: u64,
    priv_key: &XPrv,
) -> Vec<u8> {
    let sighash_type = SIGHASH_ALL | SIGHASH_FORKID;

    let mut hash_prevouts = Vec::new();
    for input in &tx.inputs {
        let mut txid_bytes = hex::decode(&input.prev_txid).unwrap_or_else(|_| vec![0; 32]);
        txid_bytes.reverse();
        hash_prevouts.extend_from_slice(&txid_bytes);
        hash_prevouts.extend_from_slice(&input.prev_out_idx.to_le_bytes());
    }
    let hash_prevouts = double_sha256(&hash_prevouts);

    let mut hash_sequence = Vec::new();
    for input in &tx.inputs {
        hash_sequence.extend_from_slice(&input.sequence.to_le_bytes());
    }
    let hash_sequence = double_sha256(&hash_sequence);

    let mut hash_outputs = Vec::new();
    for output in &tx.outputs {
        hash_outputs.extend_from_slice(&output.value.to_le_bytes());
        hash_outputs.extend_from_slice(&encode_varint(output.pk_script.len() as u64));
        hash_outputs.extend_from_slice(&output.pk_script);
    }
    let hash_outputs = double_sha256(&hash_outputs);

    let mut preimage = Vec::new();
    preimage.extend_from_slice(&tx.version.to_le_bytes());
    preimage.extend_from_slice(&hash_prevouts);
    preimage.extend_from_slice(&hash_sequence);

    let input = &tx.inputs[input_index];
    let mut txid_bytes = hex::decode(&input.prev_txid).unwrap_or_else(|_| vec![0; 32]);
    txid_bytes.reverse();
    preimage.extend_from_slice(&txid_bytes);
    preimage.extend_from_slice(&input.prev_out_idx.to_le_bytes());

    // scriptCode for P2PKH: 0x19 0x76 0xa9 0x14 <pubkey_hash> 0x88 0xac
    let pubkey = priv_key.public_key().to_bytes();
    let pubkey_hash = hash160(&pubkey);
    let mut script_code = vec![0x76, 0xa9, 0x14];
    script_code.extend_from_slice(&pubkey_hash);
    script_code.extend_from_slice(&[0x88, 0xac]);
    preimage.extend_from_slice(&encode_varint(script_code.len() as u64));
    preimage.extend_from_slice(&script_code);

    preimage.extend_from_slice(&prev_value.to_le_bytes());
    preimage.extend_from_slice(&input.sequence.to_le_bytes());
    preimage.extend_from_slice(&hash_outputs);
    preimage.extend_from_slice(&tx.lock_time.to_le_bytes());
    preimage.extend_from_slice(&sighash_type.to_le_bytes());

    let sighash = double_sha256(&preimage);

    let private_key_bytes = priv_key.private_key().to_bytes();
    let signing_key = SigningKey::from_slice(private_key_bytes.as_slice()).expect("Valid key");
    let signature: k256::ecdsa::Signature = signing_key.sign(&sighash);
    let mut der_sig = signature.to_der().to_bytes().to_vec();
    
    // Append sighash type byte to DER signature
    der_sig.push(sighash_type as u8);

    // Build final scriptSig: <sig_len><sig><pubkey_len><pubkey>
    let mut script_sig = Vec::new();
    script_sig.push(der_sig.len() as u8);
    script_sig.extend_from_slice(&der_sig);
    script_sig.push(pubkey.len() as u8);
    script_sig.extend_from_slice(&pubkey);

    script_sig
}
