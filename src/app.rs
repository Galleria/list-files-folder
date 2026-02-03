use crate::csv_export;
use crate::file_scanner::{self, format_date, format_size, is_today, FileInfo};
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use pdfium_render::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Once;
use std::thread;
use std::time::{Duration, Instant};

/// Global FFmpeg availability (checked once at startup)
static FFMPEG_CHECKED: Once = Once::new();
static FFMPEG_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Global Pdfium availability
static PDFIUM_CHECKED: Once = Once::new();
static PDFIUM_AVAILABLE: AtomicBool = AtomicBool::new(false);
static PDFIUM_DOWNLOADING: AtomicBool = AtomicBool::new(false);

/// Data for a loaded image preview
struct ImagePreviewData {
    pixels: Vec<u8>,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SortColumn {
    Name,
    Extension,
    Size,
    Path,
    Date,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

pub struct FileListerApp {
    selected_folder: Option<PathBuf>,
    files: Vec<FileInfo>,
    filtered_files: Vec<FileInfo>,
    status_message: String,
    error_message: Option<String>,
    recursive: bool,
    sort_column: SortColumn,
    sort_order: SortOrder,
    filter_text: String,
    /// Map of full_name -> count for detecting duplicates
    duplicate_counts: HashMap<String, usize>,
    /// Show only duplicate files
    show_duplicates_only: bool,
    /// Show only files modified today
    show_today_only: bool,
    /// Index of file being renamed (in filtered_files)
    editing_index: Option<usize>,
    /// Text buffer for renaming
    editing_text: String,
    /// Track if we need to request focus for the rename input
    request_rename_focus: bool,
    /// Set of selected file indices (for bulk operations)
    selected_files: HashSet<usize>,
    /// Show bulk delete confirmation modal
    show_delete_confirm: bool,
    /// File paths pending deletion (for confirmation modal)
    pending_delete_paths: Vec<(String, String)>, // (absolute_path, full_name)
    /// Receiver for background scan results
    scan_receiver: Option<Receiver<Result<Vec<FileInfo>, String>>>,
    /// Flag indicating scanning is in progress
    is_scanning: bool,
    /// Cache of loaded image textures (absolute_path -> texture)
    image_cache: HashMap<String, egui::TextureHandle>,
    /// Receiver for background image loading
    image_receiver: Option<Receiver<(String, ImagePreviewData)>>,
    /// Path currently being loaded in background
    image_loading_path: Option<String>,
    /// When the current image/video loading started (for timeout)
    image_loading_start: Option<Instant>,
}

impl Default for FileListerApp {
    fn default() -> Self {
        Self {
            selected_folder: None,
            files: Vec::new(),
            filtered_files: Vec::new(),
            status_message: String::from("Select a folder to scan"),
            error_message: None,
            recursive: false,
            sort_column: SortColumn::Name,
            sort_order: SortOrder::Ascending,
            filter_text: String::new(),
            duplicate_counts: HashMap::new(),
            show_duplicates_only: false,
            show_today_only: false,
            editing_index: None,
            editing_text: String::new(),
            request_rename_focus: false,
            selected_files: HashSet::new(),
            show_delete_confirm: false,
            pending_delete_paths: Vec::new(),
            scan_receiver: None,
            is_scanning: false,
            image_cache: HashMap::new(),
            image_receiver: None,
            image_loading_path: None,
            image_loading_start: None,
        }
    }
}

impl FileListerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load fonts with Thai/Unicode support
        let mut fonts = egui::FontDefinitions::default();

        // Platform-specific font paths for better Unicode coverage
        #[cfg(target_os = "windows")]
        let font_paths: &[&str] = &[
            "C:\\Windows\\Fonts\\segoeui.ttf",   // Segoe UI - good Unicode support
            "C:\\Windows\\Fonts\\arial.ttf",     // Arial
            "C:\\Windows\\Fonts\\tahoma.ttf",    // Tahoma
            "C:\\Windows\\Fonts\\msyh.ttc",      // Microsoft YaHei - CJK support
            "C:\\Windows\\Fonts\\msjh.ttc",      // Microsoft JhengHei
        ];

        #[cfg(target_os = "macos")]
        let font_paths: &[&str] = &[
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",  // Arial Unicode
            "/System/Library/Fonts/Helvetica.ttc",                   // Helvetica
            "/System/Library/Fonts/STHeiti Light.ttc",               // Chinese support
            "/System/Library/Fonts/Hiragino Sans GB.ttc",            // CJK support
            "/Library/Fonts/Arial Unicode.ttf",                      // User Arial Unicode
        ];

        #[cfg(target_os = "linux")]
        let font_paths: &[&str] = &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        ];

        for (i, font_path) in font_paths.iter().enumerate() {
            if let Ok(font_data) = std::fs::read(font_path) {
                let font_name = format!("unicode_font_{}", i);
                fonts.font_data.insert(
                    font_name.clone(),
                    std::sync::Arc::new(egui::FontData::from_owned(font_data)),
                );

                // Add as fallback for proportional text
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push(font_name.clone());

                // Add as fallback for monospace text
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push(font_name);
            }
        }

        cc.egui_ctx.set_fonts(fonts);

        // Check if FFmpeg is available (for video thumbnails)
        Self::check_ffmpeg_availability();

        // Check if Pdfium is available (for PDF previews)
        Self::check_pdfium_availability();

