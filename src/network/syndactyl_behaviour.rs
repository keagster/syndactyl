use libp2p_swarm_derive::NetworkBehaviour;
use libp2p::{
    gossipsub::{Behaviour as Gossipsub, Event as GossipsubEvent},
    kad::{Behaviour as Kademlia, store::MemoryStore, Event as KademliaEvent},
    request_response::{
        Behaviour as RequestResponse,
        Event as RequestResponseEvent,
        ProtocolSupport,
        cbor::Behaviour as CborBehaviour,
    },
    StreamProtocol,
};
use crate::core::models::{FileTransferRequest, FileTransferResponse};

/// Type alias for our file transfer request-response behaviour
pub type FileTransferBehaviour = CborBehaviour<FileTransferRequest, FileTransferResponse>;

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
    FileTransfer(RequestResponseEvent<FileTransferRequest, FileTransferResponse>),
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

impl From<RequestResponseEvent<FileTransferRequest, FileTransferResponse>> for SyndactylEvent {
    fn from(event: RequestResponseEvent<FileTransferRequest, FileTransferResponse>) -> Self {
        SyndactylEvent::FileTransfer(event)
    }
}
