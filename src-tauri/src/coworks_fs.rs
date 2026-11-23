//! File system operations for Coworks
//!
//! Provides directory reading, file info, and path utilities.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// File entry information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoworksFileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
}

/// Read directory contents
#[tauri::command]
pub fn coworks_read_directory(path: String) -> Result<Vec<CoworksFileEntry>, String> {
    let dir_path = Path::new(&path);

    if !dir_path.exists() {
        return Err(format!("Directory does not exist: {}", path));
    }

    if !dir_path.is_dir() {
        return Err(format!("Path is not a directory: {}", path));
    }

    let entries = fs::read_dir(dir_path)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    let mut files: Vec<CoworksFileEntry> = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                log::warn!("Failed to read entry: {}", e);
                continue;
            }
        };

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                log::warn!("Failed to get metadata for {:?}: {}", entry.path(), e);
                continue;
            }
        };

        let name = entry.file_name().to_string_lossy().to_string();
        let file_path = entry.path().to_string_lossy().to_string();

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        files.push(CoworksFileEntry {
            name,
            path: file_path,
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            modified,
        });
    }

    // Sort: directories first, then by name
    files.sort_by(|a, b| {
        if a.is_dir == b.is_dir {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        } else if a.is_dir {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    Ok(files)
}

/// Read file contents as text
#[tauri::command]
pub fn coworks_read_file_text(path: String) -> Result<String, String> {
    fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read file: {}", e))
}

/// Check if a path exists
#[tauri::command]
pub fn coworks_path_exists(path: String) -> bool {
    Path::new(&path).exists()
}

/// Get file or directory info
#[tauri::command]
pub fn coworks_get_file_info(path: String) -> Result<CoworksFileEntry, String> {
    let file_path = Path::new(&path);

    if !file_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let metadata = fs::metadata(&path)
        .map_err(|e| format!("Failed to get file info: {}", e))?;

    let name = file_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Ok(CoworksFileEntry {
        name,
        path: path.clone(),
        is_dir: metadata.is_dir(),
        size: metadata.len(),
        modified,
    })
}
