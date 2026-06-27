#!/bin/bash
# Comprehensive clippy fixes to cover all feature-gated code paths
# Run this script to automatically fix clippy warnings across all code
# Automatically commits any fixes applied by clippy

set -e

echo "Running clippy --fix across all feature combinations..."
echo ""

# Check for uncommitted changes before starting
if ! git diff-index --quiet HEAD --; then
    echo "⚠️  Warning: You have uncommitted changes."
    echo "Clippy fixes will be committed separately from your existing changes."
    echo ""
fi

# Base clippy for crates without features
echo "=== Fixing crates without features ==="
cargo clippy -p categorizer_utilities --all-targets --fix
cargo clippy -p actor_reorder --all-targets --fix
cargo clippy -p dioxus_grapher --all-targets --fix
cargo clippy -p ai_tools --all-targets --fix
echo ""

# Models crate - fix all feature combinations
echo "=== Fixing models crate ==="
cargo clippy -p models --no-default-features --all-targets --fix
cargo clippy -p models --no-default-features --features postgres --all-targets --fix
cargo clippy -p models --no-default-features --features postgres-types --all-targets --fix
cargo clippy -p models --no-default-features --features mongodb --all-targets --fix
cargo clippy -p models --no-default-features --features mongodb-types --all-targets --fix
cargo clippy -p models --no-default-features --features ai --all-targets --fix
cargo clippy -p models --no-default-features --features vector-similarity --all-targets --fix
cargo clippy -p models --no-default-features --features text-matching --all-targets --fix
cargo clippy -p models --no-default-features --features testing --all-targets --fix
# Combined features
cargo clippy -p models --no-default-features --features postgres,ai,testing --all-targets --fix
cargo clippy -p models --no-default-features --features mongodb,ai,testing --all-targets --fix
echo ""

# Database crate - postgres and mongodb are mutually exclusive
echo "=== Fixing database crate ==="
cargo clippy -p database --no-default-features --all-targets --fix
cargo clippy -p database --no-default-features --features postgres --all-targets --fix
cargo clippy -p database --no-default-features --features mongodb --all-targets --fix
cargo clippy -p database --no-default-features --features postgres,ai,testing --all-targets --fix
cargo clippy -p database --no-default-features --features mongodb,ai,testing --all-targets --fix
echo ""

# AI chat crate
echo "=== Fixing ai_chat crate ==="
cargo clippy -p ai_chat --no-default-features --all-targets --fix
cargo clippy -p ai_chat --no-default-features --features internet --all-targets --fix
echo ""

# CSV utils crate
echo "=== Fixing csv_utils crate ==="
cargo clippy -p csv_utils --no-default-features --all-targets --fix
cargo clippy -p csv_utils --no-default-features --features postgres --all-targets --fix
cargo clippy -p csv_utils --no-default-features --features vector-similarity --all-targets --fix
cargo clippy -p csv_utils --no-default-features --features postgres,vector-similarity --all-targets --fix
echo ""

# Prompts crate
echo "=== Fixing prompts crate ==="
cargo clippy -p prompts --no-default-features --all-targets --fix
cargo clippy -p prompts --no-default-features --features conversions --all-targets --fix
echo ""

# DVD catalog API - fix with different backends
echo "=== Fixing dvd_catalog_api crate ==="
cargo clippy -p dvd_catalog_api --no-default-features --all-targets --fix
cargo clippy -p dvd_catalog_api --no-default-features --features postgres --all-targets --fix
cargo clippy -p dvd_catalog_api --no-default-features --features internet --all-targets --fix
cargo clippy -p dvd_catalog_api --no-default-features --features postgres,internet --all-targets --fix
echo ""

# DVD categorizer web
echo "=== Fixing dvd_categorizer_web crate ==="
cargo clippy -p dvd_categorizer_web --no-default-features --all-targets --fix
cargo clippy -p dvd_categorizer_web --no-default-features --features web --all-targets --fix
cargo clippy -p dvd_categorizer_web --no-default-features --features desktop --all-targets --fix
echo ""

echo "✅ All clippy fixes applied!"
echo ""

# Check if clippy made any changes
if ! git diff --quiet; then
    echo "📝 Clippy made changes. Committing..."
    git add -u
    git commit -m "Apply comprehensive clippy fixes

Automatic fixes applied by clippy across all feature combinations:
- All crates without features
- models: all feature permutations
- database: postgres and mongodb backends
- ai_chat: with and without internet
- csv_utils: all feature combinations
- prompts: with and without conversions
- dvd_catalog_api: all feature combinations
- dvd_categorizer_web: web and desktop targets"
    echo "✅ Changes committed!"
else
    echo "✨ No changes needed - code is already clean!"
fi
