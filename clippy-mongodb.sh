#!/bin/bash
# Clippy fix for MongoDB backend
# Automatically commits any fixes applied by clippy

set -e

echo "Running clippy --fix for MongoDB backend..."
echo ""

cargo clippy --workspace --no-default-features \
  --features database/mongodb,dvd_catalog_api/mongodb,dvd_catalog_api/ai,ai_chat/internet,dvd_categorizer_web/web,dvd_categorizer_web/ai-backend,csv_utils/vector-similarity \
  --fix

echo ""
echo "✅ Clippy fixes applied!"
echo ""

# Check if clippy made any changes
if ! git diff --quiet; then
    echo "📝 Clippy made changes. Committing..."
    git add -u
    git commit -m "Apply clippy fixes (MongoDB backend)"
    echo "✅ Changes committed!"
else
    echo "✨ No changes needed - code is already clean!"
fi
