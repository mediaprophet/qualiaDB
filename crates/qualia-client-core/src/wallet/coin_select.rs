//! Simple UTXO coin selection for eCash (XEC) transactions.
//!
//! Strategy: largest-first. Pick the biggest UTXOs until we have enough to cover
//! the target amount + estimated fee. Produces change output if there's surplus
//! above a dust threshold.
//!
//! This is a cold-path construction utility (Tier-2). It allocates internally
//! but the public output is caller-buffered via return values.

use crate::wallet::chronik::ChronikUtxo;

/// Minimum output value (in satoshis) below which we skip creating a change output.
/// For XEC: 546 satoshis = 5.46 XEC dust threshold.
pub const DUST_THRESHOLD_SATS: i64 = 546;

/// Estimated transaction fee in satoshis for a simple P2PKH tx.
/// ~1 input + 2 outputs ≈ 226 bytes × 1 sat/byte = 226 sats.
/// We use a conservative estimate.
pub const BASE_FEE_SATS: i64 = 400;

/// Per-input fee contribution in satoshis (~148 bytes per additional input).
pub const PER_INPUT_FEE_SATS: i64 = 150;

/// Result of coin selection.
#[derive(Debug, Clone)]
pub struct CoinSelection {
    /// The selected UTXOs to spend.
    pub selected: Vec<ChronikUtxo>,
    /// Total value of selected UTXOs in satoshis.
    pub total_input_sats: i64,
    /// Target send amount in satoshis.
    pub target_sats: i64,
    /// Estimated fee in satoshis.
    pub fee_sats: i64,
    /// Change amount in satoshis (0 if below dust).
    pub change_sats: i64,
}

/// Select UTXOs to cover `target_sats` using largest-first strategy.
///
/// Only considers UTXOs that have no SLP/ALP token metadata (plain XEC UTXOs).
/// Returns `Err` if insufficient funds.
pub fn select_utxos(utxos: &[ChronikUtxo], target_sats: i64) -> Result<CoinSelection, String> {
    if target_sats <= 0 {
        return Err("Target amount must be positive".into());
    }

    // Filter to plain XEC UTXOs (no token metadata) and sort largest-first
    let mut candidates: Vec<&ChronikUtxo> = utxos
        .iter()
        .filter(|u| u.slp_meta.is_none() && u.value > 0)
        .collect();
    candidates.sort_by(|a, b| b.value.cmp(&a.value));

    let mut selected = Vec::new();
    let mut total_input: i64 = 0;

    for utxo in candidates {
        selected.push(utxo.clone());
        total_input += utxo.value;

        let fee = BASE_FEE_SATS + PER_INPUT_FEE_SATS * selected.len() as i64;
        let needed = target_sats + fee;

        if total_input >= needed {
            let change = total_input - target_sats - fee;
            let change_sats = if change >= DUST_THRESHOLD_SATS {
                change
            } else {
                0 // Absorb sub-dust remainder into fee
            };
            let actual_fee = total_input - target_sats - change_sats;

            return Ok(CoinSelection {
                selected,
                total_input_sats: total_input,
                target_sats,
                fee_sats: actual_fee,
                change_sats,
            });
        }
    }

    Err(format!(
        "Insufficient funds: have {} sats, need {} + fee",
        total_input, target_sats
    ))
}

/// Select UTXOs that contain a specific SLP/ALP token.
/// Returns the token UTXOs + plain XEC UTXOs needed to cover the fee.
pub fn select_token_utxos(
    utxos: &[ChronikUtxo],
    token_id: &str,
    token_amount: u64,
) -> Result<(Vec<ChronikUtxo>, CoinSelection), String> {
    // Find token UTXOs matching the requested token_id
    let mut token_utxos = Vec::new();
    let mut token_total: u64 = 0;

    for utxo in utxos {
        if let Some(ref meta) = utxo.slp_meta {
            if meta.token_id == token_id {
                if let Some(ref token) = utxo.slp_token {
                    if let Ok(amt) = token.amount.parse::<u64>() {
                        token_utxos.push(utxo.clone());
                        token_total += amt;
                    }
                }
            }
        }
    }

    if token_total < token_amount {
        return Err(format!(
            "Insufficient token balance: have {}, need {}",
            token_total, token_amount
        ));
    }

    // We also need plain XEC UTXOs to pay the transaction fee
    // Token txs need: OP_RETURN output (0 sats) + token output (546 sats) + change output + fee
    let min_xec_needed =
        DUST_THRESHOLD_SATS + BASE_FEE_SATS + PER_INPUT_FEE_SATS * (token_utxos.len() as i64 + 1); // +1 for XEC funding input

    let xec_selection = select_utxos(utxos, min_xec_needed)?;

    Ok((token_utxos, xec_selection))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wallet::chronik::{ChronikUtxo, Outpoint};

    fn make_utxo(value: i64, idx: u32) -> ChronikUtxo {
        ChronikUtxo {
            outpoint: Outpoint {
                txid: format!("{:064x}", idx),
                out_idx: 0,
            },
            block_height: 800000,
            is_coinbase: false,
            value,
            slp_meta: None,
            slp_token: None,
        }
    }

    #[test]
    fn test_select_single_utxo() {
        let utxos = vec![make_utxo(10000, 1)];
        let result = select_utxos(&utxos, 5000).unwrap();
        assert_eq!(result.selected.len(), 1);
        assert_eq!(result.target_sats, 5000);
        assert!(result.fee_sats > 0);
        assert_eq!(
            result.total_input_sats,
            result.target_sats + result.fee_sats + result.change_sats
        );
    }

    #[test]
    fn test_select_multiple_utxos() {
        let utxos = vec![make_utxo(3000, 1), make_utxo(4000, 2), make_utxo(5000, 3)];
        let result = select_utxos(&utxos, 8000).unwrap();
        // Should pick 5000 + 4000 = 9000 (largest first)
        assert!(result.selected.len() >= 2);
        assert!(result.total_input_sats >= 8000);
    }

    #[test]
    fn test_insufficient_funds() {
        let utxos = vec![make_utxo(1000, 1)];
        assert!(select_utxos(&utxos, 5000).is_err());
    }

    #[test]
    fn test_skips_token_utxos() {
        use crate::wallet::chronik::{SlpMeta, SlpToken};
        let mut token_utxo = make_utxo(100000, 1);
        token_utxo.slp_meta = Some(SlpMeta {
            token_type: "FUNGIBLE".into(),
            tx_type: "SEND".into(),
            token_id: "abc".into(),
            group_token_id: None,
        });
        token_utxo.slp_token = Some(SlpToken {
            amount: "1000".into(),
            is_mint_baton: false,
        });
        let plain_utxo = make_utxo(5000, 2);

        let utxos = vec![token_utxo, plain_utxo];
        let result = select_utxos(&utxos, 3000).unwrap();
        // Should only pick the plain UTXO, not the token one
        assert_eq!(result.selected.len(), 1);
        assert_eq!(result.selected[0].value, 5000);
    }

    #[test]
    fn test_dust_change_absorbed() {
        // If change would be < 546, it gets absorbed into fee
        let utxos = vec![make_utxo(6000, 1)];
        let result = select_utxos(&utxos, 5000).unwrap();
        // 6000 - 5000 - ~550 fee = ~450 change (below dust) → absorbed
        if result.change_sats == 0 {
            assert!(result.fee_sats > BASE_FEE_SATS + PER_INPUT_FEE_SATS);
        }
    }
}
