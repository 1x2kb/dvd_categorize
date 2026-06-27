#!/bin/bash
# Clippy fix for PostgreSQL backend
# Automatically commits any fixes applied by clippy

set -e

echo "Running clippy --fix for PostgreSQL backend..."
echo ""

cargo clippy --workspace --no-default-features \
  --features database/postgres,dvd_catalog_api/postgres,csv_utils/postgres,ai_chat/internet,dvd_categorizer_web/web \
  --all-targets --fix

echo ""
echo "✅ Clippy fixes applied!"
echo ""

# Check if clippy made any changes
if ! git diff --quiet; then
    echo "📝 Clippy made changes. Committing..."
    git add -u
    git commit -m "Apply clippy fixes (PostgreSQL backend)"
    echo "✅ Changes committed!"
else
    echo "✨ No changes needed - code is already clean!"
fi
