# Syndactyl Development Roadmap & Feature Checklist

This document tracks all improvements, features, and security enhancements identified during the project analysis. Items are grouped by category and priority to help with planning and implementation.

---

## 🔴 Critical Priority (Security & Stability)

These are **must-have** items before Syndactyl can be considered production-ready.

### Authentication & Authorization
- [ ] **Implement per-observer shared secrets (HMAC-based authentication)**
  - Add `shared_secret` field to `ObserverConfig`
  - Add HMAC field to `FileEventMessage`
  - Verify HMAC on all incoming file events
  - Reject unauthorized file transfer requests
  - Use constant-time comparison to prevent timing attacks

- [ ] **Add peer allowlists per observer**
  - Add `allowed_peers: Option<Vec<String>>` to `ObserverConfig`
  - Check peer authorization before serving files
  - Log unauthorized access attempts

- [ ] **Design long-term PKI-based authentication**
  - Define `ObserverPermissions` structure
  - Design peer authorization model
  - Plan for signed messages (`SignedFileEventMessage`)
  - Consider invitation token system for user-friendly onboarding

**Dependencies:** These items should be done together as they form the core security model.

---

### Conflict Resolution & Version Management
- [ ] **Implement vector clock-based conflict detection**
  - Add `FileVersion` structure with vector clocks
  - Track causality relationships between file versions
  - Detect concurrent modifications

- [ ] **Add conflict handling mechanism**
  - Create `.syndactyl/conflicts/` directory structure
  - Move conflicted files to conflicts folder
  - Generate conflict report with metadata
  - Add conflict notification system

- [ ] **Design merge strategy options**
  - Last-write-wins (current behavior)
  - Keep both versions (rename)
  - Manual resolution UI
  - Custom merge scripts (plugin system)

**Dependencies:** Vector clocks must be implemented before conflict handling can work properly.

---

### Reliability & Error Recovery
- [ ] **Implement retry logic for failed transfers**
  - Add retry counter to `TransferState`
  - Define max retry attempts (configurable)
  - Implement exponential backoff
  - Track failed chunks separately

- [ ] **Add resume capability for interrupted transfers**
  - Track received chunks in `TransferState`
  - Persist transfer state to disk
  - Resume from last successful chunk on reconnection
  - Add transfer recovery on startup

- [ ] **Implement transfer timeout detection**
  - Add `last_activity` timestamp to transfers
  - Periodic check for stalled transfers
  - Automatic cleanup of abandoned transfers
  - Configurable timeout values

**Dependencies:** These items work together to create robust transfer handling.

---

## 🟡 High Priority (Production Readiness)

These items significantly improve reliability, observability, and user experience.

### Configuration & Validation
- [ ] **Add comprehensive config validation**
  - Check that observer paths exist and are readable
  - Validate directory permissions
  - Check for duplicate observer names
  - Validate network configuration (ports, IPs)
  - Validate bootstrap peer reachability

- [ ] **Improve error messages**
  - Replace generic errors with specific error types
  - Use `thiserror` crate for structured errors
  - Add contextual information to errors
  - Create user-friendly error messages

- [ ] **Add config hot-reload capability**
  - Watch config file for changes
  - Reload configuration without restart
  - Validate before applying
  - Gracefully handle invalid changes

**Dependencies:** Error types should be implemented first, then used in validation and hot-reload.

---

### Observability & Monitoring
- [ ] **Implement structured metrics collection**
  - Track files synced counter
  - Track bytes transferred
  - Track active transfers
  - Track failed transfers
  - Track connected peers count
  - Track per-observer statistics

- [ ] **Add detailed logging with context**
  - Include observer name in all logs
  - Include peer ID in network logs
  - Add operation duration tracking
  - Log transfer speeds and performance metrics
  - Add log levels (trace, debug, info, warn, error)

- [ ] **Create metrics export endpoint (optional)**
  - Prometheus-compatible metrics
  - JSON stats endpoint
  - HTTP server for metrics (optional feature)

- [ ] **Add health check endpoint**
  - System status check
  - Observer status
  - Network connectivity status
  - Disk space checks

