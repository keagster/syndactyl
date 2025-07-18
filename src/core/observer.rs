use notify::{Event, EventKind, RecursiveMode, Result, Watcher};
use std::{path::Path, sync::mpsc};

pub fn event_listener() -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();

    // Use recommended_watcher() to automatically select the best implementation
    // for your platform. The `EventHandler` passed to this constructor can be a
    // closure, a `std::sync::mpsc::Sender`, a `crossbeam_channel::Sender`, or
    // another type the trait is implemented for.
    let mut watcher = notify::recommended_watcher(tx)?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(Path::new("."), RecursiveMode::Recursive)?;
    // Block forever, printing out events as they come in
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

    Ok(())
}
