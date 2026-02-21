#!/usr/bin/env bash
set -euo pipefail

# Downloads a Lucide release and populates icons/
# Usage: ./scripts/update-icons.sh 0.475.0

VERSION="${1:?Usage: $0 <lucide-version>}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
ICONS_DIR="$REPO_ROOT/icons"
TARBALL_URL="https://github.com/lucide-icons/lucide/archive/refs/tags/${VERSION}.tar.gz"

echo "Downloading Lucide v${VERSION}..."
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

curl -fSL "$TARBALL_URL" -o "$TMPDIR/lucide.tar.gz"

echo "Extracting icons..."
tar -xzf "$TMPDIR/lucide.tar.gz" -C "$TMPDIR"

# Clean existing icons and copy new ones (only SVGs, not .json metadata)
rm -rf "$ICONS_DIR"
mkdir -p "$ICONS_DIR"
find "$TMPDIR/lucide-${VERSION}/icons" -maxdepth 1 -name '*.svg' -exec cp {} "$ICONS_DIR/" \;

# Write version file (no trailing newline)
printf '%s' "$VERSION" > "$ICONS_DIR/VERSION"

COUNT=$(find "$ICONS_DIR" -name '*.svg' | wc -l | tr -d ' ')
echo "Done: ${COUNT} icons vendored to icons/ (Lucide v${VERSION})"

# Generate ICONS.md from .json metadata (json files stay in tmpdir, not committed)
echo "Generating ICONS.md..."
METADATA_DIR="$TMPDIR/lucide-${VERSION}/icons"
METADATA_DIR="$METADATA_DIR" VERSION="$VERSION" python3 -c "
import json, os, pathlib

metadata_dir = os.environ['METADATA_DIR']
version = os.environ['VERSION']

entries = []
for f in sorted(pathlib.Path(metadata_dir).glob('*.json')):
    with open(f) as fh:
        meta = json.load(fh)
    name = f.stem
    categories = ', '.join(meta.get('categories', []))
    tags = ', '.join(meta.get('tags', []))
    entries.append(f'| {name} | {categories} | {tags} |')

lines = [
    f'# Lucide Icons v{version}',
    '',
    f'{len(entries)} icons available. Use these names in your \`lucide-icons.toml\` manifest.',
    '',
    '| Icon | Categories | Tags |',
    '|------|------------|------|',
    *entries,
    '',
]
print('\n'.join(lines))
" > "$REPO_ROOT/ICONS.md"

echo "Generated ICONS.md with ${COUNT} icons"
