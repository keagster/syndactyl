mod core;
mod network;

use std::sync::mpsc as std_mpsc;
use std::thread;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::network::syndactyl_p2p::SyndactylP2P;
use crate::core::observer;
use crate::core::config;
use crate::core::models::{FileTransferRequest, FileTransferResponse};
use crate::core::file_handler;
use crate::network::transfer::{FileTransferTracker, generate_file_chunks};

use tokio::sync::mpsc;
use crate::network::syndactyl_p2p::SyndactylP2PEvent;
use libp2p::PeerId;

use tracing::{info, error, warn};

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    //  Begin application startup
    // Initialize configuration
    let configuration = match config::get_config() {
        Ok(configuration) => {
            info!(?configuration, "Configuration loaded successfully");
            configuration
        }
        Err(e) => {
            error!(%e, "Failed to load configuration");
            return;
        }
    };
    // End application startup

    // Spawn Observer and set up channel for file events
    let (observer_tx, observer_rx) = std_mpsc::channel::<String>();
    let observer_config = configuration.observers.clone();
    let observer_thread = thread::spawn(move || {
        let _observer = observer::event_listener(observer_config, observer_tx);
        info!("Observer started");
    });

    // P2P networking and encryption (async)
    // TODO: Once this is all working try to push a lot of this logic back into the network code
    // instead of letting it live here.
    if let Some(network_config) = configuration.network.clone() {

        let (event_sender, mut event_receiver) = mpsc::channel(32);
        let mut p2p = SyndactylP2P::new(network_config, event_sender).await.unwrap();

        // Build a map of observer name -> base path for file operations
        let mut observer_paths: HashMap<String, PathBuf> = HashMap::new();
        for obs in &configuration.observers {
            observer_paths.insert(obs.name.clone(), PathBuf::from(&obs.path));
        }

        // Track connected peers for file requests
        let mut connected_peers: Vec<PeerId> = Vec::new();
        
        // Track incoming file transfers
        let mut transfer_tracker = FileTransferTracker::new();

        // Use a tokio channel to bridge observer events into the async context
        use tokio::sync::mpsc as tokio_mpsc;
        let (obs_tx, mut obs_rx) = tokio_mpsc::channel::<String>(32);
        // Spawn a thread to forward std_mpsc observer_rx to async obs_tx
        let observer_thread_forward = {
            let observer_rx = observer_rx;
            let obs_tx = obs_tx.clone();
            thread::spawn(move || {
                while let Ok(msg) = observer_rx.recv() {
                    let _ = obs_tx.blocking_send(msg);
                }
            })
        };

        // Import StreamExt for swarm polling
        use futures::StreamExt;

        // Main async loop: handle both observer events, P2P events, and swarm events
        loop {
            tokio::select! {
                Some(msg) = obs_rx.recv() => {
                    info!(msg = %msg, "Forwarding observer event to P2P");
                    let _ = p2p.publish_gossipsub(msg.into_bytes());
                },
                Some(event) = event_receiver.recv() => {
                    match event {
                        SyndactylP2PEvent::GossipsubMessage { source, data } => {
                            // Try to deserialize as FileEventMessage
                            match serde_json::from_slice::<crate::core::models::FileEventMessage>(&data) {
                                Ok(file_event) => {
                                    info!(peer = %source, event = ?file_event, "Received FileEventMessage from P2P");
                                    
                                    // Check if this is a Create or Modify event with a file we should sync
                                    if matches!(file_event.event_type.as_str(), "Create" | "Modify") {
                                        // Check if we have this observer configured locally
                                        if let Some(base_path) = observer_paths.get(&file_event.observer) {
                                            let relative_path = std::path::Path::new(&file_event.path);
                                            let absolute_path = file_handler::to_absolute_path(relative_path, base_path);
                                            
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
                                                        transfer_tracker.start_transfer(
                                                            file_event.observer.clone(),
                                                            file_event.path.clone(),
                                                            size,
                                                            hash,
                                                            base_path.clone(),
                                                        );
                                                    }
                                                    
                                                    // Send request to the peer who sent the event
                                                    p2p.request_file(source, request);
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
                                },
                                Err(e) => {
                                    warn!(peer = %source, error = ?e, raw = %String::from_utf8_lossy(&data), "Failed to parse FileEventMessage from P2P");
                                }
                            }
                        }
                        SyndactylP2PEvent::KademliaEvent(info) => {
                            info!(%info, "Kademlia event");
                        }
                        SyndactylP2PEvent::NewListenAddr(addr) => {
                            info!(%addr, "Listening on");
                        }
                        SyndactylP2PEvent::FileTransferRequest { peer, request, channel } => {
                            info!(peer = %peer, observer = %request.observer, path = %request.path, "Received file transfer request");
                            
                            // Check if we have this observer and file
                            if let Some(base_path) = observer_paths.get(&request.observer) {
                                let relative_path = std::path::Path::new(&request.path);
                                let absolute_path = file_handler::to_absolute_path(relative_path, base_path);
                                
                                if absolute_path.exists() && absolute_path.is_file() {
                                    // Generate file chunks and send them
                                    match generate_file_chunks(
                                        &request.observer,
                                        relative_path,
                                        &absolute_path,
                                        &request.hash,
                                    ) {
                                        Ok(chunks) => {
                                            info!(
                                                observer = %request.observer,
                                                path = %request.path,
                                                chunks = chunks.len(),
                                                "Sending file chunks"
                                            );
                                            
                                            // Send the first chunk as the response
                                            // Note: In request-response, we can only send ONE response
                                            // For multi-chunk files, we need a different approach
                                            // For now, let's just send files that fit in one chunk
                                            if let Some(first_chunk) = chunks.into_iter().next() {
                                                p2p.send_file_response(channel, first_chunk);
                                            }
                                        }
                                        Err(e) => {
                                            error!(
                                                observer = %request.observer,
                                                path = %request.path,
                                                error = %e,
                                                "Failed to generate file chunks"
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
                        SyndactylP2PEvent::FileTransferResponse { peer, response } => {
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
                            match transfer_tracker.add_chunk(
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
                                        "Chunk received, waiting for more chunks"
                                    );
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
                },
                swarm_event = p2p.swarm.select_next_some() => {
                    // Process swarm events inline
                    use libp2p::swarm::SwarmEvent;
                    use libp2p::gossipsub::Event as GossipsubEvent;
                    use crate::network::syndactyl_behaviour::SyndactylEvent;

                    match swarm_event {
                        SwarmEvent::Behaviour(SyndactylEvent::Gossipsub(GossipsubEvent::Message { propagation_source, message_id: _, message })) => {
                            // Try to deserialize as FileEventMessage
                            match serde_json::from_slice::<crate::core::models::FileEventMessage>(&message.data) {
                                Ok(file_event) => {
                                    info!(peer = %propagation_source, event = ?file_event, "[syndactyl][gossipsub] Received FileEventMessage");
                                    // Here you can add logic to process/apply the event
                                },
                                Err(e) => {
                                    warn!(peer = %propagation_source, error = ?e, raw = %String::from_utf8_lossy(&message.data), "[syndactyl][gossipsub] Failed to parse FileEventMessage");
                                }
                            }
                        }
                        SwarmEvent::Behaviour(SyndactylEvent::Kademlia(event)) => {
                            info!(event = ?event, "[syndactyl][kademlia] Event");
                        }
                        SwarmEvent::NewListenAddr { address, .. } => {
                            info!(address = %address, "[syndactyl][swarm] Listening on");
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                            info!(peer_id = %peer_id, endpoint = ?endpoint, "[syndactyl][swarm] Connection established");
                            if !connected_peers.contains(&peer_id) {
                                connected_peers.push(peer_id);
                            }
                        }
                        SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                            warn!(peer_id = %peer_id, ?cause, "[syndactyl][swarm] Connection closed");
                            connected_peers.retain(|p| p != &peer_id);
                        }
                        _ => {
                            // Other swarm events
                        }
                    }
                },
                else => break,
            }
        }
        let _ = observer_thread_forward.join();
    }

    // Wait for observer thread to finish
    let _ = observer_thread.join();
}
