import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const docsRoot = path.resolve(import.meta.dirname, '..');
const edgeLlmPath = path.join(docsRoot, 'edge-llm.html');

assert.ok(fs.existsSync(edgeLlmPath), 'docs/edge-llm.html must exist');
const html = fs.readFileSync(edgeLlmPath, 'utf8');

// 1. Verify required DOM element IDs
const requiredIds = [
  'chat-messages',
  'chat-input',
  'send-btn',
  'demo-track',
  'demo-thumb',
  'demo-label',
  'daemon-badge',
  'daemon-dot',
  'daemon-text',
  'daemon-banner',
  'stream-bar',
  'sb-tokens',
  'sb-tps',
  'sb-elapsed',
  'sb-ram',
  'dynamic-nav',
];
for (const id of requiredIds) {
  assert.ok(html.includes(`id="${id}"`), `edge-llm.html must contain id="${id}"`);
}

// 2. Verify all local asset references exist on disk
const linkRegex = /(?:href|src)=["']([^"'#:]+)["']/g;
let match;
const localLinks = [];
while ((match = linkRegex.exec(html)) !== null) {
  const target = match[1];
  if (!target.startsWith('http') && !target.startsWith('//') && !target.startsWith('data:')) {
    localLinks.push(target);
  }
}
for (const rel of localLinks) {
  const resolved = path.join(docsRoot, rel.split('?')[0]);
  assert.ok(fs.existsSync(resolved), `Local asset ${rel} must exist at ${resolved}`);
}

// 3. Verify JavaScript syntax in script tags
const scriptMatches = html.match(/<script\b[^>]*>([\s\S]*?)<\/script>/gi) || [];
assert.ok(scriptMatches.length > 0, 'edge-llm.html must contain script tags');
for (const scriptTag of scriptMatches) {
  const code = scriptTag.replace(/^<script\b[^>]*>/i, '').replace(/<\/script>$/i, '').trim();
  if (!code) continue;
  try {
    new Function(code);
  } catch (err) {
    assert.fail(`Syntax error in edge-llm.html script: ${err.message}`);
  }
}

// 4. Verify presence of essential handler functions
assert.ok(html.includes('window.toggleDemo = toggleDemo'), 'toggleDemo must be exported to window');
assert.ok(html.includes('window.sendMessage = sendMessage'), 'sendMessage must be exported to window');
assert.ok(html.includes('window.clearChat = clearChat'), 'clearChat must be exported to window');
assert.ok(html.includes('window.cancelLiveInference = cancelLiveInference'), 'cancelLiveInference must be exported to window');

console.log('edge-llm-smoke test passed.');
