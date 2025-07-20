mod core;

use std::{sync::mpsc, thread};

use crate::core::observer;
use crate::core::config;

fn main() {
    
    //  Begin application startup
    // Initialize configuration
    let configuration = match config::get_config() {
        Ok(configuration) => {
            println!("Configuration loaded successfully: {:?}", configuration);
            configuration
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            return;
        }
    };
    // End application startup
    
    // Begin thread management
    let mut syndactyl_threads: Vec<thread::JoinHandle<()>> = Vec::new();
    // TODO: create a thread bus Type to manage inter thread comms for in thread logic
    let (tx, _rx) = mpsc::channel::<String>();

    // Spawn Observer
    syndactyl_threads.push(thread::spawn(move || {
        // TODO: once you get this working consider passing rx tx to observer
        // so you can deal with comms and logic from in there where that logic belongs
        let _observer = observer::event_listener(configuration.observers.clone());
        tx.send("Observer started".to_string()).unwrap();
    }));

    // spawn p2p networking and encryption
    // spawn authentication
    // spawn transfer handler. based on the tests projects i played with yesterday when
    // learning libp2p i may want to keep the transfer laying in the network layer.
    // will know more as i work with it.

    for syndactyl_thread in syndactyl_threads {
        match syndactyl_thread.join() {
            Ok(_) => println!("Thread completed successfully"),
            Err(e) => eprintln!("Thread failed: {:?}", e),
        }
    }
    // End thread management
}
