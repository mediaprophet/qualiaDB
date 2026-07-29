#[cfg(target_arch = "wasm32")]
use js_sys::Float64Array;
/// QualiaStore — in-memory quint store exposed to WASM.
///
/// A quint is a 5-tuple (Subject, Predicate, Object, Context, Metadata) of u64 IDs.
/// IDs are assigned by the JS-side Lexicon (vault-wasm.js); this module is the
/// storage layer only.
///
/// Full qualia-core-db wiring (W10) is deferred: the `qualia` Cargo feature gates
/// that dependency to avoid pulling wgpu into the default WASM binary.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct QualiaStore {
    quins: Vec<[u64; 5]>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl QualiaStore {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { quins: Vec::new() }
    }

    /// Insert a quint (s, p, o, c, m).  Returns true on success.
    pub fn insert_quin(&mut self, s: u64, p: u64, o: u64, c: u64, m: u64) -> bool {
        self.quins.push([s, p, o, c, m]);
        true
    }

    /// Return all quints with the given subject as a flat Float64Array
    /// (groups of 5: [s,p,o,c,m, s,p,o,c,m, ...]).
    pub fn query_subject(&self, s: u64) -> Float64Array {
        let matches: Vec<f64> = self
            .quins
            .iter()
            .filter(|q| q[0] == s)
            .flat_map(|q| q.iter().map(|&x| x as f64))
            .collect();
        Float64Array::from(matches.as_slice())
    }

    /// Return all quints with the given predicate as a flat Float64Array.
    pub fn query_predicate(&self, p: u64) -> Float64Array {
        let matches: Vec<f64> = self
            .quins
            .iter()
            .filter(|q| q[1] == p)
            .flat_map(|q| q.iter().map(|&x| x as f64))
            .collect();
        Float64Array::from(matches.as_slice())
    }

    /// Return all quints in the given context as a flat Float64Array.
    pub fn query_context(&self, c: u64) -> Float64Array {
        let matches: Vec<f64> = self
            .quins
            .iter()
            .filter(|q| q[3] == c)
            .flat_map(|q| q.iter().map(|&x| x as f64))
            .collect();
        Float64Array::from(matches.as_slice())
    }

    /// Total number of quints stored.
    pub fn len(&self) -> u32 {
        self.quins.len() as u32
    }

    /// Clear all stored quints.
    pub fn clear(&mut self) {
        self.quins.clear();
    }

    /// Parse a CBOR-LD byte array (CBOR array of 4 or 5 unsigned integers) and
    /// insert the resulting quin. Returns true on success, false on parse error.
    ///
    /// The qualiaDB binary gatekeeper (cbor_compiler.rs) requires this format:
    ///   CBOR array header (0x84 or 0x85) followed by 4–5 CBOR unsigned integers.
    /// All values are Lexicon-compressed u64 IDs assigned by the JS Lexicon.
    pub fn insert_from_cbor_ld(&mut self, data: &[u8]) -> bool {
        match Self::parse_cbor_quin(data) {
            Ok(q) => {
                self.quins.push(q);
                true
            }
            Err(_) => false,
        }
    }

    /// Parse one CBOR-LD quin from raw bytes.
    /// Accepts CBOR arrays of 4 or 5 unsigned integers (major type 0).
    fn parse_cbor_quin(payload: &[u8]) -> Result<[u64; 5], &'static str> {
        if payload.is_empty() {
            return Err("empty");
        }
        let header = payload[0];
        // Must be a CBOR array (major type 4 = 0x80..0x9F)
        if (header >> 5) != 4 {
            return Err("not a CBOR array");
        }
        let count = (header & 0x1F) as usize;
        if count < 4 || count > 5 {
            return Err("quin needs 4 or 5 elements");
        }
        let mut cur = 1usize;
        let mut vals = [0u64; 5];
        for i in 0..count {
            if cur >= payload.len() {
                return Err("buffer underflow");
            }
            let b = payload[cur];
            // Major type 0 = unsigned integer
            if (b >> 5) != 0 {
                return Err("element is not uint");
            }
            let add = b & 0x1F;
            cur += 1;
            vals[i] = match add {
                0..=23 => add as u64,
                24 => {
                    if cur + 1 > payload.len() {
                        return Err("overflow u8");
                    }
                    let v = payload[cur] as u64;
                    cur += 1;
                    v
                }
                25 => {
                    if cur + 2 > payload.len() {
                        return Err("overflow u16");
                    }
                    let v = u16::from_be_bytes([payload[cur], payload[cur + 1]]) as u64;
                    cur += 2;
                    v
                }
                26 => {
                    if cur + 4 > payload.len() {
                        return Err("overflow u32");
                    }
                    let mut b4 = [0u8; 4];
                    b4.copy_from_slice(&payload[cur..cur + 4]);
                    cur += 4;
                    u32::from_be_bytes(b4) as u64
                }
                27 => {
                    if cur + 8 > payload.len() {
                        return Err("overflow u64");
                    }
                    let mut b8 = [0u8; 8];
                    b8.copy_from_slice(&payload[cur..cur + 8]);
                    cur += 8;
                    u64::from_be_bytes(b8)
                }
                _ => return Err("unsupported additional info"),
            };
        }
        Ok(vals)
    }
}
