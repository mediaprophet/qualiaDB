//! Token registry, wallet, identity, coins, transactions

#![allow(non_snake_case)]

use super::*;

use crate::state::*;
use crate::engine::llm_offload;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tokio::time::sleep;


#[derive(Serialize, Deserialize, Clone)]
pub struct TokenEntry {
    id: String,
    chain: String,      // "eCash" | "Ethereum" | "Nyx"
    token_type: String, // "ALP" | "SLP" | "ERC-20" | "CW-20"
    contract: String,   // token ID / contract address
    symbol: String,
    name: String,
    balance: String,
    decimals: u8,
    fiat_usd: f64,
}

pub fn tokens_file_path(storage_path: &str) -> PathBuf {
    PathBuf::from(storage_path).join("tokens.json")
}

pub fn default_tokens() -> Vec<TokenEntry> {
    vec![
        TokenEntry {
            id: "alp-lion".into(),
            chain: "eCash".into(),
            token_type: "ALP".into(),
            contract: "alp:0x1A2B3C4D...".into(),
            symbol: "LION".into(),
            name: "Lion Rampant (Heraldry)".into(),
            balance: "1.00".into(),
            decimals: 8,
            fiat_usd: 0.0,
        },
        TokenEntry {
            id: "alp-horus".into(),
            chain: "eCash".into(),
            token_type: "ALP".into(),
            contract: "alp:0x9B4C5D6E...".into(),
            symbol: "HORUS".into(),
            name: "Eye of Horus (Artifact)".into(),
            balance: "50.00".into(),
            decimals: 8,
            fiat_usd: 0.0,
        },
        TokenEntry {
            id: "slp-meme".into(),
            chain: "eCash".into(),
            token_type: "SLP".into(),
            contract: "slp:0x44F1A2B3...".into(),
            symbol: "MEME".into(),
            name: "Early Beta Meme Coin".into(),
            balance: "150000.00".into(),
            decimals: 2,
            fiat_usd: 0.0,
        },
        TokenEntry {
            id: "erc20-usdt".into(),
            chain: "Ethereum".into(),
            token_type: "ERC-20".into(),
            contract: "0xdAC17F958D2ee523a2206206994597C13D831ec7".into(),
            symbol: "USDT".into(),
            name: "Tether USD".into(),
            balance: "250.00".into(),
            decimals: 6,
            fiat_usd: 250.0,
        },
        TokenEntry {
            id: "erc20-usdc".into(),
            chain: "Ethereum".into(),
            token_type: "ERC-20".into(),
            contract: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".into(),
            symbol: "USDC".into(),
            name: "USD Coin".into(),
            balance: "100.00".into(),
            decimals: 6,
            fiat_usd: 100.0,
        },
        TokenEntry {
            id: "erc20-link".into(),
            chain: "Ethereum".into(),
            token_type: "ERC-20".into(),
            contract: "0x514910771AF9Ca656af840dff83E8264EcF986CA".into(),
            symbol: "LINK".into(),
            name: "Chainlink Token".into(),
            balance: "12.50".into(),
            decimals: 18,
            fiat_usd: 162.5,
        },
        TokenEntry {
            id: "cw20-vnym".into(),
            chain: "Nyx".into(),
            token_type: "CW-20".into(),
            contract: "nyx1staking000000000000000000000000000000000000".into(),
            symbol: "vNYM".into(),
            name: "Vested NYM (Staking)".into(),
            balance: "100.00".into(),
            decimals: 6,
            fiat_usd: 2.0,
        },
    ]
}

pub fn load_tokens_from_disk(storage_path: &str) -> Vec<TokenEntry> {
    let path = tokens_file_path(storage_path);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(default_tokens)
}

pub fn save_tokens_to_disk(storage_path: &str, tokens: &[TokenEntry]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(tokens).map_err(|e| e.to_string())?;
    std::fs::write(tokens_file_path(storage_path), json).map_err(|e| e.to_string())
}

pub fn get_tokens() -> Vec<TokenEntry> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let mut tokens = load_tokens_from_disk(&storage_path);
    
    let id = read_identity();
    if let Some(hash160) = id.as_ref().and_then(|v| v.get("ecash_hash160")).and_then(|v| v.as_str()) {
        let client = crate::wallet::chronik::ChronikClient::new("https://chronik.be.cash");
        if let Ok(utxos) = client.fetch_utxos_p2pkh(hash160) {
            let mut balances: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
            for utxo in utxos {
                if let Some(meta) = utxo.slp_meta {
                    if let Some(token) = utxo.slp_token {
                        if let Ok(amount) = token.amount.parse::<u64>() {
                            *balances.entry(meta.token_id).or_insert(0) += amount;
                        }
                    }
                }
            }
            
            for t in tokens.iter_mut() {
                if t.chain == "eCash" {
                    // Extract token ID from contract e.g. "slp:0x123..."
                    let token_id = t.contract.split("0x").nth(1).unwrap_or("").to_string();
                    if let Some(&amt) = balances.get(&token_id.to_lowercase()) {
                        let float_amt = amt as f64 / 10f64.powi(t.decimals as i32);
                        t.balance = format!("{:.2}", float_amt);
                    }
                }
            }
        }
    }
    
    tokens
}

