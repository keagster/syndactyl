use crate::core::config::NetworkConfig;

use libp2p::{
    core::upgrade,
    gossipsub::{
        Behaviour as Gossipsub,
        Config as GossipsubConfig,
        Event as GossipsubEvent,
        MessageAuthenticity,
        IdentTopic as Topic,
    },
    identity,
    swarm::{Swarm, SwarmEvent, Config as SwarmConfig},
    kad::{
        Behaviour as Kademlia,
        Config as KademliaConfig,
        store::MemoryStore,
    },
    tcp::tokio::Transport as TokioTcpTransport,
    yamux::Config as YamuxConfig,
    PeerId, Transport,
    noise::Config as NoiseConfig,
};
use std::error::Error;
use futures::StreamExt;

use std::str::FromStr;
use crate::network::syndactyl_behaviour::{SyndactylBehaviour, SyndactylEvent};

pub struct SyndactylP2P {
    pub peer_id: PeerId,
    pub swarm: Swarm<SyndactylBehaviour>,
}

impl SyndactylP2P {
    pub async fn new(network_config: NetworkConfig) -> Result<Self, Box<dyn Error>> {
        // Generate a random keypair for this peer
        let id_keys = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(id_keys.public());

        // Set up Noise config from identity keypair
        let noise_config = NoiseConfig::new(&id_keys).unwrap();

        // Set up an encrypted TCP transport using Noise and Yamux
        let transport = TokioTcpTransport::default()
            .upgrade(upgrade::Version::V1)
            .authenticate(noise_config)
            .multiplex(YamuxConfig::default())
            .boxed();

        // Create a Gossipsub topic
        let topic = Topic::new("syndactyl-gossip");

        // Set up Gossipsub
        let gossipsub_config = GossipsubConfig::default();
        let mut gossipsub = Gossipsub::new(MessageAuthenticity::Signed(id_keys), gossipsub_config)?;
        gossipsub.subscribe(&topic)?;

        // Set up Kademlia
        let kad_config = KademliaConfig::default();
        let store = MemoryStore::new(peer_id.clone());
        let mut kademlia = Kademlia::with_config(peer_id.clone(), store, kad_config);

        // Add bootstrap peers
        for peer in &network_config.bootstrap_peers {
            let addr = format!("/ip4/{}/tcp/{}/p2p/{}", peer.ip, peer.port, peer.peer_id);
            if let Ok(multiaddr) = addr.parse() {
                if let Ok(peer_id) = PeerId::from_str(&peer.peer_id) {
                    kademlia.add_address(&peer_id, multiaddr);
                }
            }
        }

        // Combine into custom behaviour
        let behaviour = SyndactylBehaviour {
            gossipsub,
            kademlia,
        };

        // Create a Swarm to manage peers and events
        let swarm = Swarm::new(transport, behaviour, peer_id, SwarmConfig::with_tokio_executor());

        Ok(Self { peer_id, swarm })
    }

    pub async fn poll_events(&mut self) {
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(SyndactylEvent::Gossipsub(GossipsubEvent::Message { propagation_source, message_id: _, message })) => {
                    println!("Received: {:?} from {:?}", String::from_utf8_lossy(&message.data), propagation_source);
                }
                SwarmEvent::Behaviour(SyndactylEvent::Kademlia(event)) => {
                    println!("Kademlia event: {:?}", event);
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {:?}", address);
                }
                _ => {}
            }
        }
    }
}
