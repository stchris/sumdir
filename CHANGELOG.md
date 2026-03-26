# Changelog

All notable changes to this project will be documented in this file.
## [Unreleased]

### New

- Add prek config
- Cargo updates, including ftm-types 0.4.0

## [0.4.1] - 2026-03-06

### New

- Update installation section
- Release automation
- Use to_ftm_json() for proper JSON structure with nested props
- Simpler git cliff config without conventional commits
- Chore: Release sumdir version 0.4.1

## [0.4.0] - 2026-03-05

### New

- Feat: Support for walking archives (tar, gzip, zip, 7zip)
- Merge pull request #14 from stchris/archives
- New export format: ftm stub entities
- Use ftm-export 0.1.0
- Merge pull request #15 from stchris/ftm-export
- License
- Chore: Release sumdir version 0.4.0

## [0.3.0] - 2026-03-05

### New

- Feat: Use jwalk with rayon for double speed
- Merge pull request #13 from stchris/jwalk
- Chore: Release sumdir version 0.3.0

## [0.2.0] - 2026-01-14

### New

- Feat: Progress bar
- Merge pull request #12 from stchris/progress
- Delete .github/workflows/claude-code-review.yml
- Chore: Release sumdir version 0.2.0

## [0.1.2] - 2026-01-13

### New

- Fix: use cross-platform metadata.len() instead of Unix-specific size()
- Merge pull request #11 from stchris/claude/issue-10-20260113-1629
- Chore: Release sumdir version 0.1.2

## [0.1.1] - 2026-01-13

### New

- Doc: added example usage
- Chore: remove unsupported target platforms
- Chore: Release sumdir version 0.1.1

## [0.1.0] - 2026-01-13

### New

- Initial commit
- Add verbose flag
- Refactor into report and add test
- Count and show num files, folders and size
- Show a human friendly size
- Add cargo dist
- Add selectable output formats (text, csv, json)
- Add CLAUDE.md
- Ignore claude
- Add MIME type detection using infer crate
- Expand testdata with various file formats
- Add changelog generation with git-cliff
- Use proper error handling with context everywhere and disallow unwrap
- "Claude PR Assistant workflow"
- "Claude Code Review workflow"
- Merge pull request #6 from stchris/add-claude-github-actions-1768307145556
- Update Claude.md
- "Update Claude PR Assistant workflow"
- "Update Claude Code Review workflow"
- Merge pull request #7 from stchris/add-claude-github-actions-1768313499371
- Chore: add action for testing and linting PRs
- Chore: add cargo fmt --check
- Merge pull request #8 from stchris/test-lint-action
- Chore: minor cargo updates, fixed to minor versions
- Merge pull request #9 from stchris/cargo-updates
- Chore: updated changelog
- Chore: updated release steps
- Chore: release 0.1.0