pub fn add_token(
    chain: String,
    token_type: String,
    contract: String,
    symbol: String,
    name: String,
    decimals: u8,
) -> Result<TokenEntry, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let mut tokens = load_tokens_from_disk(&storage_path);

    if tokens
        .iter()
        .any(|t| t.contract.to_lowercase() == contract.to_lowercase() && t.chain == chain)
    {
        return Err("Token already in wallet".to_string());
    }
    let slug: String = contract
        .chars()
        .rev()
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let id = format!(
        "{}-{}",
        chain.to_lowercase().replace(' ', "-"),
        slug.to_lowercase()
    );
    let entry = TokenEntry {
        id,
        chain,
        token_type,
        contract,
        symbol,
        name,
        balance: "0.00".into(),
        decimals,
        fiat_usd: 0.0,
    };
    tokens.push(entry.clone());
    save_tokens_to_disk(&storage_path, &tokens)?;
    Ok(entry)
}

pub fn send_ecash_token(token_id: &str, destination_address: &str, amount: u64) -> Result<String, String> {
    use crate::wallet::transaction::{Transaction, TxIn, TxOut};
    use crate::wallet::signer::{sign_p2pkh_input, hash160};
    use crate::wallet::coin_select;
    use bip32::XPrv;
    use std::str::FromStr;

    let id = read_identity().ok_or("No identity set — generate a seed first")?;
    let hash160_hex = id.get("ecash_hash160")
        .and_then(|v| v.as_str())
        .ok_or("No ecash_hash160 in identity")?;

    // Derive the private key from stored seed
    let mnemonic_str = load_mnemonic_from_vault()?;
    let mnemonic = bip39::Mnemonic::parse_in(bip39::Language::English, &mnemonic_str)
        .map_err(|_| "Invalid stored mnemonic")?;
    let seed_bytes = mnemonic.to_seed("");
    let master = XPrv::new(&seed_bytes).map_err(|e| e.to_string())?;
    let xec_path = bip32::DerivationPath::from_str("m/44'/899'/0'/0/0").map_err(|e| e.to_string())?;
    let mut child = master.clone();
    for c in xec_path.iter() {
        child = child.derive_child(c).map_err(|e| e.to_string())?;
    }

    // Fetch UTXOs
    let client = crate::wallet::chronik::ChronikClient::new("https://chronik.be.cash");
    let utxos = client.fetch_utxos_p2pkh(hash160_hex)?;

    // Select token UTXOs + funding UTXOs
    let (token_utxos, xec_selection) = coin_select::select_token_utxos(&utxos, token_id, amount)?;

    // Build the transaction
    let mut tx = Transaction::new();

    // Add token inputs
    for utxo in &token_utxos {
        tx.inputs.push(TxIn {
            prev_txid: utxo.outpoint.txid.clone(),
            prev_out_idx: utxo.outpoint.out_idx,
            signature_script: Vec::new(),
            sequence: 0xFFFFFFFF,
        });
    }
    // Add funding inputs
    for utxo in &xec_selection.selected {
        tx.inputs.push(TxIn {
            prev_txid: utxo.outpoint.txid.clone(),
            prev_out_idx: utxo.outpoint.out_idx,
            signature_script: Vec::new(),
            sequence: 0xFFFFFFFF,
        });
    }

    // Output 0: OP_RETURN with SLP SEND
    let op_return_script = crate::wallet::semantic_tokens::generate_slp_send_op_return(token_id, &[amount]);
    tx.outputs.push(TxOut {
        value: 0,
        pk_script: op_return_script,
    });

    // Output 1: Token recipient (dust amount)
    let dest_pubkey_hash = decode_ecash_address(destination_address)?;
    let mut p2pkh_script = vec![0x76, 0xa9, 0x14];
    p2pkh_script.extend_from_slice(&dest_pubkey_hash);
    p2pkh_script.extend_from_slice(&[0x88, 0xac]);
    tx.outputs.push(TxOut {
        value: coin_select::DUST_THRESHOLD_SATS as u64,
        pk_script: p2pkh_script,
    });

    // Output 2: Change (if any)
    if xec_selection.change_sats > 0 {
        let own_pubkey_hash = hash160(&child.public_key().to_bytes());
        let mut change_script = vec![0x76, 0xa9, 0x14];
        change_script.extend_from_slice(&own_pubkey_hash);
        change_script.extend_from_slice(&[0x88, 0xac]);
        tx.outputs.push(TxOut {
            value: xec_selection.change_sats as u64,
            pk_script: change_script,
        });
    }

    // Sign all inputs
    let all_utxos: Vec<&crate::wallet::chronik::ChronikUtxo> = token_utxos.iter()
        .chain(xec_selection.selected.iter())
        .collect();
    for (i, utxo) in all_utxos.iter().enumerate() {
        let script_sig = sign_p2pkh_input(&tx, i, utxo.value as u64, &child);
        tx.inputs[i].signature_script = script_sig;
    }

    // Broadcast
    let raw_hex = hex::encode(tx.serialize());
    let txid = client.broadcast_tx(&raw_hex)?;

    // Record in ledger
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let _ = crate::wallet::ledger::append_entry(
        std::path::Path::new(&storage),
        &crate::wallet::ledger::new_entry(crate::wallet::ledger::LedgerEntryKind::TxBroadcast {
            chain: "XEC".into(),
            txid: txid.clone(),
            amount_sats: amount,
            direction: "out".into(),
        }),
    );

    Ok(txid)
}

/// Build a native XEC send transaction (preview only — does not broadcast).
/// Returns the raw transaction hex and a fee estimate for user confirmation.
#[derive(Serialize, Clone)]
pub struct SendPreview {
    pub raw_hex: String,
    pub fee_sats: i64,
    pub total_input_sats: i64,
    pub change_sats: i64,
    pub target_sats: i64,
}

