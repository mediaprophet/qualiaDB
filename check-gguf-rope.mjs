import fs from 'fs';

function readGGUFKeys(file) {
    const buf = fs.readFileSync(file);
    const tensorCount = Number(buf.readBigUInt64LE(8));
    const kvCount = Number(buf.readBigUInt64LE(16));
    let pos = 24;
    
    for (let i = 0; i < kvCount && pos < buf.length; i++) {
        const keyLen = Number(buf.readBigUInt64LE(pos));
        pos += 8;
        const key = buf.toString('utf8', pos, pos + keyLen);
        pos += keyLen;
        const vtype = buf.readUInt32LE(pos);
        pos += 4;
        
        let value = null;
        if (vtype === 8) {
            const sLen = Number(buf.readBigUInt64LE(pos));
            pos += 8;
            value = buf.toString('utf8', pos, pos + sLen);
            pos += sLen;
        } else if (vtype === 4) { value = buf.readUInt32LE(pos); pos += 4; }
        else if (vtype === 5) { value = buf.readInt32LE(pos); pos += 4; }
        else if (vtype === 6) { value = buf.readFloatLE(pos); pos += 4; }
        else if (vtype === 7) { value = buf[pos] !== 0; pos += 1; }
        else if (vtype === 10) { value = Number(buf.readBigUInt64LE(pos)); pos += 8; }
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
            value = `[array: ${arrCount} of type ${elemType}]`;
        } else { value = `[type ${vtype}]`; pos += 4; }
        
        if (key.includes('rope') || key.includes('attn') || key.includes('head') ||
            key.includes('block') || key.includes('embed') || key.includes('context') ||
            key.includes('feed') || key.includes('norm') || key.includes('arch') ||
            key.includes('eos') || key.includes('bos') || key.includes('add')) {
            console.log(`  ${key} = ${typeof value === 'string' ? JSON.stringify(value) : value}`);
        }
    }
}

for (const f of process.argv.slice(2)) {
    console.log(`\n=== ${f} ===`);
    try { readGGUFKeys(f); } catch(e) { console.log(`  ERROR: ${e.message}`); }
}
