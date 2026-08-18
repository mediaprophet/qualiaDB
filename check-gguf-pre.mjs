import fs from 'fs';

const path = 'docs/models/smollm2-360m-instruct-q8_0.gguf';
const fd = fs.openSync(path, 'r');
const buf = Buffer.alloc(102400);
fs.readSync(fd, buf, 0, 102400, 0);
fs.closeSync(fd);

const str = buf.toString('latin1');
const key = 'tokenizer.ggml.pre';
const idx = str.indexOf(key);
if (idx >= 0) {
    console.log(`Found "${key}" at byte offset ${idx}`);
    // Print surrounding bytes for context
    const ctx = buf.subarray(idx, idx + 100);
    console.log('Raw bytes:', Array.from(ctx.subarray(0, 50)).map(b => b.toString(16).padStart(2,'0')).join(' '));
    // Try to find the string value after the key
    // GGUF format: key string (u64 len + bytes) + value type (u32) + value (u64 len + bytes for string)
    let p = idx;
    // Skip key
    const keyLen = Number(buf.readBigUInt64LE(p));
    p += 8 + keyLen;
    // Value type
    const vtype = buf.readUInt32LE(p);
    p += 4;
    console.log(`Value type: ${vtype} (8 = string)`);
    if (vtype === 8) {
        const valLen = Number(buf.readBigUInt64LE(p));
        p += 8;
        const val = buf.subarray(p, p + valLen).toString('utf8');
        console.log(`tokenizer.ggml.pre = "${val}"`);
    }
} else {
    console.log(`"${key}" not found in first 100KB`);
}