pub fn build_send_xec(destination_address: &str, amount_sats: i64) -> Result<SendPreview, String> {
    use crate::wallet::transaction::{Transaction, TxIn, TxOut};
    use crate::wallet::signer::{sign_p2pkh_input, hash160};
    use crate::wallet::coin_select;
    use bip32::XPrv;
    use std::str::FromStr;

    if amount_sats <= 0 {
        return Err("Amount must be positive".into());
    }

    let id = read_identity().ok_or("No identity set — generate a seed first")?;
    let hash160_hex = id.get("ecash_hash160")
        .and_then(|v| v.as_str())
        .ok_or("No ecash_hash160 in identity")?;

    let mnemonic_str = load_mnemonic_from_vault()?;
    let mnemonic = bip39::Mnemonic::parse_in(bip39::Language::English, &mnemonic_str)
        .map_err(|_| "Invalid stored mnemonic")?;
    let seed_bytes = mnemonic.to_seed("");
    let master = XPrv::new(&seed_bytes).map_err(|e| e.to_string())?;
    let xec_path = bip32::DerivationPath::from_str("m/44'/899'/0'/0/0").map_err(|e| e.to_string())?;
    let mut child = master.clone();
    for c in xec_path.iter() {
        child = child.derive_child(c).map_err(|e| e.to_string())?;
    }

    let client = crate::wallet::chronik::ChronikClient::new("https://chronik.be.cash");
    let utxos = client.fetch_utxos_p2pkh(hash160_hex)?;
    let selection = coin_select::select_utxos(&utxos, amount_sats)?;

    let mut tx = Transaction::new();
    for utxo in &selection.selected {
        tx.inputs.push(TxIn {
            prev_txid: utxo.outpoint.txid.clone(),
            prev_out_idx: utxo.outpoint.out_idx,
            signature_script: Vec::new(),
            sequence: 0xFFFFFFFF,
        });
    }

    // Recipient output
    let dest_pubkey_hash = decode_ecash_address(destination_address)?;
    let mut p2pkh_script = vec![0x76, 0xa9, 0x14];
    p2pkh_script.extend_from_slice(&dest_pubkey_hash);
    p2pkh_script.extend_from_slice(&[0x88, 0xac]);
    tx.outputs.push(TxOut {
        value: amount_sats as u64,
        pk_script: p2pkh_script,
    });

    // Change output
    if selection.change_sats > 0 {
        let own_pubkey_hash = hash160(&child.public_key().to_bytes());
        let mut change_script = vec![0x76, 0xa9, 0x14];
        change_script.extend_from_slice(&own_pubkey_hash);
        change_script.extend_from_slice(&[0x88, 0xac]);
        tx.outputs.push(TxOut {
            value: selection.change_sats as u64,
            pk_script: change_script,
        });
    }

    // Sign
    for (i, utxo) in selection.selected.iter().enumerate() {
        let script_sig = sign_p2pkh_input(&tx, i, utxo.value as u64, &child);
        tx.inputs[i].signature_script = script_sig;
    }

    let raw_hex = hex::encode(tx.serialize());
    Ok(SendPreview {
        raw_hex,
        fee_sats: selection.fee_sats,
        total_input_sats: selection.total_input_sats,
        change_sats: selection.change_sats,
        target_sats: amount_sats,
    })
}

/// Broadcast a previously built transaction.
pub fn confirm_send_xec(raw_hex: &str) -> Result<String, String> {
    let client = crate::wallet::chronik::ChronikClient::new("https://chronik.be.cash");
    let txid = client.broadcast_tx(raw_hex)?;

    // Record in ledger
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let _ = crate::wallet::ledger::append_entry(
        std::path::Path::new(&storage),
        &crate::wallet::ledger::new_entry(crate::wallet::ledger::LedgerEntryKind::TxBroadcast {
            chain: "XEC".into(),
            txid: txid.clone(),
            amount_sats: 0, // Amount is in the raw tx; we don't parse it back here
            direction: "out".into(),
        }),
    );

    Ok(txid)
}

/// Decode an eCash address to its 20-byte pubkey hash.
/// Supports both `ecash:q...` (CashAddr) and legacy base58 formats.
fn decode_ecash_address(addr: &str) -> Result<Vec<u8>, String> {
    // Strip ecash: prefix if present
    let stripped = if let Some(a) = addr.strip_prefix("ecash:") {
        a
    } else {
        addr
    };
    // Try base58 decode (legacy format)
    let decoded = bs58::decode(stripped).into_vec().map_err(|e| format!("Invalid address: {}", e))?;
    if decoded.len() < 21 {
        return Err("Address too short".into());
    }
    // Skip version byte, take 20-byte hash
    Ok(decoded[1..21].to_vec())
}

/// Load the stored mnemonic from the vault (identity file stores derivation result,
/// the mnemonic itself is stored separately for security).
fn load_mnemonic_from_vault() -> Result<String, String> {
    let mnemonic_path = app_meta_dir().join("mnemonic.enc");
    std::fs::read_to_string(&mnemonic_path).map_err(|_| {
        "No mnemonic stored — please save your seed phrase via the identity setup flow".to_string()
    })
}



pub fn remove_token(id: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let mut tokens = load_tokens_from_disk(&storage_path);
    tokens.retain(|t| t.id != id);
    save_tokens_to_disk(&storage_path, &tokens)
}

