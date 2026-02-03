# File Lister - Specification & Requirements

## Overview

A desktop application built with Rust that lists files from a selected folder and exports the file information to CSV format. Supports both GUI and CLI modes.

## Functional Requirements

### FR-01: Folder Selection
- **FR-01.1**: User can select a folder via native file dialog (GUI mode)
- **FR-01.2**: User can specify folder path via command-line argument (CLI mode)
- **FR-01.3**: Display selected folder path in the UI

### FR-02: File Scanning
- **FR-02.1**: Scan all files in the selected folder
- **FR-02.2**: Option to scan subfolders recursively (checkbox in GUI, `-r` flag in CLI)
- **FR-02.3**: Background scanning with non-blocking UI (spinner shown during scan)
- **FR-02.4**: Extract file information:
  - File name (without extension)
  - File extension
  - Full file name (with extension)
  - Relative path (from selected folder)
  - Absolute/full path
  - File size in bytes
  - Date modified (timestamp)

### FR-03: File Display (GUI)
- **FR-03.1**: Display files in a table with columns: Checkbox, Icons, Name, Extension, Size, Date Modified, Path, Full Path
- **FR-03.2**: Table columns are resizable by dragging (except Checkbox and Icons columns)
- **FR-03.3**: Table auto-resizes with window
- **FR-03.4**: Striped rows for readability

### FR-04: Sorting
- **FR-04.1**: Sort by Name (ascending/descending)
- **FR-04.2**: Sort by Extension (ascending/descending)
- **FR-04.3**: Sort by Size (ascending/descending)
- **FR-04.4**: Sort by Path (ascending/descending)
- **FR-04.5**: Sort by Date Modified (ascending/descending)
- **FR-04.6**: Click column header to toggle sort order
- **FR-04.7**: Display sort indicator (^ or v) on active column

### FR-05: Filtering
- **FR-05.1**: Text input to filter files
- **FR-05.2**: Filter matches against: name, extension, relative path, full name
- **FR-05.3**: Case-insensitive filtering
- **FR-05.4**: Real-time filtering as user types
- **FR-05.5**: Clear button to reset filter
- **FR-05.6**: Show count: "Showing X of Y files"
- **FR-05.7**: "Show duplicates only" checkbox to filter and display only duplicate files
- **FR-05.8**: "Show today only" checkbox to filter files modified today

### FR-06: Context Menu
- **FR-06.1**: Right-click on any cell shows context menu
- **FR-06.2**: "Open file location" option opens native file manager:
  - Windows: Explorer with file selected
  - macOS: Finder with file selected
  - Linux: Default file manager (parent folder)
- **FR-06.3**: "Rename" option to rename the file (inline editing)
- **FR-06.4**: "Move to folder..." option to move file to another location
- **FR-06.5**: "Delete" option to delete the file from disk

### FR-07: CSV Export
- **FR-07.1**: Export file list to CSV format
- **FR-07.2**: Native save dialog to choose export location
- **FR-07.3**: CSV includes UTF-8 BOM for Excel compatibility
- **FR-07.4**: Export columns: File Name, Extension, Size (bytes), Relative Path, Full Path
- **FR-07.5**: Export only filtered results (if filter is active)

### FR-08: CLI Mode
- **FR-08.1**: Run without GUI using command-line arguments
- **FR-08.2**: Arguments:
  - `-f, --folder <PATH>`: Folder to scan
  - `-o, --output <PATH>`: Output CSV file (default: files.csv)
  - `-r, --recursive`: Include subfolders
- **FR-08.3**: Display progress in console

### FR-09: File Type Icons
- **FR-09.1**: Display file type icon in dedicated icon column
- **FR-09.2**: Icons based on file extension:
  - 📝 Text files (txt, md, rtf)
  - 📕 PDF documents
  - 📘 Word documents (doc, docx, odt)
  - 📗 Spreadsheets (xls, xlsx, ods)
  - 📙 Presentations (ppt, pptx, odp)
  - 🖼 Images (jpg, png, gif, etc.)
  - 🎵 Audio files (mp3, wav, etc.)
  - 🎬 Video files (mp4, avi, etc.)
  - 📦 Archives (zip, rar, 7z, etc.)
  - 💻 Source code (rs, py, js, etc.)
  - 🌐 Web files (html, css)
  - 📊 Data files (json, xml, csv, sql)
  - ⚙ Config/executables (ini, yaml, exe)
  - 🔤 Fonts (ttf, otf, woff)
  - 📄 Default for unknown types

