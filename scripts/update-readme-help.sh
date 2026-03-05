#!/usr/bin/env bash
set -euo pipefail

cargo build --release

HELP=$(./target/release/sumdir --help 2>&1)
EXAMPLE=$(./target/release/sumdir -m testdata 2>/dev/null)

python3 - "$HELP" "$EXAMPLE" <<'EOF'
import sys
import re

help_text = sys.argv[1]
example_text = sys.argv[2]

def replace_section(content, marker_start, marker_end, new_body):
    new_block = f"{marker_start}\n{new_body}\n{marker_end}"
    return re.sub(
        re.escape(marker_start) + r'.*?' + re.escape(marker_end),
        new_block,
        content,
        flags=re.DOTALL,
    )

with open('README.md', 'r') as f:
    content = f.read()

content = replace_section(
    content,
    '<!-- help-output-start -->',
    '<!-- help-output-end -->',
    f'```\n{help_text}\n```',
)

content = replace_section(
    content,
    '<!-- example-output-start -->',
    '<!-- example-output-end -->',
    f'```\n$ sumdir -m testdata/\n\n{example_text}\n```',
)

with open('README.md', 'w') as f:
    f.write(content)

print('README.md updated')
EOF

git add README.md

git cliff -o CHANGELOG.md
git add CHANGELOG.md
