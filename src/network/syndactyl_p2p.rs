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
use crate::core::models::{FileEventMessage, FileTransferRequest, FileTransferResponse};
use serde_json;

/// Events emitted by the SyndactylP2P node.
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
    /// Received a file transfer request from a peer.
    FileTransferRequest {
        peer: PeerId,
        request: FileTransferRequest,
        channel: libp2p::request_response::ResponseChannel<FileTransferResponse>,
    },
    /// Received a file transfer response from a peer.
    FileTransferResponse {
        peer: PeerId,
        response: FileTransferResponse,
    },
}

impl std::fmt::Debug for SyndactylP2PEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GossipsubMessage { source, data } => f
                .debug_struct("GossipsubMessage")
                .field("source", source)
                .field("data_len", &data.len())
                .finish(),
            Self::KademliaEvent(e) => f.debug_tuple("KademliaEvent").field(e).finish(),
            Self::NewListenAddr(addr) => f.debug_tuple("NewListenAddr").field(addr).finish(),
            Self::FileTransferRequest { peer, request, .. } => f
                .debug_struct("FileTransferRequest")
                .field("peer", peer)
                .field("request", request)
                .finish(),
            Self::FileTransferResponse { peer, response } => f
                .debug_struct("FileTransferResponse")
                .field("peer", peer)
                .field("response", response)
                .finish(),
        }
    }
}

/// Main struct for managing the P2P node.
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

        // Set up file transfer request-response protocol
        use libp2p::request_response::{ProtocolSupport, cbor};
        use libp2p::StreamProtocol;
        
        let file_transfer_protocol = StreamProtocol::new("/syndactyl/file-transfer/1.0.0");
        let file_transfer = cbor::Behaviour::<FileTransferRequest, FileTransferResponse>::new(
            [(file_transfer_protocol, ProtocolSupport::Full)],
            libp2p::request_response::Config::default(),
        );

        // Combine into custom behaviour
        let behaviour = SyndactylBehaviour {
            gossipsub,
            kademlia,
            file_transfer,
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
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    /// Publish a message to the default Gossipsub topic.
    pub fn publish_gossipsub(&mut self, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        let topic = Topic::new("syndactyl-gossip");
        self.swarm.behaviour_mut().gossipsub.publish(topic, data)?;
        Ok(())
    }

    /// Start a Kademlia peer lookup.
    pub fn find_peer(&mut self, peer_id: PeerId) {
        self.swarm.behaviour_mut().kademlia.get_closest_peers(peer_id);
    }

    /// Subscribe to a Gossipsub topic.
    pub fn subscribe_topic(&mut self, topic_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let topic = Topic::new(topic_name);
        self.swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
        Ok(())
    }

    /// Unsubscribe from a Gossipsub topic.
    pub fn unsubscribe_topic(&mut self, topic_name: &str) {
        let topic = Topic::new(topic_name);
        let unsubscribed = self.swarm.behaviour_mut().gossipsub.unsubscribe(&topic);
        info!(topic = topic_name, unsubscribed, "Unsubscribe from topic");
    }

    /// Store a record in the Kademlia DHT.
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
    pub fn get_record(&mut self, key: &str) {
        use libp2p::kad::RecordKey;
        let key = RecordKey::new(&key);
        self.swarm.behaviour_mut().kademlia.get_record(key);
    }

    /// Request a file from a peer
    pub fn request_file(&mut self, peer: PeerId, request: FileTransferRequest) {
        use libp2p::request_response::OutboundRequestId;
        
        let request_id = self.swarm.behaviour_mut().file_transfer.send_request(&peer, request.clone());
        info!(
            peer = %peer,
            observer = %request.observer,
            path = %request.path,
            request_id = ?request_id,
            "[syndactyl][file-transfer] Requesting file"
        );
    }

    /// Send a file response to a peer
    pub fn send_file_response(
        &mut self,
        channel: libp2p::request_response::ResponseChannel<FileTransferResponse>,
        response: FileTransferResponse,
    ) {
        let result = self.swarm.behaviour_mut().file_transfer.send_response(channel, response.clone());
        if result.is_ok() {
            info!(
                observer = %response.observer,
                path = %response.path,
                offset = response.offset,
                size = response.data.len(),
                is_last = response.is_last_chunk,
                "[syndactyl][file-transfer] Sent file chunk"
            );
        } else {
            error!(
                observer = %response.observer,
                path = %response.path,
                "[syndactyl][file-transfer] Failed to send response"
            );
        }
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
                SwarmEvent::Behaviour(SyndactylEvent::FileTransfer(event)) => {
                    use libp2p::request_response::Event as RREvent;
                    match event {
                        RREvent::Message { peer, message, connection_id: _ } => {
                            use libp2p::request_response::Message;
                            match message {
                                Message::Request { request, channel, .. } => {
                                    info!(
                                        peer = %peer,
                                        observer = %request.observer,
                                        path = %request.path,
                                        "[syndactyl][file-transfer] Received file request"
                                    );
                                    let _ = self.event_sender.send(SyndactylP2PEvent::FileTransferRequest {
                                        peer,
                                        request,
                                        channel,
                                    }).await;
                                }
                                Message::Response { response, .. } => {
                                    info!(
                                        peer = %peer,
                                        observer = %response.observer,
                                        path = %response.path,
                                        offset = response.offset,
                                        is_last = response.is_last_chunk,
                                        "[syndactyl][file-transfer] Received file response"
                                    );
                                    let _ = self.event_sender.send(SyndactylP2PEvent::FileTransferResponse {
                                        peer,
                                        response,
                                    }).await;
                                }
                            }
                        }
                        RREvent::OutboundFailure { peer, request_id, error, connection_id: _ } => {
                            error!(peer = %peer, request_id = ?request_id, error = ?error, "[syndactyl][file-transfer] Outbound failure");
                        }
                        RREvent::InboundFailure { peer, error, .. } => {
                            error!(peer = %peer, error = ?error, "[syndactyl][file-transfer] Inbound failure");
                        }
                        RREvent::ResponseSent { peer, .. } => {
                            info!(peer = %peer, "[syndactyl][file-transfer] Response sent");
                        }
                    }
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