**Dependencies:** Metrics should be implemented first, then export/health endpoints can use them.

---

### Testing & Quality Assurance
- [ ] **Add integration tests**
  - Test file sync between two nodes
  - Test network partition handling
  - Test large file transfers
  - Test concurrent modifications
  - Test observer hot-add/remove

- [ ] **Add unit test coverage**
  - Test all core modules (>80% coverage)
  - Test edge cases in file_handler
  - Test transfer tracker logic
  - Test configuration parsing
  - Mock network for testing

- [ ] **Add benchmark suite**
  - Benchmark transfer speed
  - Benchmark hash calculation
  - Benchmark with varying file sizes
  - Memory usage profiling
  - CPU usage profiling

- [ ] **Add network simulation tests**
  - Simulate 20+ node network
  - Test under network churn (peers joining/leaving)
  - Test with packet loss
  - Test with bandwidth limits

**Dependencies:** Can be done incrementally, but integration tests should come after core features are stable.

---

## 🟢 Medium Priority (Enhanced Features)

These items add significant value but aren't blocking production use.

### Performance & Optimization
- [ ] **Implement content-addressed storage**
  - Store files by hash in `.syndactyl/objects/`
  - Use symlinks in observer directories
  - Deduplicate identical files across observers
  - Implement garbage collection for unused objects

- [ ] **Add bandwidth management**
  - Implement token bucket rate limiting
  - Add configurable bandwidth limits (per peer, global)
  - Add QoS prioritization
  - Add bandwidth scheduling (off-peak hours)

- [ ] **Implement delta sync (rsync-style)**
  - Calculate binary diffs between file versions
  - Transfer only changed blocks
  - Use rolling hash algorithm
  - Implement chunk-level deduplication

- [ ] **Add compression support**
  - Compress chunks before transfer
  - Support multiple algorithms (zstd, lz4, gzip)
  - Make compression optional/configurable
  - Skip compression for already-compressed files

**Dependencies:** Content-addressed storage should come before delta sync. Bandwidth management is independent.

---

### Rate Limiting & Security Hardening
- [ ] **Implement request rate limiting**
  - Per-peer request limits
  - Sliding window rate limiter
  - Configurable limits per observer
  - Block abusive peers temporarily

- [ ] **Add bandwidth throttling per peer**
  - Track bandwidth usage per peer
  - Enforce per-peer limits
  - Prevent single peer from saturating connection
  - Add fairness scheduling

- [ ] **Implement connection limits**
  - Max connections per peer
  - Max total connections
  - Connection timeouts
  - Idle connection cleanup

**Dependencies:** These can all be implemented together as part of a "resource management" feature.

---

### User Experience & Interface
- [ ] **Add progress indication system**
  - Define `TransferProgress` events
  - Emit progress through channel
  - Track percentage complete
  - Estimate time remaining
  - Show transfer speed

- [ ] **Create CLI with progress bars**
  - Interactive mode with live updates
  - Use `indicatif` crate for progress bars
  - Show per-file transfer status
  - Show overall sync status
  - Add verbose/quiet modes

- [ ] **Add systemd service file**
  - Create `.service` file template
  - Add installation script
  - Support running as user service
  - Add auto-restart on failure

- [ ] **Create configuration wizard**
  - Interactive setup for first-time users
  - Generate config.json from prompts
  - Generate and save keypair
  - Test bootstrap peer connectivity

**Dependencies:** Progress system should be implemented before CLI. Service file is independent.

---

### File Filtering & Ignore Patterns
- [ ] **Implement `.syndactylignore` support**
  - Parse gitignore-style patterns
  - Support per-observer ignore files
  - Support global ignore patterns
  - Add common defaults (.git, node_modules, etc.)

- [ ] **Add file type filtering**
  - Filter by extension
  - Filter by MIME type
  - Filter by file size
  - Include/exclude patterns in config

- [ ] **Improve hidden file handling**
  - Make hidden file sync configurable
  - Add exceptions for specific hidden files
  - Respect platform conventions

