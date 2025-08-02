mod core;
mod network;

use std::sync::mpsc as std_mpsc;
use std::thread;

use crate::network::syndactyl_p2p::SyndactylP2P;
use crate::core::observer;
use crate::core::config;

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
        use tokio::sync::mpsc;
        use crate::network::syndactyl_p2p::SyndactylP2PEvent;

        let (event_sender, mut event_receiver) = mpsc::channel(32);
        let mut p2p = SyndactylP2P::new(network_config, event_sender).await.unwrap();

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

        // Main async loop: handle both observer events and P2P events
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
                                    // Here you can add logic to process/apply the event locally
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
