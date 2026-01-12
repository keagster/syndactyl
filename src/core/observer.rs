use notify::{Event, EventKind, RecursiveMode, Result, Watcher};
use std::{path::Path, sync::mpsc, thread};
use crate::core::config::ObserverConfig;
use tracing::{info, error, warn};
use crate::core::models::FileEventMessage;
use crate::core::file_handler;
use serde_json;
use std::path::PathBuf;

pub fn event_listener(observers: Vec<ObserverConfig>, tx: mpsc::Sender<String>) -> Result<()> {
    let mut handles = Vec::new();

    // TODO: You will have to write a dynamic limiter for this so it
    // cant run away with too many threads
    // start a thread for each observer
    for observer in observers {
        let observer_name = observer.name.clone();
        let observer_path = observer.path.clone();
        let tx = tx.clone();

        let handle = thread::spawn(move || {
            let (event_tx, rx) = mpsc::channel::<Result<Event>>();
            let mut watcher = notify::recommended_watcher(event_tx).expect("Failed to create watcher");
            watcher.watch(Path::new(&observer_path), RecursiveMode::Recursive).expect("Failed to watch path");

            info!(path = %observer_path, observer = %observer_name, "Watching path");
            
            for res in rx {
                match res {
                    Ok(event) => {
                        match event.kind {
                            EventKind::Any => info!(observer = %observer_name, ?event, "any event"),
                            EventKind::Access(_access_kind) => {
                                // Do not handle or send access events
                                continue;
                            },
                            EventKind::Create(ref create_kind) => {
                                if let Some(path) = event.paths.get(0) {
                                    info!(observer = %observer_name, kind = ?create_kind, path = %path.display(), "created");
                                } else {
                                    info!(observer = %observer_name, kind = ?create_kind, "created, but path unknown");
                                }
                            },
                            EventKind::Modify(ref modify_kind) => {
                                if let Some(path) = event.paths.get(0) {
                                    info!(observer = %observer_name, kind = ?modify_kind, path = %path.display(), "modified");
                                } else {
                                    info!(observer = %observer_name, kind = ?modify_kind, "modified, but path unknown");
                                }
                            },
                            EventKind::Remove(ref remove_kind) => {
                                if let Some(path) = event.paths.get(0) {
                                    info!(observer = %observer_name, kind = ?remove_kind, path = %path.display(), "removed");
                                } else {
                                    info!(observer = %observer_name, kind = ?remove_kind, "removed, but path unknown");
                                }
                            },
                            EventKind::Other => {
                                if let Some(path) = event.paths.get(0) {
                                    info!(observer = %observer_name, path = %path.display(), "other event");
                                } else {
                                    info!(observer = %observer_name, "other event, but path unknown");
                                }
                            },
                        }
                        // Build and send FileEventMessage as JSON, but skip Access events
                        let event_type = match &event.kind {
                            EventKind::Any => "Any",
                            EventKind::Access(_) => continue,
                            EventKind::Create(_) => "Create",
                            EventKind::Modify(_) => "Modify",
                            EventKind::Remove(_) => "Remove",
                            EventKind::Other => "Other",
                        }.to_string();
                        
                        let absolute_path = event.paths.get(0)
                            .map(|p| p.to_path_buf())
                            .unwrap_or_else(|| PathBuf::from("unknown"));
                        
                        // Convert to relative path
                        let base_path = Path::new(&observer_path);
                        let relative_path = file_handler::to_relative_path(&absolute_path, base_path)
                            .unwrap_or_else(|| absolute_path.clone());
                        
                        // Skip files that shouldn't be synced
                        if !file_handler::should_sync_file(&relative_path) {
                            continue;
                        }
                        
                        let path_str = relative_path.display().to_string();
                        let details = Some(format!("{:?}", event.kind));
                        
                        // For Create/Modify events, calculate hash and get metadata
                        let (hash, size, modified_time) = if matches!(event_type.as_str(), "Create" | "Modify") {
                            if absolute_path.is_file() {
                                let hash = file_handler::calculate_file_hash(&absolute_path)
                                    .ok();
                                let metadata = file_handler::get_file_metadata(&absolute_path)
                                    .ok();
                                
                                if let Some((file_size, mtime)) = metadata {
                                    (hash, Some(file_size), Some(mtime))
                                } else {
                                    (hash, None, None)
                                }
                            } else {
                                // Skip directory events for now
                                continue;
                            }
                        } else {
                            (None, None, None)
                        };
                        
                        let msg = FileEventMessage {
                            observer: observer_name.clone(),
                            event_type,
                            path: path_str,
                            details,
                            hash,
                            size,
                            modified_time,
                        };
                        
                        if let Ok(json) = serde_json::to_string(&msg) {
                            let _ = tx.send(json);
                        }
                    },
                    Err(e) => {
                        error!(observer = %observer_name, error = ?e, "watch error");
                        let msg = FileEventMessage {
                            observer: observer_name.clone(),
                            event_type: "Error".to_string(),
                            path: "error".to_string(),
                            details: Some(format!("watch error: {:?}", e)),
                            hash: None,
                            size: None,
                            modified_time: None,
                        };
                        if let Ok(json) = serde_json::to_string(&msg) {
                            let _ = tx.send(json);
                        }
                    },
                }
            }
        });

        handles.push(handle);
    }

    // Wait for all threads to finish (they won't, unless the channel closes)
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    Ok(())
}
