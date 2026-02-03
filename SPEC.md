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
- **FR-02.3**: Extract file information:
  - File name (without extension)
  - File extension
  - Full file name (with extension)
  - Relative path (from selected folder)
  - Absolute/full path
  - File size in bytes

### FR-03: File Display (GUI)
- **FR-03.1**: Display files in a table with columns: Icons, Name, Extension, Size, Path, Full Path
- **FR-03.2**: Table columns are resizable by dragging (except Icons column)
- **FR-03.3**: Table auto-resizes with window
- **FR-03.4**: Striped rows for readability

### FR-04: Sorting
- **FR-04.1**: Sort by Name (ascending/descending)
- **FR-04.2**: Sort by Extension (ascending/descending)
- **FR-04.3**: Sort by Size (ascending/descending)
- **FR-04.4**: Sort by Path (ascending/descending)
- **FR-04.5**: Click column header to toggle sort order
- **FR-04.6**: Display sort indicator (^ or v) on active column

### FR-05: Filtering
- **FR-05.1**: Text input to filter files
- **FR-05.2**: Filter matches against: name, extension, relative path, full name
- **FR-05.3**: Case-insensitive filtering
- **FR-05.4**: Real-time filtering as user types
- **FR-05.5**: Clear button to reset filter
- **FR-05.6**: Show count: "Showing X of Y files"

### FR-06: Context Menu
- **FR-06.1**: Right-click on any cell shows context menu
- **FR-06.2**: "Open file location" option opens native file manager:
  - Windows: Explorer with file selected
  - macOS: Finder with file selected
  - Linux: Default file manager (parent folder)
- **FR-06.3**: "Rename" option to rename the file (inline editing)
- **FR-06.4**: "Delete" option to delete the file from disk

### FR-12: File Rename
- **FR-12.1**: Double-click on Name column to enter inline edit mode
- **FR-12.2**: Press Enter or click outside to confirm rename
- **FR-12.3**: Press Escape to cancel rename
- **FR-12.4**: Also available via right-click context menu

### FR-13: File Delete
- **FR-13.1**: Delete file via right-click context menu
- **FR-13.2**: File is permanently deleted from disk
- **FR-13.3**: List automatically refreshes after deletion

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

## Non-Functional Requirements

### NFR-01: Unicode Support
- Support Thai, Chinese, Japanese, and other Unicode characters in file names
- Load system fonts (Segoe UI, Arial, Tahoma, Microsoft YaHei) for Unicode rendering

### NFR-02: Performance
- Handle folders with thousands of files
- Virtual scrolling for large file lists

### NFR-03: User Interface
- Minimum window size: 600x400 pixels
- Default window size: 1000x600 pixels
- Fixed header and footer panels
- Scrollable table area

### NFR-04: Platform
- Cross-platform support: Windows, macOS, Linux
- Platform-specific font loading for Unicode support
- Platform-specific file manager integration:
  - Windows: `explorer /select,`
  - macOS: `open -R`
  - Linux: `xdg-open` (parent folder)

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

### Data Structures

```rust
struct FileInfo {
    name: String,           // File name without extension
    extension: String,      // File extension
    full_name: String,      // Complete file name
    relative_path: String,  // Path relative to selected folder
    absolute_path: String,  // Full absolute path
    file_size: u64,         // Size in bytes
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
┌───────────────────────────────────────────────────────────┐
│ File Lister                                               │
│ [Select Folder...] Selected: C:\path\to\folder            │
│ ☐ Include subfolders (recursive)                          │
│ Scanned: 150 files found                                  │
├───────────────────────────────────────────────────────────┤
│ Filter: [_______________] [Clear]  ☐ Show duplicates only │
├───────────────────────────────────────────────────────────┤
│     │ [Name^] │ [Ext] │ [Size] │ [Path]    │ Full Path   │
├─────┼─────────┼───────┼────────┼───────────┼─────────────┤
│ 📕  │ doc1    │ pdf   │ 1.2 MB │ doc1.pdf  │ C:\...\     │
│ 🖼⚠ │ image   │ png   │ 500 KB │ image.png │ C:\...\     │
│ 🖼⚠ │ image   │ png   │ 300 KB │ sub\...   │ C:\...\     │
│ ...  │ ...     │ ...   │ ...    │ ...       │ ...         │
├───────────────────────────────────────────────────────────┤
│ [Export to CSV...]  |  Showing 150 of 150 files           │
└───────────────────────────────────────────────────────────┘

Icon Column Legend:
- First icon: File type (📕 PDF, 🖼 Image, 💻 Code, etc.)
- ⚠ Orange warning: Duplicate file name detected
```

## Future Enhancements (Out of Scope)

- [ ] Search within file contents
- [ ] File preview panel
- [ ] Multiple folder selection
- [ ] Export to JSON/Excel formats
- [ ] Date modified column
- [x] ~~File type icons~~ (Implemented in FR-09)
- [x] ~~Duplicate file detection~~ (Implemented in FR-10)
- [ ] Drag and drop folder
- [ ] Remember last folder location
- [ ] Keyboard shortcuts