### FR-10: Duplicate File Detection
- **FR-10.1**: Detect files with identical names (full_name)
- **FR-10.2**: Display warning icon (⚠) in orange for duplicates
- **FR-10.3**: Hover tooltip shows duplicate count
- **FR-10.4**: Duplicate detection based on all files (not affected by text filter)
- **FR-10.5**: "Show duplicates only" checkbox to filter and display only duplicate files

### FR-11: Row Hover Highlighting
- **FR-11.1**: Highlight table rows on mouse hover
- **FR-11.2**: Visual feedback for better row identification

### FR-12: File Rename
- **FR-12.1**: Double-click on Name column to enter inline edit mode
- **FR-12.2**: Press Enter or click outside to confirm rename
- **FR-12.3**: Press Escape to cancel rename
- **FR-12.4**: Also available via right-click context menu

### FR-13: File Delete
- **FR-13.1**: Delete file via right-click context menu
- **FR-13.2**: File is permanently deleted from disk
- **FR-13.3**: List automatically refreshes after deletion

### FR-14: File Move
- **FR-14.1**: Move file to another folder via right-click context menu
- **FR-14.2**: Native folder picker dialog to select destination
- **FR-14.3**: Cross-device move support (copy + delete if rename fails)
- **FR-14.4**: List automatically refreshes after move

### FR-15: Bulk Operations
- **FR-15.1**: Checkbox column for selecting multiple files
- **FR-15.2**: Header checkbox to select/deselect all visible files
- **FR-15.3**: "Move Selected (N)" button to move all selected files
- **FR-15.4**: "Delete Selected (N)" button to delete all selected files
- **FR-15.5**: Confirmation modal dialog for bulk delete with file list
- **FR-15.6**: Selection cleared when filter changes (indices would be invalid)

### FR-16: Image Hover Preview
- **FR-16.1**: Show image thumbnail on hover for image files
- **FR-16.2**: Supported formats: jpg, jpeg, png, gif, bmp, ico, webp
- **FR-16.3**: Background image loading (non-blocking UI)
- **FR-16.4**: Image cache to avoid reloading
- **FR-16.5**: Automatic resize for large images (max 400x400 preview)
- **FR-16.6**: Preview appears on icon or name column hover

### FR-17: Video Hover Preview
- **FR-17.1**: Show video thumbnail on hover for video files
- **FR-17.2**: Supported formats: mp4, avi, mkv, mov, wmv, flv, webm, m4v, mpeg, mpg, 3gp
- **FR-17.3**: Thumbnail extraction using FFmpeg (requires FFmpeg in system PATH)
- **FR-17.4**: Extract frame at 1 second (fallback to 0 seconds for short videos)
- **FR-17.5**: "Loading video thumbnail..." indicator while extracting (10-second timeout)
- **FR-17.6**: 🎬 icon indicator to distinguish video previews from images
- **FR-17.7**: Thumbnail cache to avoid re-extraction

### FR-18: PDF Hover Preview
- **FR-18.1**: Show PDF first page thumbnail on hover for PDF files
- **FR-18.2**: Supported formats: pdf
- **FR-18.3**: Thumbnail extraction using Pdfium library
- **FR-18.4**: Render first page at 150 DPI, scaled to max 400x400 pixels
- **FR-18.5**: "Loading PDF thumbnail..." indicator while rendering
- **FR-18.6**: 📕 icon indicator for PDF files
- **FR-18.7**: Thumbnail cache to avoid re-rendering

### FR-19: Manual Dependency Download
- **FR-19.1**: "Download Pdfium" button in footer when Pdfium is not available
- **FR-19.2**: "Download FFmpeg" button in footer when FFmpeg is not available
- **FR-19.3**: Downloads from official release repositories:
  - Pdfium: bblanchon/pdfium-binaries (GitHub)
  - FFmpeg: gyan.dev (official builds)
- **FR-19.4**: Background download with progress indication
- **FR-19.5**: Automatic extraction to user's local data directory
- **FR-19.6**: Button hidden once dependency is available

## Non-Functional Requirements

### NFR-01: Unicode Support
- Support Thai, Chinese, Japanese, and other Unicode characters in file names
- Load system fonts (Segoe UI, Arial, Tahoma, Microsoft YaHei) for Unicode rendering

### NFR-02: Performance
- Handle folders with thousands of files
- Virtual scrolling for large file lists
- Background scanning with non-blocking UI
- Background image/video thumbnail loading

### NFR-03: User Interface
- Minimum window size: 600x400 pixels
- Default window size: 1000x600 pixels
- Fixed header and footer panels
- Scrollable table area
- Loading spinner during scanning

### NFR-04: Platform
- Cross-platform support: Windows, macOS, Linux
- Platform-specific font loading for Unicode support
- Platform-specific file manager integration:
  - Windows: `explorer /select,`
  - macOS: `open -R`
  - Linux: `xdg-open` (parent folder)

