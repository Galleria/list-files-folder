use std::path::Path;

/// Maximum lines to show for text preview
const MAX_TEXT_LINES: usize = 100;
/// Maximum lines to show for code preview
const MAX_CODE_LINES: usize = 300;
/// Maximum rows to show for table preview
const MAX_TABLE_ROWS: usize = 100;
/// Maximum columns to show for table preview
const MAX_TABLE_COLS: usize = 20;

/// Read file bytes and decode with encoding detection
fn read_text_with_encoding(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;

    // Try UTF-8 first (most common)
    if let Ok(content) = std::str::from_utf8(&bytes) {
        return Ok(content.to_string());
    }

    // Try UTF-8 with BOM
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        if let Ok(content) = std::str::from_utf8(&bytes[3..]) {
            return Ok(content.to_string());
        }
    }

    // Try Windows-1252 (common for Thai/European text)
    let (decoded, _, had_errors) = encoding_rs::WINDOWS_1252.decode(&bytes);
    if !had_errors {
        return Ok(decoded.to_string());
    }

    // Try TIS-620 (Thai encoding)
    let (decoded, _, _) = encoding_rs::WINDOWS_874.decode(&bytes);
    Ok(decoded.to_string())
}

/// Extract text content from TXT file with encoding detection
pub fn extract_txt_text(path: &Path) -> Result<String, String> {
    let content = read_text_with_encoding(path)?;

    let total_lines = content.lines().count();
    let lines: Vec<&str> = content.lines().take(MAX_TEXT_LINES).collect();
    let truncated = lines.len() < total_lines;

    let mut result = lines.join("\n");
    if truncated {
        result.push_str(&format!(
            "\n\n... (showing first {} of {} lines)",
            MAX_TEXT_LINES, total_lines
        ));
    }

    Ok(result)
}

/// Extract code content from source files (html, js, css, xml, yaml, etc.)
pub fn extract_code_text(path: &Path) -> Result<String, String> {
    let content = read_text_with_encoding(path)?;

    let total_lines = content.lines().count();
    let lines: Vec<&str> = content.lines().take(MAX_CODE_LINES).collect();
    let truncated = lines.len() < total_lines;

    let mut result = lines.join("\n");
    if truncated {
        result.push_str(&format!(
            "\n\n... (showing first {} of {} lines)",
            MAX_CODE_LINES, total_lines
        ));
    }

    Ok(result)
}

/// Audio metadata structure
#[derive(Clone, Debug)]
pub struct AudioMetadata {
    pub duration_secs: Option<f64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub codec: Option<String>,
    pub bitrate: Option<u32>,
}

/// Extract audio metadata from file
pub fn extract_audio_metadata(path: &Path) -> Result<AudioMetadata, String> {
    use symphonia::core::codecs::CODEC_TYPE_NULL;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    let file = std::fs::File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .map_err(|e| format!("Failed to probe audio: {}", e))?;

    let format = probed.format;

    // Find the first audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("No audio track found")?;

    let codec_params = &track.codec_params;

    // Calculate duration
    let duration_secs = if let Some(n_frames) = codec_params.n_frames {
        if let Some(sample_rate) = codec_params.sample_rate {
            Some(n_frames as f64 / sample_rate as f64)
        } else {
            None
        }
    } else {
        None
    };

    // Get codec name
    let codec = symphonia::default::get_codecs()
        .get_codec(codec_params.codec)
        .map(|c| c.short_name.to_string());

    Ok(AudioMetadata {
        duration_secs,
        sample_rate: codec_params.sample_rate,
        channels: codec_params.channels.map(|c| c.count() as u8),
        codec,
        bitrate: codec_params.bits_per_sample.map(|b| b * codec_params.sample_rate.unwrap_or(0)),
    })
}

/// Format duration as MM:SS or HH:MM:SS
pub fn format_duration(secs: f64) -> String {
    let total_secs = secs as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// Extract text content from DOCX file
pub fn extract_docx_text(path: &Path) -> Result<String, String> {
    use std::fs::File;
    use std::io::{BufReader, Read};

    // Open the docx file as a zip archive
    let file = File::open(path).map_err(|e| format!("Failed to open file: {}", e))?;
    let reader = BufReader::new(file);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("Failed to read DOCX archive: {}", e))?;

    // DOCX stores the main content in word/document.xml
    let mut document_xml = archive
        .by_name("word/document.xml")
        .map_err(|e| format!("Failed to find document content: {}", e))?;

    let mut xml_content = String::new();
    document_xml
        .read_to_string(&mut xml_content)
        .map_err(|e| format!("Failed to read document: {}", e))?;

    // Extract text from XML (simple approach - strip tags and extract text between <w:t> tags)
    let text = extract_text_from_docx_xml(&xml_content);

    let total_lines = text.lines().count();
    let lines: Vec<&str> = text.lines().take(MAX_TEXT_LINES).collect();
    let truncated = lines.len() < total_lines;

    let mut result = lines.join("\n");
    if truncated {
        result.push_str(&format!(
            "\n\n... (showing first {} of {} lines)",
            MAX_TEXT_LINES, total_lines
        ));
    }

    Ok(result)
}

