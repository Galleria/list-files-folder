#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod csv_export;
mod document_parser;
mod file_scanner;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "File Lister")]
#[command(about = "Lists files from a folder and exports to CSV")]
struct Args {
    /// Folder path to scan (launches GUI if not provided)
    #[arg(short, long)]
    folder: Option<PathBuf>,

    /// Output CSV file path
    #[arg(short, long, default_value = "files.csv")]
    output: PathBuf,

    /// Scan subfolders recursively
    #[arg(short, long, default_value = "false")]
    recursive: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if let Some(folder) = args.folder {
        // CLI mode: scan folder and export directly
        run_cli_mode(folder, args.output, args.recursive)?;
    } else {
        // GUI mode: launch the application
        run_gui_mode()?;
    }

    Ok(())
}

fn run_cli_mode(folder: PathBuf, output: PathBuf, recursive: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Scanning folder: {}", folder.display());
    if recursive {
        println!("(including subfolders)");
    }

    let files = file_scanner::scan_folder(&folder, recursive)?;
    println!("Found {} files", files.len());

    csv_export::export_to_csv(&files, &output)?;
    println!("Exported to: {}", output.display());

    Ok(())
}

fn run_gui_mode() -> Result<(), Box<dyn std::error::Error>> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "File Lister",
        options,
        Box::new(|cc| Ok(Box::new(app::FileListerApp::new(cc)))),
    )?;

    Ok(())
}
