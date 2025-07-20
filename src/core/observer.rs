use notify::{Event, EventKind, RecursiveMode, Result, Watcher};
use std::{path::Path, sync::mpsc, thread};
use crate::core::config::Observer;

pub fn event_listener(observers: Vec<Observer>) -> Result<()> {
    let mut handles = Vec::new();

    // TODO: You will have to write a dynamic limiter for this so it
    // cant run away with too many threads
    // start a thread for each observer
    for observer in observers {
        let observer_name = observer.name.clone();
        let observer_path = observer.path.clone();

        let handle = thread::spawn(move || {
            let (tx, rx) = mpsc::channel::<Result<Event>>();
            let mut watcher = notify::recommended_watcher(tx).expect("Failed to create watcher");
            watcher.watch(Path::new(&observer_path), RecursiveMode::Recursive).expect("Failed to watch path");

            println!("Watching path: {} ({})", observer_path, observer_name);
            
            for res in rx {
                match res {
                    Ok(event) => match event.kind {
                        EventKind::Any => println!("any event: {:?}", event),
                        EventKind::Access(_access_kind) => {
                            if let Some(_path) = event.paths.get(0) {
                                // do nothing for now
                            } else {
                                // for now, we don't handle access events
                            }
                        },
                        EventKind::Create(create_kind) => {
                            if let Some(path) = event.paths.get(0) {
                                println!("created ({:?}): {:?}", create_kind, path);
                            } else {
                                println!("created ({:?}), but path unknown", create_kind);
                            }
                        },
                        EventKind::Modify(modify_kind) => {
                            if let Some(path) = event.paths.get(0) {
                                println!("modified ({:?}): {:?}", modify_kind, path);
                            } else {
                                println!("modified ({:?}), but path unknown", modify_kind);
                            }
                        },
                        EventKind::Remove(remove_kind) => {
                            if let Some(path) = event.paths.get(0) {
                                println!("removed ({:?}): {:?}", remove_kind, path);
                            } else {
                                println!("removed ({:?}), but path unknown", remove_kind);
                            }
                        },
                        EventKind::Other => {
                            if let Some(path) = event.paths.get(0) {
                                println!("other event: {:?}", path);
                            } else {
                                println!("other event, but path unknown");
                            }
                        },
                    },
                    Err(e) => println!("watch error: {:?}", e),
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
