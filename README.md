# sumdir

summarize + dir = sumdir

Scans a folder and outputs statistics in terms of file types and counts.

## Installation

```
$ brew install stchris/formulae/sumdir
```

or 

```
$ curl --proto '=https' --tlsv1.2 -LsSf https://github.com/stchris/sumdir/releases/download/v0.4.0/sumdir-installer.sh | sh
```

or check the [Releases](https://github.com/stchris/sumdir/releases) page.

## Usage

<!-- help-output-start -->
```
summarize a directory by file type frequency

Usage: sumdir [OPTIONS] <TARGET>

Arguments:
  <TARGET>  

Options:
  -v, --verbose          
  -o, --output <OUTPUT>  [default: text] [possible values: text, csv, json, ftm]
  -m, --mime             
  -p, --progress-bar     
  -h, --help             Print help
  -V, --version          Print version
```
<!-- help-output-end -->

## Example output

<!-- example-output-start -->
```
$ sumdir -m testdata/

31 files, 5 folders, 4.39 KiB
application/octet-stream: 9
application/zip: 4
application/x-ole-storage: 2
application/gzip: 1
application/pdf: 1
application/x-7z-compressed: 1
application/x-tar: 1
audio/mpeg: 1
audio/ogg: 1
audio/x-flac: 1
audio/x-wav: 1
image/bmp: 1
image/gif: 1
image/jpeg: 1
image/png: 1
image/tiff: 1
image/vnd.microsoft.icon: 1
image/webp: 1
text/xml: 1
```
<!-- example-output-end -->

## Releasing

This project uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) for releases and [git-cliff](https://git-cliff.org/) for changelog generation.

### Prerequisites

```bash
cargo install cargo-release git-cliff
```

### Making a Release

```bash
cargo release --no-publish {patch,minor,major}
   ```