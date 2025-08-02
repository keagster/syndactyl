mod core;
mod network;

use std::sync::mpsc as std_mpsc;
use std::thread;

use crate::network::syndactyl_p2p::SyndactylP2P;
use crate::core::observer;
use crate::core::config;

use tracing::{info, error};

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

    // Spawn Observer (still in a thread, as before)
    let (tx, _rx) = std_mpsc::channel::<String>();
    let observer_config = configuration.observers.clone();
    let observer_thread = thread::spawn(move || {
        let _observer = observer::event_listener(observer_config);
        if let Err(e) = tx.send("Observer started".to_string()) {
            error!(%e, "Failed to send observer started message");
        } else {
            info!("Observer started");
        }
    });

    // P2P networking and encryption (async)
    if let Some(network_config) = configuration.network.clone() {
        use tokio::sync::mpsc;
        use crate::network::syndactyl_p2p::SyndactylP2PEvent;

        let (event_sender, mut event_receiver) = mpsc::channel(32);
        let mut p2p = SyndactylP2P::new(network_config, event_sender).await.unwrap();

        // Spawn the poll_events loop
        let mut p2p_task = tokio::spawn(async move {
            p2p.poll_events().await;
        });

        // Handle events in main
        while let Some(event) = event_receiver.recv().await {
            match event {
                SyndactylP2PEvent::GossipsubMessage { source, data } => {
                    info!(peer = %source, msg = %String::from_utf8_lossy(&data), "Received gossip message");
                }
                SyndactylP2PEvent::KademliaEvent(info) => {
                    info!(%info, "Kademlia event");
                }
                SyndactylP2PEvent::NewListenAddr(addr) => {
                    info!(%addr, "Listening on");
                }
            }
        }

        // Optionally, wait for the p2p task to finish (it won't unless you break the loop)
        let _ = p2p_task.await;
    }

    // Wait for observer thread to finish
    let _ = observer_thread.join();
}
