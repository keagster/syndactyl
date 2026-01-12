use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEventMessage {
    pub observer: String,
    pub event_type: String,
    pub path: String,              // Relative path within the observer
    pub details: Option<String>,
    pub hash: Option<String>,      // SHA-256 hash of file content
    pub size: Option<u64>,         // File size in bytes
    pub modified_time: Option<u64>, // Unix timestamp of last modification
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileTransferRequest {
    pub observer: String,          // Which observer/share this belongs to
    pub path: String,              // Relative path within the observer
    pub hash: String,              // Expected hash for verification
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileTransferResponse {
    pub observer: String,
    pub path: String,
    pub data: Vec<u8>,             // File chunk data
    pub offset: u64,               // Byte offset of this chunk
    pub total_size: u64,           // Total file size
    pub hash: String,              // Hash of complete file
    pub is_last_chunk: bool,       // Is this the final chunk?
}
