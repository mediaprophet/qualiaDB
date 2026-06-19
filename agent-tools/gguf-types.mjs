// Minimal GGUF v3 header parser — dumps tensor (name, dims, ggml_type) so we know which
// quant layout the block-amortized dequant must target. Read-only; no deps.
import { readFileSync } from 'fs';

const GGML_TYPE = {
  0: 'F32', 1: 'F16', 2: 'Q4_0', 3: 'Q4_1', 6: 'Q5_0', 7: 'Q5_1', 8: 'Q8_0',
  9: 'Q8_1', 10: 'Q2_K', 11: 'Q3_K', 12: 'Q4_K', 13: 'Q5_K', 14: 'Q6_K',
  15: 'Q8_K', 16: 'IQ2_XXS', 17: 'IQ2_XS', 18: 'IQ3_XXS', 19: 'IQ1_S',
  20: 'IQ4_NL', 21: 'IQ3_S', 22: 'IQ2_S', 23: 'IQ4_XS', 24: 'I8', 25: 'I16',
  26: 'I32', 27: 'I64', 28: 'F64', 29: 'IQ1_M', 30: 'BF16',
};

const file = process.argv[2] || 'docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf';
const buf = readFileSync(file);
let p = 0;
const u32 = () => { const v = buf.readUInt32LE(p); p += 4; return v; };
const u64 = () => { const v = Number(buf.readBigUInt64LE(p)); p += 8; return v; };
const i64 = () => { const v = Number(buf.readBigInt64LE(p)); p += 8; return v; };
const f32 = () => { const v = buf.readFloatLE(p); p += 4; return v; };
const f64 = () => { const v = buf.readDoubleLE(p); p += 8; return v; };
const str = () => { const n = u64(); const s = buf.toString('utf8', p, p + n); p += n; return s; };

if (buf.toString('ascii', 0, 4) !== 'GGUF') throw new Error('not a GGUF');
p = 4;
const version = u32();
const tensorCount = u64();
const kvCount = u64();

function skipValue(t) {
  switch (t) {
    case 0: case 1: p += 1; break;           // u8/i8
    case 2: case 3: p += 2; break;           // u16/i16
    case 4: case 5: p += 4; break;           // u32/i32
    case 6: f32(); break;                     // f32
    case 7: p += 1; break;                    // bool
    case 8: str(); break;                     // string
    case 9: {                                 // array
      const et = u32(); const n = u64();
      for (let i = 0; i < n; i++) skipValue(et);
      break;
    }
    case 10: case 11: p += 8; break;          // u64/i64
    case 12: f64(); break;                    // f64
    default: throw new Error('unknown value type ' + t);
  }
}

for (let i = 0; i < kvCount; i++) { str(); const t = u32(); skipValue(t); }

console.log(`GGUF v${version}  tensors=${tensorCount}  kv=${kvCount}`);
const want = /blk\.0\.|token_embd|output/;
for (let i = 0; i < tensorCount; i++) {
  const name = str();
  const nd = u32();
  const dims = [];
  for (let d = 0; d < nd; d++) dims.push(u64());
  const type = u32();
  const off = u64();
  if (want.test(name)) {
    console.log(`${name.padEnd(28)} dims=[${dims.join(',')}]  type=${GGML_TYPE[type] || type}`);
  }
}
