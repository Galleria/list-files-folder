# Project: File Lister

A Rust desktop application to list files from a folder and export to CSV.

## Quick Reference

- **Language**: Rust (2021 Edition)
- **GUI Framework**: eframe/egui 0.33
- **Build**: `cargo build --release`
- **Run**: `cargo run`

## Project Structure

```
src/
├── main.rs            # Entry point, CLI parsing, GUI/CLI mode switching
├── app.rs             # GUI application (egui), sorting, filtering, context menu
├── file_scanner.rs    # File system scanning, FileInfo struct
├── csv_export.rs      # CSV export with UTF-8 BOM
├── document_parser.rs # Document parsing (docx, xlsx, csv, txt preview)
└── lib.rs             # Module declarations
```

## Key Data Structure

```rust
struct FileInfo {
    name: String,           // File name without extension
    extension: String,      // File extension
    full_name: String,      // Complete file name
    relative_path: String,  // Path relative to selected folder (with [FolderName]/ prefix for multi-folder)
    absolute_path: String,  // Full absolute path
    file_size: u64,         // Size in bytes
    modified_timestamp: i64, // Unix timestamp
    source_folder: String,  // Source folder name (for multi-folder scanning)
}
```

## Features Implemented

- [x] GUI with native folder picker (rfd)
- [x] CLI mode with clap arguments (-f, -o, -r)
- [x] Recursive folder scanning
- [x] Resizable table columns (egui_extras TableBuilder)
- [x] Sortable columns (Name, Ext, Size, Path)
- [x] Text filter with real-time search
- [x] Right-click context menu → Open file location
- [x] CSV export with UTF-8 BOM (Excel compatible)
- [x] Unicode/Thai font support
- [x] File type icons (emoji-based, by extension)
- [x] Duplicate file name detection (⚠ indicator)
- [x] Show duplicates only filter (checkbox)
- [x] Row hover highlighting
- [x] Cross-platform support (Windows, macOS, Linux)
- [x] File rename (double-click or context menu)
- [x] File delete (context menu)
- [x] File move to folder (context menu or bulk)
- [x] Show today only filter (modified today)
- [x] Background scanning (non-blocking UI)
- [x] Date Modified column (sortable)
- [x] Image hover preview (tooltip popup)
- [x] Video hover preview (FFmpeg thumbnail extraction)
- [x] PDF hover preview (first page, requires Pdfium)
- [x] Multiple folder selection (add/remove folders)
- [x] Document hover preview (docx, xlsx, csv, txt)

## Documentation

- [README.md](README.md) - User documentation, installation, usage
- [SPEC.md](SPEC.md) - Full specification and requirements

**Important**: When adding or updating features, always update SPEC.md to keep documentation in sync with the codebase.

## CLI Usage

```bash
# GUI mode
cargo run

# CLI mode
cargo run -- -f "C:\folder" -o "output.csv" -r
```

## Notes

- Uses TopBottomPanel for fixed header/footer
- Filter exports only filtered results
- Cross-platform font loading (Windows/macOS/Linux)
- Platform-specific file manager integration
- Video preview requires FFmpeg (install with: `winget install ffmpeg`)
- PDF preview requires Pdfium library (pdfium-render crate)
