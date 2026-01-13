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
}

#[derive(Debug, Default)]
struct Report {
    extensions: BTreeMap<String, i32>,
    folders: Vec<PathBuf>,
    size: u64,
}

impl Report {
    fn display(&self, format: &OutputFormat) {
        match format {
            OutputFormat::Text => self.display_text(),
            OutputFormat::Csv => self.display_csv(),
            OutputFormat::Json => self.display_json(),
        }
    }

    fn display_text(&self) {
        let num_files: i32 = self.extensions.values().sum();
        let size = friendly_bytes(self.size);
        println!("{num_files} files, {} folders, {size}", self.folders.len());
        for (ext, count) in self.extensions.iter().sorted_by(|a, b| b.1.cmp(a.1)) {
            println!("{ext}: {count}");
        }
    }

    fn display_csv(&self) {
        println!("extension,count");
        for (ext, count) in self.extensions.iter().sorted_by(|a, b| b.1.cmp(a.1)) {
            println!("{ext},{count}");
        }
    }

    fn display_json(&self) {
        let num_files: i32 = self.extensions.values().sum();
        let extensions: Vec<String> = self
            .extensions
            .iter()
            .sorted_by(|a, b| b.1.cmp(a.1))
            .map(|(ext, count)| format!("    \"{ext}\": {count}"))
            .collect();
        println!("{{");
        println!("  \"files\": {num_files},");
        println!("  \"folders\": {},", self.folders.len());
        println!("  \"size\": {},", self.size);
        println!("  \"extensions\": {{");
        println!("{}", extensions.join(",\n"));
        println!("  }}");
        println!("}}");
    }
}

fn scan(target: PathBuf) -> Report {
    let mut report = Report::default();

    for entry in WalkDir::new(target).into_iter().skip(1) {
        let entry = entry.unwrap();
        if entry.path().is_dir() {
            report.folders.push(entry.path().to_path_buf());
        } else {
            let _path: String = entry
                .path()
                .as_os_str()
                .to_owned()
                .into_string()
                .unwrap_or_default();
            let _name = entry
                .file_name()
                .to_owned()
                .into_string()
                .unwrap_or_default();
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
    report.display(&cli.output);
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
}
