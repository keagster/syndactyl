mod core;
mod network;

use std::sync::mpsc as std_mpsc;
use std::thread;

use crate::network::manager::NetworkManager;
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

    // Spawn Observer and set up channel for file events
    let (observer_tx, observer_rx) = std_mpsc::channel::<String>();
    let observer_config = configuration.observers.clone();
    let observer_thread = thread::spawn(move || {
        let _observer = observer::event_listener(observer_config, observer_tx);
        info!("Observer started");
    });

    // P2P networking and encryption (async)
    if configuration.network.is_some() {
        // Create and run the network manager
        match NetworkManager::new(configuration).await {
            Ok(network_manager) => {
                info!("Network manager created successfully");
                // Run the network manager with observer events
                network_manager.run(observer_rx).await;
            }
            Err(e) => {
                error!(%e, "Failed to create network manager");
                return;
            }
        }
    }

    // Wait for observer thread to finish
    let _ = observer_thread.join();
}
