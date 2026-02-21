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
