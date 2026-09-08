//! ML-KEM-768 adapter. Named ML-KEM on the wire, not historical Kyber.

use fips203::ml_kem_768;
use fips203::traits::{Decaps, Encaps, KeyGen, SerDes};
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::errors::CryptoError;
use super::types::{ML_KEM_768_CT_LEN, ML_KEM_768_PK_LEN, ML_KEM_768_SK_LEN, ML_KEM_SS_LEN};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MlKem768Secret {
    bytes: [u8; ML_KEM_768_SK_LEN],
}

impl MlKem768Secret {
    pub fn generate() -> Result<(Self, [u8; ML_KEM_768_PK_LEN]), CryptoError> {
        let (ek, dk) = ml_kem_768::KG::try_keygen().map_err(|_| CryptoError::CryptoFailure)?;
        Ok((Self { bytes: dk.into_bytes() }, ek.into_bytes()))
    }

    pub fn from_bytes(bytes: [u8; ML_KEM_768_SK_LEN]) -> Self {
        Self { bytes }
    }

    pub fn decapsulate(
        &self,
        ciphertext: &[u8; ML_KEM_768_CT_LEN],
    ) -> Result<[u8; ML_KEM_SS_LEN], CryptoError> {
        let dk = ml_kem_768::DecapsKey::try_from_bytes(self.bytes)
            .map_err(|_| CryptoError::CryptoFailure)?;
        let ct = ml_kem_768::CipherText::try_from_bytes(*ciphertext)
            .map_err(|_| CryptoError::CryptoFailure)?;
        let ss = dk.try_decaps(&ct).map_err(|_| CryptoError::CryptoFailure)?;
        Ok(ss.into_bytes())
    }
}

pub fn encapsulate(
    public: &[u8; ML_KEM_768_PK_LEN],
) -> Result<([u8; ML_KEM_SS_LEN], [u8; ML_KEM_768_CT_LEN]), CryptoError> {
    let ek =
        ml_kem_768::EncapsKey::try_from_bytes(*public).map_err(|_| CryptoError::CryptoFailure)?;
    let (ss, ct) = ek.try_encaps().map_err(|_| CryptoError::CryptoFailure)?;
    Ok((ss.into_bytes(), ct.into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encapsulate_decapsulate() {
        let (sk, pk) = MlKem768Secret::generate().unwrap();
        let (ss1, ct) = encapsulate(&pk).unwrap();
        let ss2 = sk.decapsulate(&ct).unwrap();
        assert_eq!(ss1, ss2);
    }
}
