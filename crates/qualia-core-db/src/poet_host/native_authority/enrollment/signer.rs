//! Concrete native signer using the engine's key owner and authenticated sealing.
//! Secret material is neither serializable nor printable; no plaintext fallback.
#[cfg(feature = "sanctuary-crypto")]
use super::super::random;
use super::super::{
    admission_digest, ed25519, AdmissionSnapshot, SessionChallenge, SignedAdmission,
    ADMISSION_CONTEXT, SESSION_CONTEXT,
};
#[cfg(feature = "sanctuary-crypto")]
use crate::crypto::sanctuary_audit::{unwrap_key, wrap_key};
use crate::identity::key_vault::KeyVault;
#[cfg(feature = "sanctuary-crypto")]
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::Path,
};
use zeroize::Zeroizing;

#[cfg(feature = "sanctuary-crypto")]
const AAD: &[u8] = b"qualia.native-controller.sealed.v1";
#[cfg(feature = "sanctuary-crypto")]
const HEADER: &[u8] = b"QNS1";
#[cfg(feature = "sanctuary-crypto")]
fn regular(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    meta.is_file() && !meta.file_type().is_symlink()
}
pub struct NativeSigner {
    seed: Zeroizing<[u8; 32]>,
}
impl NativeSigner {
    /// The caller owns vault authentication. A locked vault fails closed.
    /// Use `KeyVault::open_native_secure`, whose native persistence has no file fallback.
    pub fn from_vault(vault: &KeyVault, label: &str) -> Result<Self, String> {
        if vault.is_locked() || !vault.is_native_secure() || label.is_empty() || label.len() > 1024
        {
            return Err(
                "native signer requires strict OS-backed unlocked vault and bounded purpose".into(),
            );
        }
        let context = format!("qualia.native-controller.v1:{}:{label}", label.len());
        Ok(Self {
            seed: Zeroizing::new(vault.derive_key(&context).to_bytes()),
        })
    }
    /// Explicitly creates a new controller key, not an enrollment or identity claim.
    /// Wrapping material comes from an already authenticated native secret owner.
    /// Available only with the `sanctuary-crypto` capability.
    #[cfg(feature = "sanctuary-crypto")]
    pub fn create_sealed(path: &Path, wrapping_key: &[u8; 32]) -> Result<Self, String> {
        if *wrapping_key == [0; 32] {
            return Err("empty wrapping key".into());
        }
        let seed = Zeroizing::new(random()?);
        let sealed = wrap_key(wrapping_key, seed.as_ref(), AAD)
            .map_err(|e| format!("native signer sealing: {e:?}"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("create native signer: {e}"))?;
        file.write_all(HEADER)
            .and_then(|_| file.write_all(&sealed))
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(Self { seed })
    }
    /// Available only with the `sanctuary-crypto` capability.
    #[cfg(feature = "sanctuary-crypto")]
    pub fn open_sealed(path: &Path, wrapping_key: &[u8; 32]) -> Result<Self, String> {
        let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !regular(&meta) || meta.len() != 76 {
            return Err("native signer requires exact bounded sealed regular file".into());
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x00200000);
        }
        let mut file = options.open(path).map_err(|e| e.to_string())?;
        let opened = file.metadata().map_err(|e| e.to_string())?;
        if !regular(&opened)
            || opened.len() != 76
            || fs::symlink_metadata(path)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
        {
            return Err("native signer file changed during open".into());
        }
        let mut bytes = [0u8; 76];
        file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        let mut excess = [0u8; 1];
        if file.read(&mut excess).map_err(|e| e.to_string())? != 0 {
            return Err("native signer grew beyond byte budget".into());
        }
        if &bytes[..4] != HEADER || *wrapping_key == [0; 32] {
            return Err("native signer format/wrapping key rejected".into());
        }
        let plain = Zeroizing::new(
            unwrap_key(wrapping_key, &bytes[4..], AAD)
                .map_err(|e| format!("native signer unlock: {e:?}"))?,
        );
        if plain.len() != 32 {
            return Err("invalid signer seed length".into());
        }
        let mut seed = Zeroizing::new([0; 32]);
        seed.copy_from_slice(&plain);
        Ok(Self { seed })
    }
    pub fn public_key(&self) -> [u8; 32] {
        ed25519::public_from_seed(&self.seed)
    }
    pub(super) fn sign(&self, digest: &[u8; 32], context: &[u8]) -> Result<[u8; 64], String> {
        let mut proof = [0; 64];
        ed25519::sign_with_context(&self.seed, digest, context, &mut proof)
            .map_err(|e| format!("native signing: {e:?}"))?;
        Ok(proof)
    }
    pub fn prove_session(&self, challenge: &SessionChallenge) -> Result<[u8; 64], String> {
        self.sign(challenge.message_digest(), SESSION_CONTEXT)
    }
    /// Produces bytes only. Pinning/adoption and explicit steward approval remain separate.
    pub fn sign_admission(&self, snapshot: AdmissionSnapshot) -> Result<SignedAdmission, String> {
        let digest = admission_digest(&snapshot, 4 * 1024 * 1024)?;
        let signature = self.sign(&digest, ADMISSION_CONTEXT)?.to_vec();
        Ok(SignedAdmission {
            snapshot,
            signature,
        })
    }
}
