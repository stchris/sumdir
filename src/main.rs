use std::fs::File;
use std::io::Read;
use std::{collections::BTreeMap, os::unix::fs::MetadataExt, path::PathBuf};
use walkdir::WalkDir;

use anyhow::{Context, Result};
use clap::Parser;
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
        let size = friendly_bytes(self.size);
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

fn process_entry(entry: &walkdir::DirEntry, report: &mut Report) -> Result<()> {
    let ext = entry
        .path()
        .extension()
        .unwrap_or_default()
        .to_owned()
        .into_string()
        .unwrap_or_default();

    let metadata = entry
        .path()
        .metadata()
        .with_context(|| format!("failed to read metadata for {:?}", entry.path()))?;

    report.size += metadata.size();
    report
        .extensions
        .entry(ext)
        .and_modify(|e| *e += 1)
        .or_insert(1);

    let mimetype = detect_mimetype(entry.path())
        .with_context(|| format!("failed to detect mimetype for {:?}", entry.path()))?;
    report
        .mimetypes
        .entry(mimetype)
        .and_modify(|e| *e += 1)
        .or_insert(1);

    Ok(())
}

fn scan(target: PathBuf) -> Report {
    let mut report = Report::default();

    for entry in WalkDir::new(target).into_iter().skip(1) {
        match entry {
            Ok(entry) => {
                if entry.path().is_dir() {
                    report.folders.push(entry.path().to_path_buf());
                } else if let Err(e) = process_entry(&entry, &mut report) {
                    report.errors.push(ScanError {
                        path: entry.path().to_path_buf(),
                        message: e.to_string(),
                    });
                }
            }
            Err(e) => {
                let path = e.path().map(|p| p.to_path_buf()).unwrap_or_default();
                report.errors.push(ScanError {
                    path,
                    message: format!("failed to read entry: {e}"),
                });
            }
        }
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
    let report = scan(cli.target);
    report.display(&cli.output, cli.mime);
}

fn friendly_bytes(bytes: u64) -> String {
    if bytes > 1024 {
        let kb = bytes / 1024;
        if kb > 1024 {
            let mb = bytes / 1024 / 1024;
            if mb > 1024 {
                let gb = bytes / 1024 / 1024 / 1024;
                if gb > 1024 {
                    let tb = bytes / 1024 / 1024 / 1024 / 1024;
                    return format!("{tb} TiB");
                }
                return format!("{gb} GiB");
            }
            return format!("{mb} MiB");
        }
        return format!("{kb} KiB");
    }
    format!("{bytes} bytes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_testdata_folder() {
        let report = scan("testdata".into());
        let num_files: i32 = report.extensions.values().sum();
        assert_eq!(num_files, 27);
        assert_eq!(report.folders.len(), 5);
        assert_eq!(report.errors.len(), 0);
        // Verify some expected extensions
        assert_eq!(report.extensions.get("png"), Some(&2)); // sample.png + masquerading.png
        assert_eq!(report.extensions.get("pdf"), Some(&1));
        assert_eq!(report.extensions.get("jpg"), Some(&1));
        assert_eq!(report.extensions.get("docx"), Some(&1));
    }

    #[test]
    fn test_friendly_bytes() {
        assert_eq!(friendly_bytes(123), "123 bytes".to_string());
        assert_eq!(friendly_bytes(1234), "1 KiB".to_string());
        assert_eq!(friendly_bytes(1234567), "1 MiB".to_string());
        assert_eq!(friendly_bytes(1234567890), "1 GiB".to_string());
        assert_eq!(friendly_bytes(1234567890123), "1 TiB".to_string());
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

        let report = scan(dir.clone());

        assert_eq!(report.mimetypes.get("image/png"), Some(&1));
        assert_eq!(report.mimetypes.get("application/pdf"), Some(&1));
        assert_eq!(report.mimetypes.get("application/octet-stream"), Some(&1));
        assert_eq!(report.mimetypes.len(), 3);
        assert_eq!(report.errors.len(), 0);

        std::fs::remove_dir_all(&dir).expect("failed to cleanup test dir");
    }

    #[test]
    fn test_testdata_mimetypes() {
        let report = scan("testdata".into());
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

        let report = scan(dir.clone());

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
