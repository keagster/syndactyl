# Testing Syndactyl File Sync

## Fixed Issues

The following issues have been resolved:

1. **Build errors** - All compilation errors fixed
2. **File transfer logic** - Added proper event handling in the main swarm event loop
3. **Bootstrap peer dialing** - Peers are now explicitly dialed to establish connections
4. **Configuration** - Added missing `listen_addr` field

## How File Transfer Works

1. **Machine A** detects a file change via the observer
2. **Machine A** publishes a `FileEventMessage` via gossipsub with file metadata (hash, size, etc.)
3. **Machine B** receives the gossipsub message
4. **Machine B** checks if it needs the file (doesn't exist or hash differs)
5. **Machine B** sends a `FileTransferRequest` to Machine A
6. **Machine A** receives the request and sends back a `FileTransferResponse` with the first chunk
7. If more chunks needed, **Machine B** requests subsequent chunks until complete
8. **Machine B** writes the complete file to disk

## Setup for Testing Between Two Machines

### Machine 1 (Bootstrap Node)

1. Create `~/.config/syndactyl/config.json`:

```json
{
  "observers": [
    {
      "name": "shared-folder",
      "path": "/path/to/your/sync/folder"
    }
  ],
  "network": {
    "listen_addr": "0.0.0.0",
    "port": "49999",
    "dht_mode": "server",
    "bootstrap_peers": []
  }
}
```

2. Start the application:
```bash
cargo run
```

3. **Note the output** - you'll see:
   - `Local PeerId: 12D3KooW...` (copy this)
   - `Listening on /ip4/x.x.x.x/tcp/49999` (note the IP)

### Machine 2 (Client Node)

1. Create `~/.config/syndactyl/config.json`:

```json
{
  "observers": [
    {
      "name": "shared-folder",
      "path": "/path/to/your/sync/folder"
    }
  ],
  "network": {
    "listen_addr": "0.0.0.0",
    "port": "49998",
    "dht_mode": "client",
    "bootstrap_peers": [
      {
        "ip": "MACHINE_1_IP",
        "port": "49999",
        "peer_id": "MACHINE_1_PEER_ID"
      }
    ]
  }
}
```

**Important:** Replace:
- `MACHINE_1_IP` with Machine 1's IP address
- `MACHINE_1_PEER_ID` with the PeerId from Machine 1

2. Start the application:
```bash
cargo run
```

## Testing File Sync

1. Wait for connection to establish. You should see:
   - `[syndactyl][swarm] Connection established`
   - On both machines

2. Create or modify a file in the watched folder on **Machine 1**:
```bash
echo "Hello World" > /path/to/your/sync/folder/test.txt
```

3. Check the logs on **Machine 2** - you should see:
   - `[syndactyl][gossipsub] Received FileEventMessage`
   - `Requesting file from peer`
   - `[swarm] Received file transfer response`
   - `File transfer completed and written to disk`

4. Verify the file exists on Machine 2:
```bash
cat /path/to/your/sync/folder/test.txt
```

## Important Notes

1. **Observer names must match** - Both machines need the same observer name (e.g., "shared-folder") for sync to work
2. **Firewall** - Ensure port 49999 (or your configured port) is open
3. **Same network** - For initial testing, use machines on the same LAN
4. **Different ports** - Each machine should use a different port to avoid conflicts if testing on the same machine

## Troubleshooting

### No connection established
- Check firewall settings
- Verify IP addresses and peer IDs are correct
- Ensure both applications are running

### Files not syncing
- Check observer names match exactly
- Verify the file path is within the watched directory
- Look for "Observer not configured locally" messages

### Logs show "File already up to date, skipping"
- This is normal if the file already exists with the same hash
- Delete the file on Machine 2 and try again

## Advanced: Testing on Same Machine

You can test with two instances on the same machine:

**Terminal 1:**
```bash
mkdir -p /tmp/sync1
# Use config with port 49999, no bootstrap peers
cargo run
```

**Terminal 2:**
```bash
mkdir -p /tmp/sync2
# Use config with port 49998, bootstrap to first instance
cargo run
```

Then create a file in `/tmp/sync1` and watch it appear in `/tmp/sync2`.
