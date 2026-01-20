# Post-Quantum Cryptography - Current Implementation Options

## TL;DR: Not Practical Yet

**Question:** Can we implement post-quantum cryptography in Syndactyl now?

**Answer:** Not without significant trade-offs. Here are your realistic options:

---

## Option Analysis

### ❌ Option 1: Replace libp2p Entirely
**Effort:** 6+ months  
**Recommended:** No

Would require:
- Rewriting entire P2P networking stack
- Implementing Gossipsub from scratch
- Implementing Kademlia DHT from scratch
- Implementing connection management, NAT traversal, etc.
- Basically building a new libp2p

**Verdict:** Not feasible for a learning project.

---

### ⚠️ Option 2: Application-Layer PQ Encryption (Hybrid Approach)
**Effort:** 2-4 weeks  
**Recommended:** Only if you MUST have PQ protection now

#### Implementation

```rust
// Cargo.toml
[dependencies]
pqcrypto-kyber = "0.8"
pqcrypto-dilithium = "0.5"

// src/core/encryption.rs
use pqcrypto_kyber::kyber1024;
use pqcrypto_dilithium::dilithium5;

pub struct PQKeypair {
    pub kem: kyber1024::Keypair,      // Key encapsulation
    pub sig: dilithium5::Keypair,     // Signatures
}

pub struct PeerRegistry {
    // Map PeerId -> PQ public keys
    peers: HashMap<PeerId, PQPublicKeys>,
}

// Encrypt file before sending through libp2p
pub fn encrypt_file_content(
    content: &[u8],
    recipient_pq_pubkey: &kyber1024::PublicKey,
) -> Result<Vec<u8>> {
    // 1. Generate random symmetric key
    let symmetric_key = ChaCha20Key::generate();
    
    // 2. Encrypt content with symmetric key
    let encrypted_content = chacha20_encrypt(content, &symmetric_key);
    
    // 3. Encapsulate symmetric key with recipient's PQ public key
    let (shared_secret, ciphertext) = kyber1024::encapsulate(recipient_pq_pubkey);
    
    // 4. Combine: [ciphertext || encrypted_content]
    let mut result = Vec::new();
    result.extend_from_slice(&ciphertext);
    result.extend_from_slice(&encrypted_content);
    
    Ok(result)
}

// Modify FileTransferResponse
pub struct FileTransferResponse {
    pub observer: String,
    pub path: String,
    pub data: Vec<u8>,              // Now PQ-encrypted
    pub offset: u64,
    pub total_size: u64,
    pub hash: String,
    pub is_last_chunk: bool,
    pub pq_ciphertext: Vec<u8>,     // PQ key encapsulation
}
```

#### Problems with This Approach:

1. **Key Distribution Problem**
   - How do peers discover each other's PQ public keys?
   - Need a secure channel to exchange keys initially
   - Chicken-and-egg: need secure channel to establish secure channel

2. **Metadata Still Exposed**
   ```
   Gossipsub broadcasts FileEventMessage:
   - observer: "CompanyDocs" ← VISIBLE
   - path: "secret_project.pdf" ← VISIBLE
   - hash: "abc123..." ← VISIBLE
   
   Only file CONTENT is PQ-encrypted
   Metadata still goes through X25519 (quantum-vulnerable)
   ```

3. **Double Encryption Overhead**
   ```
   File → PQ Encrypt → libp2p Noise Encrypt → Network
                          ↑
                    This layer still uses X25519
   ```

4. **Complexity**
   - Need to manage two sets of keys (libp2p + PQ)
   - Need peer registry to track PQ keys
   - Need key rotation logic
   - Need fallback for peers without PQ support

5. **Size Overhead**
   ```
   Original file chunk: 1 MB
   + PQ ciphertext: 1,568 bytes
   + Noise encryption overhead: ~16 bytes
   Total: ~1.001 MB (minimal overhead)
   ```

#### When to Use This:
- You have extremely sensitive data
- You need PQ protection NOW
- You accept the complexity and maintenance burden
- You're willing to implement proper key management