        Self::default()
    }

    fn scan_selected_folder(&mut self) {
        self.error_message = None;
        self.selected_files.clear(); // Clear selections on rescan
        self.image_cache.clear(); // Clear image cache on rescan

        if let Some(folder) = &self.selected_folder {
            let folder = folder.clone();
            let recursive = self.recursive;

            // Create channel for receiving results
            let (tx, rx) = mpsc::channel();
            self.scan_receiver = Some(rx);
            self.is_scanning = true;
            self.status_message = String::from("Scanning...");

            // Spawn background thread for scanning
            thread::spawn(move || {
                let result = file_scanner::scan_folder(&folder, recursive)
                    .map_err(|e| e.to_string());
                let _ = tx.send(result);
            });
        }
    }

    /// Check for scan results from background thread
    fn check_scan_results(&mut self) {
        if let Some(receiver) = &self.scan_receiver {
            // Try to receive without blocking
            if let Ok(result) = receiver.try_recv() {
                match result {
                    Ok(files) => {
                        self.status_message = format!("Scanned: {} files found", files.len());
                        self.files = files;
                        self.sort_files();
                        self.apply_filter();
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Error scanning folder: {}", e));
                        self.files.clear();
                        self.filtered_files.clear();
                    }
                }
                self.is_scanning = false;
                self.scan_receiver = None;
            }
        }
    }

    /// Check for completed background image loads
    fn check_image_loads(&mut self, ctx: &egui::Context) {
        // Check for timeout (10 seconds for video thumbnails)
        if let Some(start_time) = self.image_loading_start {
            if start_time.elapsed() > Duration::from_secs(10) {
                // Timeout - clear loading state
                self.image_loading_path = None;
                self.image_receiver = None;
                self.image_loading_start = None;
                return;
            }
        }

        if let Some(receiver) = &self.image_receiver {
            // Try to receive without blocking
            if let Ok((path, data)) = receiver.try_recv() {
                let size = [data.width, data.height];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &data.pixels);
                let texture = ctx.load_texture(
                    format!("preview_{}", path),
                    color_image,
                    egui::TextureOptions::default(),
                );

                // Store in cache
                self.image_cache.insert(path.clone(), texture);
                self.image_loading_path = None;
                self.image_receiver = None;
                self.image_loading_start = None;
                ctx.request_repaint();
            }
        }
    }

    /// Get elapsed loading time in seconds (for UI display)
    fn get_loading_elapsed_secs(&self) -> Option<u64> {
        self.image_loading_start.map(|start| start.elapsed().as_secs())
    }

    fn sort_files(&mut self) {
        let order = self.sort_order;
        match self.sort_column {
            SortColumn::Name => {
                self.files.sort_by(|a, b| {
                    let cmp = a.name.to_lowercase().cmp(&b.name.to_lowercase());
                    if order == SortOrder::Descending { cmp.reverse() } else { cmp }
                });
            }
            SortColumn::Extension => {
                self.files.sort_by(|a, b| {
                    let cmp = a.extension.to_lowercase().cmp(&b.extension.to_lowercase());
                    if order == SortOrder::Descending { cmp.reverse() } else { cmp }
                });
            }
            SortColumn::Size => {
                self.files.sort_by(|a, b| {
                    let cmp = a.file_size.cmp(&b.file_size);
                    if order == SortOrder::Descending { cmp.reverse() } else { cmp }
                });
            }
            SortColumn::Path => {
                self.files.sort_by(|a, b| {
                    let cmp = a.relative_path.to_lowercase().cmp(&b.relative_path.to_lowercase());
                    if order == SortOrder::Descending { cmp.reverse() } else { cmp }
                });
            }
            SortColumn::Date => {
                self.files.sort_by(|a, b| {
                    let cmp = a.modified_timestamp.cmp(&b.modified_timestamp);
                    if order == SortOrder::Descending { cmp.reverse() } else { cmp }
                });
            }
        }
        self.apply_filter();
    }

    fn apply_filter(&mut self) {
        // Clear selections when filter changes (indices would be invalid)
        self.selected_files.clear();

        // First compute duplicates on ALL files (before filtering)
        self.compute_duplicates();

        let filter = self.filter_text.to_lowercase();

        // Apply text filter
        let text_filtered: Vec<FileInfo> = if filter.is_empty() {
            self.files.clone()
        } else {
            self.files
                .iter()
                .filter(|f| {
                    f.name.to_lowercase().contains(&filter)
                        || f.extension.to_lowercase().contains(&filter)
                        || f.relative_path.to_lowercase().contains(&filter)
                        || f.full_name.to_lowercase().contains(&filter)
                })
                .cloned()
                .collect()
        };

        // Apply duplicates filter if enabled
        let after_duplicates: Vec<FileInfo> = if self.show_duplicates_only {
            text_filtered
                .into_iter()
                .filter(|f| self.is_duplicate(&f.full_name).is_some())
                .collect()
        } else {
            text_filtered
        };

        // Apply today filter if enabled
        if self.show_today_only {
            self.filtered_files = after_duplicates
                .into_iter()
                .filter(|f| is_today(f.modified_timestamp))
                .collect();
        } else {
            self.filtered_files = after_duplicates;
        }
    }

    fn compute_duplicates(&mut self) {
        self.duplicate_counts.clear();
        // Compute duplicates on ALL files, not just filtered
        for file in &self.files {
            *self.duplicate_counts.entry(file.full_name.clone()).or_insert(0) += 1;
        }
    }

    fn is_duplicate(&self, full_name: &str) -> Option<usize> {
        self.duplicate_counts.get(full_name).and_then(|&count| {
            if count > 1 { Some(count) } else { None }
        })
    }

    /// Get file type icon based on extension
    fn get_file_type_icon(extension: &str) -> &'static str {
        match extension.to_lowercase().as_str() {
            // Documents
            "txt" | "md" | "rtf" => "📝",
            "pdf" => "📕",
            "doc" | "docx" | "odt" => "📘",
            "xls" | "xlsx" | "ods" => "📗",
            "ppt" | "pptx" | "odp" => "📙",

            // Images
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "ico" | "svg" | "webp" | "tiff" | "tif" => "🖼",
            "psd" | "ai" | "sketch" => "🎨",

            // Audio
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" => "🎵",

            // Video
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" | "m4v" => "🎬",

            // Archives
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => "📦",

            // Code
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "h" | "hpp" => "💻",
            "java" | "kt" | "go" | "rb" | "php" | "swift" | "cs" | "vb" => "💻",
            "html" | "htm" | "css" | "scss" | "sass" | "less" => "🌐",
            "sh" | "bash" | "ps1" | "bat" | "cmd" => "⚡",

            // Data
            "json" | "xml" | "csv" | "sql" | "db" | "sqlite" => "📊",
            "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf" | "config" => "⚙",

            // Executables
            "exe" | "msi" | "dll" | "so" | "dylib" => "⚙",

            // Fonts
            "ttf" | "otf" | "woff" | "woff2" | "eot" => "🔤",

            // Default
            _ => "📄",
        }
    }

    fn toggle_sort(&mut self, column: SortColumn) {
        if self.sort_column == column {
            // Toggle order if same column
            self.sort_order = match self.sort_order {
                SortOrder::Ascending => SortOrder::Descending,
                SortOrder::Descending => SortOrder::Ascending,
            };
        } else {
            // New column, start with ascending
            self.sort_column = column;
            self.sort_order = SortOrder::Ascending;
        }
        self.sort_files();
    }

    fn get_sort_indicator(&self, column: SortColumn) -> &str {
        if self.sort_column == column {
            match self.sort_order {
                SortOrder::Ascending => " ^",
                SortOrder::Descending => " v",
            }
        } else {
            ""
        }
    }

    fn open_in_explorer(file_path: &str) {
        // Open file manager and select the file (cross-platform)
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("explorer")
                .args(["/select,", file_path])
                .spawn();
        }

        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("open")
                .args(["-R", file_path])
                .spawn();
        }

        #[cfg(target_os = "linux")]
        {
            // Try to open parent folder with default file manager
            if let Some(parent) = std::path::Path::new(file_path).parent() {
                let _ = Command::new("xdg-open")
                    .arg(parent)
                    .spawn();
            }
        }
    }

    fn export_csv(&mut self, path: &PathBuf) {
        // Export filtered files
        match csv_export::export_to_csv(&self.filtered_files, path) {
            Ok(_) => {
                self.status_message = format!("Exported {} files to: {}", self.filtered_files.len(), path.display());
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Export failed: {}", e));
            }
        }
    }

    fn delete_file(&mut self, file_path: &str) {
        let path = std::path::Path::new(file_path);
        match std::fs::remove_file(path) {
            Ok(_) => {
                self.status_message = format!("Deleted: {}", path.file_name().unwrap_or_default().to_string_lossy());
                self.error_message = None;
                // Re-scan to update the list
                self.scan_selected_folder();
            }
            Err(e) => {
                self.error_message = Some(format!("Delete failed: {}", e));
            }
        }
    }

    fn move_file(&mut self, file_path: &str) {
        let source = std::path::Path::new(file_path);
        if let Some(file_name) = source.file_name() {
            if let Some(dest_folder) = rfd::FileDialog::new()
                .set_title("Select destination folder")
                .pick_folder()
            {
                let dest_path = dest_folder.join(file_name);
                match std::fs::rename(source, &dest_path) {
                    Ok(_) => {
                        self.status_message = format!("Moved: {} → {}", file_name.to_string_lossy(), dest_folder.display());
                        self.error_message = None;
                        self.scan_selected_folder();
                    }
                    Err(_) => {
                        // If rename fails (cross-device), try copy + delete
                        if let Err(copy_err) = std::fs::copy(source, &dest_path) {
                            self.error_message = Some(format!("Move failed: {}", copy_err));
                        } else if let Err(del_err) = std::fs::remove_file(source) {
                            self.error_message = Some(format!("Move partial: copied but failed to delete source: {}", del_err));
                            self.scan_selected_folder();
                        } else {
                            self.status_message = format!("Moved: {} → {}", file_name.to_string_lossy(), dest_folder.display());
                            self.error_message = None;
                            self.scan_selected_folder();
                        }
                    }
                }
            }
        }
    }

    fn move_selected_files(&mut self) {
        if self.selected_files.is_empty() {
            return;
        }

        if let Some(dest_folder) = rfd::FileDialog::new()
            .set_title("Select destination folder")
            .pick_folder()
        {
            let mut moved_count = 0;
            let mut failed_count = 0;
            let mut errors: Vec<String> = Vec::new();

            let files_to_move: Vec<(String, String)> = self.selected_files
                .iter()
                .filter_map(|&idx| {
                    self.filtered_files.get(idx).map(|f| {
                        (f.absolute_path.clone(), f.full_name.clone())
                    })
                })
                .collect();

            for (source_path, file_name) in files_to_move {
                let source = std::path::Path::new(&source_path);
                let dest_path = dest_folder.join(&file_name);

                let move_result = std::fs::rename(source, &dest_path)
                    .or_else(|_| {
                        // Try copy + delete for cross-device moves
                        std::fs::copy(source, &dest_path)?;
                        std::fs::remove_file(source)
                    });

                match move_result {
                    Ok(_) => moved_count += 1,
                    Err(e) => {
                        failed_count += 1;
                        errors.push(format!("{}: {}", file_name, e));
                    }
                }
            }

            if failed_count == 0 {
                self.status_message = format!("Moved {} files to {}", moved_count, dest_folder.display());
                self.error_message = None;
            } else {
                self.status_message = format!("Moved {} files, {} failed", moved_count, failed_count);
                self.error_message = Some(errors.join("; "));
            }

            self.selected_files.clear();
            self.scan_selected_folder();
        }
    }

    fn rename_file(&mut self, old_path: &str, new_name: &str) {
        let old = std::path::Path::new(old_path);
        if let Some(parent) = old.parent() {
            let new_path = parent.join(new_name);
            match std::fs::rename(old, &new_path) {
                Ok(_) => {
                    self.status_message = format!("Renamed to: {}", new_name);
                    self.error_message = None;
                    // Re-scan to update the list
                    self.scan_selected_folder();
                }
                Err(e) => {
                    self.error_message = Some(format!("Rename failed: {}", e));
                }
            }
        }
    }

    fn start_rename(&mut self, idx: usize) {
        if idx < self.filtered_files.len() {
            self.editing_index = Some(idx);
            self.editing_text = self.filtered_files[idx].full_name.clone();
            self.request_rename_focus = true;
        }
    }

    fn cancel_rename(&mut self) {
        self.editing_index = None;
        self.editing_text.clear();
        self.request_rename_focus = false;
    }

    fn confirm_rename(&mut self) {
        if let Some(idx) = self.editing_index {
            if idx < self.filtered_files.len() {
                let old_path = self.filtered_files[idx].absolute_path.clone();
                let new_name = self.editing_text.trim().to_string();
                if !new_name.is_empty() && new_name != self.filtered_files[idx].full_name {
                    self.rename_file(&old_path, &new_name);
                }
            }
        }
        self.cancel_rename();
    }

    fn toggle_selection(&mut self, idx: usize) {
        if self.selected_files.contains(&idx) {
            self.selected_files.remove(&idx);
        } else {
            self.selected_files.insert(idx);
        }
    }

    fn select_all(&mut self) {
        for idx in 0..self.filtered_files.len() {
            self.selected_files.insert(idx);
        }
    }

    fn deselect_all(&mut self) {
        self.selected_files.clear();
    }

    fn prepare_bulk_delete(&mut self) {
        // Collect paths of selected files for confirmation
        self.pending_delete_paths = self.selected_files
            .iter()
            .filter_map(|&idx| {
                self.filtered_files.get(idx).map(|f| {
                    (f.absolute_path.clone(), f.full_name.clone())
                })
            })
            .collect();

        if !self.pending_delete_paths.is_empty() {
            self.show_delete_confirm = true;
        }
    }

    fn execute_bulk_delete(&mut self) {
        let mut deleted_count = 0;
        let mut failed_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for (path, name) in &self.pending_delete_paths {
            match std::fs::remove_file(path) {
                Ok(_) => deleted_count += 1,
                Err(e) => {
                    failed_count += 1;
                    errors.push(format!("{}: {}", name, e));
                }
            }
        }

        // Update status message
        if failed_count == 0 {
            self.status_message = format!("Deleted {} files", deleted_count);
            self.error_message = None;
        } else {
            self.status_message = format!("Deleted {} files, {} failed", deleted_count, failed_count);
            self.error_message = Some(errors.join("; "));
        }

        // Clean up and rescan
        self.pending_delete_paths.clear();
        self.show_delete_confirm = false;
        self.selected_files.clear();
        self.scan_selected_folder();
    }

    fn cancel_bulk_delete(&mut self) {
        self.pending_delete_paths.clear();
        self.show_delete_confirm = false;
    }

    /// Check if file extension is an image type
    fn is_image_file(extension: &str) -> bool {
        let image_extensions = ["jpg", "jpeg", "png", "gif", "bmp", "ico", "webp"];
        image_extensions.contains(&extension.to_lowercase().as_str())
    }

    /// Check if file extension is a video type
    fn is_video_file(extension: &str) -> bool {
        let video_extensions = ["mp4", "avi", "mkv", "mov", "wmv", "flv", "webm", "m4v", "mpeg", "mpg", "3gp"];
        video_extensions.contains(&extension.to_lowercase().as_str())
    }

    /// Check if file extension is a PDF
    fn is_pdf_file(extension: &str) -> bool {
        extension.to_lowercase() == "pdf"
    }

    /// Check if file is previewable (image, video, or PDF)
    fn is_previewable(extension: &str) -> bool {
        Self::is_image_file(extension) || Self::is_video_file(extension) || Self::is_pdf_file(extension)
    }

    /// Load hover preview for image/video file in background
    fn load_hover_preview(&mut self, idx: usize, ctx: &egui::Context) {
        if idx >= self.filtered_files.len() {
            return;
        }

        let file = &self.filtered_files[idx];

        // Only load preview for previewable files (images and videos)
        if !Self::is_previewable(&file.extension) {
            return;
        }

        let abs_path = file.absolute_path.clone();
        let extension = file.extension.clone();

        // Already in cache - nothing to do
        if self.image_cache.contains_key(&abs_path) {
            return;
        }

        // Don't start new load if we're already loading this file
        if self.image_loading_path.as_ref() == Some(&abs_path) {
            return;
        }

        let is_video = Self::is_video_file(&extension);
        let is_pdf = Self::is_pdf_file(&extension);

        // Don't try to load video thumbnails if FFmpeg isn't ready
        if is_video && !Self::is_ffmpeg_ready() {
            Self::debug_log("[DEBUG] load_hover_preview: Skipping video (FFmpeg not ready)");
            return;
        }

        // Don't try to load PDF thumbnails if Pdfium isn't ready
        if is_pdf && !Self::is_pdfium_ready() {
            Self::debug_log("[DEBUG] load_hover_preview: Skipping PDF (Pdfium not ready)");
            return;
        }

        // Start background loading
        let (tx, rx) = mpsc::channel();
        self.image_receiver = Some(rx);
        self.image_loading_path = Some(abs_path.clone());
        self.image_loading_start = Some(Instant::now());

        Self::debug_log(&format!("[DEBUG] load_hover_preview: is_video={}, is_pdf={}, path={}", is_video, is_pdf, abs_path));

        // Spawn background thread to load and resize image/video/PDF thumbnail
        thread::spawn(move || {
            Self::debug_log(&format!("[DEBUG] Thread started for: {}", abs_path));
            let image_data = if is_video {
                // Extract thumbnail from video using FFmpeg
                Self::debug_log("[DEBUG] Calling extract_video_thumbnail...");
                Self::extract_video_thumbnail(&abs_path)
            } else if is_pdf {
                // Extract first page from PDF
                Self::debug_log("[DEBUG] Calling extract_pdf_thumbnail...");
                Self::extract_pdf_thumbnail(&abs_path)
            } else {
                // Load image directly
                std::fs::read(&abs_path).ok()
            };
            Self::debug_log(&format!("[DEBUG] image_data result: {:?}", image_data.as_ref().map(|d| d.len())));

            if let Some(data) = image_data {
                if let Ok(image) = image::load_from_memory(&data) {
                    // Resize large images for preview (max 400x400)
                    let max_size = 400u32;
                    let (width, height) = if image.width() > max_size || image.height() > max_size {
                        let aspect = image.width() as f32 / image.height() as f32;
                        if aspect > 1.0 {
                            (max_size, (max_size as f32 / aspect) as u32)
                        } else {
                            ((max_size as f32 * aspect) as u32, max_size)
                        }
                    } else {
                        (image.width(), image.height())
                    };

                    let resized = image.resize(width, height, image::imageops::FilterType::Triangle);
                    let image_buffer = resized.to_rgba8();
                    let pixels = image_buffer.into_raw();

                    let preview_data = ImagePreviewData {
                        pixels,
                        width: resized.width() as usize,
                        height: resized.height() as usize,
                    };

                    let _ = tx.send((abs_path, preview_data));
                }
            }
        });

        ctx.request_repaint();
    }

    /// Check for FFmpeg at startup (only runs once)
    fn check_ffmpeg_availability() {
        FFMPEG_CHECKED.call_once(|| {
            // Check if FFmpeg exists in system PATH
            if let Ok(output) = Command::new("where").arg("ffmpeg").output() {
                if output.status.success() {
                    let path_str = String::from_utf8_lossy(&output.stdout);
                    if path_str.lines().next().is_some() {
                        Self::debug_log("[DEBUG] FFmpeg found in system PATH");
                        FFMPEG_AVAILABLE.store(true, Ordering::SeqCst);
                        return;
                    }
                }
            }
            Self::debug_log("[DEBUG] FFmpeg not found - video thumbnails disabled");
            Self::debug_log("[DEBUG] Install FFmpeg with: winget install ffmpeg");
        });
    }

    /// Check if FFmpeg is available
    fn is_ffmpeg_ready() -> bool {
        FFMPEG_AVAILABLE.load(Ordering::SeqCst)
    }

    /// Check if FFmpeg is currently downloading (no longer used, kept for compatibility)
    fn is_ffmpeg_downloading() -> bool {
        false
    }

    /// Get the path where Pdfium library should be stored
    fn get_pdfium_path() -> PathBuf {
        // Store in user's app data directory
        let base = dirs::data_local_dir()
            .unwrap_or_else(|| std::env::temp_dir());
        base.join("file-lister").join("pdfium")
    }

    /// Check for Pdfium at startup (only runs once), download if needed
    fn check_pdfium_availability() {
        PDFIUM_CHECKED.call_once(|| {
            // Try to bind to system Pdfium first
            if Pdfium::bind_to_system_library().is_ok() {
                Self::debug_log("[DEBUG] Pdfium library found in system");
                PDFIUM_AVAILABLE.store(true, Ordering::SeqCst);
                return;
            }

            // Try to bind to downloaded Pdfium
            let pdfium_dir = Self::get_pdfium_path();
            if let Ok(bindings) = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&pdfium_dir)) {
                Self::debug_log(&format!("[DEBUG] Pdfium library found at {:?}", pdfium_dir));
                PDFIUM_AVAILABLE.store(true, Ordering::SeqCst);
                return;
            }

            Self::debug_log("[DEBUG] Pdfium not found - starting background download...");

            // Start background download
            thread::spawn(|| {
                Self::download_pdfium();
            });
        });
    }

    /// Download Pdfium library in background
    fn download_pdfium() {
        use std::io::{Read, Write};

        PDFIUM_DOWNLOADING.store(true, Ordering::SeqCst);
        let pdfium_dir = Self::get_pdfium_path();

        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(&pdfium_dir) {
            Self::debug_log(&format!("[ERROR] Failed to create Pdfium directory: {}", e));
            PDFIUM_DOWNLOADING.store(false, Ordering::SeqCst);
            return;
        }

        Self::debug_log(&format!("[DEBUG] Downloading Pdfium to {:?}...", pdfium_dir));

        // Download URL for Pdfium - using bblanchon/pdfium-binaries (verified working)
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        let download_url = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/7665/pdfium-win-x64.tgz";
        #[cfg(all(target_os = "windows", target_arch = "x86"))]
        let download_url = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/7665/pdfium-win-x86.tgz";
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        let download_url = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/7665/pdfium-mac-x64.tgz";
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        let download_url = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/7665/pdfium-mac-arm64.tgz";
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        let download_url = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/7665/pdfium-linux-x64.tgz";

        match Self::download_and_extract_pdfium(download_url, &pdfium_dir) {
            Ok(_) => {
                Self::debug_log("[DEBUG] Pdfium download completed");
                // Try to bind to verify it works
                if Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&pdfium_dir)).is_ok() {
                    PDFIUM_AVAILABLE.store(true, Ordering::SeqCst);
                    Self::debug_log("[DEBUG] Pdfium is now ready");
                } else {
                    Self::debug_log("[ERROR] Failed to bind to downloaded Pdfium");
                }
            }
            Err(e) => {
                Self::debug_log(&format!("[ERROR] Failed to download Pdfium: {}", e));
            }
        }
        PDFIUM_DOWNLOADING.store(false, Ordering::SeqCst);
    }

    /// Download and extract Pdfium from URL
    fn download_and_extract_pdfium(url: &str, dest_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use flate2::read::GzDecoder;
        use std::io::Read;
        use tar::Archive;

        Self::debug_log(&format!("[DEBUG] Downloading from {}", url));

        // Download the .tgz file
        let response = ureq::get(url).call()?;
        let mut bytes = Vec::new();
        response.into_reader().read_to_end(&mut bytes)?;

        Self::debug_log(&format!("[DEBUG] Downloaded {} bytes", bytes.len()));

        // Library name based on platform
        #[cfg(target_os = "windows")]
        let lib_name = "pdfium.dll";
        #[cfg(target_os = "macos")]
        let lib_name = "libpdfium.dylib";
        #[cfg(target_os = "linux")]
        let lib_name = "libpdfium.so";

        // Extract the .tgz file
        let cursor = std::io::Cursor::new(bytes);
        let gz = GzDecoder::new(cursor);
        let mut archive = Archive::new(gz);

        let mut found_lib = false;

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            let path_str = path.to_string_lossy().to_string();

            // Extract the main library file directly to dest_dir
            if path_str.ends_with(lib_name) {
                let outpath = dest_dir.join(lib_name);
                Self::debug_log(&format!("[DEBUG] Extracting {} to {:?}", path_str, outpath));
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut entry, &mut outfile)?;
                found_lib = true;
                break;
            }
        }

        if !found_lib {
            return Err(format!("Could not find {} in archive", lib_name).into());
        }

        Self::debug_log("[DEBUG] Extraction completed");
        Ok(())
    }

    /// Check if Pdfium is available for PDF rendering
    fn is_pdfium_ready() -> bool {
        PDFIUM_AVAILABLE.load(Ordering::SeqCst)
    }

    /// Check if Pdfium is currently downloading
    fn is_pdfium_downloading() -> bool {
        PDFIUM_DOWNLOADING.load(Ordering::SeqCst)
    }

    /// Extract first page from PDF as image
    fn extract_pdf_thumbnail(pdf_path: &str) -> Option<Vec<u8>> {
        if !Self::is_pdfium_ready() {
            Self::debug_log("[DEBUG] extract_pdf_thumbnail: Pdfium not ready");
            return None;
        }

        Self::debug_log(&format!("[DEBUG] Extracting PDF thumbnail: {}", pdf_path));

        // Try system library first, then downloaded library
        let bindings = Pdfium::bind_to_system_library()
            .or_else(|_| {
                let pdfium_dir = Self::get_pdfium_path();
                Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&pdfium_dir))
            })
            .ok()?;
        let pdfium = Pdfium::new(bindings);
        let document = pdfium.load_pdf_from_file(pdf_path, None).ok()?;

        if document.pages().len() == 0 {
            Self::debug_log("[DEBUG] PDF has no pages");
            return None;
        }

        let page = document.pages().get(0).ok()?;

        // Render at reasonable size for preview (max 400px width)
        let page_width: f32 = page.width().value;
        let page_height: f32 = page.height().value;
        let scale: f32 = (400.0_f32 / page_width).min(1.0);
        let width = (page_width * scale) as i32;
        let height = (page_height * scale) as i32;

        let bitmap = page
            .render_with_config(
                &PdfRenderConfig::new()
                    .set_target_width(width)
                    .set_target_height(height)
            )
            .ok()?;

        // Convert to PNG bytes
        let image = bitmap.as_image();
        let mut png_bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        image.write_to(&mut cursor, image::ImageFormat::Png).ok()?;

        Self::debug_log(&format!("[DEBUG] PDF thumbnail extracted: {} bytes", png_bytes.len()));
        Some(png_bytes)
    }

    /// Write debug log to file (for debugging on Windows GUI)
    fn debug_log(msg: &str) {
        use std::io::Write;
        let log_path = std::env::temp_dir().join("file_lister_debug.log");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(file, "{}", msg);
        }
    }

    /// Find FFmpeg executable in system PATH
    fn find_ffmpeg() -> Option<PathBuf> {
        if let Ok(output) = Command::new("where").arg("ffmpeg").output() {
            if output.status.success() {
                let path_str = String::from_utf8_lossy(&output.stdout);
                if let Some(first_line) = path_str.lines().next() {
                    let path = PathBuf::from(first_line.trim());
                    if path.exists() {
                        return Some(path);
                    }
                }
            }
        }
        None
    }

    /// Extract a thumbnail frame from a video file using FFmpeg (auto-downloads if needed)
    fn extract_video_thumbnail(video_path: &str) -> Option<Vec<u8>> {
        // Check if FFmpeg is ready
        if !Self::is_ffmpeg_ready() {
            Self::debug_log("[DEBUG] extract_video_thumbnail: FFmpeg not ready yet");
            return None;
        }

        let ffmpeg = match Self::find_ffmpeg() {
            Some(path) => path,
            None => {
                Self::debug_log("[DEBUG] extract_video_thumbnail: FFmpeg not found");
                return None;
            }
        };
        Self::debug_log(&format!("[DEBUG] Using FFmpeg: {:?}", ffmpeg));
        Self::debug_log(&format!("[DEBUG] Video path: {}", video_path));

        // Use a temp file instead of pipe (more reliable on Windows)
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("thumb_{}.png", std::process::id()));
        let temp_path = temp_file.to_string_lossy().to_string();

        // Try to extract a frame at 1 second
        let result = Command::new(&ffmpeg)
            .args([
                "-i", video_path,
                "-ss", "00:00:01",
                "-vframes", "1",
                "-vcodec", "png",
                "-y",
                &temp_path
            ])
            .output();

        match result {
            Ok(output) => {
                Self::debug_log(&format!("[DEBUG] FFmpeg exit status: {:?}", output.status));
                if !output.stderr.is_empty() {
                    Self::debug_log(&format!("[DEBUG] FFmpeg stderr: {}", String::from_utf8_lossy(&output.stderr)));
                }

                if output.status.success() {
                    // Read the temp file
                    if let Ok(data) = std::fs::read(&temp_file) {
                        let _ = std::fs::remove_file(&temp_file);
                        if !data.is_empty() {
                            Self::debug_log(&format!("[DEBUG] Thumbnail extracted: {} bytes", data.len()));
                            return Some(data);
                        }
                    }
                }

                // Try at 0 seconds if 1 second failed
                Self::debug_log("[DEBUG] Trying at 0 seconds...");
                let result2 = Command::new(&ffmpeg)
                    .args([
                        "-i", video_path,
                        "-ss", "00:00:00",
                        "-vframes", "1",
                        "-vcodec", "png",
                        "-y",
                        &temp_path
                    ])
                    .output();

                if let Ok(output2) = result2 {
                    Self::debug_log(&format!("[DEBUG] FFmpeg (0s) exit status: {:?}", output2.status));
                    if output2.status.success() {
                        if let Ok(data) = std::fs::read(&temp_file) {
                            let _ = std::fs::remove_file(&temp_file);
                            if !data.is_empty() {
                                Self::debug_log(&format!("[DEBUG] Thumbnail extracted at 0s: {} bytes", data.len()));
                                return Some(data);
                            }
                        }
                    }
                }

                let _ = std::fs::remove_file(&temp_file);
                Self::debug_log("[ERROR] Failed to extract thumbnail");
                None
            }
            Err(e) => {
                Self::debug_log(&format!("[ERROR] Failed to run FFmpeg: {}", e));
                None
            }
        }
    }

}

