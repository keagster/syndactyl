# HMAC-Based Authentication

This document describes the HMAC-based authentication system implemented in Syndactyl to secure file synchronization between peers.

## Overview

Syndactyl now supports **per-observer shared secret authentication** using HMAC-SHA256. This ensures that only authorized peers who know the shared secret can:
- Send file event notifications
- Request file transfers
- Receive files from the observer

## How It Works

### 1. Message Authentication

When a file event occurs, the observer:
1. Creates a `FileEventMessage` with file metadata (path, hash, size, etc.)
2. Computes an HMAC-SHA256 tag over the message fields using the configured `shared_secret`
3. Attaches the HMAC to the message
4. Broadcasts the authenticated message via Gossipsub

### 2. Message Verification

When receiving a file event from a peer, the network manager:
1. Checks if the observer is configured locally
2. If a `shared_secret` is configured for that observer:
   - Recomputes the HMAC using the local shared secret
   - Compares it with the received HMAC using **constant-time comparison** (prevents timing attacks)
   - If verification fails, the message is **rejected** and logged as unauthorized
3. Only authenticated messages are processed

### 3. HMAC Message Format

The HMAC is computed over the following fields concatenated with `||` separator:
```
observer || event_type || path || hash || size || modified_time
```

Example:
```
"my-docs" || "Create" || "file.txt" || "abc123..." || "1024" || "1234567890"
```

## Configuration

### Adding a Shared Secret to an Observer

Edit your `~/.config/syndactyl/config.json`:

```json
{
  "observers": [
    {
      "name": "my-docs",
      "path": "/home/user/Documents",
      "shared_secret": "your-secret-key-here"
    },
    {
      "name": "photos",
      "path": "/home/user/Pictures",
      "shared_secret": "another-secret-key"
    }
  ],
  "network": {
    "listen_addr": "0.0.0.0",
    "port": "4001",
    "dht_mode": "server",
    "bootstrap_peers": []
  }
}
```

### Generating a Secure Secret

Use a cryptographically secure random string generator:

```bash
# Linux/macOS
openssl rand -hex 32

# Or use this Rust snippet
use rand::Rng;
let secret: String = rand::thread_rng()
    .sample_iter(&rand::distributions::Alphanumeric)
    .take(64)
    .map(char::from)
    .collect();
```

**Important:** Keep secrets secure and distribute them only to authorized peers via a secure channel (not via the P2P network itself).

## Security Guarantees

### ✅ What HMAC Authentication Provides:

1. **Message Authentication**: Verifies that messages come from someone who knows the shared secret
2. **Message Integrity**: Detects any tampering with message contents
3. **Replay Attack Mitigation**: Combined with timestamps, prevents replay of old messages
4. **Timing Attack Resistance**: Uses constant-time comparison to prevent timing side-channels

### ❌ What HMAC Authentication Does NOT Provide:

1. **Peer Identity**: Cannot identify which specific peer sent the message (all peers with the secret look the same)
2. **Non-repudiation**: Any peer with the secret can claim to be any other peer
3. **Key Distribution**: Secrets must be shared out-of-band securely
4. **Forward Secrecy**: If the secret is compromised, all past messages are vulnerable

## Limitations & Future Work

### Current Limitations:

- **Symmetric Keys**: All peers in an observer group share the same secret
- **No Peer-Level Authorization**: Cannot grant different permissions to different peers
- **Manual Key Distribution**: Secrets must be configured manually on each node
- **No Key Rotation**: Changing secrets requires updating all peers

### Planned Improvements (See ROADMAP.md):

- **Peer Allowlists**: Restrict which specific peer IDs can access an observer (Task 2)
- **PKI-Based Authentication**: Use public-key cryptography for peer-specific authorization (Task 3)
- **Invitation System**: User-friendly way to add new peers without sharing raw secrets
- **Key Rotation**: Automated or scheduled secret rotation

## Testing

Run the test suite to verify HMAC functionality:

```bash
cargo test auth
```

Test cases cover:
- HMAC computation correctness
- Successful verification with correct secret
- Rejection with wrong secret
- Rejection of tampered messages
- Rejection of messages without HMAC
- Constant-time comparison

## Troubleshooting

### Warning: "No shared secret configured - messages will not be authenticated"

