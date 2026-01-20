use sha2::Sha256;
use hmac::{Hmac, Mac};
use crate::core::models::FileEventMessage;

type HmacSha256 = Hmac<Sha256>;

/// Compute HMAC-SHA256 for a FileEventMessage
/// Message format: observer||event_type||path||hash||size||modified_time
pub fn compute_hmac(msg: &FileEventMessage, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    
    // Build the message to authenticate
    mac.update(msg.observer.as_bytes());
    mac.update(b"||");
    mac.update(msg.event_type.as_bytes());
    mac.update(b"||");
    mac.update(msg.path.as_bytes());
    mac.update(b"||");
    
    if let Some(ref hash) = msg.hash {
        mac.update(hash.as_bytes());
    }
    mac.update(b"||");
    
    if let Some(size) = msg.size {
        mac.update(size.to_string().as_bytes());
    }
    mac.update(b"||");
    
    if let Some(mtime) = msg.modified_time {
        mac.update(mtime.to_string().as_bytes());
    }
    
    // Return hex-encoded HMAC
    format!("{:x}", mac.finalize().into_bytes())
}

/// Verify HMAC for a FileEventMessage using constant-time comparison
/// Returns true if HMAC is valid, false otherwise
pub fn verify_hmac(msg: &FileEventMessage, secret: &str) -> bool {
    let provided_hmac = match &msg.hmac {
        Some(h) => h,
        None => return false, // No HMAC provided
    };
    
    let computed_hmac = compute_hmac(msg, secret);
    
    // Constant-time comparison to prevent timing attacks
    constant_time_compare(provided_hmac, &computed_hmac)
}

/// Constant-time string comparison to prevent timing attacks
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    
    let mut result = 0u8;
    for i in 0..a_bytes.len() {
        result |= a_bytes[i] ^ b_bytes[i];
    }
    
    result == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hmac_computation() {
        let msg = FileEventMessage {
            observer: "test-observer".to_string(),
            event_type: "Create".to_string(),
            path: "test.txt".to_string(),
            details: None,
            hash: Some("abcd1234".to_string()),
            size: Some(1024),
            modified_time: Some(1234567890),
            hmac: None,
        };
        
        let secret = "test-secret";
        let hmac = compute_hmac(&msg, secret);
        
        // HMAC should be a 64-character hex string (SHA256 = 32 bytes = 64 hex chars)
        assert_eq!(hmac.len(), 64);
        assert!(hmac.chars().all(|c| c.is_ascii_hexdigit()));
    }
    
    #[test]
    fn test_hmac_verification_success() {
        let secret = "test-secret";
        let mut msg = FileEventMessage {
            observer: "test-observer".to_string(),
            event_type: "Create".to_string(),
            path: "test.txt".to_string(),
            details: None,
            hash: Some("abcd1234".to_string()),
            size: Some(1024),
            modified_time: Some(1234567890),
            hmac: None,
        };
        
        // Compute and attach HMAC
        let hmac = compute_hmac(&msg, secret);
        msg.hmac = Some(hmac);
        
        // Verification should succeed
        assert!(verify_hmac(&msg, secret));
    }
    
    #[test]
    fn test_hmac_verification_failure_wrong_secret() {
        let secret = "test-secret";
        let wrong_secret = "wrong-secret";
        
        let mut msg = FileEventMessage {
            observer: "test-observer".to_string(),
            event_type: "Create".to_string(),
            path: "test.txt".to_string(),
            details: None,
            hash: Some("abcd1234".to_string()),
            size: Some(1024),
            modified_time: Some(1234567890),
            hmac: None,
        };
        
        // Compute HMAC with correct secret
        let hmac = compute_hmac(&msg, secret);
        msg.hmac = Some(hmac);
        
        // Verification with wrong secret should fail
        assert!(!verify_hmac(&msg, wrong_secret));
    }
    
    #[test]
    fn test_hmac_verification_failure_tampered_message() {
        let secret = "test-secret";
        
        let mut msg = FileEventMessage {
            observer: "test-observer".to_string(),
            event_type: "Create".to_string(),
            path: "test.txt".to_string(),
            details: None,
            hash: Some("abcd1234".to_string()),
            size: Some(1024),
            modified_time: Some(1234567890),
            hmac: None,
        };
        
        // Compute HMAC
        let hmac = compute_hmac(&msg, secret);
        msg.hmac = Some(hmac);
        
        // Tamper with the message
        msg.path = "tampered.txt".to_string();
        
        // Verification should fail
        assert!(!verify_hmac(&msg, secret));
    }
    
    #[test]
    fn test_hmac_verification_no_hmac() {
        let msg = FileEventMessage {
            observer: "test-observer".to_string(),
            event_type: "Create".to_string(),
            path: "test.txt".to_string(),
            details: None,
            hash: Some("abcd1234".to_string()),
            size: Some(1024),
            modified_time: Some(1234567890),
            hmac: None, // No HMAC provided
        };
        
        // Verification should fail when no HMAC is provided
        assert!(!verify_hmac(&msg, "test-secret"));
    }
    
    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("hello", "hell"));
        assert!(!constant_time_compare("hell", "hello"));
    }
}
