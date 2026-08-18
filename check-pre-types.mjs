import fs from 'fs';
const files = [
    'docs/models/qwen2.5-0.5b-instruct-q4_k_m.gguf',
    'docs/models/qwen3-0.6b-q4_k_m.gguf',
    'docs/models/llama-3.2-1b-instruct-q4_k_m.gguf',
];
for (const f of files) {
    const buf = fs.readFileSync(f);
    const s = buf.toString('latin1');
    const i = s.indexOf('tokenizer.ggml.pre');
    if (i >= 0) {
        // The key is a string in GGUF format: length (u64 LE) + bytes
        // After the key string, there's a value type byte, then the value string
        // Let's find the value by scanning after the key
        let pos = i + 'tokenizer.ggml.pre'.length;
        // Skip past the key string metadata to find the value
        // In GGUF, keys are stored as: u64 len + bytes, then u32 type, then value
        // Let's search for known pre_type strings near this location
        const region = s.substring(i, i + 200);
        const preTypes = ['qwen2', 'qwen3', 'llama-bpe', 'smollm', 'gpt2', 'chatglm', 'bert'];
        let found = null;
        for (const pt of preTypes) {
            const idx = region.indexOf(pt);
            if (idx >= 0) {
                found = pt;
                break;
            }
        }
        console.log(`${f}: pre_type=${found || 'UNKNOWN'}, region=${JSON.stringify(region.substring(0, 100))}`);
    } else {
        console.log(`${f}: tokenizer.ggml.pre NOT FOUND`);
    }
}