**Dependencies:** These are all related to filtering and should be done together.

---

## 🔵 Low Priority (Nice to Have)

These items add polish and advanced features but aren't essential.

### Advanced Features
- [ ] **File versioning / history**
  - Keep last N versions of each file
  - Store versions in `.syndactyl/versions/`
  - Add version browsing capability
  - Add version restore functionality
  - Implement version retention policies

- [ ] **Plugin system**
  - Define `SyndactylPlugin` trait
  - Add plugin loading mechanism
  - Provide hooks for file events
  - Provide hooks for sync events
  - Example plugins:
    - Git integration (auto-commit on sync)
    - S3 backup
    - Notification system
    - Custom conflict resolution

- [ ] **Web UI / Dashboard**
  - Embed web server (actix-web or axum)
  - Dashboard showing sync status
  - Connected peers view
  - Transfer activity monitoring
  - Configuration editor
  - Conflict resolution UI

- [ ] **File watcher optimization**
  - Debounce rapid file changes
  - Batch multiple events
  - Reduce CPU usage during heavy I/O
  - Support for large directories (100K+ files)

**Dependencies:** Plugin system should come before web UI so UI can be implemented as a plugin.

---

### Multi-Platform Support
- [ ] **Cross-platform path handling improvements**
  - Handle Windows vs Unix path separators
  - Handle case-insensitive filesystems (macOS, Windows)
  - Handle path length limits
  - Handle special characters in filenames

- [ ] **Windows-specific improvements**
  - Windows service support
  - Windows Registry configuration
  - Windows Credential Manager integration
  - Handle file locking on Windows

- [ ] **Mobile support exploration**
  - Research uniffi for mobile FFI
  - Design mobile-friendly API
  - Consider battery-aware syncing
  - Background sync capabilities

**Dependencies:** Cross-platform issues should be addressed before mobile support.

---

## 🔮 Future / Research Items

These are longer-term ideas that need more research or depend on external factors.

### Post-Quantum Cryptography
- [ ] **Document PQ vulnerability**
  - Add security notice to README
  - Create SECURITY.md
  - Explain quantum threat timeline
  - Document mitigation strategies

- [ ] **Monitor libp2p PQ progress**
  - Watch libp2p/specs repository
  - Track PQNoise specification development
  - Test experimental implementations
  - Join community discussions

- [ ] **Design PQ migration path**
  - Add `post_quantum` config placeholder
  - Design hybrid crypto approach
  - Plan backwards compatibility
  - Create migration guide

- [ ] **Implement PQ support (when libp2p ready)**
  - Upgrade to PQ-enabled libp2p
  - Implement hybrid mode
  - Test interoperability
  - Provide classical fallback

- [ ] **Research: Fork libp2p for PQNoise (learning)**
  - Study Noise Protocol specification
  - Implement PQNoise handshake
  - Integrate Kyber/Dilithium
  - Understand the complexities (educational only)

**Dependencies:** Wait for libp2p ecosystem to mature. PQ implementation should not be prioritized until 2026+.

---

### Advanced Networking
- [ ] **NAT traversal improvements**
  - Implement hole-punching
  - Add relay server support
  - TURN server integration
  - Better UPnP/NAT-PMP support

- [ ] **Network topology optimization**
  - Optimize Gossipsub mesh degree
  - Implement custom routing logic
  - Add peer quality scoring
  - Prefer faster/closer peers

- [ ] **Multi-datacenter support**
  - Add region awareness
  - Prefer local peers when possible
  - Configurable WAN vs LAN behavior
  - Support for hybrid cloud deployments

**Dependencies:** These are advanced networking features for large-scale deployments.

---

### Experimental Features
- [ ] **CRDT-based conflict resolution**
  - Research Automerge or Yrs libraries
  - Implement automatic merging for text files
  - Support for concurrent edits
  - Real-time collaborative editing

- [ ] **Differential privacy for metadata**
  - Hide access patterns
  - Obfuscate file sizes
  - Dummy traffic generation
  - Timing attack mitigation

