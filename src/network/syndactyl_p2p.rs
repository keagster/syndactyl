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
    swarm::{Swarm, Config as SwarmConfig},
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
use tokio::sync::mpsc::Sender;
use std::str::FromStr;
use crate::network::syndactyl_behaviour::{SyndactylBehaviour, SyndactylEvent};
use tracing::{info, warn, error};
use crate::core::models::FileEventMessage;
use serde_json;

/// Events emitted by the SyndactylP2P node.
#[derive(Debug)]
#[allow(dead_code)]
pub enum SyndactylP2PEvent {
    /// Received a Gossipsub message.
    GossipsubMessage {
        source: PeerId,
        data: Vec<u8>,
    },
    /// Received a Kademlia event.
    KademliaEvent(String),
    /// Node is listening on a new address.
    NewListenAddr(String),
}

/// Main struct for managing the P2P node.
#[allow(dead_code)]
pub struct SyndactylP2P {
    pub peer_id: PeerId,
    pub swarm: Swarm<SyndactylBehaviour>,
    pub event_sender: Sender<SyndactylP2PEvent>,
}

impl SyndactylP2P {
    /// Create a new SyndactylP2P node with the given config and event sender.
    pub async fn new(network_config: NetworkConfig, event_sender: Sender<SyndactylP2PEvent>) -> Result<Self, Box<dyn Error>> {
        use std::fs;

        // Try to load keypair from disk, or generate and save if not present
        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").expect("HOME not set");
                std::path::PathBuf::from(home).join(".config")
            });
        let syndactyl_dir = config_dir.join("syndactyl");
        let keypair_path = syndactyl_dir.join("syndactyl_keypair.key");
        if !syndactyl_dir.exists() {
            std::fs::create_dir_all(&syndactyl_dir).map_err(|e| {
                eprintln!("[syndactyl][error] Failed to create config dir: {}", e);
                e
            })?;
        }
        let id_keys = if keypair_path.exists() {
            let bytes = fs::read(&keypair_path).map_err(|e| {
                eprintln!("[syndactyl][error] Failed to read keypair: {}", e);
                e
            })?;
            identity::Keypair::from_protobuf_encoding(&bytes).map_err(|e| {
                eprintln!("[syndactyl][error] Failed to decode keypair: {}", e);
                e
            })?
        } else {
            let kp = identity::Keypair::generate_ed25519();
            let bytes = kp.to_protobuf_encoding().map_err(|e| {
                eprintln!("[syndactyl][error] Failed to encode keypair: {}", e);
                e
            })?;
            fs::write(&keypair_path, &bytes).map_err(|e| {
                eprintln!("[syndactyl][error] Failed to write keypair: {}", e);
                e
            })?;
            kp
        };
        let peer_id = PeerId::from(id_keys.public());
        info!(peer_id = %peer_id, "[syndactyl] Local PeerId");
        info!(key_path = %keypair_path.display(), "[syndactyl] Your persistent key is stored at");

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
        let mut swarm = Swarm::new(transport, behaviour, peer_id, SwarmConfig::with_tokio_executor());

        // Listen on the address and port specified in network_config
        let listen_addr = format!(
            "/ip4/{}/tcp/{}",
            network_config.listen_addr, network_config.port
        );
        let listen_addr = listen_addr.parse()?;
        swarm.listen_on(listen_addr)?;

        Ok(Self { peer_id, swarm, event_sender })
    }

    /// Get the local PeerId.
    #[allow(dead_code)]
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    /// Publish a message to the default Gossipsub topic.
    #[allow(dead_code)]
    pub fn publish_gossipsub(&mut self, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        let topic = Topic::new("syndactyl-gossip");
        self.swarm.behaviour_mut().gossipsub.publish(topic, data)?;
        Ok(())
    }

    /// Start a Kademlia peer lookup.
    #[allow(dead_code)]
    pub fn find_peer(&mut self, peer_id: PeerId) {
        self.swarm.behaviour_mut().kademlia.get_closest_peers(peer_id);
    }

    /// Subscribe to a Gossipsub topic.
    #[allow(dead_code)]
    pub fn subscribe_topic(&mut self, topic_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let topic = Topic::new(topic_name);
        self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        Ok(())
    }

    /// Unsubscribe from a Gossipsub topic.
    #[allow(dead_code)]
    pub fn unsubscribe_topic(&mut self, topic_name: &str) {
        let topic = Topic::new(topic_name);
        let unsubscribed = self.swarm.behaviour_mut().gossipsub.unsubscribe(&topic);
        info!(topic = topic_name, unsubscribed, "Unsubscribe from topic");
    }

    /// Store a record in the Kademlia DHT.
    #[allow(dead_code)]
    pub fn put_record(&mut self, key: &str, value: Vec<u8>) {
        use libp2p::kad::{Record, Quorum, RecordKey};
        let record = Record {
            key: RecordKey::new(&key),
            value,
            publisher: None,
            expires: None,
        };
        if let Err(e) = self.swarm.behaviour_mut().kademlia.put_record(record, Quorum::One) {
            error!(%e, "[syndactyl][error] Failed to store record");
        }
    }

    /// Retrieve a record from the Kademlia DHT.
    #[allow(dead_code)]
    pub fn get_record(&mut self, key: &str) {
        use libp2p::kad::RecordKey;
        let key = RecordKey::new(&key);
        self.swarm.behaviour_mut().kademlia.get_record(key);
    }

    pub async fn poll_events(&mut self) {
        use libp2p::swarm::SwarmEvent;
        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::Behaviour(SyndactylEvent::Gossipsub(GossipsubEvent::Message { propagation_source, message_id: _, message })) => {
                    // Try to deserialize as FileEventMessage
                    match serde_json::from_slice::<FileEventMessage>(&message.data) {
                        Ok(file_event) => {
                            info!(peer = %propagation_source, event = ?file_event, "[syndactyl][gossipsub] Received FileEventMessage");
                            // Here you can add logic to process/apply the event
                        },
                        Err(e) => {
                            warn!(peer = %propagation_source, error = ?e, raw = %String::from_utf8_lossy(&message.data), "[syndactyl][gossipsub] Failed to parse FileEventMessage");
                        }
                    }
                    let _ = self.event_sender.send(SyndactylP2PEvent::GossipsubMessage {
                        source: propagation_source,
                        data: message.data,
                    }).await;
                }
                SwarmEvent::Behaviour(SyndactylEvent::Kademlia(event)) => {
                    info!(event = ?event, "[syndactyl][kademlia] Event");
                    let _ = self.event_sender.send(SyndactylP2PEvent::KademliaEvent(format!("{:?}", event))).await;
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!(address = %address, "[syndactyl][swarm] Listening on");
                    let _ = self.event_sender.send(SyndactylP2PEvent::NewListenAddr(address.to_string())).await;
                }
                SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                    info!(peer_id = %peer_id, endpoint = ?endpoint, "[syndactyl][swarm] Connection established");
                }
                SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                    warn!(peer_id = %peer_id, ?cause, "[syndactyl][swarm] Connection closed");
                }
                _ => {
                    // Uncomment for verbose debugging:
                    // println!("[syndactyl][swarm] Other event");
                }
            }
        }
    }
}
