use std::{collections::BTreeMap, os::unix::fs::MetadataExt, path::PathBuf};
use walkdir::WalkDir;

use clap::Parser;
use itertools::Itertools;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(required = true)]
    target: PathBuf,

    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Debug, Default)]
struct Report {
    extensions: BTreeMap<String, i32>,
    folders: Vec<PathBuf>,
    size: u64,
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
    let num_files: i32 = report.extensions.values().sum();
    let num_folders = report.folders.len();
    let size = friendly_bytes(report.size);
    println!("{num_files} files, {num_folders} folders, {size}");
    for (ext, count) in report
        .extensions
        .iter()
        .sorted_by(|a, b| Ord::cmp(&b.1, &a.1))
    {
        println!("{ext}: {count}")
    }
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
        assert_eq!(friendly_bytes(1234), "1 KB".to_string());
        assert_eq!(friendly_bytes(1234567), "1 MB".to_string());
        assert_eq!(friendly_bytes(1234567890), "1 GB".to_string());
        assert_eq!(friendly_bytes(1234567890123), "1 TB".to_string());
    }
}
