use libp2p_swarm_derive::NetworkBehaviour;
use libp2p::{
    gossipsub::{Behaviour as Gossipsub, Event as GossipsubEvent},
    kad::{Behaviour as Kademlia, store::MemoryStore, Event as KademliaEvent},
};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "SyndactylEvent")]
pub struct SyndactylBehaviour {
    pub gossipsub: Gossipsub,
    pub kademlia: Kademlia<MemoryStore>,
}

pub enum SyndactylEvent {
    Gossipsub(GossipsubEvent),
    Kademlia(KademliaEvent),
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