/// Extract plain text from DOCX XML content
fn extract_text_from_docx_xml(xml: &str) -> String {
    let mut result = String::new();
    let mut in_text_tag = false;
    let mut current_paragraph = String::new();

    // Simple state machine to extract text from <w:t> tags
    let mut chars = xml.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Check for tag
            let mut tag = String::new();
            while let Some(&next_ch) = chars.peek() {
                if next_ch == '>' {
                    chars.next();
                    break;
                }
                tag.push(chars.next().unwrap());
            }

            if tag.starts_with("w:t") && !tag.starts_with("w:t/") {
                in_text_tag = true;
            } else if tag == "/w:t" {
                in_text_tag = false;
            } else if tag == "/w:p" {
                // End of paragraph - add newline
                if !current_paragraph.is_empty() {
                    result.push_str(&current_paragraph);
                    result.push('\n');
                    current_paragraph.clear();
                }
            }
        } else if in_text_tag {
            current_paragraph.push(ch);
        }
    }

    // Add any remaining text
    if !current_paragraph.is_empty() {
        result.push_str(&current_paragraph);
    }

    result
}

/// Extract table data from XLSX file
/// Returns (headers, rows, sheet_name)
pub fn extract_xlsx_table(
    path: &Path,
) -> Result<(Vec<String>, Vec<Vec<String>>, Option<String>), String> {
    use calamine::{open_workbook, Reader, Xlsx};

    let mut workbook: Xlsx<_> =
        open_workbook(path).map_err(|e| format!("Failed to open XLSX: {}", e))?;

    // Get first sheet
    let sheet_names = workbook.sheet_names().to_vec();
    let sheet_name = sheet_names.first().cloned();

    if let Some(name) = &sheet_name {
        if let Ok(range) = workbook.worksheet_range(name) {
            let mut headers = Vec::new();
            let mut rows = Vec::new();
            let total_rows = range.rows().len();

            for (row_idx, row) in range.rows().enumerate() {
                if row_idx > MAX_TABLE_ROWS {
                    break;
                }

                let cells: Vec<String> = row
                    .iter()
                    .take(MAX_TABLE_COLS)
                    .map(|c| c.to_string())
                    .collect();

                if row_idx == 0 {
                    headers = cells;
                } else {
                    rows.push(cells);
                }
            }

            // Add truncation note if needed
            if total_rows > MAX_TABLE_ROWS + 1 {
                let note = format!(
                    "... (showing first {} of {} rows)",
                    MAX_TABLE_ROWS,
                    total_rows - 1
                );
                rows.push(vec![note]);
            }

            return Ok((headers, rows, sheet_name));
        }
    }

    Err("No readable sheet found".to_string())
}

/// Extract table data from CSV file
/// Returns (headers, rows)
pub fn extract_csv_table(path: &Path) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)
        .map_err(|e| format!("Failed to open CSV: {}", e))?;

    // Get headers
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| format!("Failed to read headers: {}", e))?
        .iter()
        .take(MAX_TABLE_COLS)
        .map(|s| s.to_string())
        .collect();

    // Get rows
    let mut rows = Vec::new();
    let mut total_rows = 0;
    for result in reader.records() {
        total_rows += 1;
        if rows.len() >= MAX_TABLE_ROWS {
            continue; // Keep counting but don't add more rows
        }
        if let Ok(record) = result {
            let row: Vec<String> = record
                .iter()
                .take(MAX_TABLE_COLS)
                .map(|s| s.to_string())
                .collect();
            rows.push(row);
        }
    }

    // Add truncation note if needed
    if total_rows > MAX_TABLE_ROWS {
        let note = format!(
            "... (showing first {} of {} rows)",
            MAX_TABLE_ROWS, total_rows
        );
        rows.push(vec![note]);
    }

    Ok((headers, rows))
}
