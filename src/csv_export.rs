use crate::file_scanner::FileInfo;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn export_to_csv(files: &[FileInfo], output_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(output_path)?;

    // Write UTF-8 BOM for Excel compatibility with non-English characters
    file.write_all(&[0xEF, 0xBB, 0xBF])?;

    let mut writer = csv::Writer::from_writer(file);

    // Write header manually for better column names
    writer.write_record(["File Name", "Extension", "Size (bytes)", "Relative Path", "Full Path"])?;

    // Write data rows
    for file_info in files {
        writer.write_record([
            &file_info.name,
            &file_info.extension,
            &file_info.file_size.to_string(),
            &file_info.relative_path,
            &file_info.absolute_path,
        ])?;
    }

    writer.flush()?;
    Ok(())
}
