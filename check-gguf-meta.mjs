import fs from 'fs';

function readGGUFKeys(file) {
    const buf = fs.readFileSync(file);
    // GGUF header: magic (4) + version (4) + tensor_count (8) + kv_count (8)
    const magic = buf.readUInt32LE(0);
    const version = buf.readUInt32LE(4);
    const tensorCount = Number(buf.readBigUInt64LE(8));
    const kvCount = Number(buf.readBigUInt64LE(16));
    console.log(`${file}: magic=0x${magic.toString(16)} v${version} tensors=${tensorCount} kv=${kvCount}`);

    let pos = 24;
    const keys = {};
    for (let i = 0; i < kvCount && pos < buf.length; i++) {
        // Read key string: u64 len + bytes
        const keyLen = Number(buf.readBigUInt64LE(pos));
        pos += 8;
        const key = buf.toString('utf8', pos, pos + keyLen);
        pos += keyLen;
        // Read value type (u32)
        const vtype = buf.readUInt32LE(pos);
        pos += 4;
        
        // Type 0=u8, 1=i8, 2=u16, 3=i16, 4=u32, 5=i32, 6=f32, 7=bool, 8=string, 9=array, 10=u64, 11=i64, 12=f64
        let value = null;
        if (vtype === 8) {
            const sLen = Number(buf.readBigUInt64LE(pos));
            pos += 8;
            value = buf.toString('utf8', pos, pos + sLen);
            pos += sLen;
        } else if (vtype === 4) {
            value = buf.readUInt32LE(pos);
            pos += 4;
        } else if (vtype === 5) {
            value = buf.readInt32LE(pos);
            pos += 4;
        } else if (vtype === 6) {
            value = buf.readFloatLE(pos);
            pos += 4;
        } else if (vtype === 7) {
            value = buf[pos] !== 0;
            pos += 1;
        } else if (vtype === 10) {
            value = Number(buf.readBigUInt64LE(pos));
            pos += 8;
        } else if (vtype === 9) {
            // Array: read element type + count
            const elemType = buf.readUInt32LE(pos);
            pos += 4;
            const arrCount = Number(buf.readBigUInt64LE(pos));
            pos += 8;
            // Skip the array data
            const elemSizes = [1, 1, 2, 2, 4, 4, 4, 1, 0, 0, 8, 8, 8];
            if (elemType === 8) {
                // String array - skip
                for (let j = 0; j < arrCount && pos < buf.length; j++) {
                    const sLen = Number(buf.readBigUInt64LE(pos));
                    pos += 8 + sLen;
                }
            } else {
                pos += arrCount * (elemSizes[elemType] || 0);
            }
            value = `[array: ${arrCount} elements of type ${elemType}]`;
        } else {
            value = `[type ${vtype}]`;
            // Try to skip - assume 4 bytes for unknown
            pos += 4;
        }
        
        // Only print architecture-related keys
        if (key.includes('block_count') || key.includes('embedding_length') || 
            key.includes('context_length') || key.includes('attention.head_count') ||
            key.includes('attention.key_length') || key.includes('feed_forward_length') ||
            key.includes('rope.dimension_count') || key.includes('attention.head_count_kv') ||
            key.includes('tokenizer.ggml.pre') || key.includes('tokenizer.ggml.model') ||
            key.includes('tokenizer.chat_template') || key.includes('general.architecture') ||
            key.includes('general.name') || key.includes('attention.layer_norm_rms_epsilon') ||
            key.includes('expert_count') || key.includes('expert_used_count') ||
            key.includes('attention.causal') || key.includes('gemma') ||
            key.includes('attn_norm') || key.includes('shared_classifier') ||
            key.includes('decoder.start_token_id')) {
            console.log(`  ${key} = ${typeof value === 'string' ? JSON.stringify(value) : value}`);
        }
    }
}

for (const f of process.argv.slice(2)) {
    try { readGGUFKeys(f); } catch(e) { console.log(`${f}: ERROR ${e.message}`); }
}
