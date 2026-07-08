use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChronikUtxo {
    pub outpoint: Outpoint,
    pub block_height: i32,
    pub is_coinbase: bool,
    pub value: i64,
    #[serde(default)]
    pub slp_meta: Option<SlpMeta>,
    #[serde(default)]
    pub slp_token: Option<SlpToken>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outpoint {
    pub txid: String,
    pub out_idx: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlpMeta {
    pub token_type: String, 
    pub tx_type: String,    
    pub token_id: String,
    pub group_token_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlpToken {
    pub amount: String, 
    pub is_mint_baton: bool,
}

pub struct ChronikClient {
    pub base_url: String,
    client: reqwest::blocking::Client,
}

impl ChronikClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn fetch_utxos_p2pkh(&self, hash160_hex: &str) -> Result<Vec<ChronikUtxo>, String> {
        let url = format!("{}/script/p2pkh/{}/utxos", self.base_url, hash160_hex);
        let resp = self.client.get(&url).send().map_err(|e| e.to_string())?;
        
        if !resp.status().is_success() {
            return Err(format!("Chronik API error: {}", resp.status()));
        }

        #[derive(Deserialize)]
        struct ScriptUtxos {
            utxos: Vec<ChronikUtxo>,
        }

        let result: Vec<ScriptUtxos> = resp.json().map_err(|e| e.to_string())?;
        
        let mut all_utxos = Vec::new();
        for script_utxo in result {
            all_utxos.extend(script_utxo.utxos);
        }
        Ok(all_utxos)
    }

    pub fn broadcast_tx(&self, raw_tx_hex: &str) -> Result<String, String> {
        let url = format!("{}/broadcast-tx", self.base_url);
        
        let resp = self.client.post(&url)
            .header("Content-Type", "text/plain")
            .body(raw_tx_hex.to_string())
            .send().map_err(|e| e.to_string())?;
        
        if !resp.status().is_success() {
            let err_text = resp.text().unwrap_or_default();
            return Err(format!("Broadcast failed: {}", err_text));
        }

        #[derive(Deserialize)]
        struct BroadcastResponse {
            txid: String,
        }
        
        let res: BroadcastResponse = resp.json().map_err(|e| e.to_string())?;
        Ok(res.txid)
    }

    /// Fetch transaction history for a P2PKH script hash.
    /// Returns the most recent transactions (up to `page_size`, default 25).
    pub fn fetch_tx_history_p2pkh(
        &self,
        hash160_hex: &str,
        page: u32,
        page_size: u32,
    ) -> Result<ChronikTxHistoryPage, String> {
        let url = format!(
            "{}/script/p2pkh/{}/history?page={}&page_size={}",
            self.base_url, hash160_hex, page, page_size
        );
        let resp = self.client.get(&url).send().map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("Chronik history API error: {}", resp.status()));
        }

        let result: ChronikTxHistoryPage = resp.json().map_err(|e| e.to_string())?;
        Ok(result)
    }
}

/// A page of transaction history from Chronik.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChronikTxHistoryPage {
    #[serde(default)]
    pub txs: Vec<ChronikTx>,
    #[serde(default)]
    pub num_pages: u32,
    #[serde(default)]
    pub num_txs: u32,
}

/// A transaction returned by Chronik's history endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChronikTx {
    pub txid: String,
    #[serde(default)]
    pub version: i32,
    #[serde(default)]
    pub inputs: Vec<ChronikTxIO>,
    #[serde(default)]
    pub outputs: Vec<ChronikTxIO>,
    #[serde(default)]
    pub block: Option<ChronikBlock>,
    #[serde(default)]
    pub time_first_seen: i64,
    #[serde(default)]
    pub size: u32,
    #[serde(default)]
    pub is_coinbase: bool,
}

/// An input or output in a Chronik transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChronikTxIO {
    #[serde(default)]
    pub value: i64,
    #[serde(default)]
    pub output_script: String,
    #[serde(default)]
    pub slp_meta: Option<SlpMeta>,
    #[serde(default)]
    pub slp_token: Option<SlpToken>,
}

/// Block info for a confirmed transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChronikBlock {
    pub height: i32,
    #[serde(default)]
    pub hash: String,
    #[serde(default)]
    pub timestamp: i64,
}

