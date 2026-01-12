use crate::core::models::{FileTransferRequest, FileTransferResponse};
use crate::core::file_handler;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use tracing::{info, warn, error};

/// Chunk size for file transfers (1MB)
pub const CHUNK_SIZE: usize = 1024 * 1024;

/// Maximum file size to transfer (100MB for now)
pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// In-progress file transfer tracking
pub struct FileTransferTracker {
    /// Map of (observer, path) -> received chunks
    transfers: HashMap<(String, String), TransferState>,
}

struct TransferState {
    observer: String,
    path: String,
    total_size: u64,
    expected_hash: String,
    chunks: HashMap<u64, Vec<u8>>, // offset -> data
    base_path: PathBuf,
}

impl FileTransferTracker {
    pub fn new() -> Self {
        Self {
            transfers: HashMap::new(),
        }
    }
    
    /// Start tracking a new file transfer
    pub fn start_transfer(
        &mut self,
        observer: String,
        path: String,
        total_size: u64,
        hash: String,
        base_path: PathBuf,
    ) {
        let key = (observer.clone(), path.clone());
        let state = TransferState {
            observer: observer.clone(),
            path: path.clone(),
            total_size,
            expected_hash: hash,
            chunks: HashMap::new(),
            base_path,
        };
        
        self.transfers.insert(key, state);
        info!(observer = %observer, path = %path, size = total_size, "Started tracking file transfer");
    }
    
    /// Add a chunk to an in-progress transfer
    pub fn add_chunk(
        &mut self,
        observer: &str,
        path: &str,
        offset: u64,
        data: Vec<u8>,
        is_last_chunk: bool,
    ) -> Result<Option<PathBuf>, String> {
        let key = (observer.to_string(), path.to_string());
        
        let state = self.transfers.get_mut(&key)
            .ok_or_else(|| format!("No transfer in progress for {}/{}", observer, path))?;
        
        // Add chunk
        state.chunks.insert(offset, data);
        
        if is_last_chunk {
            // All chunks received, assemble file
            return self.complete_transfer(&key);
        }
        
        Ok(None)
    }
    
    /// Complete a file transfer by assembling all chunks
    fn complete_transfer(&mut self, key: &(String, String)) -> Result<Option<PathBuf>, String> {
        let state = self.transfers.remove(key)
            .ok_or_else(|| "Transfer not found".to_string())?;
        
        // Sort chunks by offset
        let mut offsets: Vec<u64> = state.chunks.keys().copied().collect();
        offsets.sort();
        
        // Assemble file content
        let mut file_content = Vec::with_capacity(state.total_size as usize);
        for offset in offsets {
            if let Some(chunk) = state.chunks.get(&offset) {
                file_content.extend_from_slice(chunk);
            }
        }
        
        // Verify size
        if file_content.len() != state.total_size as usize {
            error!(
                expected = state.total_size,
                received = file_content.len(),
                "File size mismatch"
            );
            return Err("File size mismatch".to_string());
        }
        
        // Verify hash
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&file_content);
        let calculated_hash = format!("{:x}", hasher.finalize());
        
        if calculated_hash != state.expected_hash {
            error!(
                expected = %state.expected_hash,
                calculated = %calculated_hash,
                "File hash mismatch"
            );
            return Err("File hash mismatch".to_string());
        }
        
        // Write file to disk
        let absolute_path = file_handler::to_absolute_path(Path::new(&state.path), &state.base_path);
        
        if let Err(e) = file_handler::write_file_content(&absolute_path, &file_content) {
            error!(path = %absolute_path.display(), error = ?e, "Failed to write file");
            return Err(format!("Failed to write file: {}", e));
        }
        
        info!(
            observer = %state.observer,
            path = %state.path,
            size = state.total_size,
            "File transfer completed successfully"
        );
        
        Ok(Some(absolute_path))
    }
    
    /// Cancel a transfer
    pub fn cancel_transfer(&mut self, observer: &str, path: &str) {
        let key = (observer.to_string(), path.to_string());
        if self.transfers.remove(&key).is_some() {
            info!(observer = %observer, path = %path, "Cancelled file transfer");
        }
    }
}

/// Generate file transfer response chunks for a file
pub fn generate_file_chunks(
    observer: &str,
    relative_path: &Path,
    absolute_path: &Path,
    hash: &str,
) -> Result<Vec<FileTransferResponse>, String> {
    // Check file size
    let metadata = file_handler::get_file_metadata(absolute_path)
        .map_err(|e| format!("Failed to get file metadata: {}", e))?;
    
    let total_size = metadata.0;
    
    if total_size > MAX_FILE_SIZE {
        return Err(format!("File too large: {} bytes (max: {})", total_size, MAX_FILE_SIZE));
    }
    
    let mut chunks = Vec::new();
    let mut offset = 0u64;
    
    while offset < total_size {
        let chunk_data = file_handler::read_file_chunk(absolute_path, offset, CHUNK_SIZE)
            .map_err(|e| format!("Failed to read file chunk: {}", e))?;
        
        let is_last = offset + chunk_data.len() as u64 >= total_size;
        
        let response = FileTransferResponse {
            observer: observer.to_string(),
            path: relative_path.display().to_string(),
            data: chunk_data.clone(),
            offset,
            total_size,
            hash: hash.to_string(),
            is_last_chunk: is_last,
        };
        
        chunks.push(response);
        offset += chunk_data.len() as u64;
    }
    
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;
    
    #[test]
    fn test_file_transfer_tracker() {
        let temp_dir = TempDir::new().unwrap();
        let mut tracker = FileTransferTracker::new();
        
        let observer = "test-observer".to_string();
        let path = "test.txt".to_string();
        let content = b"Hello, World!";
        let hash = {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(content);
            format!("{:x}", hasher.finalize())
        };
        
        tracker.start_transfer(
            observer.clone(),
            path.clone(),
            content.len() as u64,
            hash.clone(),
            temp_dir.path().to_path_buf(),
        );
        
        let result = tracker.add_chunk(
            &observer,
            &path,
            0,
            content.to_vec(),
            true,
        );
        
        assert!(result.is_ok());
        let file_path = result.unwrap().unwrap();
        
        // Verify file was written
        let written_content = std::fs::read(&file_path).unwrap();
        assert_eq!(written_content, content);
    }
}
