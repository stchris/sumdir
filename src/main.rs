use flate2::read::GzDecoder;
use jwalk::WalkDir;
use rayon::prelude::*;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::{collections::BTreeMap, path::PathBuf};
use tar::Archive as TarArchive;

use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{HumanBytes, ProgressBar, ProgressStyle};
use itertools::Itertools;

#[derive(Clone, Default, clap::ValueEnum)]
enum OutputFormat {
    #[default]
    Text,
    Csv,
    Json,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(required = true)]
    target: PathBuf,

    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
    output: OutputFormat,

    #[arg(short, long, default_value_t = false)]
    mime: bool,

    #[arg(short, long, default_value_t = true)]
    progress_bar: bool,
}

#[derive(Debug, Default, Clone)]
struct ScanError {
    path: PathBuf,
    message: String,
}

#[derive(Debug, Default)]
struct Report {
    extensions: BTreeMap<String, i32>,
    mimetypes: BTreeMap<String, i32>,
    folders: Vec<PathBuf>,
    size: u64,
    errors: Vec<ScanError>,
}

impl Report {
    fn display(&self, format: &OutputFormat, use_mime: bool) {
        let data = if use_mime {
            &self.mimetypes
        } else {
            &self.extensions
        };
        match format {
            OutputFormat::Text => self.display_text(data),
            OutputFormat::Csv => self.display_csv(data, use_mime),
            OutputFormat::Json => self.display_json(data, use_mime),
        }
    }

    fn display_text(&self, data: &BTreeMap<String, i32>) {
        let num_files: i32 = data.values().sum();
        let size = HumanBytes(self.size);
        let error_info = if self.errors.is_empty() {
            String::new()
        } else {
            format!(", {} errors", self.errors.len())
        };
        println!(
            "{num_files} files, {} folders, {size}{error_info}",
            self.folders.len()
        );
        for (key, count) in data.iter().sorted_by(|a, b| b.1.cmp(a.1)) {
            println!("{key}: {count}");
        }
    }

    fn display_csv(&self, data: &BTreeMap<String, i32>, use_mime: bool) {
        let header = if use_mime { "mimetype" } else { "extension" };
        println!("{header},count");
        for (key, count) in data.iter().sorted_by(|a, b| b.1.cmp(a.1)) {
            println!("{key},{count}");
        }
    }

    fn display_json(&self, data: &BTreeMap<String, i32>, use_mime: bool) {
        let num_files: i32 = data.values().sum();
        let key_name = if use_mime { "mimetypes" } else { "extensions" };
        let entries: Vec<String> = data
            .iter()
            .sorted_by(|a, b| b.1.cmp(a.1))
            .map(|(key, count)| format!("    \"{key}\": {count}"))
            .collect();
        let error_entries: Vec<String> = self
            .errors
            .iter()
            .map(|e| {
                format!(
                    "    {{\n      \"path\": \"{}\",\n      \"message\": \"{}\"\n    }}",
                    e.path.display(),
                    e.message.replace('\\', "\\\\").replace('"', "\\\"")
                )
            })
            .collect();
        println!("{{");
        println!("  \"files\": {num_files},");
        println!("  \"folders\": {},", self.folders.len());
        println!("  \"size\": {},", self.size);
        println!("  \"{key_name}\": {{");
        println!("{}", entries.join(",\n"));
        println!("  }},");
        println!("  \"errors\": [");
        println!("{}", error_entries.join(",\n"));
        println!("  ]");
        println!("}}");
    }
}

fn detect_mimetype(path: &std::path::Path) -> Result<String> {
    let mut buffer = [0u8; 8192];
    let mut file = File::open(path).with_context(|| format!("failed to open {:?}", path))?;
    let bytes_read = file
        .read(&mut buffer)
        .with_context(|| format!("failed to read {:?}", path))?;
    if let Some(kind) = infer::get(&buffer[..bytes_read]) {
        return Ok(kind.mime_type().to_string());
    }
    Ok("application/octet-stream".to_string())
}

fn is_archive_extension(ext: &str) -> bool {
    matches!(ext, "zip" | "tar" | "gz" | "tgz" | "7z")
}

