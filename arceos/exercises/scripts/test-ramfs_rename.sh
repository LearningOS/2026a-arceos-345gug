#!/usr/bin/env bash
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$HERE/../ramfs_rename/scripts/test-ramfs_rename.sh" "$@"
