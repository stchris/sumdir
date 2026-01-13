# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build and Test Commands

- **Build**: `cargo build`
- **Run**: `cargo run -- <directory>`
- **Test all**: `cargo test`
- **Test single**: `cargo test <test_name>` (e.g., `cargo test test_friendly_bytes`)
- **Release build**: `cargo build --profile dist`

## Project Overview

sumdir is a CLI tool that scans a directory and outputs statistics about file types, counts, and total size. It uses:
- `clap` for CLI argument parsing (derive feature)
- `walkdir` for recursive directory traversal
- `itertools` for sorting results

## Architecture

Single-file application (`src/main.rs`) with:
- `Cli` struct: clap-derived argument parser with `target` (required path) and `verbose` flag
- `Report` struct: accumulates scan results (extension counts via BTreeMap, folder paths, total size)
- `scan()`: walks directory tree, counts files by extension, tracks folders and total bytes
- `friendly_bytes()`: converts byte counts to human-readable format (KiB, MiB, GiB, TiB)

Test data lives in `testdata/` folder with sample files of various types.

## Planned Feature: MIME Type Detection

Use the `infer` crate to detect file types by magic bytes instead of relying solely on file extensions.

### Design

**New CLI flag**: `-m, --mime` to group files by MIME type instead of extension

**Changes to Report struct**:
```rust
struct Report {
    extensions: BTreeMap<String, i32>,
    mimetypes: BTreeMap<String, i32>,  // NEW: tracks MIME type counts
    folders: Vec<PathBuf>,
    size: u64,
}
```

**Changes to scan()**:
- Read first 8KB of each file (enough for magic byte detection)
- Call `infer::get()` to detect MIME type
- Fall back to "application/octet-stream" for unknown types
- Store results in `mimetypes` map

**New display methods**:
- `display_text_mime()`, `display_csv_mime()`, `display_json_mime()`
- Or add a `group_by: GroupBy` enum to existing display methods

**Dependency**:
```toml
infer = "0.16"
```

**Example usage**:
```bash
sumdir ./mydir -m           # group by MIME type
sumdir ./mydir -m -o json   # MIME types as JSON
```

**Example output**:
```
15 files, 3 folders, 1.2 MiB
image/png: 5
text/plain: 4
application/pdf: 3
application/octet-stream: 3
```