- [ ] **Blockchain-based audit log (experimental)**
  - Immutable sync history
  - Cryptographic proof of file versions
  - Distributed audit trail
  - Research viability

**Dependencies:** These are research topics and may never be implemented in core.

---

## 📋 Implementation Guidelines

### How to Use This Checklist

1. **Review & Prioritize**
   - Read through each section
   - Mark items relevant to your use case
   - Consider your threat model and requirements

2. **Group Related Items**
   - Items in the same section often have dependencies
   - Implement related features together
   - Example: Authentication + Authorization as one milestone

3. **Start with Critical**
   - Address security issues first
   - Then stability and reliability
   - Then user experience
   - Finally advanced features

4. **Iterative Development**
   - Don't try to do everything at once
   - Pick 2-3 items per sprint/milestone
   - Test thoroughly before moving on
   - Get feedback from users

### Suggested Milestone Plan

#### Milestone 1: Security Foundation (4-6 weeks)
- [ ] Authentication (shared secrets + HMAC)
- [ ] Peer allowlists
- [ ] Config validation
- [ ] Better error types
- [ ] Document PQ limitations

**Goal:** Make Syndactyl safe to use on a trusted network

---

#### Milestone 2: Reliability (3-4 weeks)
- [ ] Conflict detection (vector clocks)
- [ ] Conflict handling
- [ ] Retry logic
- [ ] Resume capability
- [ ] Transfer timeouts

**Goal:** Make Syndactyl robust against network issues

---

#### Milestone 3: Observability (2-3 weeks)
- [ ] Structured metrics
- [ ] Detailed logging
- [ ] Progress indication
- [ ] CLI with progress bars
- [ ] Integration tests

**Goal:** Make Syndactyl debuggable and monitorable

---

#### Milestone 4: Performance (3-4 weeks)
- [ ] Rate limiting
- [ ] Bandwidth management
- [ ] Content-addressed storage
- [ ] Compression support

**Goal:** Make Syndactyl efficient and scalable

---

#### Milestone 5: User Experience (2-3 weeks)
- [ ] `.syndactylignore` support
- [ ] File filtering
- [ ] Configuration wizard
- [ ] systemd service
- [ ] Documentation improvements

**Goal:** Make Syndactyl easy to use and deploy

---

#### Milestone 6: Advanced Features (4-6 weeks)
- [ ] File versioning
- [ ] Plugin system
- [ ] Web UI
- [ ] Delta sync

**Goal:** Make Syndactyl feature-competitive

---

#### Milestone 7: Production Hardening (2-4 weeks)
- [ ] Comprehensive test suite
- [ ] Benchmark suite
- [ ] Security audit
- [ ] Performance profiling
- [ ] Documentation completion

**Goal:** Make Syndactyl production-ready

---

## 🎯 Quick Start Recommendations

If you're unsure where to start, I recommend this order:

### Week 1-2: Foundation
1. ✅ Better error types (use `thiserror`)
2. ✅ Config validation
3. ✅ Document security limitations

### Week 3-4: Security
4. ✅ HMAC-based authentication
5. ✅ Peer allowlists
6. ✅ Rate limiting basics

### Week 5-6: Reliability
7. ✅ Retry logic
8. ✅ Vector clock conflict detection
9. ✅ Basic conflict handling

### Week 7-8: Testing & Polish
10. ✅ Integration tests
11. ✅ Progress indication + CLI
12. ✅ `.syndactylignore` support

**After 8 weeks:** You'll have a secure, reliable, and usable file sync system!

---

## 📝 Notes

- **Check off items** as you complete them (change `[ ]` to `[x]`)
- **Add dates** when starting/completing items for tracking
- **Link to PRs/commits** for reference
- **Update priorities** as requirements change
- **Add your own items** as new ideas emerge

This is a living document - update it as the project evolves!

---

## 🤝 Contributing

When contributing to Syndactyl, please:
1. Check this roadmap for planned features
2. Open an issue before starting major work
3. Reference the relevant checklist item in PRs
4. Update the checklist when completing items

---

Last Updated: 2024-01-13
