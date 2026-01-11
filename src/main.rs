use std::{collections::BTreeMap, path::PathBuf};
use walkdir::WalkDir;

use clap::Parser;
use itertools::Itertools;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(required = true)]
    target: PathBuf,
}

fn main() {
    let cli = Cli::parse();
    let mut extensions = BTreeMap::new();

    for entry in WalkDir::new(cli.target).into_iter().skip(1) {
        let entry = entry.unwrap();
        let _parent = entry.path().parent();
        let path: String = entry
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
        extensions.entry(ext).and_modify(|e| *e += 1).or_insert(1);
        println!("{path}");
    }
    for (ext, count) in extensions.iter().sorted_by(|a, b| Ord::cmp(&b.1, &a.1)) {
        println!("{ext}: {count}")
    }
}
