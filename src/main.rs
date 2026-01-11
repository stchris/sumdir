use std::{collections::BTreeMap, path::PathBuf};
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
    for (ext, count) in report
        .extensions
        .iter()
        .sorted_by(|a, b| Ord::cmp(&b.1, &a.1))
    {
        println!("{ext}: {count}")
    }
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
    }
}
