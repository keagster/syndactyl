use crate::network::syndactyl_p2p::{SyndactylP2P, SyndactylP2PEvent};
use crate::network::transfer::{FileTransferTracker, generate_first_chunk, CHUNK_SIZE};
use crate::network::syndactyl_behaviour::SyndactylEvent;
use crate::core::models::{FileTransferRequest, FileTransferResponse, FileChunkRequest, FileEventMessage};
use crate::core::config::{Config, ObserverConfig};
use crate::core::{file_handler, auth};

use std::collections::HashMap;
use std::path::PathBuf;
use std::thread;

use libp2p::PeerId;
use tokio::sync::mpsc as tokio_mpsc;
use futures::StreamExt;
use tracing::{info, error, warn};

/// Manages the P2P network, file transfers, and observer event integration
pub struct NetworkManager {
    p2p: SyndactylP2P,
    observer_configs: HashMap<String, ObserverConfig>,
    connected_peers: Vec<PeerId>,
    transfer_tracker: FileTransferTracker,
    event_receiver: tokio_mpsc::Receiver<SyndactylP2PEvent>,
}

impl NetworkManager {
    /// Create a new NetworkManager from configuration
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let network_config = config.network
            .ok_or("Network configuration is required")?;

        // Build a map of observer name -> ObserverConfig for authentication and file operations
        let mut observer_configs: HashMap<String, ObserverConfig> = HashMap::new();
        for obs in &config.observers {
            observer_configs.insert(obs.name.clone(), obs.clone());
        }

        // Create P2P node
        let (event_sender, event_receiver) = tokio_mpsc::channel(32);
        let p2p = SyndactylP2P::new(network_config, event_sender).await?;

