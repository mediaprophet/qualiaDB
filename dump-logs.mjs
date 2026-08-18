import fs from 'fs';
const r = JSON.parse(fs.readFileSync('test-wasm-results.json', 'utf8'));
if (r.models) {
    for (const [k, v] of Object.entries(r.models)) {
        console.log(`\n=== ${k} ===`);
        console.log('Console logs:');
        for (const l of (v.consoleLogs || [])) {
            console.log(`  [${l.type}] ${l.text.substring(0, 200)}`);
        }
    }
}