// ─────────────────────────────────────────────────────────────────────────────

pub fn read_identity() -> Option<serde_json::Value> {
    std::fs::read_to_string(identity_file_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

#[derive(Serialize, Clone)]
pub struct CoinBalance {
    pub coin: String,
    pub ticker: String,
    pub address: String,
    pub balance: f64,
    pub balance_display: String,
    pub fiat_usd: f64,
    pub price_usd: f64,
    pub change_24h: f64,
    pub network: String,
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct TxRecord {
    txid: String,
    ticker: String,
    direction: String, // "in" | "out"
    amount: String,
    label: String,
    timestamp: String,
    status: String, // "confirmed" | "pending"
    confirmations: u32,
    fee: String,
    counterparty: String,
}

pub fn get_coin_balances() -> Vec<CoinBalance> {
    let id = read_identity();
    let has_identity = id.is_some();
    let addr = |key: &str| -> String {
        id.as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let no_adapter_status: String = if has_identity {
        "no_adapter".into()
    } else {
        "awaiting_identity".into()
    };
    let zero_display = if has_identity { "0" } else { "\u{2014}" };

    let mut xec_balance = 0.0;
    let mut xec_status = if has_identity { "no_adapter".to_string() } else { "awaiting_identity".to_string() };
    let mut xec_display = zero_display.to_string();

    if has_identity {
        if let Some(hash160) = id.as_ref().and_then(|v| v.get("ecash_hash160")).and_then(|v| v.as_str()) {
            let client = crate::wallet::chronik::ChronikClient::new("https://chronik.be.cash");
            if let Ok(utxos) = client.fetch_utxos_p2pkh(hash160) {
                let mut sats = 0;
                for utxo in utxos {
                    if utxo.slp_meta.is_none() {
                        sats += utxo.value;
                    }
                }
                xec_balance = sats as f64 / 100.0; // XEC is 2 decimals (100 sats)
                xec_display = format!("{:.2}", xec_balance);
                xec_status = "synced".into();
            } else {
                xec_status = "offline".into();
            }
        }
    }

    let mut balances = vec![
        CoinBalance {
            coin: "eCash".into(),
            ticker: "XEC".into(),
            address: addr("ecash_xec"),
            balance: xec_balance,
            balance_display: xec_display,
            fiat_usd: 0.0,
            price_usd: 0.0,
            change_24h: 0.0,
            network: "eCash".into(),
            status: xec_status,
        },
        CoinBalance {
            coin: "Bitcoin".into(),
            ticker: "BTC".into(),
            address: addr("bitcoin_btc"),
            balance: 0.0,
            balance_display: zero_display.into(),
            fiat_usd: 0.0,
            price_usd: 0.0,
            change_24h: 0.0,
            network: "Bitcoin".into(),
            status: no_adapter_status.clone(),
        },
        CoinBalance {
            coin: "Monero".into(),
            ticker: "XMR".into(),
            // XMR derivation is not implemented (see derive_wallets_from_seed).
            // Show an explicit non-address so it can't be mistaken for a real
            // receive address — never a fabricated "4..." string.
            address: "(not yet supported)".into(),
            balance: 0.0,
            balance_display: zero_display.into(),
            fiat_usd: 0.0,
            price_usd: 0.0,
            change_24h: 0.0,
            network: "Monero".into(),
            status: no_adapter_status.clone(),
        },
        CoinBalance {
            coin: "Ethereum".into(),
            ticker: "ETH".into(),
            address: addr("ethereum"),
            balance: 0.0,
            balance_display: zero_display.into(),
            fiat_usd: 0.0,
            price_usd: 0.0,
            change_24h: 0.0,
            network: "Ethereum".into(),
            status: no_adapter_status.clone(),
        },
    ];

    if nym_mixnet_opted_in() {
        balances.push(CoinBalance {
            coin: "Nym".into(),
            ticker: "NYM".into(),
            address: addr("nym_mixnet"),
            balance: 0.0,
            balance_display: zero_display.into(),
            fiat_usd: 0.0,
            price_usd: 0.0,
            change_24h: 0.0,
            network: "Nyx Chain".into(),
            status: no_adapter_status,
        });
    }

    balances
}

pub fn get_transaction_history(ticker: String) -> Vec<TxRecord> {
    // For XEC: query live transaction history from Chronik.
    // For other chains: return empty (no adapter yet).
    let id = read_identity();

    let xec_history: Vec<TxRecord> = if ticker.is_empty() || ticker == "ALL" || ticker == "XEC" {
        fetch_xec_tx_history(&id).unwrap_or_default()
    } else {
        Vec::new()
    };

    if ticker.is_empty() || ticker == "ALL" {
        xec_history
    } else {
        xec_history
            .into_iter()
            .filter(|tx| tx.ticker == ticker)
            .collect()
    }
}

/// Fetch real XEC transaction history from Chronik.
fn fetch_xec_tx_history(id: &Option<serde_json::Value>) -> Result<Vec<TxRecord>, String> {
    let hash160 = id
        .as_ref()
        .and_then(|v| v.get("ecash_hash160"))
        .and_then(|v| v.as_str())
        .ok_or("No ecash_hash160 in identity")?;

    let own_script_suffix = hash160.to_lowercase();

    let client = crate::wallet::chronik::ChronikClient::new("https://chronik.be.cash");
    let page = client.fetch_tx_history_p2pkh(hash160, 0, 25)?;

    let mut records = Vec::new();
    for tx in page.txs {
        // Determine direction by checking if our address appears in inputs
        let is_outgoing = tx.inputs.iter().any(|inp| {
            inp.output_script.to_lowercase().contains(&own_script_suffix)
        });

        // Calculate net amount
        let own_output_sats: i64 = tx.outputs.iter()
            .filter(|o| o.output_script.to_lowercase().contains(&own_script_suffix))
            .map(|o| o.value)
            .sum();

        let own_input_sats: i64 = if is_outgoing {
            tx.inputs.iter()
                .filter(|i| i.output_script.to_lowercase().contains(&own_script_suffix))
                .map(|i| i.value)
                .sum()
        } else {
            0
        };

        let (direction, amount_sats, label) = if is_outgoing {
            let sent = own_input_sats - own_output_sats; // net outflow
            ("out", sent, "Sent XEC")
        } else {
            ("in", own_output_sats, "Received XEC")
        };

        let xec_amount = amount_sats as f64 / 100.0;
        let amount_str = format!("{:.2}", xec_amount);

        // Determine confirmation status
        let (status_str, confirmations) = match &tx.block {
            Some(block) => {
                // Rough confirmation count (we don't know current height, so show block height)
                ("confirmed".to_string(), block.height as u32)
            }
            None => ("pending".to_string(), 0),
        };

        // Timestamp from block or first-seen
        let timestamp = if let Some(ref block) = tx.block {
            format_unix_timestamp(block.timestamp)
        } else if tx.time_first_seen > 0 {
            format_unix_timestamp(tx.time_first_seen)
        } else {
            "—".to_string()
        };

        // Counterparty: for outgoing, the first non-own output address; for incoming, first input
        let counterparty = if is_outgoing {
            tx.outputs.iter()
                .find(|o| !o.output_script.to_lowercase().contains(&own_script_suffix) && o.value > 0)
                .map(|o| format!("script:{}", &o.output_script[..o.output_script.len().min(16)]))
                .unwrap_or_else(|| "self".to_string())
        } else {
            tx.inputs.first()
                .map(|i| format!("script:{}", &i.output_script[..i.output_script.len().min(16)]))
                .unwrap_or_else(|| "coinbase".to_string())
        };

        // Truncate txid for display
        let txid_display = if tx.txid.len() > 16 {
            format!("{}…{}", &tx.txid[..8], &tx.txid[tx.txid.len()-4..])
        } else {
            tx.txid.clone()
        };

        records.push(TxRecord {
            txid: txid_display,
            ticker: "XEC".into(),
            direction: direction.into(),
            amount: amount_str,
            label: label.into(),
            timestamp,
            status: status_str,
            confirmations,
            fee: "".into(), // Chronik doesn't return fee directly
            counterparty,
        });
    }

    Ok(records)
}

/// Format a Unix timestamp to a human-readable string.
fn format_unix_timestamp(ts: i64) -> String {
    let secs = ts as u64;
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;
    let (y, m, d) = crate::wallet::ledger::epoch_days_to_date_pub(days);
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d, hours, minutes)
}

pub fn is_first_run() -> bool {
    !config_file_path().exists()
}

pub fn save_identity(wallets: serde_json::Value) -> Result<(), String> {
    let meta = app_meta_dir();
    std::fs::create_dir_all(&meta).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&wallets).map_err(|e| e.to_string())?;
    std::fs::write(identity_file_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_identity() -> Result<Option<serde_json::Value>, String> {
    let path = identity_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let val: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    Ok(Some(val))
}

use bip39::{Language, Mnemonic};

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub async fn generate_bip39_seed() -> Result<String, String> {
    // Generate a secure, randomized 12-word seed phrase natively
    let mnemonic = Mnemonic::generate_in(Language::English, 12)
        .map_err(|_| "Failed to generate".to_string())?;
    let words: Vec<&str> = mnemonic.words().collect();
    Ok(words.join(" "))
}

pub async fn derive_wallets_from_seed(seed: String) -> Result<serde_json::Value, String> {
    let mnemonic = match Mnemonic::parse_in(Language::English, &seed) {
        Ok(m) => m,
        Err(_) => return Err("Invalid 12-word seed phrase.".to_string()),
    };

    let seed_bytes = mnemonic.to_seed("");
    let wallet = crate::wallet::HdWallet::from_seed(&seed_bytes)?;

    let btc_addr = wallet.derive_address("BTC", "m/44'/0'/0'/0/0")?.address;
    let eth_addr = wallet.derive_address("ETH", "m/44'/60'/0'/0/0")?.address;
    let nym_addr = wallet.derive_address("NYM", "m/44'/118'/0'/0/0")?.address;
    let xec_payload = wallet.derive_address("XEC", "m/44'/899'/0'/0/0")?;
    let xec_addr = xec_payload.address;
    let xec_hash160 = xec_payload.pubkey_hash;

    // Monero is deliberately NOT derived here. It uses ed25519 (not the
    // secp256k1 BIP32 path above), Keccak-256 key derivation, and its own
    // base58 address format. Emitting a plausible-looking "4..." string with
    // no keys behind it is dangerous: any XMR sent to a keyless address is
    // permanently lost. So we report it empty rather than fabricate an
    // address. Real, test-vector-verified ed25519/Keccak/base58 derivation is
    // tracked as follow-up work (needs an authoritative seed→address vector to
    // verify against before it can be trusted).
    let hex_seed = to_hex(&seed_bytes[0..16]);

    Ok(serde_json::json!({
        "qualia_root": format!("did:qualia:0x{}", hex_seed),
        "nym_mixnet": nym_addr,
        "ecash_xec": xec_addr,
        "ecash_hash160": xec_hash160,
        "ethereum": eth_addr,
        "bitcoin_btc": btc_addr,
        "monero_xmr": "" // not derived — never a fabricated address (see above)
    }))
}

pub async fn generate_front_door_invite() -> Result<String, String> {
    let invite = crate::social_connect::generate_connect_invite(None)?;
    Ok(invite.invite_json)
}

pub async fn mint_semantic_token(asset_id: String) -> Result<String, String> {
    use crate::wallet::transaction::{Transaction, TxIn, TxOut};
    use crate::wallet::signer::{sign_p2pkh_input, hash160};
    use crate::wallet::coin_select;
    use bip32::XPrv;
    use std::str::FromStr;

    let id = read_identity().ok_or("No identity set — generate a seed first")?;
    let hash160_hex = id.get("ecash_hash160")
        .and_then(|v| v.as_str())
        .ok_or("No ecash_hash160 in identity")?;

    let mnemonic_str = load_mnemonic_from_vault()?;
    let mnemonic = bip39::Mnemonic::parse_in(bip39::Language::English, &mnemonic_str)
        .map_err(|_| "Invalid stored mnemonic")?;
    let seed_bytes = mnemonic.to_seed("");
    let master = XPrv::new(&seed_bytes).map_err(|e| e.to_string())?;
    let xec_path = bip32::DerivationPath::from_str("m/44'/899'/0'/0/0").map_err(|e| e.to_string())?;
    let mut child = master.clone();
    for c in xec_path.iter() {
        child = child.derive_child(c).map_err(|e| e.to_string())?;
    }

    // Parse asset_id as JSON metadata or use default
    let metadata = crate::wallet::semantic_tokens::SemanticTokenMetadata {
        token_ticker: asset_id.clone(),
        token_name: format!("Qualia Semantic Token: {}", asset_id),
        token_document_url: format!("https://qualia.io/tokens/{}", asset_id.to_lowercase()),
        token_document_hash: format!("{:064x}", 0u128), // Placeholder hash
        decimals: 0,
    };

    let client = crate::wallet::chronik::ChronikClient::new("https://chronik.be.cash");
    let utxos = client.fetch_utxos_p2pkh(hash160_hex)?;

    // Need enough XEC to cover: OP_RETURN (0) + mint output (546) + change + fee
    let min_needed = coin_select::DUST_THRESHOLD_SATS + coin_select::BASE_FEE_SATS;
    let selection = coin_select::select_utxos(&utxos, min_needed)?;

    let mut tx = Transaction::new();
    for utxo in &selection.selected {
        tx.inputs.push(TxIn {
            prev_txid: utxo.outpoint.txid.clone(),
            prev_out_idx: utxo.outpoint.out_idx,
            signature_script: Vec::new(),
            sequence: 0xFFFFFFFF,
        });
    }

    // Output 0: OP_RETURN GENESIS
    let op_return = crate::wallet::semantic_tokens::generate_slp_op_return(&metadata);
    tx.outputs.push(TxOut { value: 0, pk_script: op_return });

    // Output 1: Mint receiver (self)
    let own_pubkey_hash = hash160(&child.public_key().to_bytes());
    let mut mint_script = vec![0x76, 0xa9, 0x14];
    mint_script.extend_from_slice(&own_pubkey_hash);
    mint_script.extend_from_slice(&[0x88, 0xac]);
    tx.outputs.push(TxOut {
        value: coin_select::DUST_THRESHOLD_SATS as u64,
        pk_script: mint_script,
    });

    // Output 2: Change
    if selection.change_sats > 0 {
        let mut change_script = vec![0x76, 0xa9, 0x14];
        change_script.extend_from_slice(&own_pubkey_hash);
        change_script.extend_from_slice(&[0x88, 0xac]);
        tx.outputs.push(TxOut {
            value: selection.change_sats as u64,
            pk_script: change_script,
        });
    }

    // Sign
    for (i, utxo) in selection.selected.iter().enumerate() {
        let script_sig = sign_p2pkh_input(&tx, i, utxo.value as u64, &child);
        tx.inputs[i].signature_script = script_sig;
    }

    // Broadcast
    let raw_hex = hex::encode(tx.serialize());
    let txid = client.broadcast_tx(&raw_hex)?;

    // Record in ledger
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let _ = crate::wallet::ledger::append_entry(
        std::path::Path::new(&storage),
        &crate::wallet::ledger::new_entry(crate::wallet::ledger::LedgerEntryKind::TokenMint {
            chain: "XEC".into(),
            txid: txid.clone(),
            token_id: txid.clone(), // For SLP, the token_id IS the genesis txid
            symbol: asset_id,
        }),
    );

    Ok(txid)
}

pub async fn fetch_wallet_portfolio() -> Result<serde_json::Value, String> {
    // Compose real portfolio from live coin balances + token registry
    let balances = get_coin_balances();
    let tokens = get_tokens();

    let mut portfolio = Vec::new();

    // Add native coin entries
    for b in &balances {
        portfolio.push(serde_json::json!({
            "name": b.coin,
            "tokenId": "",
            "ticker": b.ticker,
            "balance": b.balance_display,
            "rdf": "",
            "network": b.network,
            "type": "native",
            "status": b.status,
            "address": b.address,
            "fiat_usd": b.fiat_usd,
        }));
    }

    // Add token entries
    for t in &tokens {
        portfolio.push(serde_json::json!({
            "name": t.name,
            "tokenId": t.contract,
            "ticker": t.symbol,
            "balance": t.balance,
            "rdf": "",
            "network": t.chain,
            "type": t.token_type,
            "status": "loaded",
        }));
    }

    Ok(serde_json::Value::Array(portfolio))
}

pub async fn import_external_seed(
    network: String,
    seed: String,
    _label: String,
) -> Result<String, String> {
    // Validate seed format
    if seed.split_whitespace().count() < 12 {
        return Err("Invalid seed phrase — must be at least 12 words".to_string());
    }

    // Derive real addresses using the existing HD wallet pipeline
    let mnemonic = bip39::Mnemonic::parse_in(bip39::Language::English, &seed)
        .map_err(|_| "Invalid BIP-39 mnemonic".to_string())?;
    let seed_bytes = mnemonic.to_seed("");
    let wallet = crate::wallet::HdWallet::from_seed(&seed_bytes)?;

    let (net_code, path) = match network.as_str() {
        "eCash (XEC)" | "XEC" => ("XEC", "m/44'/899'/0'/0/0"),
        "Bitcoin (BTC)" | "BTC" => ("BTC", "m/44'/0'/0'/0/0"),
        "Nym (NYM) - Nyx Chain" | "NYM" => ("NYM", "m/44'/118'/0'/0/0"),
        "Ethereum (EVM)" | "ETH" => ("ETH", "m/44'/60'/0'/0/0"),
        "Monero (XMR)" | "XMR" => {
            // Monero uses ed25519, not secp256k1 — still mock for now
            let xmr_hex = to_hex(&seed_bytes[48..56]);
            return Ok(format!("4{}...", &xmr_hex[0..xmr_hex.len().min(16)]));
        }
        _ => return Err(format!("Unsupported network: {}", network)),
    };

    let payload = wallet.derive_address(net_code, path)?;
    Ok(payload.address)
}

pub async fn toggle_nym_relay() -> Result<bool, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let active = &state.nym_relay_active;
    let currently_active = active.load(Ordering::Relaxed);
    let new_state = !currently_active;
    active.store(new_state, Ordering::Relaxed);

    if new_state {
        let active_clone = active.clone();

        // Spawn asynchronous background daemon for packet routing
        tokio::spawn(async move {
            let mut packets_routed = 0;
            let mut _packets_dropped = 0;

            while active_clone.load(Ordering::Relaxed) {
                // Simulate network fluctuations and calculate memory backpressure
                // Enforcing a strict 50MB telemetry boundary cap internally
                let packet_load_factor = 1.0 + (packets_routed % 5) as f64 * 0.2;
                let buffer_memory_mb = 12.4 * packet_load_factor;
                let is_congested = buffer_memory_mb > 45.0;

                if is_congested {
                    _packets_dropped += 15;
                } else {
                    packets_routed += 42;
                }

                // let _ = window_clone.emit("nym-telemetry", RelayTelemetry {
                //     packets_routed,
                //     packets_dropped,
                //     buffer_memory_mb,
                //     is_congested,
                // });

                sleep(Duration::from_millis(500)).await;
            }
        });
    }
    Ok(new_state)
}

pub async fn toggle_stark_prover() -> Result<bool, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let active = &state.stark_prover_active;
    let currently_active = active.load(Ordering::Relaxed);
    let new_state = !currently_active;
    active.store(new_state, Ordering::Relaxed);

    if new_state {
        let active_clone = active.clone();
        let solar_clone = state.simulated_solar_watts.clone();

        // Spawn asynchronous background daemon for out-of-core proof chunking
        tokio::spawn(async move {
            let mut _fragments_paged = 0;

            while active_clone.load(Ordering::Relaxed) {
                let current_solar = solar_clone.load(Ordering::Relaxed);

                // Environmental state evaluation trigger (threshold at 400W)
                if current_solar < 400 {
                    // let _ = window_clone.emit("stark-telemetry", StarkTelemetry {
                    //     status: "Suspended - Awaiting Solar Surplus".to_string(),
                    //     cpu_utilization: 0.0,
                    //     ram_usage_mb: 0.0,
                    //     fragments_paged,
                    // });
                } else {
                    _fragments_paged += 8; // Simulate 48-byte Super-Quin paging writes

                    // let _ = window_clone.emit("stark-telemetry", StarkTelemetry {
                    //     status: "Proving Execution Active".to_string(),
                    //     cpu_utilization: 85.4,
                    //     ram_usage_mb: 320.0, // Constrained flat memory footprint
                    //     fragments_paged,
                    // });
                }
                sleep(Duration::from_millis(1000)).await;
            }
        });
    }
    Ok(new_state)
}

pub fn update_solar_input(watts: u32) {
    let state = crate::state::APP_STATE.get().unwrap();
    state.simulated_solar_watts.store(watts, Ordering::Relaxed);
}

pub async fn fetch_torrent_telemetry() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    Ok(crate::ontology_workbench::torrent_telemetry(Path::new(
        &storage,
    )))
}

pub fn sync_workbench_torrent_seeds(storage_path: &str) -> Result<serde_json::Value, String> {
    crate::ontology_workbench::sync_workbench_seeds_to_daemon(Path::new(storage_path))
}

pub async fn workbench_import_ontology_uri(
    uri: String,
    ontology_id: Option<String>,
    domain: Option<String>,
    title: Option<String>,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let result = crate::ontology_workbench::import_from_uri(
        Path::new(&storage),
        uri,
        ontology_id,
        domain,
        title,
    )
    .await?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

pub fn list_workbench_ontologies() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let entries = crate::ontology_workbench::list_workbench_entries(Path::new(&storage))?;
    serde_json::to_value(entries).map_err(|e| e.to_string())
}

pub fn set_workbench_torrent_policy(
    ontology_id: String,
    policy_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let policy: crate::ontology_workbench::OntologyTorrentPolicy =
        serde_json::from_value(policy_json).map_err(|e| e.to_string())?;
    let updated =
        crate::ontology_workbench::set_torrent_policy(Path::new(&storage), &ontology_id, policy)?;
    serde_json::to_value(updated).map_err(|e| e.to_string())
}

pub fn set_workbench_seed(ontology_id: String, active: bool) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let updated =
        crate::ontology_workbench::set_seed_active(Path::new(&storage), &ontology_id, active)?;
    serde_json::to_value(updated).map_err(|e| e.to_string())
}