### NFR-05: External Dependencies
- Video preview requires FFmpeg in system PATH or downloaded via app
- PDF preview requires Pdfium library (auto-downloaded on first use or via button)
- Download locations:
  - Pdfium: `%LOCALAPPDATA%/pdfium/pdfium.dll` (Windows)
  - FFmpeg: User's PATH or downloaded via app button

## Technical Specifications

### Technology Stack

| Component | Technology | Version |
|-----------|------------|---------|
| Language | Rust | 2021 Edition |
| GUI Framework | eframe/egui | 0.33 |
| Table Component | egui_extras | 0.33 |
| File Dialog | rfd | 0.15 |
| CSV Writing | csv | 1.4 |
| Serialization | serde | 1.0 |
| CLI Parsing | clap | 4.5 |
| Image Processing | image | 0.25 |
| PDF Rendering | pdfium-render | 0.8 |
| HTTP Client | ureq | 2.9 |
| ZIP Extraction | zip | 0.6 |
| TGZ Extraction | flate2 + tar | 1.0 / 0.4 |
| File Opening | open | 5.0 |
| User Directories | dirs | 5.0 |

### Data Structures

```rust
struct FileInfo {
    name: String,              // File name without extension
    extension: String,         // File extension
    full_name: String,         // Complete file name
    relative_path: String,     // Path relative to selected folder
    absolute_path: String,     // Full absolute path
    file_size: u64,            // Size in bytes
    modified_timestamp: i64,   // Unix timestamp of last modification
}
```

### Module Structure

```
src/
├── main.rs           # Entry point, CLI parsing
├── app.rs            # GUI application logic
├── file_scanner.rs   # File system operations
├── csv_export.rs     # CSV writing
└── lib.rs            # Module declarations
```

## User Interface Layout

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ [Select Folder...] Selected: C:\path\to\folder                              │
│ ☐ Include subfolders (recursive)  [Scanning spinner...]                     │
│ Scanned: 150 files found                                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│ Filter: [___________] [Clear]  ☐ Show duplicates only  ☐ Show today only   │
│                                        [Move Selected (3)] [Delete Selected (3)] │
├─────────────────────────────────────────────────────────────────────────────┤
│ ☐  │     │ [Name^] │ [Ext] │ [Size]  │ [Date]      │ [Path]    │ Full Path │
├────┼─────┼─────────┼───────┼─────────┼─────────────┼───────────┼───────────┤
│ ☐  │ 📕  │ doc1    │ pdf   │ 1.2 MB  │ 2024-01-15  │ doc1.pdf  │ C:\...\   │
│ ☑  │ 🖼⚠ │ image   │ png   │ 500 KB  │ 2024-01-15  │ image.png │ C:\...\   │
│ ☑  │ 🖼⚠ │ image   │ png   │ 300 KB  │ 2024-01-14  │ sub\...   │ C:\...\   │
│ ☑  │ 🎬  │ video   │ mp4   │ 50 MB   │ 2024-01-15  │ video.mp4 │ C:\...\   │
│ ...│ ... │ ...     │ ...   │ ...     │ ...         │ ...       │ ...       │
├─────────────────────────────────────────────────────────────────────────────┤
│ [Export to CSV...] [Download Pdfium] [Download FFmpeg] | Showing 150 of 150 │
└─────────────────────────────────────────────────────────────────────────────┘

Icon Column Legend:
- First icon: File type (📕 PDF, 🖼 Image, 🎬 Video, 💻 Code, etc.)
- ⚠ Orange warning: Duplicate file name detected

Hover Preview:
- Hover over image files (🖼) to see thumbnail preview
- Hover over video files (🎬) to see thumbnail preview (requires FFmpeg)
- Hover over PDF files (📕) to see first page preview (requires Pdfium)
```

## Future Enhancements (Out of Scope)

- [ ] Search within file contents
- [ ] File preview panel (full preview, not just hover)
- [ ] Multiple folder selection
- [ ] Export to JSON/Excel formats
- [ ] Drag and drop folder
- [ ] Remember last folder location
- [ ] Keyboard shortcuts
- [ ] Audio file preview/playback
- [x] ~~File type icons~~ (Implemented in FR-09)
- [x] ~~Duplicate file detection~~ (Implemented in FR-10)
- [x] ~~Date modified column~~ (Implemented in FR-03, FR-04)
- [x] ~~Image hover preview~~ (Implemented in FR-16)
- [x] ~~Video hover preview~~ (Implemented in FR-17)
- [x] ~~PDF hover preview~~ (Implemented in FR-18)
