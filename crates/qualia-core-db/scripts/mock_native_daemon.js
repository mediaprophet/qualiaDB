// mock_native_daemon.js
// Verification Script to test the Qualia Native Daemon Offload (Port 4242)
// Run with: node mock_native_daemon.js

const WebSocket = require('ws');

const wss = new WebSocket.Server({ port: 4242 });
console.log('Mock Qualia Native Daemon listening on ws://127.0.0.1:4242');

wss.on('connection', function connection(ws) {
  let authenticated = false;

  ws.on('message', function incoming(message) {
    const data = message.toString();
    console.log('Received payload:', data);
    
    try {
      const parsed = JSON.parse(data);

      // Handle did:q42 Challenge Handshake
      if (parsed.challenge === 'did:q42') {
        console.log(`Authenticating DID: ${parsed.did}`);
        authenticated = true;
        ws.send(JSON.stringify({ authenticated: true }));
        return;
      }

      // Handle AgentIntent
      if (parsed.rpc === 'infer_local_model' && authenticated) {
        console.log(`Processing intent for prompt: ${parsed.prompt}`);
        
        // Stream back tokens asynchronously to prove non-blocking
        const tokens = ["Gener", "ation ", "is ", "non-", "block", "ing!"];
        let i = 0;
        
        const interval = setInterval(() => {
          if (i < tokens.length) {
            ws.send(JSON.stringify({ text: tokens[i] }));
            i++;
          } else {
            clearInterval(interval);
            ws.send(JSON.stringify({ event: "eos" }));
            console.log("Finished streaming.");
          }
        }, 100); // 100ms delay per token
      } else {
        console.log("Unauthenticated or unknown RPC call.");
        ws.send(JSON.stringify({ error: "Unauthenticated" }));
      }
      
    } catch (e) {
      console.error('Failed to parse JSON', e);
    }
  });
});
