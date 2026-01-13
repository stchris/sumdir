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
