use bip32::{XPrv, XPub, DerivationPath as Bip32Path};
use ripemd::Ripemd160;
use sha2::Sha256;
use sha3::Keccak256;
use bs58;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct DerivationPath(pub String);

#[derive(Debug, Clone)]
pub struct AddressPayload {
    pub network: String,
    pub address: String,
    pub path: String,
    pub pubkey_hash: String,
}

pub struct HdWallet {
    master_key: XPrv,
}

impl HdWallet {
    pub fn from_seed(seed_bytes: &[u8]) -> Result<Self, String> {
        let master_key = XPrv::new(seed_bytes).map_err(|e| e.to_string())?;
        Ok(Self { master_key })
    }

    pub fn derive_address(&self, network: &str, path: &str) -> Result<AddressPayload, String> {
        let derivation_path = Bip32Path::from_str(path).map_err(|e| e.to_string())?;
        let mut child_xprv = self.master_key.clone();
        for child in derivation_path.iter() {
            child_xprv = child_xprv.derive_child(child).map_err(|e| e.to_string())?;
        }
        let public_key = child_xprv.public_key();
        
        let address = match network {
            "BTC" => Self::derive_btc_address(&public_key),
            "XEC" => Self::derive_xec_address(&public_key),
            "ETH" => Self::derive_eth_address(&public_key),
            "NYM" => Self::derive_nym_address(&public_key),
            _ => return Err(format!("Unsupported network: {}", network)),
        };

        let pubkey_bytes = public_key.to_bytes();
        let sha256_hash = <Sha256 as sha2::Digest>::digest(&pubkey_bytes);
        let ripemd160_hash = <Ripemd160 as ripemd::Digest>::digest(&sha256_hash);
        let pubkey_hash = hex::encode(&ripemd160_hash);

        Ok(AddressPayload {
            network: network.to_string(),
            address,
            path: path.to_string(),
            pubkey_hash,
        })
    }

    fn derive_btc_address(pubkey: &XPub) -> String {
        let pubkey_bytes = pubkey.to_bytes();
        let sha256_hash = <Sha256 as sha2::Digest>::digest(&pubkey_bytes);
        let ripemd160_hash = <Ripemd160 as ripemd::Digest>::digest(&sha256_hash);
        
        let mut payload = vec![0x00]; // Mainnet pubkey hash version byte
        payload.extend_from_slice(&ripemd160_hash);
        
        let checksum_hash1 = <Sha256 as sha2::Digest>::digest(&payload);
        let checksum_hash2 = <Sha256 as sha2::Digest>::digest(&checksum_hash1);
        
        payload.extend_from_slice(&checksum_hash2[0..4]);
        
        bs58::encode(payload).into_string()
    }
    
    fn derive_xec_address(pubkey: &XPub) -> String {
        // We will output a base58 legacy address format with an ecash prefix
        let btc_addr = Self::derive_btc_address(pubkey);
        format!("ecash:{}", btc_addr) 
    }

    fn derive_eth_address(pubkey: &XPub) -> String {
        let pk = pubkey.public_key(); 
        let uncompressed = pk.to_encoded_point(false);
        let pub_bytes = &uncompressed.as_bytes()[1..]; 
        
        let keccak_hash = <Keccak256 as sha3::Digest>::digest(pub_bytes);
        let addr_bytes = &keccak_hash[12..]; 
        
        format!("0x{}", hex::encode(addr_bytes))
    }

    fn derive_nym_address(pubkey: &XPub) -> String {
        let pubkey_bytes = pubkey.to_bytes();
        let sha256_hash = <Sha256 as sha2::Digest>::digest(&pubkey_bytes);
        let ripemd160_hash = <Ripemd160 as ripemd::Digest>::digest(&sha256_hash);
        
        format!("n1{}", hex::encode(&ripemd160_hash[0..16]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_derivation() {
        // A known seed for deterministic testing
        let seed = [0u8; 64]; 
        let wallet = HdWallet::from_seed(&seed).unwrap();

        let btc = wallet.derive_address("BTC", "m/44'/0'/0'/0/0").unwrap();
        assert_eq!(btc.network, "BTC");
        assert!(!btc.address.is_empty());

        let eth = wallet.derive_address("ETH", "m/44'/60'/0'/0/0").unwrap();
        assert_eq!(eth.network, "ETH");
        assert!(eth.address.starts_with("0x"));

        let xec = wallet.derive_address("XEC", "m/44'/899'/0'/0/0").unwrap();
        assert_eq!(xec.network, "XEC");
        assert!(xec.address.starts_with("ecash:"));
        
        let nym = wallet.derive_address("NYM", "m/44'/118'/0'/0/0").unwrap();
        assert_eq!(nym.network, "NYM");
        assert!(nym.address.starts_with("n1"));
    }
}
