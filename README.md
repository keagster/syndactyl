# syndactyl
Rust based file sync project to sync files between my computers and shared company files

Project to learn rust on after overcoming my denial that it exists, its good and i need it ☺️

### Initial Feature List
- Some way of "joining" a sync network and being authenticated to it
	- This would be the method of tracking connections or pools of connections. Possibly using graphql but for now this can just be a simple json file describing the connections and directories. 
	- NOTE: Make sure you design this schema in a way that is clean and convertible to a graph system later on.
- Authentication service to allow a client to be connected to a share
- Transfer security and using strongest encryption standards. 
	- This will also need to include a fail safe to ensure that no data can be transferred without a properly established encrypted connection

Core
- Monitor for file changes in assigned directory
	- life cycle events would be quite handy here to allow different scripts to run at different times like git hooks works
	- allow git events to to act as triggers if configured
	- use git to track changes and allow version control
	- event triggers to trigger project code or scripts
Transfer
- file transfer via rsync
	- make sure you write this section in a way that can replace rsync if i find something better
	- transfer only changes to keep connection lean as possible
Network
- ability to add a "node" or directory to "the network" that can be added for syncing via authentication and decryption
Security
- Authentication Guard
- Encryption checks

### Todo
TODO: Write a simple mod to check OS type and set paths for OS type e.g. os = linux = XDG config path
