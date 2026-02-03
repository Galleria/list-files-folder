# File Lister

A Rust desktop application to list files from a folder and export to CSV.

## Features

- **GUI Mode**: Native window with folder picker, file table, and export
- **CLI Mode**: Command-line interface for scripting
- **Recursive Scanning**: Option to include subfolders
- **Sortable Columns**: Click headers to sort by Name, Extension, Size, or Path
- **Filter**: Real-time text filtering
- **CSV Export**: UTF-8 compatible export for Excel
- **Context Menu**: Right-click to open file location in Explorer
- **Unicode Support**: Thai, Chinese, Japanese, and other languages

## Screenshots

```
┌─────────────────────────────────────────────────────┐
│ File Lister                                         │
│ [Select Folder...] Selected: C:\Documents           │
│ ☐ Include subfolders (recursive)                    │
├─────────────────────────────────────────────────────┤
│ Filter: [pdf_______________] [Clear]                │
├─────────────────────────────────────────────────────┤
│ [Name^] │ [Ext] │ [Size]  │ [Path]     │ Full Path │
│ report  │ pdf   │ 1.2 MB  │ report.pdf │ C:\...    │
│ invoice │ pdf   │ 500 KB  │ invoice.pdf│ C:\...    │
├─────────────────────────────────────────────────────┤
│ [Export to CSV...]  |  Showing 2 of 150 files       │
└─────────────────────────────────────────────────────┘
```

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (1.70 or later)

### Build from Source

```bash
# Clone the repository
git clone https://github.com/yourusername/list-file-in-folders.git
cd list-file-in-folders

# Build release version
cargo build --release

# The executable will be at:
# Windows: target/release/list-file-in-folders.exe
# Linux/macOS: target/release/list-file-in-folders
```

## Usage

### GUI Mode

Simply run the application without arguments:

```bash
# Development
cargo run

# Or run the built executable
./target/release/list-file-in-folders
```

**How to use:**
1. Click **"Select Folder..."** to choose a folder
2. Check **"Include subfolders"** for recursive scanning
3. Use the **Filter** box to search files
4. Click column headers to **sort**
5. **Right-click** any row to open file location
6. Click **"Export to CSV..."** to save the list

### CLI Mode

Use command-line arguments for scripting:

```bash
# Basic usage
cargo run -- --folder "C:\Documents" --output "files.csv"

# With recursive scanning
cargo run -- -f "C:\Documents" -o "files.csv" -r

# Show help
cargo run -- --help
```

**CLI Arguments:**

| Argument | Short | Description | Default |
|----------|-------|-------------|---------|
| `--folder` | `-f` | Folder path to scan | *(launches GUI)* |
| `--output` | `-o` | Output CSV file path | `files.csv` |
| `--recursive` | `-r` | Include subfolders | `false` |

## CSV Output Format

The exported CSV includes:

| Column | Description |
|--------|-------------|
| File Name | Name without extension |
| Extension | File extension |
| Size (bytes) | File size in bytes |
| Relative Path | Path from selected folder |
| Full Path | Absolute file path |

**Example output:**
```csv
File Name,Extension,Size (bytes),Relative Path,Full Path
report,pdf,1258000,report.pdf,C:\Documents\report.pdf
image,png,524288,images\image.png,C:\Documents\images\image.png
```

## Project Structure

```
list-file-in-folders/
├── Cargo.toml          # Dependencies and project config
├── README.md           # This file
├── SPEC.md             # Specification document
└── src/
    ├── main.rs         # Entry point, CLI parsing
    ├── app.rs          # GUI application
    ├── file_scanner.rs # File scanning logic
    ├── csv_export.rs   # CSV export
    └── lib.rs          # Module declarations
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| [eframe](https://crates.io/crates/eframe) | GUI framework (egui) |
| [egui_extras](https://crates.io/crates/egui_extras) | Table component |
| [rfd](https://crates.io/crates/rfd) | Native file dialogs |
| [csv](https://crates.io/crates/csv) | CSV writing |
| [serde](https://crates.io/crates/serde) | Serialization |
| [clap](https://crates.io/crates/clap) | CLI argument parsing |

## Development

```bash
# Run in development mode
cargo run

# Run with logging
RUST_LOG=debug cargo run

# Build optimized release
cargo build --release

# Run tests
cargo test

# Check for issues
cargo clippy
```

## License

MIT License

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request




