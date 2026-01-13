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
