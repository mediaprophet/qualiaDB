import fs from 'fs';
const r = JSON.parse(fs.readFileSync('test-wasm-results.json', 'utf8'));
if (r.models) {
    for (const [k, v] of Object.entries(r.models)) {
        console.log(`\n=== ${k} ===`);
        console.log('  loadError:', v.error || 'none');
        if (v.phases?.modelLoad) {
            console.log('  modelLoad terminal:', v.phases.modelLoad.terminal?.substring(0, 500));
        }
        if (v.generation) {
            console.log('  output:', JSON.stringify(v.generation.output).substring(0, 300));
            console.log('  tps:', v.generation.tps);
            console.log('  terminal:', v.generation.terminal?.substring(0, 500));
        }
    }
}
