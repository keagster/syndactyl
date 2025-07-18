mod core;

use crate::core::observer;
use crate::core::config;

fn main() {
    // Initialize configuration
    
    // Start file observer
    let _file_observer = observer::event_listener();
}
