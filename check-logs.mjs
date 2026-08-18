import fs from 'fs';
const r = JSON.parse(fs.readFileSync('single-test-result.json', 'utf8'));
for (const l of r.logs) {
    if (l.includes('prefill') || l.includes('arch') || l.includes('MC8')) {
        console.log(l);
    }
}
