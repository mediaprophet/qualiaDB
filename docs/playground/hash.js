/**
 * Canonical Qualia-DB FNV-1a 64-bit hash module.
 *
 * This is the JavaScript reference implementation of `crate::q_hash` (Rust).
 * Both MUST produce identical outputs for identical UTF-8 inputs — they share
 * the same seeds, the same XOR-first/multiply-second step order, and the same
 * 64-bit unsigned truncation via `BigInt.asUintN(64, ...)`.
 *
 * All public functions return `bigint` (unsigned 64-bit) so callers can safely
 * compare them against u64 values decoded from .q42 or .q42-lex files without
 * IEEE-754 precision loss.
 */

const FNV_OFFSET = 0xcbf29ce484222325n; // same literal as Rust
const FNV_PRIME  = 0x100000001b3n;
const U64_MASK   = 0xffffffffffffffffn;
const MSB        = 1n << 63n;

const _encoder = new TextEncoder();

/**
 * FNV-1a 64-bit hash of a UTF-8 string.
 * Algorithm: for each byte b — hash ^= b, hash = (hash * prime) mod 2^64.
 * Matches `crate::q_hash` and `identifier::fnv1a` exactly.
 *
 * @param {string} str
 * @returns {bigint} unsigned 64-bit hash
 */
export function fnv1a64(str) {
    const bytes = _encoder.encode(str);
    let h = FNV_OFFSET;
    for (const b of bytes) {
        h = BigInt.asUintN(64, (h ^ BigInt(b)) * FNV_PRIME);
    }
    return h;
}

/**
 * Set bit 63 of a BigInt value — the MSB flag used by the Webizen VM to
 * distinguish did:q42 topological pointers from plain dictionary hashes.
 *
 * @param {bigint} hash
 * @returns {bigint}
 */
export function withMsb(hash) {
    return hash | MSB;
}

/**
 * True if bit 63 of `value` is set (did:q42 direct-jump path in the VM).
 *
 * @param {bigint} value
 * @returns {boolean}
 */
export function hasMsb(value) {
    return (value & MSB) !== 0n;
}

/**
 * Parse a `did:q42:` URI and return its 64-bit topological pointer (MSB=1).
 * Returns `null` for any input that does not start with `did:q42:` or has an
 * empty payload — mirroring `crate::identifier::parse_did_q42`.
 *
 * @param {string} uri
 * @returns {bigint|null}
 */
export function parseDidQ42(uri) {
    const PREFIX = 'did:q42:';
    if (!uri.startsWith(PREFIX)) return null;
    const payload = uri.slice(PREFIX.length);
    if (!payload) return null;
    return withMsb(fnv1a64(payload));
}

/**
 * Hash a single N-Triples token, applying did:q42 routing when applicable.
 * Preserves `<...>` IRI delimiters and `"..."` literal delimiters matching
 * the canonical Q42 v3 volume layout and Q42LEX dictionary.
 * Applies the 60-bit term identity mask (`0x0FFF_FFFF_FFFF_FFFFn`).
 *
 * @param {string} token   — one S/P/O token from an N-Triples line
 * @returns {bigint}
 */
export function hashToken(token) {
    if (!token) return 0n;
    let t = token.trim();

    if (t.startsWith('<did:q42:') && t.endsWith('>')) {
        const ptr = parseDidQ42(t.slice(1, -1));
        if (ptr !== null) return ptr;
    }
    if (t.startsWith('did:q42:')) {
        const ptr = parseDidQ42(t);
        if (ptr !== null) return ptr;
    }

    // Canonicalize raw HTTP(S) URLs to <URL> if delimiters were omitted
    if ((t.startsWith('http://') || t.startsWith('https://') || t.startsWith('urn:')) && !t.startsWith('<')) {
        t = `<${t}>`;
    }

    return fnv1a64(t) & 0x0fff_ffff_ffff_ffffn;
}

/**
 * Strip outer URI (<...>) or literal ("...") delimiters from a token.
 *
 * @param {string} token
 * @returns {string}
 */
export function stripDelimiters(token) {
    if (!token) return '';
    let t = token.trim();
    if (t.startsWith('<') && t.endsWith('>')) {
        return t.slice(1, -1);
    }
    if (t.startsWith('"')) {
        const m = t.match(/^"((?:[^"\\]|\\.)*)"/);
        if (m) return m[1];
    }
    return t;
}

/**
 * Return both the verbatim token hash and the delimiter-stripped token hash.
 * This guarantees interoperability across volumes ingested with delimiters
 * preserved (e.g. Schema.org NT) and volumes ingested with delimiters stripped
 * (e.g. Princeton WordNet RDF/XML via mini_parser).
 *
 * @param {string} token
 * @returns {bigint[]}
 */
export function hashTokenVariants(token) {
    if (!token) return [0n];
    const raw = hashToken(token);
    const stripped = hashToken(stripDelimiters(token));
    if (raw === stripped) return [raw];
    return [raw, stripped];
}

/**
 * Format a BigInt as a zero-padded 16-character lowercase hex string,
 * suitable for displaying a 64-bit hash value in the UI.
 *
 * @param {bigint} value
 * @returns {string}
 */
export function toHex16(value) {
    return value.toString(16).padStart(16, '0');
}

/**
 * Parse a u64 decimal string (as emitted by the Rust WASM JSON serialiser)
 * back into a BigInt.  Falls back gracefully for plain JS Number strings.
 *
 * @param {string} s
 * @returns {bigint}
 */
export function parseBigDecimal(s) {
    try { return BigInt(s); }
    catch (_) { return 0n; }
}
