// QualiaDB Trusted Execution Environment (TEE) Bindings
// Handles Cryptographic signatures (Bilateral Guardianship/DID) natively within the
// Secure Enclave, guaranteeing private keys never leak into the Rust RAM boundaries.

#[cfg(target_os = "android")]
pub mod android_keystore {
    use super::*;
    use std::os::raw::c_int;

    extern "C" {
        // Native C-API hooks into the Android Hardware-Backed Keystore (Titan M / TrustZone)
        // Passes the 48-byte NQuin memory block directly to the secure enclave for signing.
        pub fn AKeyStore_signData(
            key_alias: *const u8,
            key_alias_len: usize,
            data: *const NQuin,
            data_len: usize,
            out_signature: *mut *mut u8,
            out_signature_len: *mut usize,
        ) -> c_int;

        pub fn AKeyStore_verifySignature(
            key_alias: *const u8,
            key_alias_len: usize,
            data: *const NQuin,
            data_len: usize,
            signature: *const u8,
            signature_len: usize,
        ) -> c_int;
    }
}

#[cfg(target_vendor = "apple")]
pub mod apple_secure_enclave {
    use std::ffi::c_void;

    #[link(name = "Security", kind = "framework")]
    extern "C" {
        // CoreFoundation & Security framework native bindings for the Apple Secure Enclave
        // `dataToSign` points to the CFDataRef wrapper of the 48-byte NQuin
        pub fn SecKeyCreateSignature(
            key: *mut c_void,        // SecKeyRef
            algorithm: *mut c_void,  // SecKeyAlgorithm
            dataToSign: *mut c_void, // CFDataRef
            error: *mut *mut c_void, // CFErrorRef
        ) -> *mut c_void; // Returns CFDataRef signature

        pub fn SecKeyVerifySignature(
            key: *mut c_void,        // SecKeyRef
            algorithm: *mut c_void,  // SecKeyAlgorithm
            signedData: *mut c_void, // CFDataRef
            signature: *mut c_void,  // CFDataRef
            error: *mut *mut c_void, // CFErrorRef
        ) -> bool;
    }
}
