use libp2p::{
    kad::{store::MemoryStore, Behaviour as Kademlia},
    mdns::tokio::Behaviour as Mdns,
    request_response::{self, ProtocolSupport},
    swarm::NetworkBehaviour,
    StreamProtocol,
};
use crate::p2p::protocol::QualiaSyncCodec;

#[derive(NetworkBehaviour)]
pub struct QualiaBehaviour {
    pub mdns: Mdns,
    pub kademlia: Kademlia<MemoryStore>,
    pub request_response: request_response::Behaviour<QualiaSyncCodec>,
}

pub fn build_behaviour(local_peer_id: libp2p::PeerId) -> QualiaBehaviour {
    let mdns = Mdns::new(libp2p::mdns::Config::default(), local_peer_id).unwrap();
    let store = MemoryStore::new(local_peer_id);
    let kademlia = Kademlia::new(local_peer_id, store);

    let protocol = StreamProtocol::new("/qualia/crdt-sync/1.0.0");
    let req_res = request_response::Behaviour::new(
        [(protocol, ProtocolSupport::Full)],
        request_response::Config::default(),
    );

    QualiaBehaviour {
        mdns,
        kademlia,
        request_response: req_res,
    }
}
