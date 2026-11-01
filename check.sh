#!/bin/bash
# check.sh <crate> <mongodb|postgres|none>
set -e
crate="$1"; backend="$2"

case "$crate:$backend" in
  database:mongodb)      feats="database/mongodb" ;;
  database:postgres)     feats="database/postgres" ;;
  dvd_catalog_api:mongodb)  feats="dvd_catalog_api/mongodb,dvd_catalog_api/ai" ;;
  dvd_catalog_api:postgres) feats="dvd_catalog_api/postgres,dvd_catalog_api/ai" ;;
  csv_utils:mongodb)     feats="csv_utils/vector-similarity" ;;
  csv_utils:postgres)    feats="csv_utils/postgres" ;;
  ai_chat:*)             feats="ai_chat/internet" ;;
  dvd_categorizer_web:*) feats="dvd_categorizer_web/web,dvd_categorizer_web/ai-backend" ;;
  models:*)              feats="" ;;
  *) echo "usage: check.sh <crate> <mongodb|postgres|none>" >&2; exit 2 ;;
esac

if [ -n "$feats" ]; then
  cargo check -p "$crate" --no-default-features --features "$feats" --message-format=short
else
  cargo check -p "$crate" --message-format=short
fi