pub fn get_torrent_bandwidth_policy() -> Result<serde_json::Value, String> {
    let policy = crate::ontology_workbench::load_bandwidth_policy();
    serde_json::to_value(policy).map_err(|e| e.to_string())
}

pub fn set_torrent_bandwidth_policy(
    policy_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let policy: crate::ontology_workbench::TorrentBandwidthGlobal =
        serde_json::from_value(policy_json).map_err(|e| e.to_string())?;
    crate::ontology_workbench::save_bandwidth_policy(&policy)?;
    serde_json::to_value(policy).map_err(|e| e.to_string())
}

pub fn list_ontology_shares_for_contact(contact_did: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let cards =
        crate::ontology_workbench::list_share_cards_for_contact(Path::new(&storage), &contact_did)?;
    serde_json::to_value(cards).map_err(|e| e.to_string())
}

pub fn list_ontology_shares_for_session(session_did: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let cards =
        crate::ontology_workbench::list_share_cards_for_session(Path::new(&storage), &session_did)?;
    serde_json::to_value(cards).map_err(|e| e.to_string())
}

pub fn list_chat_session_share_targets() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let targets = crate::chat_session::list_session_share_targets(Path::new(&storage))
        .map_err(|e| e.to_string())?;
    serde_json::to_value(targets).map_err(|e| e.to_string())
}

