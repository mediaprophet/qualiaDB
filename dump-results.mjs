import fs from 'fs';
const r = JSON.parse(fs.readFileSync('test-wasm-results.json', 'utf8'));
const models = r.models || {};
for (const [k, v] of Object.entries(models)) {
    console.log(`\n=== ${k} ===`);
    console.log('errors:', JSON.stringify(v.pageErrors || []));
    console.log('gen terminal:', v.generationTerminal);
    console.log('model terminal:', v.modelLoadTerminal);
    console.log('output:', JSON.stringify(v.output));
    console.log('tps:', v.tps);
    // Show all console logs
    for (const l of (v.consoleLogs || [])) {
        console.log(`  [${l.type}] ${l.text.substring(0, 300)}`);
    }
}