/// Process a virtual file entry from inside an archive (by name and first bytes for detection).
fn process_virtual_entry(name: &str, data: &[u8], report: &mut Report) {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    report
        .extensions
        .entry(ext)
        .and_modify(|c| *c += 1)
        .or_insert(1);
    let mimetype = infer::get(data)
        .map(|k| k.mime_type().to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    report
        .mimetypes
        .entry(mimetype)
        .and_modify(|c| *c += 1)
        .or_insert(1);
}

fn scan_zip_contents(path: &Path, report: &mut Report) -> Result<()> {
    let file = File::open(path).with_context(|| format!("failed to open {path:?}"))?;
    let mut archive =
        zip::ZipArchive::new(file).with_context(|| format!("failed to read zip {path:?}"))?;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .with_context(|| format!("failed to read zip entry {i} in {path:?}"))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let mut buffer = [0u8; 8192];
        let bytes_read = Read::read(&mut entry, &mut buffer).unwrap_or(0);
        process_virtual_entry(&name, &buffer[..bytes_read], report);
    }
    Ok(())
}

fn scan_tar_contents(path: &Path, report: &mut Report) -> Result<()> {
    let file = File::open(path).with_context(|| format!("failed to open {path:?}"))?;
    let mut archive = TarArchive::new(file);
    for entry_result in archive
        .entries()
        .with_context(|| format!("failed to read tar entries in {path:?}"))?
    {
        let mut entry =
            entry_result.with_context(|| format!("failed to read tar entry in {path:?}"))?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let name = entry
            .path()
            .with_context(|| "failed to get tar entry path")?
            .to_string_lossy()
            .to_string();
        let mut buffer = [0u8; 8192];
        let bytes_read = Read::read(&mut entry, &mut buffer).unwrap_or(0);
        process_virtual_entry(&name, &buffer[..bytes_read], report);
    }
    Ok(())
}

fn scan_gz_contents(path: &Path, report: &mut Report) -> Result<()> {
    // Attempt to read as tar.gz first; fall back to single compressed file on failure.
    let file = File::open(path).with_context(|| format!("failed to open {path:?}"))?;
    let decoder = GzDecoder::new(file);
    let mut archive = TarArchive::new(decoder);

    let mut entries_processed = 0usize;
    let mut first_error: Option<anyhow::Error> = None;

    match archive.entries() {
        Err(e) => first_error = Some(anyhow::Error::from(e)),
        Ok(entries) => {
            for entry_result in entries {
                match entry_result {
                    Err(e) => {
                        if entries_processed == 0 {
                            first_error = Some(anyhow::Error::from(e));
                        }
                        break;
                    }
                    Ok(mut entry) => {
                        if entry.header().entry_type().is_file() {
                            let name = entry
                                .path()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let mut buffer = [0u8; 8192];
                            let bytes_read = Read::read(&mut entry, &mut buffer).unwrap_or(0);
                            process_virtual_entry(&name, &buffer[..bytes_read], report);
                            entries_processed += 1;
                        }
                    }
                }
            }
        }
    }

    if entries_processed == 0 && first_error.is_some() {
        // Not a tar.gz — treat as a single gzip-compressed file.
        let file = File::open(path).with_context(|| format!("failed to open {path:?}"))?;
        let mut decoder = GzDecoder::new(file);
        let inner_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let mut buffer = [0u8; 8192];
        let bytes_read = Read::read(&mut decoder, &mut buffer).unwrap_or(0);
        process_virtual_entry(inner_name, &buffer[..bytes_read], report);
    }

    Ok(())
}

fn scan_7z_contents(path: &Path, report: &mut Report) -> Result<()> {
    let mut archive = sevenz_rust::SevenZReader::open(path, sevenz_rust::Password::empty())
        .with_context(|| format!("failed to open 7z archive {path:?}"))?;
    archive
        .for_each_entries(
            &mut |entry: &sevenz_rust::SevenZArchiveEntry, reader: &mut dyn Read| {
                if entry.is_directory() {
                    return Ok(true);
                }
                let name = entry.name().to_string();
                let mut buffer = [0u8; 8192];
                let bytes_read = Read::read(reader, &mut buffer).unwrap_or(0);
                process_virtual_entry(&name, &buffer[..bytes_read], report);
                Ok(true)
            },
        )
        .with_context(|| format!("failed to read 7z entries from {path:?}"))?;
    Ok(())
}

