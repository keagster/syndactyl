use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use tracing::info;

/// Calculate SHA-256 hash of a file
pub fn calculate_file_hash(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}

/// Read entire file into memory (for files up to reasonable size)
pub fn read_file_content(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path)
}

/// Read a chunk of a file
pub fn read_file_chunk(path: &Path, offset: u64, chunk_size: usize) -> io::Result<Vec<u8>> {
    use std::io::Seek;
    
    let mut file = File::open(path)?;
    file.seek(io::SeekFrom::Start(offset))?;
    
    let mut buffer = vec![0u8; chunk_size];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);
    
    Ok(buffer)
}

/// Write file content to disk, creating parent directories if needed
pub fn write_file_content(path: &Path, content: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let mut file = File::create(path)?;
    file.write_all(content)?;
    file.sync_all()?;
    
    Ok(())
}

/// Append chunk to a file (for chunked transfers)
pub fn append_file_chunk(path: &Path, content: &[u8], offset: u64) -> io::Result<()> {
    use std::io::Seek;
    use std::fs::OpenOptions;
    
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(path)?;
    
    file.seek(io::SeekFrom::Start(offset))?;
    file.write_all(content)?;
    file.sync_all()?;
    
    Ok(())
}

/// Get file metadata (size, modified time)
pub fn get_file_metadata(path: &Path) -> io::Result<(u64, u64)> {
    let metadata = fs::metadata(path)?;
    let size = metadata.len();
    
    let modified_time = metadata.modified()?
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    Ok((size, modified_time))
}

/// Convert absolute path to relative path within observer base path
pub fn to_relative_path(absolute_path: &Path, base_path: &Path) -> Option<PathBuf> {
    absolute_path.strip_prefix(base_path).ok().map(|p| p.to_path_buf())
}

/// Convert relative path to absolute path using observer base path
pub fn to_absolute_path(relative_path: &Path, base_path: &Path) -> PathBuf {
    base_path.join(relative_path)
}

/// Move file to trash directory
pub fn move_to_trash(path: &Path, base_path: &Path) -> io::Result<()> {
    let trash_dir = base_path.join(".syndactyl").join("trash");
    fs::create_dir_all(&trash_dir)?;
    
    // Generate unique trash filename with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    
    let filename = path.file_name().unwrap_or_default();
    let trash_path = trash_dir.join(format!("{}.{}", filename.to_string_lossy(), timestamp));
    
    fs::rename(path, &trash_path)?;
    info!(original = %path.display(), trash = %trash_path.display(), "Moved file to trash");
    
    Ok(())
}

/// Check if file should be synced (not in .syndactyl directory, etc.)
pub fn should_sync_file(relative_path: &Path) -> bool {
    // Skip .syndactyl internal directory
    if relative_path.starts_with(".syndactyl") {
        return false;
    }
    
    // Skip hidden files (optional - you can change this)
    if let Some(filename) = relative_path.file_name() {
        if filename.to_string_lossy().starts_with('.') {
            return false;
        }
    }
    
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    
    #[test]
    fn test_calculate_file_hash() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"hello world").unwrap();
        
        let hash = calculate_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
    }
    
    #[test]
    fn test_relative_paths() {
        let base = PathBuf::from("/home/user/sync");
        let absolute = PathBuf::from("/home/user/sync/subdir/file.txt");
        
        let relative = to_relative_path(&absolute, &base).unwrap();
        assert_eq!(relative, PathBuf::from("subdir/file.txt"));
        
        let back_to_absolute = to_absolute_path(&relative, &base);
        assert_eq!(back_to_absolute, absolute);
    }
}