impl eframe::App for FileListerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for background scan results
        self.check_scan_results();

        // Check for background image load results
        self.check_image_loads(ctx);

        // Keep repainting while scanning or loading images
        if self.is_scanning || self.image_receiver.is_some() {
            ctx.request_repaint();
        }

        // Top panel for controls
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(10.0);

            // Title
            //ui.heading("File Lister");
            //ui.add_space(10.0);

            // Folder selection section
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!self.is_scanning, |ui| {
                    if ui.button("Select Folder...").clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            self.selected_folder = Some(folder);
                            self.scan_selected_folder();
                        }
                    }
                });

                if let Some(folder) = &self.selected_folder {
                    ui.label(format!("Selected: {}", folder.display()));
                }
            });

            ui.add_space(5.0);

            // Recursive checkbox (disabled while scanning)
            ui.horizontal(|ui| {
                ui.add_enabled_ui(!self.is_scanning, |ui| {
                    let old_recursive = self.recursive;
                    ui.checkbox(&mut self.recursive, "Include subfolders (recursive)");

                    // Re-scan if checkbox changed and folder is selected
                    if old_recursive != self.recursive && self.selected_folder.is_some() {
                        self.scan_selected_folder();
                    }
                });

                // Show loading spinner while scanning
                if self.is_scanning {
                    ui.spinner();
                    ui.label("Scanning files...");
                }
            });

            ui.add_space(5.0);

            // Error display
            if let Some(error) = &self.error_message {
                ui.colored_label(egui::Color32::RED, error);
            }

            // Status display
            ui.label(&self.status_message);

            ui.add_space(5.0);
        });

        // Bottom panel for export button and tools (fixed footer)
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if !self.files.is_empty() {
                    if ui.button("Export to CSV...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("CSV files", &["csv"])
                            .set_file_name("files.csv")
                            .save_file()
                        {
                            self.export_csv(&path);
                        }
                    }

                    ui.label(format!("  |  Showing {} of {} files", self.filtered_files.len(), self.files.len()));
                }

                // Spacer to push download buttons to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Pdfium download button
                    if Self::is_pdfium_ready() {
                        ui.colored_label(egui::Color32::GREEN, "✓ PDF");
                    } else if Self::is_pdfium_downloading() {
                        ui.spinner();
                        ui.label("Downloading Pdfium...");
                        ctx.request_repaint(); // Keep updating while downloading
                    } else {
                        if ui.button("📥 Download Pdfium").clicked() {
                            // Set downloading flag BEFORE spawning thread to avoid race condition
                            PDFIUM_DOWNLOADING.store(true, Ordering::SeqCst);
                            thread::spawn(|| {
                                Self::download_pdfium();
                            });
                        }
                    }

                    ui.separator();

                    // FFmpeg status/install button
                    if Self::is_ffmpeg_ready() {
                        ui.colored_label(egui::Color32::GREEN, "✓ Video");
                    } else {
                        if ui.button("📥 Install FFmpeg").clicked() {
                            // Open FFmpeg download page
                            let _ = open::that("https://www.gyan.dev/ffmpeg/builds/");
                        }
                        ui.label("⚠").on_hover_text("FFmpeg not found.\nClick to download, or run:\nwinget install ffmpeg");
                    }

                    ui.separator();
                    ui.label("Preview Tools:");
                });
            });
            ui.add_space(10.0);
        });

        // Central panel for filter and table
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.files.is_empty() {
                // Filter input
                ui.horizontal(|ui| {
                    ui.label("Filter:");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.filter_text)
                            .hint_text("Type to filter by name, extension, or path...")
                            .desired_width(300.0)
                    );
                    if response.changed() {
                        self.apply_filter();
                    }
                    if ui.button("Clear").clicked() {
                        self.filter_text.clear();
                        self.apply_filter();
                    }

                    ui.add_space(20.0);

                    // Show duplicates only checkbox
                    let old_show_duplicates = self.show_duplicates_only;
                    ui.checkbox(&mut self.show_duplicates_only, "Show duplicates only");
                    if old_show_duplicates != self.show_duplicates_only {
                        self.apply_filter();
                    }

                    ui.add_space(10.0);

                    // Show today only checkbox
                    let old_show_today = self.show_today_only;
                    ui.checkbox(&mut self.show_today_only, "Show today only");
                    if old_show_today != self.show_today_only {
                        self.apply_filter();
                    }

                    ui.add_space(20.0);

                    // Move Selected and Delete Selected buttons
                    let selected_count = self.selected_files.len();
                    ui.add_enabled_ui(selected_count > 0, |ui| {
                        if ui.button(format!("Move Selected ({})", selected_count)).clicked() {
                            self.move_selected_files();
                        }
                        if ui.button(format!("Delete Selected ({})", selected_count)).clicked() {
                            self.prepare_bulk_delete();
                        }
                    });
                });

                ui.add_space(5.0);
                ui.separator();
                ui.add_space(5.0);

                let available_height = ui.available_height();

                // Store paths and duplicate info for table (to avoid borrow issues)
                let file_paths: Vec<String> = self.filtered_files
                    .iter()
                    .map(|f| f.absolute_path.clone())
                    .collect();

                let duplicate_info: Vec<Option<usize>> = self.filtered_files
                    .iter()
                    .map(|f| self.is_duplicate(&f.full_name))
                    .collect();

                // Track header checkbox state
                let all_selected = !self.filtered_files.is_empty()
                    && self.selected_files.len() == self.filtered_files.len();

                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .sense(egui::Sense::hover())  // Enable hover detection
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .min_scrolled_height(100.0)
                    .max_scroll_height(available_height)
                    .column(Column::initial(30.0).resizable(false).clip(true))  // Checkbox
                    .column(Column::initial(50.0).resizable(false).clip(true))  // Icons (type + dup)
                    .column(Column::initial(150.0).resizable(true).clip(true))  // Name
                    .column(Column::initial(70.0).resizable(true).clip(true))   // Extension
                    .column(Column::initial(80.0).resizable(true).clip(true))   // Size
                    .column(Column::initial(130.0).resizable(true).clip(true))  // Date Modified
                    .column(Column::initial(200.0).resizable(true).clip(true))  // Path
                    .column(Column::remainder().resizable(true).clip(true))     // Full Path
                    .header(24.0, |mut header| {
                        header.col(|ui| {
                            // Header checkbox for select all/none
                            let mut header_checked = all_selected;
                            if ui.checkbox(&mut header_checked, "").changed() {
                                if header_checked {
                                    self.select_all();
                                } else {
                                    self.deselect_all();
                                }
                            }
                        });
                        header.col(|ui| {
                            ui.strong("");  // Icon column - no header text
                        });
                        header.col(|ui| {
                            if ui.button(format!("Name{}", self.get_sort_indicator(SortColumn::Name))).clicked() {
                                self.toggle_sort(SortColumn::Name);
                            }
                        });
                        header.col(|ui| {
                            if ui.button(format!("Ext{}", self.get_sort_indicator(SortColumn::Extension))).clicked() {
                                self.toggle_sort(SortColumn::Extension);
                            }
                        });
                        header.col(|ui| {
                            if ui.button(format!("Size{}", self.get_sort_indicator(SortColumn::Size))).clicked() {
                                self.toggle_sort(SortColumn::Size);
                            }
                        });
                        header.col(|ui| {
                            if ui.button(format!("Date{}", self.get_sort_indicator(SortColumn::Date))).clicked() {
                                self.toggle_sort(SortColumn::Date);
                            }
                        });
                        header.col(|ui| {
                            if ui.button(format!("Path{}", self.get_sort_indicator(SortColumn::Path))).clicked() {
                                self.toggle_sort(SortColumn::Path);
                            }
                        });
                        header.col(|ui| {
                            ui.strong("Full Path");
                        });
                    })
                    .body(|body| {
                        body.rows(24.0, self.filtered_files.len(), |mut row| {
                            let idx = row.index();
                            // Clone all file data upfront to avoid borrow conflicts
                            let file_name = self.filtered_files[idx].name.clone();
                            let file_extension = self.filtered_files[idx].extension.clone();
                            let file_size = self.filtered_files[idx].file_size;
                            let file_modified = self.filtered_files[idx].modified_timestamp;
                            let file_relative_path = self.filtered_files[idx].relative_path.clone();
                            let file_absolute_path = self.filtered_files[idx].absolute_path.clone();
                            let file_path = file_paths[idx].clone();
                            let is_editing = self.editing_index == Some(idx);
                            let dup_count = duplicate_info[idx];
                            let is_selected = self.selected_files.contains(&idx);

                            // Checkbox column for selection
                            row.col(|ui| {
                                let mut checked = is_selected;
                                if ui.checkbox(&mut checked, "").changed() {
                                    self.toggle_selection(idx);
                                }
                            });

                            // Icon column: file type + duplicate indicator + preview on hover
                            row.col(|ui| {
                                let icon_response = ui.horizontal(|ui| {
                                    // File type icon
                                    let icon_label = ui.add(
                                        egui::Label::new(Self::get_file_type_icon(&file_extension))
                                            .sense(egui::Sense::hover())
                                    );

                                    // Duplicate indicator
                                    if let Some(count) = dup_count {
                                        let dup_label = ui.colored_label(
                                            egui::Color32::from_rgb(255, 140, 0), // Orange
                                            "⚠"
                                        );
                                        dup_label.on_hover_text(format!("Duplicate: {} files with this name", count));
                                    }

                                    icon_label
                                }).inner;

                                // Show preview on hover for image/video/PDF files (on icon)
                                if icon_response.hovered() && Self::is_previewable(&file_extension) {
                                    let is_video = Self::is_video_file(&file_extension);
                                    let is_pdf = Self::is_pdf_file(&file_extension);
                                    // Get cached texture or trigger background load
                                    if let Some(tex) = self.image_cache.get(&file_absolute_path) {
                                        // Show from cache immediately
                                        icon_response.on_hover_ui_at_pointer(|ui| {
                                            ui.set_max_width(420.0);
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(&file_name).strong());
                                                if is_video {
                                                    ui.label(egui::RichText::new(" 🎬").color(egui::Color32::GRAY));
                                                } else if is_pdf {
                                                    ui.label(egui::RichText::new(" 📄").color(egui::Color32::GRAY));
                                                }
                                            });
                                            ui.add_space(4.0);
                                            let size = tex.size();
                                            ui.image((tex.id(), egui::vec2(size[0] as f32, size[1] as f32)));
                                        });
                                    } else {
                                        // Show status for videos
                                        if is_video {
                                            if !Self::is_ffmpeg_ready() {
                                                icon_response.on_hover_text("📹 Video preview requires FFmpeg\nInstall: winget install ffmpeg");
                                            } else {
                                                // Start loading in background if not already loading this file
                                                if self.image_loading_path.as_ref() != Some(&file_absolute_path) {
                                                    self.load_hover_preview(idx, ctx);
                                                }
                                                let elapsed = self.get_loading_elapsed_secs().unwrap_or(0);
                                                let status = if elapsed > 0 {
                                                    format!("Loading video thumbnail... {}s", elapsed)
                                                } else {
                                                    "Loading video thumbnail...".to_string()
                                                };
                                                icon_response.on_hover_text(status);
                                                ctx.request_repaint();
                                            }
                                        } else if is_pdf {
                                            // Show status for PDFs
                                            if !Self::is_pdfium_ready() {
                                                if Self::is_pdfium_downloading() {
                                                    icon_response.on_hover_text("⏳ Downloading Pdfium (first time setup)...");
                                                    ctx.request_repaint();
                                                } else {
                                                    icon_response.on_hover_text("📄 PDF preview - Pdfium not available");
                                                }
                                            } else {
                                                // Start loading in background if not already loading this file
                                                if self.image_loading_path.as_ref() != Some(&file_absolute_path) {
                                                    self.load_hover_preview(idx, ctx);
                                                }
                                                let elapsed = self.get_loading_elapsed_secs().unwrap_or(0);
                                                let status = if elapsed > 0 {
                                                    format!("Loading PDF preview... {}s", elapsed)
                                                } else {
                                                    "Loading PDF preview...".to_string()
                                                };
                                                icon_response.on_hover_text(status);
                                                ctx.request_repaint();
                                            }
                                        } else {
                                            // Start loading in background if not already loading this file
                                            if self.image_loading_path.as_ref() != Some(&file_absolute_path) {
                                                self.load_hover_preview(idx, ctx);
                                            }
                                        }
                                    }
                                }
                            });

                            // Name column: supports rename via double-click
                            row.col(|ui| {
                                if is_editing {
                                    // Show text edit for renaming
                                    let response = ui.add(
                                        egui::TextEdit::singleline(&mut self.editing_text)
                                            .desired_width(ui.available_width() - 10.0)
                                    );

                                    // Request focus on first frame
                                    if self.request_rename_focus {
                                        response.request_focus();
                                        self.request_rename_focus = false;
                                    }

                                    // Confirm on Enter, cancel on Escape
                                    if response.lost_focus() {
                                        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                            self.confirm_rename();
                                        } else {
                                            // Clicked outside or pressed Escape
                                            self.confirm_rename();
                                        }
                                    }
                                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                        self.cancel_rename();
                                    }
                                } else {
                                    // Normal label with double-click to rename
                                    let label = ui.add(
                                        egui::Label::new(&file_name).sense(egui::Sense::click())
                                    );
                                    if label.double_clicked() {
                                        self.start_rename(idx);
                                    }

                                    // Show preview on hover for image/video/PDF files
                                    if label.hovered() && Self::is_previewable(&file_extension) {
                                        let is_video = Self::is_video_file(&file_extension);
                                        let is_pdf = Self::is_pdf_file(&file_extension);
                                        // Get cached texture or trigger background load
                                        if let Some(tex) = self.image_cache.get(&file_absolute_path) {
                                            // Show from cache immediately
                                            label.clone().on_hover_ui_at_pointer(|ui| {
                                                ui.set_max_width(420.0);
                                                ui.horizontal(|ui| {
                                                    ui.label(egui::RichText::new(&file_name).strong());
                                                    if is_video {
                                                        ui.label(egui::RichText::new(" 🎬").color(egui::Color32::GRAY));
                                                    } else if is_pdf {
                                                        ui.label(egui::RichText::new(" 📄").color(egui::Color32::GRAY));
                                                    }
                                                });
                                                ui.add_space(4.0);
                                                let size = tex.size();
                                                ui.image((tex.id(), egui::vec2(size[0] as f32, size[1] as f32)));
                                            });
                                        } else {
                                            // Show status for videos
                                            if is_video {
                                                if !Self::is_ffmpeg_ready() {
                                                    label.clone().on_hover_text("📹 Video preview requires FFmpeg\nInstall: winget install ffmpeg");
                                                } else {
                                                    // Start loading in background if not already loading this file
                                                    if self.image_loading_path.as_ref() != Some(&file_absolute_path) {
                                                        self.load_hover_preview(idx, ctx);
                                                    }
                                                    let elapsed = self.get_loading_elapsed_secs().unwrap_or(0);
                                                    let status = if elapsed > 0 {
                                                        format!("Loading video thumbnail... {}s", elapsed)
                                                    } else {
                                                        "Loading video thumbnail...".to_string()
                                                    };
                                                    label.clone().on_hover_text(status);
                                                    ctx.request_repaint();
                                                }
                                            } else if is_pdf {
                                                // Show status for PDFs
                                                if !Self::is_pdfium_ready() {
                                                    if Self::is_pdfium_downloading() {
                                                        label.clone().on_hover_text("⏳ Downloading Pdfium (first time setup)...");
                                                        ctx.request_repaint();
                                                    } else {
                                                        label.clone().on_hover_text("📄 PDF preview - Pdfium not available");
                                                    }
                                                } else {
                                                    // Start loading in background if not already loading this file
                                                    if self.image_loading_path.as_ref() != Some(&file_absolute_path) {
                                                        self.load_hover_preview(idx, ctx);
                                                    }
                                                    let elapsed = self.get_loading_elapsed_secs().unwrap_or(0);
                                                    let status = if elapsed > 0 {
                                                        format!("Loading PDF preview... {}s", elapsed)
                                                    } else {
                                                        "Loading PDF preview...".to_string()
                                                    };
                                                    label.clone().on_hover_text(status);
                                                    ctx.request_repaint();
                                                }
                                            } else {
                                                // Start loading in background if not already loading this file
                                                if self.image_loading_path.as_ref() != Some(&file_absolute_path) {
                                                    self.load_hover_preview(idx, ctx);
                                                }
                                            }
                                        }
                                    }

                                    label.context_menu(|ui| {
                                        if ui.button("📂 Open file location").clicked() {
                                            Self::open_in_explorer(&file_path);
                                            ui.close();
                                        }
                                        if ui.button("✏️ Rename").clicked() {
                                            self.start_rename(idx);
                                            ui.close();
                                        }
                                        if ui.button("📁 Move to folder...").clicked() {
                                            self.move_file(&file_path);
                                            ui.close();
                                        }
                                        ui.separator();
                                        if ui.button("🗑️ Delete").clicked() {
                                            self.delete_file(&file_path);
                                            ui.close();
                                        }
                                    });
                                }
                            });

                            row.col(|ui| {
                                let label = ui.label(&file_extension);
                                label.context_menu(|ui| {
                                    if ui.button("📂 Open file location").clicked() {
                                        Self::open_in_explorer(&file_path);
                                        ui.close();
                                    }
                                    if ui.button("✏️ Rename").clicked() {
                                        self.start_rename(idx);
                                        ui.close();
                                    }
                                    if ui.button("📁 Move to folder...").clicked() {
                                        self.move_file(&file_path);
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("🗑️ Delete").clicked() {
                                        self.delete_file(&file_path);
                                        ui.close();
                                    }
                                });
                            });
                            row.col(|ui| {
                                let label = ui.label(format_size(file_size));
                                label.context_menu(|ui| {
                                    if ui.button("📂 Open file location").clicked() {
                                        Self::open_in_explorer(&file_path);
                                        ui.close();
                                    }
                                    if ui.button("✏️ Rename").clicked() {
                                        self.start_rename(idx);
                                        ui.close();
                                    }
                                    if ui.button("📁 Move to folder...").clicked() {
                                        self.move_file(&file_path);
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("🗑️ Delete").clicked() {
                                        self.delete_file(&file_path);
                                        ui.close();
                                    }
                                });
                            });
                            row.col(|ui| {
                                let label = ui.label(format_date(file_modified));
                                label.context_menu(|ui| {
                                    if ui.button("📂 Open file location").clicked() {
                                        Self::open_in_explorer(&file_path);
                                        ui.close();
                                    }
                                    if ui.button("✏️ Rename").clicked() {
                                        self.start_rename(idx);
                                        ui.close();
                                    }
                                    if ui.button("📁 Move to folder...").clicked() {
                                        self.move_file(&file_path);
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("🗑️ Delete").clicked() {
                                        self.delete_file(&file_path);
                                        ui.close();
                                    }
                                });
                            });
                            row.col(|ui| {
                                let label = ui.label(&file_relative_path);
                                label.context_menu(|ui| {
                                    if ui.button("📂 Open file location").clicked() {
                                        Self::open_in_explorer(&file_path);
                                        ui.close();
                                    }
                                    if ui.button("✏️ Rename").clicked() {
                                        self.start_rename(idx);
                                        ui.close();
                                    }
                                    if ui.button("📁 Move to folder...").clicked() {
                                        self.move_file(&file_path);
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("🗑️ Delete").clicked() {
                                        self.delete_file(&file_path);
                                        ui.close();
                                    }
                                });
                            });
                            row.col(|ui| {
                                let label = ui.label(&file_absolute_path);
                                label.context_menu(|ui| {
                                    if ui.button("📂 Open file location").clicked() {
                                        Self::open_in_explorer(&file_path);
                                        ui.close();
                                    }
                                    if ui.button("✏️ Rename").clicked() {
                                        self.start_rename(idx);
                                        ui.close();
                                    }
                                    if ui.button("📁 Move to folder...").clicked() {
                                        self.move_file(&file_path);
                                        ui.close();
                                    }
                                    ui.separator();
                                    if ui.button("🗑️ Delete").clicked() {
                                        self.delete_file(&file_path);
                                        ui.close();
                                    }
                                });
                            });

                            // Set hover highlighting after all columns are rendered
                            row.set_hovered(row.response().hovered());
                        });
                    });
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Select a folder to view files");
                });
            }
        });

        // Bulk delete confirmation modal
        if self.show_delete_confirm {
            // Semi-transparent overlay
            egui::Area::new(egui::Id::new("modal_overlay"))
                .fixed_pos(egui::Pos2::ZERO)
                .show(ctx, |ui| {
                    #[allow(deprecated)]
                    let screen_rect = ctx.screen_rect();
                    ui.painter().rect_filled(
                        screen_rect,
                        0.0,
                        egui::Color32::from_black_alpha(120),
                    );
                });

            egui::Window::new("Confirm Delete")
                .collapsible(false)
                .resizable(false)
                .title_bar(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .min_width(350.0)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);

                        // Warning icon
                        ui.label(
                            egui::RichText::new("⚠")
                                .size(48.0)
                                .color(egui::Color32::from_rgb(255, 180, 0))
                        );

                        ui.add_space(12.0);

                        // Title
                        ui.label(
                            egui::RichText::new("Confirm Delete")
                                .size(20.0)
                                .strong()
                        );

                        ui.add_space(8.0);

                        // Description
                        let count = self.pending_delete_paths.len();
                        ui.label(
                            egui::RichText::new(format!(
                                "Are you sure you want to permanently delete {} file{}?",
                                count,
                                if count == 1 { "" } else { "s" }
                            ))
                            .size(14.0)
                            .color(egui::Color32::GRAY)
                        );

                        ui.add_space(16.0);

                        // File list in a frame - full width, white bg, black border, show 10 rows
                        let row_height = 22.0;
                        let max_visible_rows = 10;
                        let list_height = row_height * max_visible_rows as f32;

                        ui.scope(|ui| {
                            ui.set_width(ui.available_width());
                            egui::Frame::new()
                                .fill(egui::Color32::TRANSPARENT)
                                .stroke(egui::Stroke::new(1.0, egui::Color32::GRAY))
                                .corner_radius(egui::CornerRadius::same(8))
                                .inner_margin(egui::Margin::same(8))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    egui::ScrollArea::vertical()
                                        .max_height(list_height)
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            for (_, name) in &self.pending_delete_paths {
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("•")
                                                            .color(egui::Color32::from_rgb(200, 60, 60))
                                                    );
                                                    ui.label(name);
                                                });
                                            }
                                        });
                                });
                        });

                        ui.add_space(20.0);

                        // Action buttons - centered with rounded corners
                        ui.horizontal(|ui| {
                            let button_width = 120.0;
                            let button_height = 36.0;
                            let spacing = 16.0;
                            let total_width = button_width * 2.0 + spacing;
                            let available_width = ui.available_width();
                            let offset = (available_width - total_width) / 2.0;

                            ui.add_space(offset);

                            // Cancel button with rounded corners
                            if ui.add_sized(
                                [button_width, button_height],
                                egui::Button::new(
                                    egui::RichText::new("Cancel").size(14.0)
                                )
                                .corner_radius(egui::CornerRadius::same(8))
                            ).clicked() {
                                self.cancel_bulk_delete();
                            }

                            ui.add_space(spacing);

                            // Delete button (red) with rounded corners
                            if ui.add_sized(
                                [button_width, button_height],
                                egui::Button::new(
                                    egui::RichText::new("Delete")
                                        .size(14.0)
                                        .color(egui::Color32::WHITE)
                                )
                                .fill(egui::Color32::from_rgb(200, 60, 60))
                                .corner_radius(egui::CornerRadius::same(8))
                            ).clicked() {
                                self.execute_bulk_delete();
                            }
                        });

                        ui.add_space(20.0);
                    });
                });
        }
    }
}