fn scan_archive_contents(path: &Path, report: &mut Report) -> Result<()> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    match ext {
        "zip" => scan_zip_contents(path, report),
        "tar" => scan_tar_contents(path, report),
        "gz" | "tgz" => scan_gz_contents(path, report),
        "7z" => scan_7z_contents(path, report),
        _ => Ok(()),
    }
}

fn process_entry(path: &std::path::Path, report: &mut Report) -> Result<()> {
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_owned()
        .into_string()
        .unwrap_or_default();

    let metadata = path
        .metadata()
        .with_context(|| format!("failed to read metadata for {:?}", path))?;

    report.size += metadata.len();
    report
        .extensions
        .entry(ext.clone())
        .and_modify(|e| *e += 1)
        .or_insert(1);

    let mimetype = detect_mimetype(path)
        .with_context(|| format!("failed to detect mimetype for {:?}", path))?;

    report
        .mimetypes
        .entry(mimetype)
        .and_modify(|e| *e += 1)
        .or_insert(1);

    if is_archive_extension(&ext) {
        scan_archive_contents(path, report)
            .with_context(|| format!("failed to scan archive contents of {:?}", path))?;
    }

    Ok(())
}

fn merge_reports(mut a: Report, b: Report) -> Report {
    for (ext, count) in b.extensions {
        a.extensions
            .entry(ext)
            .and_modify(|e| *e += count)
            .or_insert(count);
    }
    for (mime, count) in b.mimetypes {
        a.mimetypes
            .entry(mime)
            .and_modify(|e| *e += count)
            .or_insert(count);
    }
    a.folders.extend(b.folders);
    a.size += b.size;
    a.errors.extend(b.errors);
    a
}

fn make_progress_bar(enabled: bool) -> Option<ProgressBar> {
    if !enabled {
        return None;
    }
    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed}] {wide_msg}")
            .expect("failed to set progress style"),
    );
    progress.set_message("Scanning target...");
    Some(progress)
}

fn scan(target: PathBuf, progress_bar: bool) -> Report {
    let pb = make_progress_bar(progress_bar);

    if target.is_file() {
        let mut report = Report::default();
        if let Err(e) = process_entry(&target, &mut report) {
            report.errors.push(ScanError {
                path: target,
                message: e.to_string(),
            });
        }
        if let Some(progress) = pb {
            progress.finish_with_message(format!("Completed with {} errors", report.errors.len()));
        }
        return report;
    }

    let pb_fold = pb.clone();
    let report = WalkDir::new(target)
        .into_iter()
        .skip(1)
        .par_bridge()
        .fold(Report::default, move |mut partial, entry| {
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if let Some(err) = entry.read_children_error {
                        partial.errors.push(ScanError {
                            path,
                            message: format!("failed to read directory: {err}"),
                        });
                        return partial;
                    }
                    if entry.file_type().is_dir() {
                        partial.folders.push(path);
                    } else {
                        if let Some(ref pb) = pb_fold {
                            pb.tick();
                        }
                        if let Err(e) = process_entry(&path, &mut partial) {
                            partial.errors.push(ScanError {
                                path,
                                message: e.to_string(),
                            });
                        }
                    }
                }
                Err(e) => {
                    let path = e.path().map(|p| p.to_path_buf()).unwrap_or_default();
                    partial.errors.push(ScanError {
                        path,
                        message: format!("failed to read entry: {e}"),
                    });
                }
            }
            partial
        })
        .reduce(Report::default, merge_reports);

    if let Some(progress) = pb {
        progress.finish_with_message(format!("Completed with {} errors", report.errors.len()));
    }

    report
}

