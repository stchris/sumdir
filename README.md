# sumdir

summarize + dir = sumdir

Scans a folder and outputs statistics in terms of file types and counts.

## Usage

```bash
sumdir <directory>              # group by file extension
sumdir <directory> -m           # group by MIME type (magic byte detection)
sumdir <directory> -o csv       # output as CSV
sumdir <directory> -o json      # output as JSON
sumdir <directory> -m -o json   # combine flags
```

## Example output

```bash
$ sumdir -m testdata/

27 files, 5 folders, 1 KiB
application/octet-stream: 5
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

## Installation

```bash
cargo install sumdir
```

## Releasing

This project uses [cargo-dist](https://opensource.axo.dev/cargo-dist/) for releases and [git-cliff](https://git-cliff.org/) for changelog generation.

### Prerequisites

```bash
cargo install cargo-release git-cliff
```

### Making a Release

1. Ensure the changelog is up to date (`$ git cliff -o CHANGELOG.md`)
2. Ensure all changes are committed
3. Run cargo-release with the desired version bump:
   ```bash
   cargo release --no-publish patch  # 0.1.0 -> 0.1.1
   cargo release --no-publish minor  # 0.1.0 -> 0.2.0
   cargo release --no-publish major  # 0.1.0 -> 1.0.0
   ```

This will automatically:
- Update version in `Cargo.toml`
- Generate/update `CHANGELOG.md` via git-cliff (pre-release hook)
- Create a git commit and tag
- Push to trigger the release workflow

### Changelog Generation

The changelog is automatically generated from commit history using git-cliff. Commits are grouped by type based on conventional commit prefixes:

| Prefix | Changelog Section |
|--------|-------------------|
| `feat` | Added |
| `fix` | Fixed |
| `refactor` | Changed |
| `doc` | Documentation |
| `perf` | Performance |
| `test` | Testing |

To manually regenerate the changelog:
```bash
git cliff -o CHANGELOG.md
```

To preview what a release would look like:
```bash
git cliff --tag v1.0.0
```
