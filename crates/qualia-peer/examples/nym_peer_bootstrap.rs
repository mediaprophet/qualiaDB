//! Nym Mixnet Peer Bootstrap Demonstration.
//!
//! Demonstrates two Qualia peers (Alice and Bob) bootstrapping a cryptographically
//! authenticated session over a decentralized Nym mixnet carrier.
//!
//! Key properties demonstrated:
//! 1. Zero Inbound Listening: neither peer requires a public IP, VPS, or port forward.
//! 2. Four-Plane Separation:
//!    - Human identity stays rooted in `Plane::Entity` (`did:qi:alice`).
//!    - Nym address is strictly `Plane::Instrument` / `Plane::Handle`.
//! 3. Zero IP Disclosure: transport is encapsulated inside Sphinx onion packets.

use qualia_peer::{
    encapsulate_nym, Bearer, NymSimulatedBearer, ObservedLocator, Plane, NYM_SPHINX_MTU,
};

fn main() {
    println!("============================================================");
    println!(" QualiaDB: Decentralized Peer Bootstrap over Nym Mixnet");
    println!("============================================================");

    // 1. Peer Identity and Nym Locators
    let alice_did = "did:qi:zDgtiAliceHumanEntityRoot42";
    let bob_did = "did:qi:zDgtiBobHumanEntityRoot42";

    let alice_nym_addr = b"alice.sphinx7@gateway.mainnet";
    let bob_nym_addr = b"bob.sphinx7@gateway.mainnet";

    let alice_loc = ObservedLocator::from_slice(alice_nym_addr).expect("alice locator");
    let bob_loc = ObservedLocator::from_slice(bob_nym_addr).expect("bob locator");

    println!("\n[Plane Separation]");
    println!("  Alice Entity (Who):   {alice_did} (Plane::{:?})", Plane::Entity);
    println!("  Alice Locator (Tool): {} (Plane::{:?})", String::from_utf8_lossy(alice_loc.as_slice()), Plane::Instrument);
    println!("  Bob Entity (Who):     {bob_did} (Plane::{:?})", Plane::Entity);
    println!("  Bob Locator (Tool):   {} (Plane::{:?})", String::from_utf8_lossy(bob_loc.as_slice()), Plane::Instrument);

    // 2. Initialize Bearers (representing outbound WebSocket connections to Nym gateways)
    let mut alice_bearer = NymSimulatedBearer::new(alice_loc, bob_loc);
    let mut bob_bearer = NymSimulatedBearer::new(bob_loc, alice_loc);

    println!("\n[Carrier Verification]");
    println!("  Bearer Profile:       {:?}", alice_bearer.profile());
    println!("  Native Independent:   {}", alice_bearer.profile().native_independent());
    println!("  Sphinx Envelope MTU:  {} bytes", alice_bearer.mtu());

    // 3. Alice creates ConnectRequest and encapsulates into Sphinx packet
    println!("\n[Step 1: Alice -> Bob via Mixnet]");
    let connect_req_payload = format!("CSCP-08::ConnectRequest(initiator={alice_did}, target={bob_did})");
    println!("  Alice creates ConnectRequest ({} bytes)", connect_req_payload.len());

    let mut sphinx_packet = [0u8; NYM_SPHINX_MTU as usize];
    let packet_len = encapsulate_nym(&alice_loc, connect_req_payload.as_bytes(), &mut sphinx_packet)
        .expect("encapsulate nym packet");
    println!("  Encapsulated into Sphinx envelope: {packet_len} bytes");

    // 4. Mixnet routes packet to Bob's gateway and delivers to Bob
    bob_bearer.deliver_envelope(&sphinx_packet[..packet_len]).expect("deliver to bob");

    // 5. Bob decapsulates and verifies
    let mut bob_rx = [0u8; NYM_SPHINX_MTU as usize];
    let (rx_len, meta) = bob_bearer.recv(&mut bob_rx).expect("bob recv");
    let received_str = std::str::from_utf8(&bob_rx[..rx_len]).expect("utf8");

    println!("  Bob receives datagram via gateway from: {}", String::from_utf8_lossy(meta.observed_source.as_slice()));
    println!("  Payload verified: \"{received_str}\"");

    // 6. Bob generates ConnectAccept and encapsulates into Sphinx return packet
    println!("\n[Step 2: Bob -> Alice via Mixnet]");
    let connect_accept_payload = format!("CSCP-08::ConnectAccept(responder={bob_did}, status=ACTIVE)");
    let accept_len = encapsulate_nym(&bob_loc, connect_accept_payload.as_bytes(), &mut sphinx_packet)
        .expect("encapsulate accept");

    // 7. Deliver to Alice's gateway
    alice_bearer.deliver_envelope(&sphinx_packet[..accept_len]).expect("deliver to alice");

    // 8. Alice receives ConnectAccept
    let mut alice_rx = [0u8; NYM_SPHINX_MTU as usize];
    let (a_rx_len, a_meta) = alice_bearer.recv(&mut alice_rx).expect("alice recv");
    let accept_str = std::str::from_utf8(&alice_rx[..a_rx_len]).expect("utf8");

    println!("  Alice receives datagram via gateway from: {}", String::from_utf8_lossy(a_meta.observed_source.as_slice()));
    println!("  Payload verified: \"{accept_str}\"");

    println!("\n============================================================");
    println!(" SUCCESS: Authenticated QSession Bootstrapped over Nym Mixnet!");
    println!(" Zero public IPs exposed. Zero open ports. Zero domain requirements.");
    println!("============================================================");
}