        Ok(Self {
            p2p,
            observer_configs,
            connected_peers: Vec::new(),
            transfer_tracker: FileTransferTracker::new(),
            event_receiver,
        })
    }

    /// Run the network manager event loop, integrating observer events
    pub async fn run(mut self, observer_rx: std::sync::mpsc::Receiver<String>) {
        // Use a tokio channel to bridge observer events into the async context
        let (obs_tx, mut obs_rx) = tokio_mpsc::channel::<String>(32);
        
        // Spawn a thread to forward std_mpsc observer_rx to async obs_tx
        let _observer_thread_forward = thread::spawn(move || {
            while let Ok(msg) = observer_rx.recv() {
                let _ = obs_tx.blocking_send(msg);
            }
        });

        info!("[NetworkManager] Starting event loop");

        // Main async loop: handle both observer events, P2P events, and swarm events
        loop {
            tokio::select! {
                Some(msg) = obs_rx.recv() => {
                    self.handle_observer_message(msg);
                },
                Some(event) = self.event_receiver.recv() => {
                    self.handle_p2p_event(event).await;
                },
                swarm_event = self.p2p.swarm.select_next_some() => {
                    self.handle_swarm_event(swarm_event).await;
                },
                else => {
                    info!("[NetworkManager] All channels closed, shutting down");
                    break;
                }
            }
        }
    }

    /// Handle observer file change messages
    fn handle_observer_message(&mut self, msg: String) {
        info!(msg = %msg, "Forwarding observer event to P2P");
        let _ = self.p2p.publish_gossipsub(msg.into_bytes());
    }

    /// Handle P2P events from the event channel
    async fn handle_p2p_event(&mut self, event: SyndactylP2PEvent) {
        match event {
            SyndactylP2PEvent::GossipsubMessage { source, data } => {
                self.handle_gossipsub_message(source, data);
            }
            SyndactylP2PEvent::KademliaEvent(info) => {
                info!(%info, "Kademlia event");
            }
            SyndactylP2PEvent::NewListenAddr(addr) => {
                info!(%addr, "Listening on");
            }
            SyndactylP2PEvent::FileTransferRequest { peer, request, channel } => {
                self.handle_file_transfer_request(peer, request, channel);
            }
            SyndactylP2PEvent::FileTransferResponse { peer, response } => {
                self.handle_file_transfer_response(peer, response);
            }
            SyndactylP2PEvent::FileChunkRequest { peer, request, channel } => {
                self.handle_file_chunk_request(peer, request, channel);
            }
        }
    }

    /// Handle Gossipsub messages (file events from other peers)
    fn handle_gossipsub_message(&mut self, source: PeerId, data: Vec<u8>) {
        match serde_json::from_slice::<FileEventMessage>(&data) {
            Ok(file_event) => {
                info!(peer = %source, event = ?file_event, "Received FileEventMessage from P2P");
                
                // Verify HMAC if we have a shared secret for this observer
                if let Some(observer_config) = self.observer_configs.get(&file_event.observer) {
                    if let Some(ref secret) = observer_config.shared_secret {
                        // Verify HMAC
                        if !auth::verify_hmac(&file_event, secret) {
                            warn!(
                                peer = %source,
                                observer = %file_event.observer,
                                "HMAC verification failed - rejecting unauthorized file event"
                            );
                            return;
                        }
                        info!(peer = %source, observer = %file_event.observer, "HMAC verified successfully");
                    } else {
                        warn!(
                            peer = %source,
                            observer = %file_event.observer,
                            "No shared secret configured for observer - accepting unauthenticated message (INSECURE)"
                        );
                    }
                } else {
                    info!(observer = %file_event.observer, "Observer not configured locally, ignoring event");
                    return;
                }
                
                // Check if this is a Create or Modify event with a file we should sync
                if matches!(file_event.event_type.as_str(), "Create" | "Modify") {
                    self.process_file_event(source, file_event);
                }
            },
            Err(e) => {
                warn!(peer = %source, error = ?e, raw = %String::from_utf8_lossy(&data), "Failed to parse FileEventMessage from P2P");
            }
        }
    }

    /// Process a file event and potentially request the file
    fn process_file_event(&mut self, peer: PeerId, file_event: FileEventMessage) {
        // Check if we have this observer configured locally
        if let Some(observer_config) = self.observer_configs.get(&file_event.observer) {
            let base_path = PathBuf::from(&observer_config.path);
            let relative_path = std::path::Path::new(&file_event.path);
            let absolute_path = file_handler::to_absolute_path(relative_path, &base_path);
            
            // Check if we need to request this file
            let should_request = if absolute_path.exists() {
                // File exists, check if hash is different
                if let Some(remote_hash) = &file_event.hash {
                    if let Ok(local_hash) = file_handler::calculate_file_hash(&absolute_path) {
                        &local_hash != remote_hash
                    } else {
                        true // Can't calculate local hash, request file
                    }
                } else {
                    false // No hash provided, skip
                }
            } else {
                true // File doesn't exist, request it
            };
            
            if should_request {
                if let Some(hash) = file_event.hash {
                    info!(
                        observer = %file_event.observer,
                        path = %file_event.path,
                        "Requesting file from peer"
                    );
                    
                    let request = FileTransferRequest {
                        observer: file_event.observer.clone(),
                        path: file_event.path.clone(),
                        hash: hash.clone(),
                    };
                    
                    // Start tracking this transfer
                    if let Some(size) = file_event.size {
                        self.transfer_tracker.start_transfer(
                            file_event.observer.clone(),
                            file_event.path.clone(),
                            size,
                            hash,
                            base_path.clone(),
                        );
                    }
                    
                    // Send request to the peer who sent the event
                    self.p2p.request_file(peer, request);
                } else {
                    warn!(observer = %file_event.observer, path = %file_event.path, "No hash provided in file event");
                }
            } else {
                info!(observer = %file_event.observer, path = %file_event.path, "File already up to date, skipping");
            }
        } else {
            info!(observer = %file_event.observer, "Observer not configured locally, ignoring event");
        }
    }

    /// Handle file transfer request
    fn handle_file_transfer_request(
        &mut self,
        peer: PeerId,
        request: FileTransferRequest,
        channel: libp2p::request_response::ResponseChannel<FileTransferResponse>,
    ) {
        info!(peer = %peer, observer = %request.observer, path = %request.path, "Received file transfer request");
        
        // Check if we have this observer configured
        if let Some(observer_config) = self.observer_configs.get(&request.observer) {
            // TODO: In the next task, we'll add peer allowlist checking here
            // For now, we log that authorization should be checked
            if observer_config.shared_secret.is_some() {
                info!(peer = %peer, observer = %request.observer, "Observer has authentication enabled");
                // Note: Peer allowlist will be checked in the next implementation phase
            } else {
                warn!(peer = %peer, observer = %request.observer, "Observer has no authentication - serving file (INSECURE)");
            }
            
            let base_path = PathBuf::from(&observer_config.path);
            let relative_path = std::path::Path::new(&request.path);
            let absolute_path = file_handler::to_absolute_path(relative_path, &base_path);
            
            if absolute_path.exists() && absolute_path.is_file() {
                // Generate only the first chunk for initial response
                match generate_first_chunk(
                    &request.observer,
                    relative_path,
                    &absolute_path,
                    &request.hash,
                ) {
                    Ok(first_chunk) => {
                        info!(
                            observer = %request.observer,
                            path = %request.path,
                            size = first_chunk.total_size,
                            is_last = first_chunk.is_last_chunk,
                            "Sending first file chunk"
                        );
                        self.p2p.send_file_response(channel, first_chunk);
                    }
                    Err(e) => {
                        error!(
                            observer = %request.observer,
                            path = %request.path,
                            error = %e,
                            "Failed to generate first chunk"
                        );
                    }
                }
            } else {
                warn!(
                    observer = %request.observer,
                    path = %request.path,
                    "File not found or not a file"
                );
            }
        } else {
            warn!(observer = %request.observer, "Observer not configured locally");
        }
    }

    /// Handle file transfer response
    fn handle_file_transfer_response(&mut self, peer: PeerId, response: FileTransferResponse) {
        info!(
            peer = %peer,
            observer = %response.observer,
            path = %response.path,
            offset = response.offset,
            size = response.data.len(),
            is_last = response.is_last_chunk,
            "Received file transfer response"
        );
        
        // Add chunk to transfer tracker
        match self.transfer_tracker.add_chunk(
            &response.observer,
            &response.path,
            response.offset,
            response.data.clone(),
            response.is_last_chunk,
        ) {
            Ok(Some(file_path)) => {
                info!(
                    observer = %response.observer,
                    path = %response.path,
                    file = %file_path.display(),
                    "File transfer completed and written to disk"
                );
            }
            Ok(None) => {
                info!(
                    observer = %response.observer,
                    path = %response.path,
                    "Chunk received, requesting next chunk"
                );
                // Request next chunk if not last
                if !response.is_last_chunk {
                    let next_offset = response.offset + response.data.len() as u64;
                    let chunk_request = FileChunkRequest {
                        observer: response.observer.clone(),
                        path: response.path.clone(),
                        offset: next_offset,
                        hash: response.hash.clone(),
                    };
                    self.p2p.request_file_chunk(peer, chunk_request);
                }
            }
            Err(e) => {
                error!(
                    observer = %response.observer,
                    path = %response.path,
                    error = %e,
                    "Failed to process file chunk"
                );
            }
        }
    }

    /// Handle file chunk request
    fn handle_file_chunk_request(
        &mut self,
        peer: PeerId,
        request: FileChunkRequest,
        channel: libp2p::request_response::ResponseChannel<FileTransferResponse>,
    ) {
        info!(
            peer = %peer,
            observer = %request.observer,
            path = %request.path,
            offset = request.offset,
            "Received file chunk request"
        );
        
        // Check if we have this observer configured
        if let Some(observer_config) = self.observer_configs.get(&request.observer) {
            // TODO: In the next task, we'll add peer allowlist checking here
            if observer_config.shared_secret.is_some() {
                info!(peer = %peer, observer = %request.observer, "Observer has authentication enabled");
                // Note: Peer allowlist will be checked in the next implementation phase
            }
            
            let base_path = PathBuf::from(&observer_config.path);
            let relative_path = std::path::Path::new(&request.path);
            let absolute_path = file_handler::to_absolute_path(relative_path, &base_path);
            if absolute_path.exists() && absolute_path.is_file() {
                match file_handler::read_file_chunk(&absolute_path, request.offset, CHUNK_SIZE) {
                    Ok(data) => {
                        let total_size = absolute_path.metadata().map(|m| m.len()).unwrap_or(0);
                        let is_last_chunk = request.offset + data.len() as u64 >= total_size;
                        let response = FileTransferResponse {
                            observer: request.observer.clone(),
                            path: request.path.clone(),
                            data,
                            offset: request.offset,
                            total_size,
                            hash: request.hash.clone(),
                            is_last_chunk,
                        };
                        self.p2p.send_file_response(channel, response);
                    }
                    Err(e) => {
                        error!(
                            observer = %request.observer,
                            path = %request.path,
                            error = %e,
                            "Failed to read file chunk"
                        );
                    }
                }
            } else {
                warn!(
                    observer = %request.observer,
                    path = %request.path,
                    "File not found or not a file for chunk request"
                );
            }
        } else {
            warn!(observer = %request.observer, "Observer not configured locally for chunk request");
        }
    }

    /// Handle swarm events directly
    async fn handle_swarm_event(&mut self, event: libp2p::swarm::SwarmEvent<SyndactylEvent>) {
        use libp2p::swarm::SwarmEvent;
        use libp2p::gossipsub::Event as GossipsubEvent;

        match event {
            SwarmEvent::Behaviour(SyndactylEvent::Gossipsub(GossipsubEvent::Message { propagation_source, message_id: _, message })) => {
                // Try to deserialize as FileEventMessage
                match serde_json::from_slice::<FileEventMessage>(&message.data) {
                    Ok(file_event) => {
                        info!(peer = %propagation_source, event = ?file_event, "[syndactyl][gossipsub] Received FileEventMessage");
                        
                        // Check if this is a Create or Modify event with a file we should sync
                        if matches!(file_event.event_type.as_str(), "Create" | "Modify") {
                            self.process_file_event(propagation_source, file_event);
                        }
                    },
                    Err(e) => {
                        warn!(peer = %propagation_source, error = ?e, raw = %String::from_utf8_lossy(&message.data), "[syndactyl][gossipsub] Failed to parse FileEventMessage");
                    }
                }
            }
            SwarmEvent::Behaviour(SyndactylEvent::Kademlia(event)) => {
                info!(event = ?event, "[syndactyl][kademlia] Event");
            }
            SwarmEvent::Behaviour(SyndactylEvent::FileTransfer(event)) => {
                self.handle_file_transfer_swarm_event(event);
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                info!(address = %address, "[syndactyl][swarm] Listening on");
            }
            SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                info!(peer_id = %peer_id, endpoint = ?endpoint, "[syndactyl][swarm] Connection established");
                if !self.connected_peers.contains(&peer_id) {
                    self.connected_peers.push(peer_id);
                }
            }
            SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                warn!(peer_id = %peer_id, ?cause, "[syndactyl][swarm] Connection closed");
                self.connected_peers.retain(|p| p != &peer_id);
            }
            _ => {
                // Other swarm events
            }
        }
    }

    /// Handle file transfer events from the swarm
    fn handle_file_transfer_swarm_event(
        &mut self,
        event: libp2p::request_response::Event<
            crate::core::models::SyndactylRequest,
            FileTransferResponse,
        >,
    ) {
        use libp2p::request_response::Event as RREvent;
        use libp2p::request_response::Message;
        use crate::core::models::SyndactylRequest;

        match event {
            RREvent::Message { peer, message, .. } => {
                match message {
                    Message::Request { request, channel, .. } => {
                        // Handle incoming file transfer requests
                        match request {
                            SyndactylRequest::FileTransfer(req) => {
                                info!(
                                    peer = %peer,
                                    observer = %req.observer,
                                    path = %req.path,
                                    "[swarm] Received file transfer request"
                                );
                                
                                // Check if we have this observer configured
                                if let Some(observer_config) = self.observer_configs.get(&req.observer) {
                                    let base_path = PathBuf::from(&observer_config.path);
                                    let relative_path = std::path::Path::new(&req.path);
                                    let absolute_path = file_handler::to_absolute_path(relative_path, &base_path);
                                    
                                    if absolute_path.exists() && absolute_path.is_file() {
                                        // Generate only the first chunk for initial response
                                        match generate_first_chunk(
                                            &req.observer,
                                            relative_path,
                                            &absolute_path,
                                            &req.hash,
                                        ) {
                                            Ok(first_chunk) => {
                                                info!(
                                                    observer = %req.observer,
                                                    path = %req.path,
                                                    size = first_chunk.total_size,
                                                    is_last = first_chunk.is_last_chunk,
                                                    "Sending first file chunk"
                                                );
                                                self.p2p.send_file_response(channel, first_chunk);
                                            }
                                            Err(e) => {
                                                error!(
                                                    observer = %req.observer,
                                                    path = %req.path,
                                                    error = %e,
                                                    "Failed to generate first chunk"
                                                );
                                            }
                                        }
                                    } else {
                                        warn!(
                                            observer = %req.observer,
                                            path = %req.path,
                                            "File not found or not a file"
                                        );
                                    }
                                } else {
                                    warn!(observer = %req.observer, "Observer not configured locally");
                                }
                            }
                            SyndactylRequest::FileChunk(chunk_req) => {
                                info!(
                                    peer = %peer,
                                    observer = %chunk_req.observer,
                                    path = %chunk_req.path,
                                    offset = chunk_req.offset,
                                    "[swarm] Received file chunk request"
                                );
                                
                                // Check if we have this observer configured
                                if let Some(observer_config) = self.observer_configs.get(&chunk_req.observer) {
                                    let base_path = PathBuf::from(&observer_config.path);
                                    let relative_path = std::path::Path::new(&chunk_req.path);
                                    let absolute_path = file_handler::to_absolute_path(relative_path, &base_path);
                                    if absolute_path.exists() && absolute_path.is_file() {
                                        match file_handler::read_file_chunk(&absolute_path, chunk_req.offset, CHUNK_SIZE) {
                                            Ok(data) => {
                                                let total_size = absolute_path.metadata().map(|m| m.len()).unwrap_or(0);
                                                let is_last_chunk = chunk_req.offset + data.len() as u64 >= total_size;
                                                let response = FileTransferResponse {
                                                    observer: chunk_req.observer.clone(),
                                                    path: chunk_req.path.clone(),
                                                    data,
                                                    offset: chunk_req.offset,
                                                    total_size,
                                                    hash: chunk_req.hash.clone(),
                                                    is_last_chunk,
                                                };
                                                self.p2p.send_file_response(channel, response);
                                            }
                                            Err(e) => {
                                                error!(
                                                    observer = %chunk_req.observer,
                                                    path = %chunk_req.path,
                                                    error = %e,
                                                    "Failed to read file chunk"
                                                );
                                            }
                                        }
                                    } else {
                                        warn!(
                                            observer = %chunk_req.observer,
                                            path = %chunk_req.path,
                                            "File not found or not a file for chunk request"
                                        );
                                    }
                                } else {
                                    warn!(observer = %chunk_req.observer, "Observer not configured locally for chunk request");
                                }
                            }
                        }
                    }
                    Message::Response { response, .. } => {
                        // Handle incoming file transfer responses
                        info!(
                            peer = %peer,
                            observer = %response.observer,
                            path = %response.path,
                            offset = response.offset,
                            size = response.data.len(),
                            is_last = response.is_last_chunk,
                            "[swarm] Received file transfer response"
                        );
                        
                        // Add chunk to transfer tracker
                        match self.transfer_tracker.add_chunk(
                            &response.observer,
                            &response.path,
                            response.offset,
                            response.data.clone(),
                            response.is_last_chunk,
                        ) {
                            Ok(Some(file_path)) => {
                                info!(
                                    observer = %response.observer,
                                    path = %response.path,
                                    file = %file_path.display(),
                                    "File transfer completed and written to disk"
                                );
                            }
                            Ok(None) => {
                                info!(
                                    observer = %response.observer,
                                    path = %response.path,
                                    "Chunk received, requesting next chunk"
                                );
                                // Request next chunk if not last
                                if !response.is_last_chunk {
                                    let next_offset = response.offset + response.data.len() as u64;
                                    let chunk_request = FileChunkRequest {
                                        observer: response.observer.clone(),
                                        path: response.path.clone(),
                                        offset: next_offset,
                                        hash: response.hash.clone(),
                                    };
                                    self.p2p.request_file_chunk(peer, chunk_request);
                                }
                            }
                            Err(e) => {
                                error!(
                                    observer = %response.observer,
                                    path = %response.path,
                                    error = %e,
                                    "Failed to process file chunk"
                                );
                            }
                        }
                    }
                }
            }
            RREvent::OutboundFailure { peer, request_id, error, .. } => {
                error!(peer = %peer, request_id = ?request_id, error = ?error, "[swarm] File transfer outbound failure");
            }
            RREvent::InboundFailure { peer, error, .. } => {
                error!(peer = %peer, error = ?error, "[swarm] File transfer inbound failure");
            }
            RREvent::ResponseSent { peer, .. } => {
                info!(peer = %peer, "[swarm] File transfer response sent");
            }
        }
    }
}