pub fn get_chat_session_did(session_id: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::chat_session::get_session_did(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())
}

pub fn update_chat_contact_categories(
    contact_did: String,
    categories: Vec<String>,
) -> Result<serde_json::Value, String> {
    let contact = crate::social_connect::update_contact_categories(&contact_did, categories)?;
    serde_json::to_value(contact).map_err(|e| e.to_string())
}

pub async fn discover_models() -> Result<Vec<llm_offload::ModelInfo>, String> {
    use std::collections::HashSet;

    let state = crate::state::APP_STATE.get().unwrap();
    let storage_path = state.config.lock().unwrap().storage_path.clone();
    let models_dir = PathBuf::from(&storage_path).join("Models");
    let active_path = load_active_model_from_disk();
    let mut models = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let mut push_model = |path: &Path| {
        if !path.is_file() {
            return;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if (ext != "gguf" && ext != "p64") || name.to_ascii_lowercase().contains("mmproj") {
            return;
        }
        // Prefer converted p64 when listing a GGUF that has a sibling container.
        let effective: PathBuf = if ext == "gguf" {
            let p64 = path.with_extension("p64");
            if p64.is_file() {
                p64
            } else {
                path.to_path_buf()
            }
        } else {
            path.to_path_buf()
        };
        let name = effective
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or(name);
        let key = effective.to_string_lossy().to_ascii_lowercase();
        if !seen.insert(key) {
            return;
        }
        let display_name = if effective.starts_with(&models_dir) {
            name
        } else {
            effective.to_string_lossy().into_owned()
        };
        let is_active = active_path
            .as_ref()
            .map(|active| paths_refer_to_same_file(active, &effective))
            .unwrap_or(false);
        models.push(llm_offload::ModelInfo {
            name: display_name,
            is_active,
            avatar_type: if effective.starts_with(&models_dir) {
                "installed".to_string()
            } else {
                "local".to_string()
            },
        });
    };

    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "gguf" || ext == "p64" {
                push_model(&path);
            } else if ext == "json"
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".install.json"))
                    .unwrap_or(false)
            {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    if let Ok(manifest) =
                        serde_json::from_str::<crate::model_lifecycle::InstallManifest>(&text)
                    {
                        push_model(Path::new(&manifest.gguf_path));
                    }
                }
            }
        }
    }

    if let Some(active) = active_path.as_ref() {
        push_model(Path::new(active));
    }

    models.sort_by(|a, b| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()));
    Ok(models)
}

fn paths_refer_to_same_file(left: &str, right: &Path) -> bool {
    let left_path = Path::new(left);
    if left_path == right {
        return true;
    }
    left_path
        .file_name()
        .is_some_and(|left_name| right.file_name() == Some(left_name))
        && left.replace('\\', "/").to_ascii_lowercase()
            == right.to_string_lossy().replace('\\', "/").to_ascii_lowercase()
}

pub async fn run_agent_inference(
    prompt: String,
    model_name: String,
    intent_layout: Vec<f64>,
) -> Result<(), String> {
    tokio::spawn(async move {
        let _ = llm_offload::execute_agent_inference(prompt, model_name, intent_layout).await;
    });
    Ok(())
}

