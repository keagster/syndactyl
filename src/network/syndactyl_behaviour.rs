use libp2p_swarm_derive::NetworkBehaviour;
use libp2p::{
    gossipsub::{Behaviour as Gossipsub, Event as GossipsubEvent},
    kad::{Behaviour as Kademlia, store::MemoryStore, Event as KademliaEvent},
    request_response::{
        Event as RequestResponseEvent,
        cbor::Behaviour as CborBehaviour,
    },
};
use crate::core::models::{SyndactylRequest, FileTransferResponse};

/// Type alias for our file transfer request-response behaviour
pub type FileTransferBehaviour = CborBehaviour<SyndactylRequest, FileTransferResponse>;

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "SyndactylEvent")]
pub struct SyndactylBehaviour {
    pub gossipsub: Gossipsub,
    pub kademlia: Kademlia<MemoryStore>,
    pub file_transfer: FileTransferBehaviour,
}

pub enum SyndactylEvent {
    Gossipsub(GossipsubEvent),
    Kademlia(KademliaEvent),
    FileTransfer(RequestResponseEvent<SyndactylRequest, FileTransferResponse>),
}

impl From<GossipsubEvent> for SyndactylEvent {
    fn from(event: GossipsubEvent) -> Self {
        SyndactylEvent::Gossipsub(event)
    }
}

impl From<KademliaEvent> for SyndactylEvent {
    fn from(event: KademliaEvent) -> Self {
        SyndactylEvent::Kademlia(event)
    }
}

impl From<RequestResponseEvent<SyndactylRequest, FileTransferResponse>> for SyndactylEvent {
    fn from(event: RequestResponseEvent<SyndactylRequest, FileTransferResponse>) -> Self {
        SyndactylEvent::FileTransfer(event)
    }
}