---

### ⚠️ Option 3: Fork libp2p and Add PQ Support
**Effort:** 1-3 months  
**Recommended:** No (unless you're a cryptography expert)

Would require:
1. Fork `rust-libp2p` repository
2. Replace `libp2p-noise` with custom PQ-Noise implementation
3. Implement hybrid Noise protocol (XX pattern with PQ)
4. Test thoroughly for security vulnerabilities
5. Maintain fork forever (merge upstream changes)
6. No interoperability with standard libp2p nodes

**Verdict:** Only viable for well-funded projects with crypto expertise.

---

### ✅ Option 4: Wait for libp2p (RECOMMENDED)
**Effort:** Low (just monitor and upgrade)  
**Recommended:** Yes, for most users

#### Current Status:
- **libp2p team is aware** of PQ requirements
- **Discussions ongoing** in libp2p/specs repository
- **NIST standards finalized** in 2024 (ML-KEM, ML-DSA)
- **Experimental implementations** may appear in 2025-2026

#### What You Should Do:

1. **Monitor libp2p progress:**
   - Watch: https://github.com/libp2p/specs/issues
   - Search for: "post-quantum", "PQ", "quantum-resistant"
   - Join: https://discuss.libp2p.io/

2. **Document current limitation:**
   ```markdown
   # README.md
   
   ## Security Notice
   
   ⚠️ **Post-Quantum Cryptography:** Syndactyl currently uses libp2p's 
   Noise protocol (X25519 + ChaCha20-Poly1305), which is NOT resistant 
   to quantum computers. While this is secure against classical computers, 
   future quantum computers could potentially decrypt recorded traffic.
   
   **Mitigation:** 
   - Use OS-level disk encryption (LUKS, FileVault, BitLocker)
   - Avoid syncing data that must remain secret for 20+ years
   - Monitor for libp2p post-quantum updates
   
   We will upgrade to post-quantum cryptography when libp2p provides 
   stable support (estimated 2026-2027).
   ```

3. **Design for future upgrade:**
   ```rust
   // Add configuration option now (not implemented yet)
   pub struct NetworkConfig {
       pub listen_addr: String,
       pub port: String,
       pub dht_mode: String,
       pub bootstrap_peers: Vec<BootstrapPeer>,
       
       // Future: enable when libp2p supports it
       pub enable_post_quantum: bool,  // Default: false
   }
   ```

4. **Simple interim protections:**
   ```rust
   // Use stronger hashing
   use sha2::Sha512;  // Instead of Sha256
   
   // Recommend full-disk encryption in docs
   // This protects "data at rest" which is currently unencrypted
   ```

---

## Realistic Timeline

```
2024 (Now)
  ├─ NIST finalizes PQ standards ✅
  ├─ Community begins implementations
  └─ Syndactyl: Document limitation, monitor progress

2025
  ├─ First experimental PQ libraries for libp2p
  ├─ Proof-of-concept implementations
  └─ Syndactyl: Test experimental versions

2026-2027
  ├─ Stable libp2p PQ support
  ├─ Community testing and adoption
  └─ Syndactyl: Upgrade to PQ-enabled libp2p

2028-2030
  ├─ PQ becomes standard
  ├─ Classical-only protocols deprecated
  └─ Syndactyl: Full PQ migration complete
```

---

## What To Do NOW (Practical Steps)

### 1. Add Security Documentation
Create `docs/SECURITY.md`:

```markdown
# Security Policy

## Cryptography

Syndactyl uses the following cryptographic primitives:

- **Transport Encryption:** Noise Protocol (via libp2p)
  - Key Exchange: X25519 (Curve25519)
  - Cipher: ChaCha20-Poly1305
  - MAC: Poly1305

- **File Integrity:** SHA-256 hashing

- **Peer Identity:** Ed25519 keypairs

## Known Limitations

### Post-Quantum Vulnerability

The current cryptography is **not resistant to quantum computers**. 
Specifically:

- **X25519** is vulnerable to Shor's Algorithm
- **Ed25519** is vulnerable to Shor's Algorithm
- **ChaCha20/SHA-256** are partially resistant (128-bit effective security)

### Threat Model

**Protected Against:**
- Network eavesdropping (passive listeners)
- Man-in-the-middle attacks
- Packet tampering
- Replay attacks

**NOT Protected Against:**
- Quantum computer attacks (future threat, ~2030-2040)
- Harvest-now-decrypt-later attacks (recording traffic for future decryption)
- Unauthorized peer access (no authentication implemented yet)
- Physical disk access (files stored as plaintext)

## Recommendations

For sensitive data that must remain confidential beyond 2030:

1. **Use full-disk encryption** (LUKS, FileVault, BitLocker)
2. **Consider additional encryption** at application level
3. **Limit sensitive data** to air-gapped systems
4. **Monitor for updates** when libp2p adds PQ support

## Reporting Vulnerabilities

Please report security issues to: [your contact]
```

### 2. Update README.md

Add a security section:

```markdown
## 🔒 Security

**Transport Encryption:** All network traffic is encrypted using the Noise 
Protocol (X25519 + ChaCha20-Poly1305).

**⚠️ Post-Quantum Note:** Current encryption is not quantum-resistant. While 
secure today, future quantum computers (estimated 2030+) could potentially 
decrypt recorded traffic. See [SECURITY.md](docs/SECURITY.md) for details.

**Recommendations:**
- Use OS-level disk encryption for data at rest
- Avoid syncing data requiring 20+ years of secrecy
- Stay updated for post-quantum migration (planned when libp2p supports it)
```

### 3. Add Configuration Placeholder

```rust
// In src/core/config.rs
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    pub listen_addr: String,
    pub port: String,
    pub dht_mode: String,
    pub bootstrap_peers: Vec<BootstrapPeer>,
    
    // Future feature: enable when libp2p supports it
    #[serde(default)]
    pub post_quantum: Option<PostQuantumConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostQuantumConfig {
    pub enabled: bool,
    pub algorithm: String,  // "ml-kem-1024", "ml-dsa-87", etc.
}

// For now, just log a warning if someone tries to enable it
if let Some(pq) = &network_config.post_quantum {
    if pq.enabled {
        warn!("Post-quantum cryptography is not yet supported. Ignoring configuration.");
    }
}
```

---

## Conclusion

**Should you implement PQ now?** 

**No**, unless:
- You work for a government agency
- You're syncing classified/highly sensitive data
- You have strong crypto expertise
- You're willing to maintain complex custom code

**What you SHOULD do:**

1. ✅ Document the limitation clearly
2. ✅ Monitor libp2p for PQ support
3. ✅ Design with future upgrade in mind
4. ✅ Recommend OS-level encryption to users
5. ✅ Wait for libp2p to provide stable PQ support

**Timeline:** Realistically, you'll be able to add PQ support via a simple 
libp2p upgrade in 2026-2027. Trying to implement it before then is not worth 
the effort and risk.

---

## Alternative: Interim Solution (If You Must)

If you absolutely need some PQ protection now, the simplest approach:

```rust
// Add optional end-to-end file encryption
// Users manually exchange PQ public keys out-of-band

pub struct E2EEncryption {
    my_keypair: kyber1024::Keypair,
    peer_keys: HashMap<String, kyber1024::PublicKey>,
}

// In config.json
{
  "observers": [
    {
      "name": "HighlySensitive",
      "path": "/home/user/sensitive",
      "e2e_encryption": {
        "enabled": true,
        "authorized_peers": {
          "peer_id_123": "pq_public_key_base64...",
          "peer_id_456": "pq_public_key_base64..."
        }
      }
    }
  ]
}
```

This adds PQ encryption only for file content, not metadata. Still has all 
the problems mentioned above, but better than nothing.

**Bottom line:** Wait for libp2p. It's coming.