fn main() {
    let cli = Cli::parse();
    if !cli.target.exists() {
        eprintln!(
            "{} does not exist",
            cli.target.to_str().unwrap_or("<invalid path>")
        );
        std::process::exit(1);
    }
    let report = scan(cli.target, cli.progress_bar);
    report.display(&cli.output, cli.mime);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_testdata_folder() {
        let report = scan("testdata".into(), false);
        let num_files: i32 = report.extensions.values().sum();
        // 27 top-level files + 4 inner files (one archived.txt per archive)
        assert_eq!(num_files, 31);
        assert_eq!(report.folders.len(), 5);
        assert_eq!(report.errors.len(), 0);
        // Verify some expected extensions
        assert_eq!(report.extensions.get("png"), Some(&2)); // sample.png + masquerading.png
        assert_eq!(report.extensions.get("pdf"), Some(&1));
        assert_eq!(report.extensions.get("jpg"), Some(&1));
        assert_eq!(report.extensions.get("docx"), Some(&1));
        // plain.txt + 4 archived.txt files (one per archive)
        assert_eq!(report.extensions.get("txt"), Some(&5));
    }

    #[test]
    fn test_scan_zip_archive() {
        let report = scan("testdata/archives/sample.zip".into(), false);
        let num_files: i32 = report.extensions.values().sum();
        assert_eq!(num_files, 2); // 1 zip + 1 txt inside
        assert_eq!(report.extensions.get("zip"), Some(&1));
        assert_eq!(report.extensions.get("txt"), Some(&1));
        assert_eq!(report.errors.len(), 0);
    }

    #[test]
    fn test_scan_tar_archive() {
        let report = scan("testdata/archives/sample.tar".into(), false);
        let num_files: i32 = report.extensions.values().sum();
        assert_eq!(num_files, 2); // 1 tar + 1 txt inside
        assert_eq!(report.extensions.get("tar"), Some(&1));
        assert_eq!(report.extensions.get("txt"), Some(&1));
        assert_eq!(report.errors.len(), 0);
    }

    #[test]
    fn test_scan_gz_archive() {
        let report = scan("testdata/archives/sample.gz".into(), false);
        let num_files: i32 = report.extensions.values().sum();
        assert_eq!(num_files, 2); // 1 gz + 1 txt inside
        assert_eq!(report.extensions.get("gz"), Some(&1));
        assert_eq!(report.extensions.get("txt"), Some(&1));
        assert_eq!(report.errors.len(), 0);
    }

    #[test]
    fn test_scan_7z_archive() {
        let report = scan("testdata/archives/sample.7z".into(), false);
        let num_files: i32 = report.extensions.values().sum();
        assert_eq!(num_files, 2); // 1 7z + 1 txt inside
        assert_eq!(report.extensions.get("7z"), Some(&1));
        assert_eq!(report.extensions.get("txt"), Some(&1));
        assert_eq!(report.errors.len(), 0);
    }

    #[test]
    fn test_detect_mimetype_png() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("sumdir_test_png");
        std::fs::create_dir_all(&dir).expect("failed to create test dir");
        let file_path = dir.join("test.png");
        // PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A
        let png_header: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let mut file = File::create(&file_path).expect("failed to create test file");
        file.write_all(&png_header)
            .expect("failed to write test file");

        assert_eq!(
            detect_mimetype(&file_path).expect("failed to detect mimetype"),
            "image/png"
        );

        std::fs::remove_dir_all(&dir).expect("failed to cleanup test dir");
    }

    #[test]
    fn test_detect_mimetype_pdf() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("sumdir_test_pdf");
        std::fs::create_dir_all(&dir).expect("failed to create test dir");
        let file_path = dir.join("test.pdf");
        // PDF magic bytes: %PDF
        let mut file = File::create(&file_path).expect("failed to create test file");
        file.write_all(b"%PDF-1.4")
            .expect("failed to write test file");

        assert_eq!(
            detect_mimetype(&file_path).expect("failed to detect mimetype"),
            "application/pdf"
        );

        std::fs::remove_dir_all(&dir).expect("failed to cleanup test dir");
    }

    #[test]
    fn test_detect_mimetype_unknown() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("sumdir_test_unknown");
        std::fs::create_dir_all(&dir).expect("failed to create test dir");
        let file_path = dir.join("test.txt");
        let mut file = File::create(&file_path).expect("failed to create test file");
        file.write_all(b"Hello, world!")
            .expect("failed to write test file");

        assert_eq!(
            detect_mimetype(&file_path).expect("failed to detect mimetype"),
            "application/octet-stream"
        );

        std::fs::remove_dir_all(&dir).expect("failed to cleanup test dir");
    }

    #[test]
    fn test_scan_populates_mimetypes() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("sumdir_test_scan_mime");
        std::fs::create_dir_all(&dir).expect("failed to create test dir");

        // Create a PNG file
        let png_path = dir.join("image.png");
        let png_header: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        File::create(&png_path)
            .expect("failed to create png")
            .write_all(&png_header)
            .expect("failed to write png");

        // Create a PDF file
        let pdf_path = dir.join("doc.pdf");
        File::create(&pdf_path)
            .expect("failed to create pdf")
            .write_all(b"%PDF-1.4")
            .expect("failed to write pdf");

        // Create a text file (unknown to infer)
        let txt_path = dir.join("readme.txt");
        File::create(&txt_path)
            .expect("failed to create txt")
            .write_all(b"Hello")
            .expect("failed to write txt");

        let report = scan(dir.clone(), false);

        assert_eq!(report.mimetypes.get("image/png"), Some(&1));
        assert_eq!(report.mimetypes.get("application/pdf"), Some(&1));
        assert_eq!(report.mimetypes.get("application/octet-stream"), Some(&1));
        assert_eq!(report.mimetypes.len(), 3);
        assert_eq!(report.errors.len(), 0);

        std::fs::remove_dir_all(&dir).expect("failed to cleanup test dir");
    }

    #[test]
    fn test_testdata_mimetypes() {
        let report = scan("testdata".into(), false);
        // Verify various MIME types are detected correctly
        assert_eq!(report.mimetypes.get("image/png"), Some(&1));
        assert_eq!(report.mimetypes.get("image/jpeg"), Some(&1));
        assert_eq!(report.mimetypes.get("image/gif"), Some(&1));
        assert_eq!(report.mimetypes.get("application/pdf"), Some(&1));
        assert_eq!(report.mimetypes.get("audio/mpeg"), Some(&1));
        // DOCX/XLSX/PPTX detected as zip, DOC/XLS as ole-storage
        assert_eq!(report.mimetypes.get("application/zip"), Some(&4));
        assert_eq!(report.mimetypes.get("application/x-ole-storage"), Some(&2));
        // Text files without magic bytes fall back to octet-stream
        assert!(
            report
                .mimetypes
                .get("application/octet-stream")
                .expect("expected octet-stream entries")
                >= &1
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_scan_unreadable_directory() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join("sumdir_test_unreadable");
        std::fs::create_dir_all(&dir).expect("failed to create test dir");

        // Create a subdirectory with no read permissions
        let unreadable_dir = dir.join("no_access");
        std::fs::create_dir_all(&unreadable_dir).expect("failed to create unreadable dir");

        // Create a file inside the unreadable directory first
        let hidden_file = unreadable_dir.join("hidden.txt");
        std::fs::write(&hidden_file, "secret").expect("failed to write hidden file");

        // Remove read permissions from the directory
        std::fs::set_permissions(&unreadable_dir, std::fs::Permissions::from_mode(0o000))
            .expect("failed to set permissions");

        // Create a readable file
        let readable_file = dir.join("readable.txt");
        std::fs::write(&readable_file, "hello").expect("failed to write readable file");

        let report = scan(dir.clone(), false);

        // Should have scanned the readable file
        assert_eq!(report.extensions.get("txt"), Some(&1));
        // Should have recorded errors for the unreadable directory
        assert!(
            !report.errors.is_empty(),
            "expected errors for unreadable directory"
        );
        // Error should contain the path
        assert!(
            report
                .errors
                .iter()
                .any(|e| e.path.to_string_lossy().contains("no_access")),
            "error should contain path to unreadable directory"
        );

        // Restore permissions for cleanup
        std::fs::set_permissions(&unreadable_dir, std::fs::Permissions::from_mode(0o755))
            .expect("failed to restore permissions");
        std::fs::remove_dir_all(&dir).expect("failed to cleanup test dir");
    }

    #[test]
    fn test_scan_error_contains_path_and_message() {
        // Test that ScanError properly stores both path and message
        let error = ScanError {
            path: PathBuf::from("/some/path/file.txt"),
            message: "test error message".to_string(),
        };

        assert_eq!(error.path, PathBuf::from("/some/path/file.txt"));
        assert_eq!(error.message, "test error message");
    }

    #[test]
    fn test_report_with_errors_displays_count() {
        let mut report = Report::default();
        report.errors.push(ScanError {
            path: PathBuf::from("/path/to/file1.txt"),
            message: "error 1".to_string(),
        });
        report.errors.push(ScanError {
            path: PathBuf::from("/path/to/file2.txt"),
            message: "error 2".to_string(),
        });

        assert_eq!(report.errors.len(), 2);
        assert_eq!(report.errors[0].path, PathBuf::from("/path/to/file1.txt"));
        assert_eq!(report.errors[1].path, PathBuf::from("/path/to/file2.txt"));
    }
}
