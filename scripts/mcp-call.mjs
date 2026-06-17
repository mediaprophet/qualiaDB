#!/usr/bin/env node
/**
 * Send a single JSON-RPC line to the Qualia MCP TCP service.
 * Usage: node scripts/mcp-call.mjs tools/call run_docs_tests '{"mode":"logic"}'
 */
import net from 'node:net';

const [,, method, toolName, ...rest] = process.argv;
const argsJson =
  process.env.QUALIA_MCP_ARGS ||
  (rest.length > 0 ? rest.join(' ') : '{}');
const bind = process.env.QUALIA_MCP_BIND || '127.0.0.1:4244';
const [host, portStr] = bind.includes(':') ? bind.split(':') : ['127.0.0.1', bind];
const port = Number(portStr);

let payload;
if (method === 'tools/call') {
  payload = {
    jsonrpc: '2.0',
    id: 'cli',
    method: 'tools/call',
    params: { name: toolName, arguments: JSON.parse(argsJson) },
  };
} else {
  payload = { jsonrpc: '2.0', id: 'cli', method: method || 'ping' };
}

const line = JSON.stringify(payload) + '\n';

const socket = net.connect({ host, port }, () => {
  socket.write(line);
});

let buf = '';
socket.on('data', (chunk) => {
  buf += chunk.toString();
  if (buf.includes('\n')) {
    console.log(buf.trim());
    socket.end();
  }
});
socket.on('error', (err) => {
  console.error(`MCP connect ${bind} failed:`, err.message);
  process.exit(1);
});