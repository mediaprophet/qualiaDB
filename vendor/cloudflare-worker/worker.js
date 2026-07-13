// Qualia Discovery Protocol - Offline Mailbox Worker
// This worker acts as a dumb pipe that verifies signatures, queues messages in R2, and is blind to the payload.

export default {
  async fetch(request, env, ctx) {
    const url = new URL(request.url);
    const path = url.pathname;

    // POST /inbox/:did - Receive an encrypted message
    if (request.method === "POST" && path.startsWith("/inbox/")) {
      const targetDid = path.split("/")[2];
      
      // We expect the payload to be signed by the sender
      const signatureHex = request.headers.get("X-Signature");
      const senderPubKeyHex = request.headers.get("X-Sender-Pubkey");

      if (!signatureHex || !senderPubKeyHex) {
        return new Response("Missing signature or pubkey", { status: 400 });
      }

      const bodyBuffer = await request.arrayBuffer();
      
      // Verify Ed25519 signature
      const isValid = await verifyEd25519(bodyBuffer, signatureHex, senderPubKeyHex);
      if (!isValid) {
        return new Response("Invalid signature", { status: 401 });
      }

      // Store in R2 (blind to the encrypted payload)
      const msgId = crypto.randomUUID();
      const r2Key = `inbox/${targetDid}/${msgId}`;
      
      // env.OFFLINE_STORE is the R2 bucket binding
      await env.OFFLINE_STORE.put(r2Key, bodyBuffer, {
        customMetadata: {
          sender: senderPubKeyHex,
          timestamp: Date.now().toString()
        }
      });

      return new Response(JSON.stringify({ success: true, msgId }), { 
        status: 202,
        headers: { "Content-Type": "application/json" }
      });
    }

    // GET /inbox/:did - Retrieve messages (Owner only)
    if (request.method === "GET" && path.startsWith("/inbox/")) {
      const targetDid = path.split("/")[2];
      
      // Simple auth for the owner (the desktop app will provide a bearer token)
      const auth = request.headers.get("Authorization");
      if (auth !== `Bearer ${env.OWNER_SECRET}`) {
        return new Response("Unauthorized", { status: 401 });
      }

      const prefix = `inbox/${targetDid}/`;
      const listed = await env.OFFLINE_STORE.list({ prefix });
      
      const messages = [];
      for (const obj of listed.objects) {
        messages.push({
          id: obj.key.replace(prefix, ""),
          sender: obj.customMetadata?.sender,
          timestamp: obj.customMetadata?.timestamp
        });
      }

      return new Response(JSON.stringify({ messages }), {
        status: 200,
        headers: { "Content-Type": "application/json" }
      });
    }
    
    // GET /inbox/:did/:msgId - Download specific message
    if (request.method === "GET" && path.startsWith("/download/")) {
       const parts = path.split("/");
       const targetDid = parts[2];
       const msgId = parts[3];
       
       const auth = request.headers.get("Authorization");
       if (auth !== `Bearer ${env.OWNER_SECRET}`) {
         return new Response("Unauthorized", { status: 401 });
       }
       
       const obj = await env.OFFLINE_STORE.get(`inbox/${targetDid}/${msgId}`);
       if (!obj) {
           return new Response("Not found", { status: 404 });
       }
       
       return new Response(obj.body, { status: 200 });
    }

    // DELETE /inbox/:did/:msgId - Remove message after sync
    if (request.method === "DELETE" && path.startsWith("/inbox/")) {
      const parts = path.split("/");
      const targetDid = parts[2];
      const msgId = parts[3];

      const auth = request.headers.get("Authorization");
      if (auth !== `Bearer ${env.OWNER_SECRET}`) {
        return new Response("Unauthorized", { status: 401 });
      }

      await env.OFFLINE_STORE.delete(`inbox/${targetDid}/${msgId}`);
      return new Response(JSON.stringify({ success: true }), { status: 200 });
    }

    return new Response("Not Found", { status: 404 });
  }
};

// Helper to verify Ed25519 signatures via WebCrypto API
async function verifyEd25519(data, signatureHex, pubKeyHex) {
  try {
    const signatureBuffer = hexToArrayBuffer(signatureHex);
    const pubKeyBuffer = hexToArrayBuffer(pubKeyHex);

    const key = await crypto.subtle.importKey(
      "raw",
      pubKeyBuffer,
      { name: "Ed25519" },
      false,
      ["verify"]
    );

    return await crypto.subtle.verify(
      "Ed25519",
      key,
      signatureBuffer,
      data
    );
  } catch (e) {
    return false;
  }
}

function hexToArrayBuffer(hex) {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes.buffer;
}
