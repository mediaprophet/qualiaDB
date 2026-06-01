# Webizen Protocol (RFC Draft)

**Layer 1 Application Protocol over Qualia-DB**

This specification defines the `Webizen` protocol—the Layer 1 application standard that provides Identity Management, Social Graph resolution, and Permissive Commons rules over the bare-metal Qualia-DB (Layer 0) engine.

## 1. The Webizen Address Book (Identity Layer)

The Webizen Address Book acts as a "Solid-like" directory stored entirely within the user's local Qualia-DB.

### 1.1 Identity Creation Event
When a user initializes their Webizen Extension, the system generates a root `ed25519-dalek` keypair. This is mathematically anchored into the database as the Root Identity Quin. The private key never leaves the extension's secure storage.

### 1.2 Contact Management
Adding a "friend" or "contact" is fundamentally the act of adding their public DID (Decentralized Identifier) to the local directory. By storing a contact's DID, the local Sentinel VM is authorized to evaluate that contact's signed Quins as part of the user's trusted social graph.

## 2. The `window.webizen` Provider API

To interact with Web3/Social-Web applications, the Webizen browser extension injects a global `window.webizen` object into the DOM. This abstracts the complexity of WebSockets and WASM fallback layers.

### 2.1 Connection Request
```javascript
// A decentralized app requests access to the user's graph
const access = await window.webizen.requestAccess({
    domain: "social-app.com",
    scopes: ["read:public_profile", "write:status"]
});
```
*Intercept:* The extension intercepts this call, preventing direct access to the daemon. It presents a UI prompt asking the user to select an Identity from their Address Book to present to the app.

## 3. Capability Delegation & The Authorization Flow

Web applications cannot write to Qualia-DB directly. 

1. **Quin Construction:** The web app constructs the 48-byte Quin (e.g., posting a status).
2. **Delegated Signing:** The app passes the raw bytes to `window.webizen.signAndInject(quin)`.
3. **Merkle Aggregation:** The Webizen extension cryptographically signs the Merkle Sub-Root using the user's secure Ed25519 key, injecting the authorized data into the Layer 0 graph.

## 4. The Permissive Commons Architecture

*Note: This section requires significant historical input regarding the legacy Permissive Commons frameworks.*

The Permissive Commons defines how shared data (the Bilateral Micro-Commons) is legally and computationally governed between peers. While the Webizen Protocol provides the cryptographic means to share and verify data, the Permissive Commons dictates the **ramifications and rights** associated with that data.

### Open Questions for Permissive Commons Integration:
1. **Revocation Rights:** If a user revokes consent to a previously shared dataset, how does the Permissive Commons dictate the physical erasure of those Quins via Epoch Compaction?
2. **Derivative Works:** How do we encode Permissive Commons licensing rules directly into the 48-byte Quin metadata to prevent unauthorized execution by the Sentinel VM?
3. **Historical Alignment:** (Pending integration of historical Permissive Commons works).