**Cause:** The observer does not have a `shared_secret` configured.

**Solution:** Add `"shared_secret": "your-secret-here"` to the observer configuration.

**Security Impact:** Messages from this observer are **not authenticated** and can be forged by any peer. This is **INSECURE** for untrusted networks.

---

### Warning: "HMAC verification failed - rejecting unauthorized file event"

**Cause:** The received message's HMAC does not match the computed HMAC.

**Possible Reasons:**
1. The sending peer has a different `shared_secret` configured
2. The message was tampered with in transit (unlikely with libp2p encryption)
3. The message fields were corrupted

**Solution:**
- Verify all peers have the **exact same** `shared_secret` for the observer
- Check for typos in the secret
- Ensure secrets don't have trailing whitespace or special characters

---

### Warning: "No shared secret configured for observer - accepting unauthenticated message (INSECURE)"

**Cause:** You received a message for an observer that exists locally but has no `shared_secret` configured.

**Security Impact:** You will accept **any** message from **any** peer for this observer.

**Solution:** Configure a `shared_secret` for the observer if you want authentication.

---

### Error: "Observer not configured locally, ignoring event"

**Cause:** You received a file event for an observer you don't have configured.

**This is normal behavior** - you only sync observers you explicitly configure.

## Example Configuration Scenarios

### Scenario 1: Two Peers Sharing Documents

**Peer A (Alice):**
```json
{
  "observers": [
    {
      "name": "shared-docs",
      "path": "/home/alice/SharedDocs",
      "shared_secret": "abc123secretXYZ789"
    }
  ]
}
```

**Peer B (Bob):**
```json
{
  "observers": [
    {
      "name": "shared-docs",
      "path": "/home/bob/SharedDocs",
      "shared_secret": "abc123secretXYZ789"
    }
  ]
}
```

**Result:** Alice and Bob can sync files in `shared-docs` because they have the same secret.

---

### Scenario 2: Multiple Observers with Different Secrets

**Configuration:**
```json
{
  "observers": [
    {
      "name": "work-docs",
      "path": "/home/user/Work",
      "shared_secret": "work-secret-key-123"
    },
    {
      "name": "personal-photos",
      "path": "/home/user/Photos",
      "shared_secret": "personal-photos-key-456"
    }
  ]
}
```

**Result:** Each observer has its own authentication boundary. Peers need the appropriate secret for each observer they want to access.

---

### Scenario 3: Mixed Authentication (Not Recommended)

**Peer A:**
```json
{
  "observers": [
    {
      "name": "public-data",
      "path": "/home/user/Public"
      // No shared_secret - unauthenticated
    }
  ]
}
```

**Result:** This observer accepts messages from **any peer**. Only use this for truly public data on trusted networks.

## Best Practices

1. ✅ **Always use strong secrets**: Minimum 32 characters, random alphanumeric
2. ✅ **Use different secrets for different observers**: Limits damage if one secret is compromised
3. ✅ **Rotate secrets periodically**: Change secrets every 6-12 months
4. ✅ **Distribute secrets securely**: Use encrypted channels (Signal, GPG, etc.)
5. ✅ **Monitor logs for auth failures**: Failed HMAC verifications may indicate attacks
6. ❌ **Never commit secrets to version control**: Use `.gitignore` for config files
7. ❌ **Don't share secrets over the P2P network**: Use out-of-band channels

## Related Documentation

- [ROADMAP.md](ROADMAP.md) - Next steps: Peer Allowlists and PKI Authentication
- [project-analysis-and-recommendations.md](project-analysis-and-recommendations.md) - Security analysis

## Implementation Details

For developers interested in the implementation:

- **HMAC Module**: `src/core/auth.rs`
- **Config Schema**: `src/core/config.rs` - `ObserverConfig.shared_secret`
- **Message Schema**: `src/core/models.rs` - `FileEventMessage.hmac`
- **Observer (Sender)**: `src/core/observer.rs` - Computes and attaches HMAC
- **Network Manager (Receiver)**: `src/network/manager.rs` - Verifies HMAC

---

**Status:** ✅ Implemented (2024-01-13)  
**Roadmap Task:** Authentication & Authorization - Task 1  
**Next Steps:** Implement Peer Allowlists (Task 2)
