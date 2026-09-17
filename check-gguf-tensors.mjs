import fs from 'fs';

function readGGUFTensors(file) {
    const buf = fs.readFileSync(file);
    const tensorCount = Number(buf.readBigUInt64LE(8));
    const kvCount = Number(buf.readBigUInt64LE(16));
    let pos = 24;
    
    // Skip KV pairs
    for (let i = 0; i < kvCount && pos < buf.length; i++) {
        const keyLen = Number(buf.readBigUInt64LE(pos));
        pos += 8 + keyLen;
        const vtype = buf.readUInt32LE(pos);
        pos += 4;
        if (vtype === 8) {
            const sLen = Number(buf.readBigUInt64LE(pos));
            pos += 8 + sLen;
        } else if (vtype === 4) { pos += 4; }
        else if (vtype === 5) { pos += 4; }
        else if (vtype === 6) { pos += 4; }
        else if (vtype === 7) { pos += 1; }
        else if (vtype === 10) { pos += 8; }
        else if (vtype === 9) {
            const elemType = buf.readUInt32LE(pos); pos += 4;
            const arrCount = Number(buf.readBigUInt64LE(pos)); pos += 8;
            if (elemType === 8) {
                for (let j = 0; j < arrCount && pos < buf.length; j++) {
                    const sLen = Number(buf.readBigUInt64LE(pos));
                    pos += 8 + sLen;
                }
            } else {
                const elemSizes = [1,1,2,2,4,4,4,1,0,0,8,8,8];
                pos += arrCount * (elemSizes[elemType] || 0);
            }
        } else { pos += 4; }
    }
    
    // Read tensor info
    const tensors = [];
    for (let i = 0; i < tensorCount && pos < buf.length; i++) {
        const nameLen = Number(buf.readBigUInt64LE(pos));
        pos += 8;
        const name = buf.toString('utf8', pos, pos + nameLen);
        pos += nameLen;
        const nDims = buf.readUInt32LE(pos); pos += 4;
        const dims = [];
        for (let d = 0; d < nDims; d++) {
            dims.push(Number(buf.readBigUInt64LE(pos)));
            pos += 8;
        }
        const ggmlType = buf.readUInt32LE(pos); pos += 4;
        const dataOff = Number(buf.readBigUInt64LE(pos)); pos += 8;
        tensors.push({name, dims, type: ggmlType, off: dataOff});
    }
    
    // Print key tensors
    const keys = ['token_embd.weight', 'output.weight', 'output_norm.weight', 
                  'blk.0.attn_q.weight', 'blk.0.attn_k.weight', 'blk.0.attn_v.weight',
                  'blk.0.ffn_gate.weight', 'blk.0.ffn_down.weight', 'blk.0.ffn_up.weight'];
    for (const k of keys) {
        const t = tensors.find(t => t.name === k);
        if (t) {
            console.log(`  ${k}: dims=[${t.dims.join(',')}] type=${t.type}`);
        } else {
            console.log(`  ${k}: NOT FOUND`);
        }
    }
    console.log(`  Total tensors: ${tensors.length}`);
    // Check for tied embeddings
    const emb = tensors.find(t => t.name === 'token_embd.weight');
    const out = tensors.find(t => t.name === 'output.weight');
    if (emb && out) {
        console.log(`  Tied: ${emb.off === out.off ? 'YES (same offset)' : 'NO (different offsets)'}`);
    } else if (emb && !out) {
        console.log(`  Tied: YES (no output.weight, using token_embd)`);
    }
}

for (const f of process.argv.slice(2)) {
    console.log(`\n=== ${f} ===`);
    try { readGGUFTensors(f); } catch(e) { console.log(`  ERROR: ${e.message}`); }
}
