use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub name: String,
    pub extension: String,
    pub full_name: String,
    pub relative_path: String,
    pub absolute_path: String,
    pub file_size: u64,
}

/// Format file size to human readable string
pub fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.2} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.2} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.2} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

pub fn scan_folder(path: &Path, recursive: bool) -> Result<Vec<FileInfo>, std::io::Error> {
    let mut files = Vec::new();

    if !path.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotADirectory,
            "Path is not a directory",
        ));
    }

    scan_folder_internal(path, path, recursive, &mut files)?;

    // Sort alphabetically by relative path
    files.sort_by(|a, b| a.relative_path.to_lowercase().cmp(&b.relative_path.to_lowercase()));

    Ok(files)
}

fn scan_folder_internal(
    base_path: &Path,
    current_path: &Path,
    recursive: bool,
    files: &mut Vec<FileInfo>,
) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(current_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let full_name = entry.file_name().to_string_lossy().to_string();
            let extension = path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();
            let name = path
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Calculate relative path from base folder
            let relative_path = path
                .strip_prefix(base_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| full_name.clone());

            // Get absolute path
            let absolute_path = path
                .canonicalize()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string_lossy().to_string());

            // Get file size
            let file_size = entry.metadata().map(|m| m.len()).unwrap_or(0);

            files.push(FileInfo {
                name,
                extension,
                full_name,
                relative_path,
                absolute_path,
                file_size,
            });
        } else if path.is_dir() && recursive {
            // Recursively scan subdirectories
            scan_folder_internal(base_path, &path, recursive, files)?;
        }
    }

    Ok(())
}
