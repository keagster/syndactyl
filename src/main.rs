mod core;

use crate::core::observer;
use crate::core::config;

fn main() {
    // Initialize configuration
    match config::get_config() {
        Ok(configuration) => {
            println!("Configuration loaded successfully: {:?}", configuration);
            // Start file observer
            let _file_observer = observer::event_listener(configuration.observers);
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            return;
        }
    }
    
}
