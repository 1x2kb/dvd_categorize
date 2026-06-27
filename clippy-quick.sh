#!/bin/bash
# Quick clippy fix covering main feature combinations
# Faster than clippy-all.sh but still covers critical code paths
# Automatically commits any fixes applied by clippy

set -e

echo "Running quick clippy --fix on main feature combinations..."
echo ""

# Fix workspace with postgres backend (most common)
echo "=== Fixing workspace with postgres backend ==="
cargo clippy --workspace --no-default-features \
  --features database/postgres,dvd_catalog_api/postgres,csv_utils/postgres,ai_chat/internet \
  --all-targets --fix
echo ""

# Fix workspace with mongodb backend
echo "=== Fixing workspace with mongodb backend ==="
cargo clippy --workspace --no-default-features \
  --features database/mongodb,ai_chat/internet \
  --all-targets --fix
echo ""

# Fix dvd_categorizer_web separately (different target)
echo "=== Fixing dvd_categorizer_web ==="
cargo clippy -p dvd_categorizer_web --features web --all-targets --fix
echo ""

echo "✅ Quick clippy fixes applied!"
echo ""

# Check if clippy made any changes
if ! git diff --quiet; then
    echo "📝 Clippy made changes. Committing..."
    git add -u
    git commit -m "Apply clippy fixes

Automatic fixes applied by clippy across feature combinations:
- postgres backend
- mongodb backend
- web frontend"
    echo "✅ Changes committed!"
else
    echo "✨ No changes needed - code is already clean!"
fi
