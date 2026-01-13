use std::fs::File;
use std::io::Read;
use std::{collections::BTreeMap, os::unix::fs::MetadataExt, path::PathBuf};
use walkdir::WalkDir;

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

#[derive(Debug, Default)]
struct Report {
    extensions: BTreeMap<String, i32>,
    mimetypes: BTreeMap<String, i32>,
    folders: Vec<PathBuf>,
    size: u64,
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
        println!("{num_files} files, {} folders, {size}", self.folders.len());
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
        println!("{{");
        println!("  \"files\": {num_files},");
        println!("  \"folders\": {},", self.folders.len());
        println!("  \"size\": {},", self.size);
        println!("  \"{key_name}\": {{");
        println!("{}", entries.join(",\n"));
        println!("  }}");
        println!("}}");
    }
}

fn detect_mimetype(path: &std::path::Path) -> String {
    let mut buffer = [0u8; 8192];
    if let Ok(mut file) = File::open(path) {
        let bytes_read = file.read(&mut buffer).unwrap_or(0);
        if let Some(kind) = infer::get(&buffer[..bytes_read]) {
            return kind.mime_type().to_string();
        }
    }
    "application/octet-stream".to_string()
}

fn scan(target: PathBuf) -> Report {
    let mut report = Report::default();

    for entry in WalkDir::new(target).into_iter().skip(1) {
        let entry = entry.unwrap();
        if entry.path().is_dir() {
            report.folders.push(entry.path().to_path_buf());
        } else {
            let ext = entry
                .path()
                .extension()
                .unwrap_or_default()
                .to_owned()
                .into_string()
                .unwrap_or_default();
            report.size += entry.path().metadata().unwrap().size();
            report
                .extensions
                .entry(ext)
                .and_modify(|e| *e += 1)
                .or_insert(1);

            let mimetype = detect_mimetype(entry.path());
            report
                .mimetypes
                .entry(mimetype)
                .and_modify(|e| *e += 1)
                .or_insert(1);
        }
    }

    report
}

fn main() {
    let cli = Cli::parse();
    if !cli.target.exists() {
        println!("{} does not exist", cli.target.to_str().unwrap_or_default());
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
        let expected = BTreeMap::from([
            ("csv".to_string(), 1),
            ("html".to_string(), 1),
            ("png".to_string(), 1),
            ("txt".to_string(), 1),
        ]);
        assert_eq!(report.extensions, expected);
        assert_eq!(report.folders.len(), 1);
        assert_eq!(report.size, 97);
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
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.png");
        // PNG magic bytes: 89 50 4E 47 0D 0A 1A 0A
        let png_header: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let mut file = File::create(&file_path).unwrap();
        file.write_all(&png_header).unwrap();

        assert_eq!(detect_mimetype(&file_path), "image/png");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_detect_mimetype_pdf() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("sumdir_test_pdf");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.pdf");
        // PDF magic bytes: %PDF
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"%PDF-1.4").unwrap();

        assert_eq!(detect_mimetype(&file_path), "application/pdf");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_detect_mimetype_unknown() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("sumdir_test_unknown");
        std::fs::create_dir_all(&dir).unwrap();
        let file_path = dir.join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"Hello, world!").unwrap();

        assert_eq!(detect_mimetype(&file_path), "application/octet-stream");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_scan_populates_mimetypes() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("sumdir_test_scan_mime");
        std::fs::create_dir_all(&dir).unwrap();

        // Create a PNG file
        let png_path = dir.join("image.png");
        let png_header: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        File::create(&png_path).unwrap().write_all(&png_header).unwrap();

        // Create a PDF file
        let pdf_path = dir.join("doc.pdf");
        File::create(&pdf_path).unwrap().write_all(b"%PDF-1.4").unwrap();

        // Create a text file (unknown to infer)
        let txt_path = dir.join("readme.txt");
        File::create(&txt_path).unwrap().write_all(b"Hello").unwrap();

        let report = scan(dir.clone());

        assert_eq!(report.mimetypes.get("image/png"), Some(&1));
        assert_eq!(report.mimetypes.get("application/pdf"), Some(&1));
        assert_eq!(report.mimetypes.get("application/octet-stream"), Some(&1));
        assert_eq!(report.mimetypes.len(), 3);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_testdata_mimetypes() {
        let report = scan("testdata".into());
        // All testdata files are text-based, so infer returns octet-stream
        assert_eq!(report.mimetypes.get("application/octet-stream"), Some(&4));
    }
}
